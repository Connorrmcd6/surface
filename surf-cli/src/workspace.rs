//! Filesystem side of config: discover `surf.toml` by walking up from a starting
//! directory (like `git`/`ruff`, §9.1.5), then enumerate hub files via its globs.
//! This is the I/O layer that `surf-core`'s pure parsers feed into.

use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};
use surf_core::config::{parse_config, Config, CONFIG_FILE};
use surf_core::{doc_kind, parse_anchor, parse_hub, Anchor, DocKind, Hub, HubError, Lang};

pub struct Workspace {
    pub root: PathBuf,
    pub config: Config,
}

/// One hub file located, read, and parsed. `hub` carries the parse result per-hub so each
/// command must consciously decide what to do with a malformed hub (block, skip, warn)
/// rather than re-implementing — and diverging on — that choice. `kind` marks OKF reserved
/// files (`index.md`/`log.md`), which hold no claims and must never block the gate when they
/// lack frontmatter.
pub struct LoadedHub {
    pub rel: String,
    pub kind: DocKind,
    pub hub: Result<Hub, HubError>,
}

impl Workspace {
    pub fn discover(start: &Path) -> Result<Workspace> {
        for dir in start.ancestors() {
            let candidate = dir.join(CONFIG_FILE);
            if candidate.is_file() {
                let content = std::fs::read_to_string(&candidate)
                    .with_context(|| format!("reading {}", candidate.display()))?;
                let config = parse_config(&content)?;
                return Ok(Workspace {
                    root: dir.to_path_buf(),
                    config,
                });
            }
        }
        anyhow::bail!(
            "no {CONFIG_FILE} found in {} or any parent directory",
            start.display()
        )
    }

    pub fn hub_paths(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        // Flat hub globs, verbatim.
        for pattern in &self.config.hubs {
            self.glob_into(pattern, &mut out)?;
        }
        // OKF bundle roots: each is a directory tree, so match every `.md` beneath it. Reserved
        // files (index.md/log.md) are swept up here and classified/skipped downstream.
        for root in &self.config.bundles {
            let joined = PathBuf::from(root).join("**/*.md");
            let pattern = joined
                .to_str()
                .with_context(|| format!("bundle root is not valid UTF-8: {root}"))?;
            self.glob_into(pattern, &mut out)?;
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    fn glob_into(&self, pattern: &str, out: &mut Vec<PathBuf>) -> Result<()> {
        let joined = self.root.join(pattern);
        let pattern = joined
            .to_str()
            .with_context(|| format!("hub glob is not valid UTF-8: {}", joined.display()))?;
        for entry in glob::glob(pattern).context("invalid hub glob pattern")? {
            out.push(entry?);
        }
        Ok(())
    }

    /// Read and parse every hub. I/O failure hard-errors the run (an unreadable hub is
    /// exceptional); a *parse* failure is carried per-hub in `LoadedHub::hub` so each
    /// caller handles it explicitly.
    pub fn iter_hubs(&self) -> Result<Vec<LoadedHub>> {
        let mut out = Vec::new();
        for path in self.hub_paths()? {
            let rel = path
                .strip_prefix(&self.root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            let hub = parse_hub(&content);
            let kind = doc_kind(&rel);
            out.push(LoadedHub { rel, kind, hub });
        }
        Ok(out)
    }
}

/// Normalize a `refs:` path (written relative to the referencing hub, per #4) to a
/// workspace-relative path, so it can be matched against `LoadedHub::rel`. `.`/`..` are
/// resolved by component arithmetic — no filesystem access — and the result uses `/` to match
/// the forward-slash rels `iter_hubs` produces.
pub fn resolve_ref_path(referencing_rel: &str, ref_path: &str) -> String {
    let mut stack: Vec<String> = Vec::new();
    let base = Path::new(referencing_rel).parent().unwrap_or(Path::new(""));
    for comp in base.components().chain(Path::new(ref_path).components()) {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                stack.pop();
            }
            Component::Normal(c) => stack.push(c.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    stack.join("/")
}

/// Why a single `at:` site couldn't be loaded for hashing/resolution. Distinct variants so
/// `check`/`verify` can report the precise cause rather than a generic "does not resolve".
#[derive(Debug)]
pub enum SiteError {
    BadAnchor(String),
    UnsupportedType(String),
    Unreadable(String),
}

impl std::fmt::Display for SiteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SiteError::BadAnchor(e) => write!(f, "invalid anchor: {e}"),
            SiteError::UnsupportedType(file) => write!(f, "unsupported file type: {file}"),
            SiteError::Unreadable(file) => {
                write!(f, "cannot read `{file}` (file moved or removed?)")
            }
        }
    }
}

/// Parse an `at:` site, detect its language, and read its source — reporting the precise
/// failure. (Symbol resolution within the source is a separate, later step.)
pub fn read_site(
    ws: &Workspace,
    site: &str,
) -> std::result::Result<(String, Lang, Anchor), SiteError> {
    let anchor = parse_anchor(site).map_err(|e| SiteError::BadAnchor(e.to_string()))?;
    let lang = Lang::from_path(&anchor.file)
        .ok_or_else(|| SiteError::UnsupportedType(anchor.file.clone()))?;
    let source = std::fs::read_to_string(ws.root.join(&anchor.file))
        .map_err(|_| SiteError::Unreadable(anchor.file.clone()))?;
    Ok((source, lang, anchor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discovers_config_from_nested_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join(CONFIG_FILE), "hubs = [\"hubs/*.md\"]\n").unwrap();

        let nested = root.join("a/b/c");
        fs::create_dir_all(&nested).unwrap();

        let ws = Workspace::discover(&nested).unwrap();
        assert_eq!(
            ws.root.canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
        assert_eq!(ws.config.hubs, vec!["hubs/*.md".to_string()]);
    }

    #[test]
    fn errors_when_no_config_anywhere() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(Workspace::discover(tmp.path()).is_err());
    }

    #[test]
    fn globs_hub_files_relative_to_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join(CONFIG_FILE), "").unwrap();
        fs::create_dir_all(root.join("hubs")).unwrap();
        fs::write(root.join("hubs/auth.md"), "---\nsummary: x\n---\n").unwrap();
        fs::write(root.join("hubs/billing.md"), "---\nsummary: y\n---\n").unwrap();
        fs::write(root.join("hubs/notes.txt"), "ignored").unwrap();

        let ws = Workspace::discover(root).unwrap();
        let hubs = ws.hub_paths().unwrap();
        let names: Vec<_> = hubs
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        assert_eq!(names, vec!["auth.md", "billing.md"]);
    }

    #[test]
    fn bundle_root_discovers_nested_concepts_and_reserved_files() {
        // An OKF bundle root is a directory tree: every `.md` beneath it is discovered (concepts and
        // reserved index.md/log.md alike), classified by `LoadedHub::kind`.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join(CONFIG_FILE), "hubs = []\nbundles = [\"sales\"]\n").unwrap();
        fs::create_dir_all(root.join("sales/tables")).unwrap();
        fs::write(root.join("sales/index.md"), "# Sales\n").unwrap();
        fs::write(
            root.join("sales/tables/orders.md"),
            "---\ntype: BigQuery Table\ndescription: orders\n---\n# Orders\n",
        )
        .unwrap();
        fs::write(root.join("sales/tables/log.md"), "# Log\n").unwrap();

        let ws = Workspace::discover(root).unwrap();
        let loaded = ws.iter_hubs().unwrap();
        let index = loaded
            .iter()
            .find(|l| l.rel.ends_with("index.md"))
            .expect("index.md discovered");
        assert_eq!(index.kind, DocKind::Index);
        let orders = loaded
            .iter()
            .find(|l| l.rel.ends_with("orders.md"))
            .expect("concept discovered");
        assert_eq!(orders.kind, DocKind::Concept);
        assert!(loaded.iter().any(|l| l.kind == DocKind::Log));
    }
}

//! `surf new <name>` — scaffold a fresh, lint-clean hub. Lowers the cost of starting a hub
//! (claim maintenance is the main adoption risk, §8). The template has no anchors yet, so
//! `surf lint` passes immediately; the author fills it in and runs `surf verify`.

use crate::workspace::Workspace;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::ExitCode;

pub fn run(ws: &Workspace, name: &str) -> Result<ExitCode> {
    let dir = ws.root.join(hub_dir(&ws.config.hubs));
    let path = dir.join(format!("{name}.md"));
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    std::fs::write(&path, template(name)).with_context(|| format!("writing {}", path.display()))?;

    let rel = path.strip_prefix(&ws.root).unwrap_or(&path);
    println!("created {}", rel.display());
    println!("next: add an anchor, then `surf lint` and `surf verify`.");
    Ok(ExitCode::SUCCESS)
}

/// The directory a new hub should go in: the literal prefix of the first hub glob, up to the
/// first glob metacharacter. `hubs/*.md` -> `hubs`, `docs/hubs/*.md` -> `docs/hubs`.
pub(crate) fn hub_dir(globs: &[String]) -> PathBuf {
    let glob = globs.first().map(String::as_str).unwrap_or("hubs/*.md");
    let literal = glob.split(['*', '?', '[']).next().unwrap_or("");
    match literal.rfind('/') {
        Some(i) => PathBuf::from(&literal[..i]),
        None => PathBuf::from("hubs"),
    }
}

fn template(name: &str) -> String {
    // Explicit `\n` (not `\`-continuation, which would strip the example's indentation).
    let mut s = String::new();
    s.push_str("---\n");
    s.push_str("summary: TODO one-line summary of this domain.\n");
    s.push_str("anchors: []\n");
    s.push_str("refs: []\n");
    s.push_str("---\n\n");
    s.push_str(&format!("# {name}\n\n"));
    s.push_str(
        "TODO: prose first. A hub is an onboarding doc — frame the system, not each symbol.\n",
    );
    s.push_str("Lead with the single most important distinction a reader needs.\n\n");
    s.push_str("## How it works\n\n");
    s.push_str("TODO: how the pieces fit together.\n\n");
    s.push_str("## Boundary\n\n");
    s.push_str("TODO: what this system does *not* cover.\n\n");
    s.push_str(
        "Then add coarse claims in the frontmatter above. Prefer one claim per *behavior*,\n",
    );
    s.push_str(
        "listing every site it spans under one multi-site `at:` — not one claim per symbol:\n\n",
    );
    s.push_str("    anchors:\n");
    s.push_str("      - claim: the invariant this behavior guarantees, in prose\n");
    s.push_str("        at:\n");
    s.push_str("          - path/to/file.rs > symbolName\n");
    s.push_str("          - path/to/other.rs > relatedSymbol\n\n");
    s.push_str("then run `surf verify` to stamp the hash.\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use surf_core::parse_hub;

    #[test]
    fn derives_hub_dir_from_glob() {
        assert_eq!(hub_dir(&["hubs/*.md".into()]), PathBuf::from("hubs"));
        assert_eq!(
            hub_dir(&["docs/hubs/*.md".into()]),
            PathBuf::from("docs/hubs")
        );
        assert_eq!(hub_dir(&[]), PathBuf::from("hubs"));
    }

    #[test]
    fn creates_a_lint_clean_hub() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("surf.toml"), "").unwrap();
        let ws = Workspace::discover(root).unwrap();

        run(&ws, "auth").unwrap();
        let created = root.join("hubs/auth.md");
        assert!(created.exists());

        let hub = parse_hub(&fs::read_to_string(&created).unwrap()).unwrap();
        assert!(hub.frontmatter.anchors.is_empty());
        assert!(hub.body.contains("# auth"));
    }

    #[test]
    fn refuses_to_overwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("surf.toml"), "").unwrap();
        let ws = Workspace::discover(root).unwrap();

        run(&ws, "auth").unwrap();
        assert!(run(&ws, "auth").is_err());
    }
}

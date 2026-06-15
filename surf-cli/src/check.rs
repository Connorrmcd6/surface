//! `surf check` — the gate (§9.1.3, §5). For each anchored span: resolve, AST-canonical
//! hash, compare to the stored hash. Any divergence blocks (non-zero exit). `--format json`
//! emits the structured report that optional layers attach to; the verdict itself is
//! deterministic and needs no git. `old_code`/`magnitude` are recovered best-effort from the
//! previous source via `git show <base>:<path>` and are advisory only.

use crate::format::Format;
use crate::git;
use crate::workspace::{read_site, Workspace};
use anyhow::Result;
use std::process::ExitCode;
use surf_core::{
    diff_magnitude, hash_anchor_with, parse_anchor, resolve, CheckReport, Divergence,
    DivergenceKind, HashOpts, HubError,
};

pub fn run(
    ws: &Workspace,
    format: Format,
    base: Option<&str>,
    files: &[String],
) -> Result<ExitCode> {
    let (divergences, unmatched_globs) = check_workspace(ws, base, files)?;

    match format {
        Format::Json => {
            let report = CheckReport::new(divergences.clone());
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Format::Human => print_human(&divergences),
    }

    // Warnings go to stderr so JSON stdout stays machine-parseable.
    for pattern in &unmatched_globs {
        eprintln!("surf check: --files glob \"{pattern}\" matched no anchored files.");
    }
    // A typo'd --files scopes the gate to nothing and must not go green (#78); but only
    // when *every* glob matched nothing, so a partially-correct invocation still succeeds.
    let all_empty = !files.is_empty() && unmatched_globs.len() == files.len();

    Ok(if divergences.is_empty() && !all_empty {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Returns the divergences in scope plus the `--files` patterns that matched no anchored
/// file, so the caller can refuse to call a run that checked nothing "clean" (#78).
fn check_workspace(
    ws: &Workspace,
    base: Option<&str>,
    files: &[String],
) -> Result<(Vec<Divergence>, Vec<String>)> {
    let mut scope = Scope::build(ws, base, files)?;
    // Enrichment always needs a ref; an explicit --base doubles as the diff base, else HEAD.
    let enrich_base = base.unwrap_or("HEAD");

    let mut out = Vec::new();
    for loaded in ws.iter_hubs()? {
        let hub = match loaded.hub {
            Ok(hub) => hub,
            Err(e) => {
                // The gate fails closed: an unparseable hub is unenforceable, not clean.
                out.push(malformed_hub_divergence(&loaded.rel, &e));
                continue;
            }
        };

        for claim in &hub.frontmatter.anchors {
            if !scope.includes(claim) {
                continue;
            }
            if let Some(d) = check_claim(ws, &loaded.rel, claim, enrich_base) {
                out.push(d);
            }
        }
    }
    Ok((out, scope.unmatched_globs()))
}

fn malformed_hub_divergence(hub: &str, err: &HubError) -> Divergence {
    Divergence {
        hub: hub.to_string(),
        claim: String::new(),
        at: String::new(),
        kind: DivergenceKind::Unresolvable,
        old_hash: None,
        new_hash: None,
        old_code: None,
        new_code: None,
        prose: String::new(),
        magnitude: None,
        detail: Some(format!("invalid hub: {err}")),
    }
}

/// Which claims `check` evaluates. Each active filter narrows the set; a claim must satisfy
/// every active filter (intersection). With neither filter active, every claim is in scope.
struct Scope {
    changed: Option<std::collections::HashSet<String>>,
    globs: Vec<GlobFilter>,
}

/// One `--files` pattern plus whether it ever matched an anchored file, so a typo'd
/// pattern that scopes the gate to nothing is detectable after the walk (#78).
struct GlobFilter {
    raw: String,
    pattern: glob::Pattern,
    matched: bool,
}

impl Scope {
    fn build(ws: &Workspace, base: Option<&str>, files: &[String]) -> Result<Scope> {
        // A bad ref / non-repo yields None — we fall back to a full check rather than
        // silently checking nothing.
        let changed = base.and_then(|b| git::changed_files(&ws.root, b));
        // Invalid glob *syntax* must fail loudly: silently dropping a `--files` pattern
        // changes the gate's scope with no signal (#38).
        let globs = files
            .iter()
            .map(|p| {
                glob::Pattern::new(p)
                    .map(|pattern| GlobFilter {
                        raw: p.clone(),
                        pattern,
                        matched: false,
                    })
                    .map_err(|e| anyhow::anyhow!("invalid --files glob \"{p}\": {e}"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Scope { changed, globs })
    }

    fn includes(&mut self, claim: &surf_core::Claim) -> bool {
        let anchor_files: Vec<String> = claim
            .at
            .sites()
            .iter()
            .filter_map(|s| parse_anchor(s).ok().map(|a| a.file))
            .collect();

        // Tally glob matches before the --base filter, so a glob that names anchored-but-
        // unchanged files still counts as matched (only never-matching globs are suspect).
        let mut glob_pass = self.globs.is_empty();
        for g in &mut self.globs {
            if anchor_files.iter().any(|f| g.pattern.matches(f)) {
                g.matched = true;
                glob_pass = true;
            }
        }
        if !glob_pass {
            return false;
        }
        if let Some(changed) = &self.changed {
            if !anchor_files.iter().any(|f| changed.contains(f)) {
                return false;
            }
        }
        true
    }

    fn unmatched_globs(&self) -> Vec<String> {
        self.globs
            .iter()
            .filter(|g| !g.matched)
            .map(|g| g.raw.clone())
            .collect()
    }
}

fn check_claim(
    ws: &Workspace,
    hub: &str,
    claim: &surf_core::Claim,
    base: &str,
) -> Option<Divergence> {
    let prose = claim.claim.trim().to_string();
    let opts = HashOpts {
        ignore_literals: claim.ignore_literals,
    };
    let sites = claim.at.sites();
    let at_display = sites.join("  +  ");
    let single = sites.len() == 1;

    let mk = |kind, old_hash, new_hash, old_code, new_code, magnitude, detail| {
        Some(Divergence {
            hub: hub.to_string(),
            claim: prose.clone(),
            at: at_display.clone(),
            kind,
            old_hash,
            new_hash,
            old_code,
            new_code,
            prose: prose.clone(),
            magnitude,
            detail,
        })
    };
    let unresolvable = |detail: String| {
        mk(
            DivergenceKind::Unresolvable,
            claim.hash.clone(),
            None,
            None,
            None,
            None,
            Some(detail),
        )
    };

    // Resolve and hash every site; the claim's hash is the combination (§6.3).
    let mut site_hashes = Vec::with_capacity(sites.len());
    let mut first_new_code = None;
    for site in sites {
        let (current, lang, anchor) = match read_site(ws, site) {
            Ok(parts) => parts,
            Err(e) => return unresolvable(e.to_string()),
        };
        let span = match resolve(&current, lang, &anchor) {
            Ok(span) => span,
            Err(e) => return unresolvable(e.to_string()),
        };
        if single {
            first_new_code = current
                .get(span.start_byte..span.end_byte)
                .map(str::to_string);
        }
        let hash = match hash_anchor_with(&current, lang, &anchor, opts) {
            Ok(h) => h,
            Err(e) => return unresolvable(e.to_string()),
        };
        site_hashes.push(hash);
    }
    let new_hash = surf_core::combine_site_hashes(&site_hashes);

    match &claim.hash {
        None => mk(
            DivergenceKind::Unverified,
            None,
            Some(new_hash),
            None,
            first_new_code,
            None,
            None,
        ),
        Some(stored) if *stored == new_hash => None, // clean
        Some(stored) => {
            // Best-effort old_code + magnitude from git, for single-site anchors only.
            let (old_code, magnitude) = if single {
                enrich_from_git(ws, base, &sites[0])
            } else {
                (None, None)
            };
            mk(
                DivergenceKind::Changed,
                Some(stored.clone()),
                Some(new_hash),
                old_code,
                first_new_code,
                magnitude,
                None,
            )
        }
    }
}

fn enrich_from_git(
    ws: &Workspace,
    base: &str,
    site: &str,
) -> (Option<String>, Option<surf_core::Magnitude>) {
    let Ok((current, lang, anchor)) = read_site(ws, site) else {
        return (None, None);
    };
    let Some(old_source) = git::show(&ws.root, base, &anchor.file) else {
        return (None, None);
    };
    let old_code = resolve(&old_source, lang, &anchor).ok().and_then(|sp| {
        old_source
            .get(sp.start_byte..sp.end_byte)
            .map(str::to_string)
    });
    let magnitude = diff_magnitude(&old_source, &current, lang, &anchor).ok();
    (old_code, magnitude)
}

fn print_human(divergences: &[Divergence]) {
    for d in divergences {
        let (tag, hint) = match d.kind {
            DivergenceKind::Changed => ("DIVERGED", None),
            DivergenceKind::Unverified => ("UNVERIFIED", Some("run `surf verify`")),
            DivergenceKind::Unresolvable => ("UNRESOLVED", Some("run `surf lint`")),
        };
        println!("{tag}  {} :: {}", d.hub, d.at);
        if let Some(detail) = &d.detail {
            println!("    {detail}");
        }
        if let (Some(old), Some(new)) = (&d.old_hash, &d.new_hash) {
            let mag = d
                .magnitude
                .map(|m| format!("  (magnitude: {m:?})"))
                .unwrap_or_default();
            println!("    stored {old} → now {new}{mag}");
        }
        if let Some(hint) = hint {
            println!("    {hint}");
        }
        // Malformed-hub divergences have no claim — don't print a dangling label (#83).
        if !d.prose.is_empty() {
            println!("    claim: {}", d.prose);
        }
    }

    if divergences.is_empty() {
        println!("surf check: all anchored spans match their stored hashes.");
    } else {
        println!("surf check: {} divergence(s).", divergences.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use surf_core::{hash_anchor, parse_anchor, Lang};

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    fn ws_at(root: PathBuf) -> Workspace {
        Workspace::discover(&root).unwrap()
    }

    fn stored_hash(src: &str, anchor: &str) -> String {
        hash_anchor(src, Lang::Rust, &parse_anchor(anchor).unwrap()).unwrap()
    }

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn clean_when_hash_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        write(root, "surf.toml", "");
        write(root, "src/m.rs", src);
        let h = stored_hash(src, "src/m.rs > add");
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        assert!(check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .0
            .is_empty());
    }

    #[test]
    fn invalid_files_glob_syntax_errors() {
        // A malformed `--files` pattern must fail loudly, not silently widen/narrow scope (#38).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        let err =
            check_workspace(&ws_at(root.to_path_buf()), None, &["src/[".to_string()]).unwrap_err();
        assert!(err.to_string().contains("src/["), "got: {err}");
    }

    #[test]
    fn per_symbol_not_per_file() {
        // Anchor `add`; modify the *other* function in the same file. Must stay clean.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let original = "pub fn add(a: i64, b: i64) -> i64 { a + b }\npub fn other() -> i64 { 1 }\n";
        let h = stored_hash(original, "src/m.rs > add");
        write(root, "surf.toml", "");
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a + b }\npub fn other() -> i64 { 999 }\n",
        );
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        assert!(check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .0
            .is_empty());
    }

    #[test]
    fn unverified_when_no_stored_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(root, "src/m.rs", "pub fn add() -> i64 { 1 }\n");
        write(
            root,
            "hubs/a.md",
            "---\nsummary: x\nanchors:\n  - claim: c\n    at: src/m.rs > add\n---\n",
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .0;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, DivergenceKind::Unverified);
    }

    #[test]
    fn changed_span_diverges_with_old_code_and_magnitude_from_git() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let v1 = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let h = stored_hash(v1, "src/m.rs > add");
        write(root, "surf.toml", "");
        write(root, "src/m.rs", v1);
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        git(root, &["init", "-q"]);
        git(
            root,
            &["-c", "user.email=t@t", "-c", "user.name=t", "add", "."],
        );
        git(
            root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "v1",
            ],
        );

        // Working-tree change: flip the operator.
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .0;
        assert_eq!(d.len(), 1);
        let d = &d[0];
        assert_eq!(d.kind, DivergenceKind::Changed);
        assert_eq!(d.old_hash.as_deref(), Some(h.as_str()));
        assert!(d.new_hash.is_some() && d.new_hash != d.old_hash);
        assert!(d.old_code.as_deref().unwrap().contains("a + b"));
        assert!(d.new_code.as_deref().unwrap().contains("a - b"));
        assert!(d.magnitude.is_some());
    }

    #[test]
    fn unsupported_file_type_is_unresolvable_with_detail() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(root, "schema.sql", "CREATE TABLE users (id int);\n");
        write(
            root,
            "hubs/a.md",
            "---\nsummary: x\nanchors:\n  - claim: c\n    at: schema.sql > users\n---\n",
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .0;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, DivergenceKind::Unresolvable);
        assert_eq!(
            d[0].detail.as_deref(),
            Some("unsupported file type: schema.sql")
        );
    }

    #[test]
    fn malformed_hub_blocks_check() {
        // A frontmatter typo must fail the gate, not pass silently (#35).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(
            root,
            "hubs/a.md",
            "---\nsummary: x\nanchors:\n  - claim: [unterminated\n---\n",
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .0;
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].kind, DivergenceKind::Unresolvable);
        assert!(d[0].detail.as_deref().unwrap().starts_with("invalid hub"));
    }

    #[test]
    fn json_contract_has_expected_keys() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let v1 = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let h = stored_hash(v1, "src/m.rs > add");
        write(root, "surf.toml", "");
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
        );
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        let d = check_workspace(&ws_at(root.to_path_buf()), None, &[])
            .unwrap()
            .0;
        let json = serde_json::to_value(CheckReport::new(d)).unwrap();
        assert_eq!(json["version"], surf_core::REPORT_VERSION);
        let obj = json["divergences"][0].as_object().unwrap();
        for key in [
            "hub", "claim", "at", "kind", "old_hash", "new_hash", "new_code", "prose",
        ] {
            assert!(obj.contains_key(key), "missing key `{key}` in {obj:?}");
        }
    }

    /// Two diverged claims in different files; both surface with no scope, but a `--files`
    /// glob narrows the result to the matching file.
    fn two_diverged_files(root: &Path) {
        let a = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let s = "pub fn sub(a: i64, b: i64) -> i64 { a - b }\n";
        let ha = stored_hash(a, "src/a.rs > add");
        let hs = stored_hash(s, "src/b.rs > sub");
        write(root, "surf.toml", "");
        // Working tree diverges from the stored hashes.
        write(
            root,
            "src/a.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
        );
        write(
            root,
            "src/b.rs",
            "pub fn sub(a: i64, b: i64) -> i64 { a + b }\n",
        );
        write(
            root,
            "hubs/a.md",
            &format!(
                "---\nsummary: x\nanchors:\n  - claim: add\n    at: src/a.rs > add\n    hash: {ha}\n  - claim: sub\n    at: src/b.rs > sub\n    hash: {hs}\n---\n"
            ),
        );
    }

    #[test]
    fn files_scope_limits_claims() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        two_diverged_files(root);
        let ws = ws_at(root.to_path_buf());

        assert_eq!(check_workspace(&ws, None, &[]).unwrap().0.len(), 2);

        let scoped = check_workspace(&ws, None, &["src/a.rs".to_string()])
            .unwrap()
            .0;
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].at, "src/a.rs > add");

        let globbed = check_workspace(&ws, None, &["src/b*.rs".to_string()])
            .unwrap()
            .0;
        assert_eq!(globbed.len(), 1);
        assert_eq!(globbed[0].at, "src/b.rs > sub");
    }

    #[test]
    fn base_scope_limits_to_changed() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        two_diverged_files(root);

        // Commit the diverged working tree as v0, then change only src/a.rs.
        git(root, &["init", "-q"]);
        git(
            root,
            &["-c", "user.email=t@t", "-c", "user.name=t", "add", "."],
        );
        git(
            root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "v0",
            ],
        );
        write(
            root,
            "src/a.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a * b }\n",
        );

        let ws = ws_at(root.to_path_buf());
        let scoped = check_workspace(&ws, Some("HEAD"), &[]).unwrap().0;
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].at, "src/a.rs > add");
    }

    #[test]
    fn zero_match_files_glob_is_reported_and_fails_alone() {
        // A typo'd --files pattern scopes the gate to nothing; that must not read as a
        // clean run (#78).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let h = stored_hash(src, "src/m.rs > add");
        write(root, "surf.toml", "");
        write(root, "src/m.rs", src);
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );
        let ws = ws_at(root.to_path_buf());

        let typo = "src/lables/*.rs".to_string();
        let (d, unmatched) = check_workspace(&ws, None, std::slice::from_ref(&typo)).unwrap();
        assert!(d.is_empty());
        assert_eq!(unmatched, vec![typo.clone()]);

        let code = run(&ws, Format::Human, None, &[typo]).unwrap();
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::FAILURE));
    }

    #[test]
    fn partially_matching_files_globs_still_succeed() {
        // One good glob + one typo: the typo is reported but a partially-correct
        // invocation keeps a clean exit (mirrors `suggest`, #78).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let h = stored_hash(src, "src/m.rs > add");
        write(root, "surf.toml", "");
        write(root, "src/m.rs", src);
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );
        let ws = ws_at(root.to_path_buf());

        let globs = vec!["src/*.rs".to_string(), "zzz/nope/*.go".to_string()];
        let (d, unmatched) = check_workspace(&ws, None, &globs).unwrap();
        assert!(d.is_empty());
        assert_eq!(unmatched, vec!["zzz/nope/*.go".to_string()]);

        let code = run(&ws, Format::Human, None, &globs).unwrap();
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    #[test]
    fn files_glob_matching_unchanged_anchors_under_base_is_not_flagged() {
        // With --base narrowing scope to changed files, a glob that names anchored but
        // unchanged files is still a *valid* glob — it must not trip the zero-match guard.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        two_diverged_files(root);
        git(root, &["init", "-q"]);
        git(
            root,
            &["-c", "user.email=t@t", "-c", "user.name=t", "add", "."],
        );
        git(
            root,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "v0",
            ],
        );
        // Only src/a.rs changes; the glob targets the *unchanged* src/b.rs.
        write(
            root,
            "src/a.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a * b }\n",
        );
        let ws = ws_at(root.to_path_buf());

        let (d, unmatched) =
            check_workspace(&ws, Some("HEAD"), &["src/b*.rs".to_string()]).unwrap();
        assert!(d.is_empty()); // b is unchanged and a is excluded by the glob
        assert!(unmatched.is_empty(), "glob matched an anchored file");
    }

    #[test]
    fn base_scope_works_when_workspace_is_a_repo_subdir() {
        // The workspace (surf.toml) sits in `proj/`, a subdirectory of the git repo. A real
        // logic drift in an anchored file must still fail `check --base`. Regression for the
        // silent bypass where `git diff` emitted repo-root-relative paths (`proj/src/m.rs`)
        // that never matched the workspace-relative anchor (`src/m.rs`), scoping the gate to
        // zero claims and exiting clean.
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        let proj = repo.join("proj");

        let v1 = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let h = stored_hash(v1, "src/m.rs > add");
        write(&proj, "surf.toml", "");
        write(&proj, "src/m.rs", v1);
        write(
            &proj,
            "hubs/a.md",
            &format!("---\nsummary: x\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {h}\n---\n"),
        );

        // Initialize the repo at the parent, not the workspace.
        git(repo, &["init", "-q"]);
        git(
            repo,
            &["-c", "user.email=t@t", "-c", "user.name=t", "add", "."],
        );
        git(
            repo,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "-m",
                "v1",
            ],
        );

        // Diverge the anchored span in the working tree.
        write(
            &proj,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
        );

        let ws = ws_at(proj.clone());
        let scoped = check_workspace(&ws, Some("HEAD"), &[]).unwrap().0;
        assert_eq!(scoped.len(), 1, "subdir --base must still catch the drift");
        assert_eq!(scoped[0].kind, DivergenceKind::Changed);
    }

    #[test]
    fn no_flags_checks_everything() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        two_diverged_files(root);
        let ws = ws_at(root.to_path_buf());
        assert_eq!(check_workspace(&ws, None, &[]).unwrap().0.len(), 2);
    }
}

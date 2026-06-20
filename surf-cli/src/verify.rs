//! `surf verify` — the human escape hatch (§8, §9.1.4). Re-hash anchors after a human has
//! confirmed the prose still holds and write the hash back into the frontmatter ("I looked,
//! still true"). `--follow` re-points a renamed single-segment anchor and re-hashes in one
//! step (§6.4). Writes are surgical (only the touched line changes) and skipped entirely
//! when nothing changed, so a no-op verify leaves the file byte-identical.

use crate::format::Format;
use crate::git;
use crate::workspace::{read_site, SiteError, Workspace};
use anyhow::{Context, Result};
use serde::Serialize;
use std::process::ExitCode;
use surf_core::{
    combine_site_hashes, find_renamed, format_stamp, hash_anchor_raw, hash_anchor_with,
    parse_anchor, parse_hub, parse_stamp, set_anchor_at, set_anchor_hash, HashOpts, Recipe,
};

enum Plan {
    Hash(String),
    Follow { new_at: String, new_hash: String },
    Unchanged,
    Skip(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
enum VerifyOutcome {
    Stamped,
    Followed { new_at: String },
    Unchanged,
    Skipped { reason: String },
}

#[derive(Debug, Clone, Serialize)]
struct AnchorResult {
    hub: String,
    at: String,
    #[serde(flatten)]
    outcome: VerifyOutcome,
}

#[derive(Debug, Default, Serialize)]
struct VerifyReport {
    stamped: usize,
    unchanged: usize,
    errors: usize,
    anchors: Vec<AnchorResult>,
    #[serde(skip)]
    updated_files: Vec<String>,
}

pub fn run(ws: &Workspace, target: Option<&str>, follow: bool, format: Format) -> Result<ExitCode> {
    let report = verify_all(ws, target, follow)?;

    match format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        Format::Human => print_human(&report),
    }

    Ok(if report.errors == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn print_human(report: &VerifyReport) {
    for a in &report.anchors {
        match &a.outcome {
            VerifyOutcome::Followed { new_at } => {
                println!("followed {} :: {} → {new_at}", a.hub, a.at)
            }
            VerifyOutcome::Skipped { reason } => {
                println!("error {} :: {} ({reason})", a.hub, a.at)
            }
            _ => {}
        }
    }
    for f in &report.updated_files {
        println!("updated {f}");
    }
    println!(
        "surf verify: stamped {} anchor(s), {} skipped, {} error(s).",
        report.stamped, report.unchanged, report.errors
    );
}

fn verify_all(ws: &Workspace, target: Option<&str>, follow: bool) -> Result<VerifyReport> {
    let mut report = VerifyReport::default();
    let mut matched_any = false;

    for hub_path in ws.hub_paths()? {
        let rel = hub_path
            .strip_prefix(&ws.root)
            .unwrap_or(&hub_path)
            .display()
            .to_string();
        let original = std::fs::read_to_string(&hub_path)
            .with_context(|| format!("reading {}", hub_path.display()))?;
        let Ok(hub) = parse_hub(&original) else {
            continue;
        };
        let mut text = original.clone();

        for (idx, claim) in hub.frontmatter.anchors.iter().enumerate() {
            let sites = claim.at.sites();
            if let Some(t) = target {
                if !sites.iter().any(|s| s == t) {
                    continue;
                }
            }
            matched_any = true;
            let at = sites.join("  +  ");

            let outcome = match plan_claim(ws, claim, follow) {
                Plan::Hash(new_hash) => match set_anchor_hash(&text, idx, &new_hash) {
                    Some(updated) => {
                        text = updated;
                        report.stamped += 1;
                        VerifyOutcome::Stamped
                    }
                    None => {
                        report.errors += 1;
                        VerifyOutcome::Skipped {
                            reason: "could not write hash".into(),
                        }
                    }
                },
                Plan::Follow { new_at, new_hash } => {
                    match set_anchor_at(&text, idx, &new_at)
                        .and_then(|t| set_anchor_hash(&t, idx, &new_hash))
                    {
                        Some(updated) => {
                            text = updated;
                            report.stamped += 1;
                            VerifyOutcome::Followed { new_at }
                        }
                        None => {
                            report.errors += 1;
                            VerifyOutcome::Skipped {
                                reason: "could not rewrite at:".into(),
                            }
                        }
                    }
                }
                Plan::Unchanged => {
                    report.unchanged += 1;
                    VerifyOutcome::Unchanged
                }
                Plan::Skip(reason) => {
                    report.errors += 1;
                    VerifyOutcome::Skipped { reason }
                }
            };

            report.anchors.push(AnchorResult {
                hub: rel.clone(),
                at,
                outcome,
            });
        }

        if text != original {
            std::fs::write(&hub_path, &text)
                .with_context(|| format!("writing {}", hub_path.display()))?;
            report.updated_files.push(rel);
        }
    }

    if let Some(t) = target {
        if !matched_any {
            anyhow::bail!("no anchor matching `{t}`");
        }
    }

    Ok(report)
}

fn plan_claim(ws: &Workspace, claim: &surf_core::Claim, follow: bool) -> Plan {
    let sites = claim.at.sites();
    let opts = HashOpts {
        ignore_literals: claim.ignore_literals,
    };

    let mut site_hashes = Vec::with_capacity(sites.len());
    let mut failure: Option<String> = None;
    for site in sites {
        match site_hash(ws, site, opts) {
            Ok(h) => site_hashes.push(h),
            Err(reason) => {
                failure = Some(reason);
                break;
            }
        }
    }

    match failure {
        None => {
            // verify always (re-)stamps under the current recipe, so one pass also upgrades a
            // still-matching v1 stamp to v2 — a narrow, intentional exception to #22's
            // skip-unchanged (#140).
            let combined = combine_site_hashes(&site_hashes);
            let new_stamp = format_stamp(Recipe::CURRENT, &combined);
            if claim.hash.as_deref() == Some(new_stamp.as_str()) {
                Plan::Unchanged
            } else {
                Plan::Hash(new_stamp)
            }
        }
        Some(reason) if !follow => Plan::Skip(reason),
        Some(_) => plan_follow(ws, claim, opts),
    }
}

fn plan_follow(ws: &Workspace, claim: &surf_core::Claim, opts: HashOpts) -> Plan {
    let sites = claim.at.sites();
    if sites.len() != 1 {
        return Plan::Skip("--follow supports single-site anchors only".into());
    }
    let Some(stored) = claim.hash.as_deref() else {
        return Plan::Skip("--follow needs a stored hash to match against".into());
    };
    match read_site(ws, &sites[0]) {
        // The file reads but the symbol no longer resolves — a symbol rename within the file.
        Ok((_, _, anchor)) if anchor.segments.len() != 1 => {
            Plan::Skip("--follow supports single-segment anchors only".into())
        }
        Ok((source, lang, anchor)) => follow_symbol(&source, lang, &anchor.file, stored, opts),
        // The file itself is gone — try git rename detection (best-effort, never gates).
        Err(SiteError::Unreadable(_)) => follow_file(ws, &sites[0], stored, opts),
        Err(e) => Plan::Skip(e.to_string()),
    }
}

/// A symbol rename within a still-readable file: relocate by stored-hash match (§6.4).
fn follow_symbol(
    source: &str,
    lang: surf_core::Lang,
    file: &str,
    stored: &str,
    opts: HashOpts,
) -> Plan {
    match find_renamed(source, lang, stored, opts) {
        Ok(Some(new_name)) => {
            let new_at = format!("{file} > {new_name}");
            match parse_anchor(&new_at)
                .ok()
                .and_then(|a| hash_anchor_with(source, lang, &a, opts).ok())
            {
                Some(new_hash) => Plan::Follow { new_at, new_hash },
                None => Plan::Skip("rename target did not re-resolve".into()),
            }
        }
        _ => Plan::Skip("does not resolve and no rename match; run `surf lint`".into()),
    }
}

/// A file rename: ask git where the file moved, then re-point the anchor — but only when the
/// code is otherwise unchanged (its hash still matches, possibly under a renamed symbol). If the
/// body also changed, we refuse to re-stamp it; the human must verify after the path is fixed.
fn follow_file(ws: &Workspace, site: &str, stored: &str, opts: HashOpts) -> Plan {
    let Ok(anchor) = parse_anchor(site) else {
        return Plan::Skip("invalid anchor".into());
    };
    let Some(new_file) = git::renamed_to(&ws.root, &anchor.file) else {
        return Plan::Skip("file unreadable and no git rename match; run `surf lint`".into());
    };
    // Reconstruct the anchor at the new path, preserving the symbol portion (`file > sym...`).
    let Some((_, rest)) = site.split_once('>') else {
        return Plan::Skip("file unreadable and anchor has no symbol".into());
    };
    let new_at = format!("{new_file} >{rest}");
    let (source, lang, new_anchor) = match read_site(ws, &new_at) {
        Ok(parts) => parts,
        Err(e) => return Plan::Skip(e.to_string()),
    };
    // Same symbol path, code unchanged under the stored recipe → re-point, upgrading the stamp
    // to the current recipe (the move is a chance to migrate v1 → v2).
    if let Some((recipe, stored_hex)) = parse_stamp(stored) {
        if hash_anchor_raw(&source, lang, &new_anchor, opts, recipe).as_deref() == Ok(stored_hex) {
            if let Ok(new_hash) = hash_anchor_with(&source, lang, &new_anchor, opts) {
                return Plan::Follow { new_at, new_hash };
            }
        }
    }
    // The symbol may also have been renamed in the move (single-segment only).
    if new_anchor.segments.len() == 1 {
        return follow_symbol(&source, lang, &new_file, stored, opts);
    }
    Plan::Skip("file moved but its anchored code changed; run `surf lint`".into())
}

/// One site's bare per-site digest under the current recipe — combined and prefixed into the
/// stored stamp by the caller. Bare (not a full stamp) so multi-site combination stays correct.
fn site_hash(ws: &Workspace, site: &str, opts: HashOpts) -> std::result::Result<String, String> {
    let (source, lang, anchor) = read_site(ws, site).map_err(|e| e.to_string())?;
    hash_anchor_raw(&source, lang, &anchor, opts, Recipe::CURRENT).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use surf_core::{hash_anchor, hash_anchor_raw, HashOpts, Lang, Recipe};

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    #[test]
    fn verify_stamps_then_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a + b }\n",
        );
        write(
            root,
            "hubs/a.md",
            "---\nsummary: s\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n---\n# Body\n",
        );

        let ws = Workspace::discover(root).unwrap();
        run(&ws, None, false, Format::Human).unwrap();

        // Hash now present and equals the canonical hash of the symbol.
        let after = fs::read_to_string(root.join("hubs/a.md")).unwrap();
        let hub = parse_hub(&after).unwrap();
        let expected = hash_anchor(
            "pub fn add(a: i64, b: i64) -> i64 { a + b }\n",
            Lang::Rust,
            &parse_anchor("src/m.rs > add").unwrap(),
        )
        .unwrap();
        assert_eq!(
            hub.frontmatter.anchors[0].hash.as_deref(),
            Some(expected.as_str())
        );

        // Second verify is a no-op: byte-identical, and reported as skipped not stamped.
        let report = verify_all(&ws, None, false).unwrap();
        assert_eq!(report.stamped, 0);
        assert_eq!(report.unchanged, 1);
        assert_eq!(report.errors, 0);
        assert!(report.updated_files.is_empty());
        assert_eq!(fs::read_to_string(root.join("hubs/a.md")).unwrap(), after);
    }

    #[test]
    fn verify_upgrades_v1_stamp_to_v2() {
        // A repo stamped under the old recipe: one `surf verify` re-stamps the still-matching v1
        // anchor as v2 (the narrow exception to skip-unchanged), and is then idempotent (#140).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = "pub fn add(a: i64, b: i64) -> i64 { a + b }\n";
        let v1 = hash_anchor_raw(
            src,
            Lang::Rust,
            &parse_anchor("src/m.rs > add").unwrap(),
            HashOpts::default(),
            Recipe::V1,
        )
        .unwrap();
        assert!(!v1.contains(':'), "starts as a bare v1 stamp");
        write(root, "surf.toml", "");
        write(root, "src/m.rs", src);
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: s\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n    hash: {v1}\n---\n"),
        );

        let ws = Workspace::discover(root).unwrap();
        let report = verify_all(&ws, None, false).unwrap();
        assert_eq!(report.stamped, 1, "the v1 anchor is re-stamped to v2");

        let expected =
            hash_anchor(src, Lang::Rust, &parse_anchor("src/m.rs > add").unwrap()).unwrap();
        assert!(expected.starts_with("2:"));
        let hub = parse_hub(&fs::read_to_string(root.join("hubs/a.md")).unwrap()).unwrap();
        assert_eq!(
            hub.frontmatter.anchors[0].hash.as_deref(),
            Some(expected.as_str())
        );

        // Idempotent on the upgraded stamp.
        let again = verify_all(&ws, None, false).unwrap();
        assert_eq!(again.stamped, 0);
        assert_eq!(again.unchanged, 1);
    }

    #[test]
    fn follow_repoints_renamed_anchor() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = "pub fn rotate_token(t: &str) -> String { t.to_string() }\n";
        let stored = hash_anchor(
            src,
            Lang::Rust,
            &parse_anchor("src/a.rs > rotate_token").unwrap(),
        )
        .unwrap();
        write(root, "surf.toml", "");
        write(root, "src/a.rs", src);
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: s\nanchors:\n  - claim: rotation\n    at: src/a.rs > rotate\n    hash: {stored}\n---\n"),
        );

        let ws = Workspace::discover(root).unwrap();
        let code = run(&ws, None, true, Format::Human).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        let hub = parse_hub(&fs::read_to_string(root.join("hubs/a.md")).unwrap()).unwrap();
        assert_eq!(
            hub.frontmatter.anchors[0].at.sites(),
            &["src/a.rs > rotate_token".to_string()]
        );
        assert_eq!(
            hub.frontmatter.anchors[0].hash.as_deref(),
            Some(stored.as_str())
        );
    }

    #[test]
    fn follow_repoints_renamed_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let src = "pub fn rotate(t: &str) -> String { t.to_string() }\n";
        let stored =
            hash_anchor(src, Lang::Rust, &parse_anchor("src/a.rs > rotate").unwrap()).unwrap();
        write(root, "surf.toml", "");
        write(root, "src/a.rs", src);
        write(
            root,
            "hubs/a.md",
            &format!("---\nsummary: s\nanchors:\n  - claim: rotation\n    at: src/a.rs > rotate\n    hash: {stored}\n---\n"),
        );

        // Commit, then `git mv` the file so git recognizes the rename.
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .current_dir(root)
                .args(args)
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?} failed");
        };
        git(&["init", "-q"]);
        git(&["-c", "user.email=t@t", "-c", "user.name=t", "add", "."]);
        git(&[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "v1",
        ]);
        git(&["mv", "src/a.rs", "src/b.rs"]);

        let ws = Workspace::discover(root).unwrap();
        let code = run(&ws, None, true, Format::Human).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);

        // The anchor's file path was rewritten; the hash is unchanged (code only moved).
        let hub = parse_hub(&fs::read_to_string(root.join("hubs/a.md")).unwrap()).unwrap();
        assert_eq!(
            hub.frontmatter.anchors[0].at.sites(),
            &["src/b.rs > rotate".to_string()]
        );
        assert_eq!(
            hub.frontmatter.anchors[0].hash.as_deref(),
            Some(stored.as_str())
        );
    }

    #[test]
    fn unresolved_without_follow_is_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(root, "src/m.rs", "pub fn other() {}\n");
        write(
            root,
            "hubs/a.md",
            "---\nsummary: s\nanchors:\n  - claim: c\n    at: src/m.rs > ghost\n---\n",
        );

        let ws = Workspace::discover(root).unwrap();
        let code = run(&ws, None, false, Format::Human).unwrap();
        assert_eq!(code, ExitCode::FAILURE);
        // Unchanged: no hash written.
        let hub = parse_hub(&fs::read_to_string(root.join("hubs/a.md")).unwrap()).unwrap();
        assert_eq!(hub.frontmatter.anchors[0].hash, None);
    }

    #[test]
    fn report_serializes_and_stamps_side_effect() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write(root, "surf.toml", "");
        write(
            root,
            "src/m.rs",
            "pub fn add(a: i64, b: i64) -> i64 { a + b }\n",
        );
        write(
            root,
            "hubs/a.md",
            "---\nsummary: s\nanchors:\n  - claim: add sums\n    at: src/m.rs > add\n---\n",
        );

        let ws = Workspace::discover(root).unwrap();
        let report = verify_all(&ws, None, false).unwrap();

        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["stamped"], 1);
        let anchor = json["anchors"][0].as_object().unwrap();
        for key in ["hub", "at", "outcome"] {
            assert!(
                anchor.contains_key(key),
                "missing key `{key}` in {anchor:?}"
            );
        }
        assert_eq!(anchor["outcome"], "stamped");

        // Side effect intact: the hub file was stamped.
        let hub = parse_hub(&fs::read_to_string(root.join("hubs/a.md")).unwrap()).unwrap();
        assert!(hub.frontmatter.anchors[0].hash.is_some());
    }
}

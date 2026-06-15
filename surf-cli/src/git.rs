//! Best-effort git queries. The deterministic verdict never depends on these: they scope and
//! enrich `check` (advisory `old_code`/`magnitude`) and let `lint`/`verify` recognize a moved
//! file. Every function returns `None`/empty when git can't answer (no repo, bad ref, shallow
//! clone), so the gate degrades to a full, git-free check rather than failing.

use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Files changed between the merge base of `base`..HEAD and the working tree. Paths are
/// emitted relative to `root` (the workspace root) via `--relative`, so they match
/// `Anchor.file` (also workspace-root-relative) even when the workspace is a *subdirectory*
/// of the git repo — without `--relative`, `git diff` reports repo-root-relative paths
/// (e.g. `proj/src/x.rs`) that never intersect a workspace-relative anchor (`src/x.rs`),
/// silently scoping the `--base` gate to zero claims and passing real drift (exit 0).
/// `--relative` also drops changes outside the workspace, which can never be anchored anyway.
/// `None` if git can't answer.
pub fn changed_files(root: &Path, base: &str) -> Option<HashSet<String>> {
    let merge_base = Command::new("git")
        .current_dir(root)
        .args(["merge-base", base, "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        // Shallow clones may lack the merge base; diff against the ref directly.
        .unwrap_or_else(|| base.to_string());

    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", "--relative", &merge_base])
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    })
}

/// One commit from the single-spawn history stream: its full SHA, its parents (first parent
/// first, empty for a root commit), and the paths it changed versus its first parent as
/// `--name-status` rows (`A`/`M`/`D`/`T`).
pub struct CommitRec {
    pub sha: String,
    pub parents: Vec<String>,
    pub changes: Vec<(char, String)>,
}

/// The whole history window in one spawn (#72): every reachable commit, newest first with
/// children before parents (`--topo-order`), each carrying its parents and its first-parent
/// name-status diff. Replaces the per-commit `diff-tree`/`rev-parse`/`ls-tree` triple whose
/// process-creation cost made `surf stats` minutes-slow on large repos.
///
/// Merges are *included* — `--diff-merges=first-parent` (git ≥ 2.31) gives them the same
/// first-parent diff as ordinary commits, so hub state can be propagated through them — and
/// `--no-renames` keeps parity with the old plumbing `diff-tree` (a rename reads as delete+add).
/// `None` if git can't answer.
pub fn log_stream(root: &Path, since: Option<&str>, until: Option<&str>) -> Option<Vec<CommitRec>> {
    let mut args: Vec<String> = vec![
        "log".into(),
        "--topo-order".into(),
        "--diff-merges=first-parent".into(),
        "--no-renames".into(),
        "--name-status".into(),
        // \x01 marks commit headers; it can't appear in a quoted git path, so the parser can
        // split records without guessing whether a line is a path or a header.
        "--format=%x01%H %P".into(),
    ];
    if let Some(s) = since {
        args.push(format!("--since={s}"));
    }
    if let Some(u) = until {
        args.push(format!("--until={u}"));
    }
    let output = Command::new("git")
        .current_dir(root)
        .args(&args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let mut out: Vec<CommitRec> = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(header) = line.strip_prefix('\x01') {
            let mut fields = header.split_whitespace();
            let sha = fields.next()?.to_string();
            out.push(CommitRec {
                sha,
                parents: fields.map(str::to_string).collect(),
                changes: Vec::new(),
            });
        } else if let Some((status, path)) = line.split_once('\t') {
            let status = status.chars().next()?;
            out.last_mut()?.changes.push((status, path.to_string()));
        }
    }
    Some(out)
}

/// Every tracked file at `sha` (repo-root-relative). Used to find the hub set as it existed at a
/// past commit. `None` if git can't answer.
pub fn list_files_at(root: &Path, sha: &str) -> Option<Vec<String>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-tree", "-r", "--name-only", sha])
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    })
}

/// The contents of `rel_file` at `base` (e.g. `git show HEAD:src/x.rs`). `None` if unavailable.
pub fn show(root: &Path, base: &str, rel_file: &str) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", &format!("{base}:{rel_file}")])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The path `old_path` was renamed to, per git's rename detection between HEAD and the working
/// tree. `None` if git can't answer or no rename pairs with `old_path`. Best-effort: a pure
/// `mv` without a content match may show as delete+add and not be detected.
pub fn renamed_to(root: &Path, old_path: &str) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-status", "--find-renames", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        // Rename rows are `R<score>\t<old>\t<new>`.
        let mut parts = line.split('\t');
        let Some(status) = parts.next() else {
            continue;
        };
        if !status.starts_with('R') {
            continue;
        }
        let (Some(old), Some(new)) = (parts.next(), parts.next()) else {
            continue;
        };
        if old == old_path {
            return Some(new.to_string());
        }
    }
    None
}

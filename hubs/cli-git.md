---
summary: Best-effort git queries for scoping and rename-following — advisory only, the gate never depends on them.
anchors:
  - claim: >
      Every query here is best-effort and advisory: each returns None/empty when git can't answer
      (no repo, bad ref, shallow clone), so surf degrades to a full, git-free check rather than
      failing. The deterministic verdict never depends on any of them.
    at:
      - surf-cli/src/git.rs > changed_files
      - surf-cli/src/git.rs > show
      - surf-cli/src/git.rs > renamed_to
      - surf-cli/src/git.rs > log_stream
      - surf-cli/src/git.rs > list_files_at
    hash: 2:95e280660c73
  - claim: >
      changed_files returns workspace-root-relative paths changed between the merge base of
      base..HEAD and the working tree (git diff --relative), so the set intersects
      workspace-relative anchors even when the workspace is a repo subdirectory; a missing merge
      base (shallow clone) falls back to diffing the ref directly.
    at: surf-cli/src/git.rs > changed_files
    hash: 2:86115d32f1c7
  - claim: >
      log_stream returns the whole history window in one git spawn: every reachable commit (newest
      first, children before parents) with its parents and its first-parent name-status diff.
      Merges are included with --diff-merges=first-parent so surf stats can propagate hub state
      through them, and --no-renames keeps a rename reading as delete+add.
    at: surf-cli/src/git.rs > log_stream
    hash: 2:a410122a0052
  - claim: >
      renamed_to asks git's rename detection (diff --name-status --find-renames HEAD) for the new
      path a file moved to, letting lint warn and verify --follow re-point instead of hard-blocking.
      Best-effort: a pure mv with no content match may show as delete+add and go undetected.
    at: surf-cli/src/git.rs > renamed_to
    hash: 2:260267073598
refs: []
---

# git helpers

A thin wrapper over `git` via `std::process::Command` — no `git2` dependency.

**The one distinction that matters:** these only *enrich* the gate; they never decide it. `check`'s
verdict is computed from anchored code alone, so a missing or broken git environment degrades the
gate gracefully (a full, git-free check) instead of failing closed on infrastructure.

The five helpers split by job: `changed_files` diff-scopes `surf check --base`; `log_stream` and
`list_files_at` feed `surf stats` history; `show` recovers prior source for the advisory
`old_code`/`magnitude` enrichment in the JSON report; `renamed_to` powers file-rename recognition
in `lint`/`verify` (symbol renames are [`rename.md`](./rename.md)). The first claim seals the
contract they all share; the rest pin down the non-trivial mechanics.

**Boundary:** nothing here is part of the deterministic verdict, and none of these mutate the repo —
they only read git state.

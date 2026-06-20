---
summary: surf check — the gate. Hash each anchored span, compare to the stored hash, block on divergence. Optionally scope to changed files.
anchors:
  - claim: >
      Per claim: resolve and hash every site under the stored stamp's own recipe (v1/v2),
      combine into one hash, compare to the stored hash. No stored hash → Unverified; an anchor
      that no longer resolves, or a stamp with an unrecognized version prefix → Unresolvable;
      a mismatch → Changed; a clean match is tagged with whether the stamp was still v1. The
      verdict is deterministic and needs no git.
    at: surf-cli/src/check.rs > check_claim
    hash: 2:36cbbc039ab1
  - claim: >
      Scoping is opt-in and intersective: with neither --base nor --files every claim is checked.
      A claim is in scope when any of its anchored files matches each active filter — the --base
      changed-files set (merge-base..working-tree) and/or the --files globs. A bad ref or non-repo
      yields no changed set, falling back to a full check rather than checking nothing. Each glob
      records whether it ever matched an anchored file (tallied before the --base filter), so a
      pattern that scopes the gate to nothing is detectable after the walk.
    at: surf-cli/src/check.rs > Scope > includes
    hash: 2:d459cc00d69b
  - claim: >
      The gate fails closed: a hub whose frontmatter won't parse yields an Unresolvable
      divergence (blocking the run) rather than being silently skipped, so a frontmatter typo
      can't pass as clean. Alongside the divergences it returns the --files patterns that
      matched no anchored file (run warns on stderr for each and exits non-zero when every
      pattern matched nothing, so a typo'd --files can't read as a clean run) and a count of
      clean anchors still stamped under v1, so run can nudge the one-time `surf verify` upgrade.
    at: surf-cli/src/check.rs > check_workspace
    hash: 2:d8957ecb971d
refs: []
---

# surf check

`check_claim` is the verdict; the git helpers in [`cli-git.md`](./cli-git.md) only feed the
advisory `old_code`/`magnitude` in the `--format json` report. Any divergence makes `run` exit
non-zero (the CI-blocking signal). `Scope` narrows which claims `check_workspace` evaluates when
`--base`/`--files` are given.

---
summary: surf lint - anchors must resolve to one symbol (renames warn); plus advisory granularity warnings.
anchors:
  - claim: >
      lint produces a Finding per anchor site: ambiguous or vanished anchors block, while a
      renamed-but-present symbol (stored-hash match) only warns and points at verify --follow -
      as does a file that git reports has moved. Block-level findings set a non-zero exit;
      warnings alone keep exit 0.
    at: surf-cli/src/lint.rs > lint_site
    hash: 2:97f0946e74b0
  - claim: >
      Advisory granularity guidance (§8), never blocking: lint_under_coverage flags public
      symbols - top-level functions and methods - in an already-anchored file that no claim
      covers. Coverage is workspace-wide: a symbol anchored by any hub is covered, so a second
      hub touching the same file is never nagged about symbols another hub owns, and each
      uncovered symbol is reported once against the file's first anchoring hub. It runs only on
      files whose anchors all resolved cleanly, so coverage nags never pile onto broken anchors.
    at: surf-cli/src/lint.rs > lint_under_coverage
    hash: 2:1a94fd3c8328
  - claim: >
      AGENTS.md enforcement is opt-in (§11.6): only when the file carries a surf:hubs marker
      block does lint require it to link the configured hubs directory (which must exist),
      blocking otherwise. It points agents at the directory to search - never enumerating
      individual hubs, which would push an agent to read everything.
    at: surf-cli/src/lint.rs > lint_agents_pointer
    hash: 2:ac139b65f5f0
refs: []
---

# surf lint

`lint_workspace` loads every hub and runs `lint_site` over each anchor; `run` prints the
findings and chooses the exit code. Beyond resolution, lint emits advisory warnings (§8) that
nudge granularity: a near-whole-file span (`lint_coarse_span`), too many anchors in one hub,
and public symbols - functions and methods - with no covering claim anywhere in the workspace
(`lint_under_coverage`). It also validates the
`AGENTS.md` pointer block (`lint_agents_pointer`).

---
summary: surf verify — re-seal a claim after a human confirms the prose, with optional --follow.
anchors:
  - claim: >
      For each claim, plan_claim re-hashes every site (combined) under the current recipe when
      all resolve, returning Unchanged only when the stored stamp already matches that recipe's
      stamp, else Hash to re-stamp — so one pass also upgrades a still-matching v1 stamp to v2.
      Under --follow, a site that no longer resolves re-points a renamed single-segment anchor
      via find_renamed; a site whose file is unreadable asks git where it moved and re-points the
      path (only when the code is otherwise unchanged under the stored recipe). Otherwise it skips
      with a reason. It never edits prose, only the hash/at line.
    at: surf-cli/src/verify.rs > plan_claim
    hash: 2:18df2a40dd9d
refs: []
---

# surf verify

The human escape hatch. `verify_all` applies each `plan_claim` result through the surgical hub
editor and only rewrites a file when something actually changed; `run` then renders the
collected report as human text or JSON.

---
summary: Deterministic, git-free rename detection via stored-hash match.
anchors:
  - claim: >
      When an anchor no longer resolves, find_renamed walks every current definition and
      returns the one whose canonical hash equals the claim's stored hash - because the hash
      alpha-renames identifiers, a renamed-but-unchanged symbol still matches. No git, no
      similarity threshold.
    at: surf-core/src/rename.rs > find_renamed
    hash: 2:a6612bef104e
refs: []
---

# Rename detection

This is what lets `lint` *warn* on a rename (and `verify --follow` re-point it) instead of
hard-blocking. It catches symbol renames within a file; a file rename surfaces as a broken
reference at the lint layer.

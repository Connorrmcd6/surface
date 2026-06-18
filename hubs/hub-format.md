---
summary: The hub document format and the minimal-diff frontmatter editor used by verify.
anchors:
  - claim: >
      A hub is a `---`-fenced YAML frontmatter block followed by a markdown body; `at:` is a
      scalar or a list, hash is optional until verified, and unknown fields are rejected — though
      forward-declared fields (`refs`, `covers`) are accepted and stored but inert in the verdict.
    at: surf-core/src/hub.rs > parse_hub
    hash: e97cc54f48d3
  - claim: >
      verify writes hashes back surgically: set_anchor_hash locates the Nth anchor item and
      replaces/inserts only its hash line, so an unchanged hash is byte-identical.
    at: surf-core/src/hub.rs > set_anchor_hash
    hash: a65d5c324dc5
refs: []
covers:
  - surf-core/src/hub.rs
---

# Hub format

`parse_hub` is the contract everything binds to. Writes go through the line-level editor
(`set_anchor_hash` / `set_anchor_at`) rather than re-serializing, to keep diffs reviewable.

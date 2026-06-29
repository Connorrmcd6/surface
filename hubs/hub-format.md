---
summary: The hub document format and the minimal-diff frontmatter editor used by verify.
anchors:
  - claim: >
      A hub is a `---`-fenced YAML frontmatter block followed by a markdown body; `at:` is a
      scalar or a list, hash is optional until verified, and unknown fields are rejected — though
      forward-declared fields (`refs`, `covers`) are accepted and stored but inert in the verdict.
    at: surf-core/src/hub.rs > parse_hub
    hash: 2:55be573a0ca2
  - claim: >
      verify writes hashes back surgically: set_anchor_hash locates the Nth anchor item and
      replaces/inserts only its hash line, so an unchanged hash is byte-identical.
    at: surf-core/src/hub.rs > set_anchor_hash
    hash: 2:a65d5c324dc5
refs: []
covers:
  - surf-core/src/hub.rs
---

# Hub format

A hub is the unit every command reads and writes: a `---`-fenced YAML frontmatter block (the
machine-checkable `anchors`) followed by a markdown body (the prose a human or agent reads).
`parse_hub` is the contract everything else binds to — its shape is why `at:` can be a scalar or a
list, why `hash` is optional until verified, and why unknown fields are rejected (so a typo can't
masquerade as a new field) while the forward-declared `refs`/`covers` are accepted but inert.

**The distinction that drives the design:** a human reviews every write, so edits must be
*surgical*. Writes go through the line-level editor (`set_anchor_hash` / `set_anchor_at`) rather
than re-serializing the frontmatter — re-serializing would reorder keys and reflow scalars, burying
the one changed line in a noisy diff. An unchanged hash rewrite is therefore byte-identical.

**Boundary:** this module is pure parsing and text editing — it resolves no anchors and computes no
hashes; it only produces the structure [`lint`](./cli-lint.md)/[`check`](./cli-check.md) act on.

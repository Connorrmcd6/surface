---
summary: The `at:` anchor grammar parser - qualified paths plus the `@N` positional selector.
anchors:
  - claim: >
      An anchor is a file path followed by `>`-separated symbol segments; a segment may carry
      a 1-based `@N` positional suffix for genuine name collisions. Empty/zero/missing parts
      are typed parse errors.
    at: surf-core/src/anchor.rs > parse_anchor
    hash: 2:a41aca45f340
refs: []
---

# Anchor grammar

`parse_anchor` turns an `at:` string into a `file` plus ordered `Segment`s. It is pure string
parsing - resolution against a real tree happens later in `resolve`.

---
summary: Supported languages, file-extension detection, and bundled tree-sitter grammars.
anchors:
  - claim: >
      Language is detected purely by file extension (ts/tsx/mts/cts, js/jsx/mjs/cjs, rs,
      py/pyi, go); an unknown extension yields None and the anchor is treated as unsupported.
    at: surf-core/src/lang.rs > Lang > from_path
    hash: 2:fabba17dc0f9
  - claim: >
      The set of Lang variants is enumerated identically across from_path (extension → variant),
      tree_sitter_language (variant → grammar), and family (variant → Family). Adding or removing a
      language must touch all three in lockstep; this claim is stale if any one of them changes on
      its own — the cross-file contract that keeps "adding a language is additive" honest.
    at:
      - surf-core/src/lang.rs > Lang > from_path
      - surf-core/src/lang.rs > Lang > tree_sitter_language
      - surf-core/src/lang.rs > Lang > family
    hash: c93ef85daf46
refs: []
---

# Languages

`Lang` maps extensions to a bundled, version-pinned tree-sitter grammar. Adding a language is
additive: one `Lang` variant, an extension arm, a grammar, and a `Family`.

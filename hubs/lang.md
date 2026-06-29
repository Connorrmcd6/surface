---
summary: Supported languages, file-extension detection, and bundled tree-sitter grammars.
anchors:
  - claim: >
      Language is detected purely by file extension (ts/tsx/mts/cts, js/jsx/mjs/cjs, rs,
      py/pyi, go); an unknown extension yields None and the anchor is treated as unsupported.
    at: surf-core/src/lang.rs > Lang > from_path
    hash: 2:e8a03d2b4a80
refs: []
---

# Languages

`Lang` maps extensions to a bundled, version-pinned tree-sitter grammar. Adding a language is
additive: one `Lang` variant, an extension arm, a grammar, and a `Family`.

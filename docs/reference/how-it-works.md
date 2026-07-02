---
title: How the gate works
description: Locate, canonicalize, hash, compare - the four steps behind surf check, and the versioned JSON seam every plugin reads.
---

The gate runs in four steps.

1. **Locate.** tree-sitter parses the file and resolves the `at:` path (a qualified `file > A > B`
   path, with `@N` for genuine name collisions) to the exact node span. A scope is treated as a
   *set* of nodes, so a type and its `impl`/methods - which share a name - disambiguate by path:
   `Type` alone is ambiguous, `Type > method` is unique. In Python the path also resolves
   non-callables: module constants, type aliases, and class attributes.
2. **Canonicalize.** Walk that span's syntax tree into a token stream. Whitespace and comments
   aren't in the tree, so they drop out for free; operators, keywords, and literal *values* are
   kept verbatim. Identifiers split into two kinds: a **bound** name (the symbol's own name,
   parameters, locals, loop/destructuring binders) is alpha-renamed to a positional placeholder,
   so a *consistent* local rename yields the same tokens; a **free** name (external members, call
   targets, types, enum/constant references, object keys, decorators) is kept verbatim, so
   re-pointing a span at a *different* symbol - `PointsTier.TIER_1` → `TIER_2`, `getHighest` →
   `getLowest`, `@cache` → `@lru_cache` - changes the hash even when the name occurs once. (This
   bound/free split is the **v2** recipe; see [Hash recipes](./hash-recipes.md).)
3. **Hash.** SHA-256 of that stream, truncated to 12 hex. A list `at:` combines its sites into one
   hash, so the claim is stale if *any* listed span changes.
4. **Compare** against the stamp stored in the frontmatter (written by `surf verify`). The stamp
   carries its recipe - a v2 stamp is prefixed `2:`, a bare hex stamp is an older v1 - and is
   verified under *its own* recipe, so existing v1 stamps keep passing until `surf verify` upgrades
   them. Equal → pass; different → block.

Quiet on cosmetics, loud on logic - and **reproducible**, because the parser ships *inside* the
binary and is version-pinned. There is no separate formatter or language server in CI to skew the
result.

A claim can opt a narrower scope with `ignore_literals: true`, which excludes string-literal
*content* from its hash (a copy edit no longer re-opens the gate; logic still does). The stored
hash is computed in that mode, so the option lives on the claim.

## The JSON seam

`surf check --format json` is the seam every optional layer reads. The payload is a **versioned
envelope**:

```json
{
  "version": 1,
  "divergences": [
    {
      "hub": "hubs/auth.md",
      "claim": "refresh rotation is single-use; reuse triggers global logout",
      "at": "src/auth/refresh.ts > rotateRefreshToken",
      "kind": "changed",
      "old_hash": "9b1c33ade8f1",
      "new_hash": "4d5e6f2a0b7c",
      "new_code": "function rotateRefreshToken(...) { ... }",
      "prose": "refresh rotation is single-use; reuse triggers global logout",
      "magnitude": "small"
    }
  ]
}
```

Per diverged claim: `hub`, `claim`, `at`, `kind` (`changed` | `unverified` | `unresolvable`),
`old_hash`, `new_hash`, `old_code`, `new_code`, `prose`, `magnitude`, and a `detail` string on an
unresolvable claim. `magnitude` (`small` / `medium` / `large`) is advisory triage only - it helps a
human decide which blocked claim to read first, and it **never** affects pass/fail.

**Stability.** `version` is the contract version. Within a major version the shape is
**additive-only**: new optional fields may appear, but existing fields are never removed, renamed,
or repurposed. A breaking change bumps `version`. Consumers should read `.divergences` and tolerate
unknown fields.

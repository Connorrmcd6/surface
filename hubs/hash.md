---
summary: AST-canonical hashing — quiet on cosmetics, loud on logic — and per-claim combination.
anchors:
  - claim: >
      The canonical token stream drops comments and keeps operators, keywords, and literal
      values verbatim. Under v1 every identifier is alpha-renamed to a positional placeholder;
      under v2 (the bound/free split) only bound identifiers are alpha-renamed and free
      identifiers are emitted verbatim, so a consistent local rename stays quiet while
      re-pointing at a different external symbol is loud even when it occurs once. A
      member-access name is kept verbatim even when its text collides with a bound local. The
      per-claim ignore_literals option drops string-literal content so a copy edit doesn't
      re-open the gate.
    at: surf-core/src/hash.rs > emit
    hash: 2:ac52f23c70c8
  - claim: >
      Under v2 only names bound inside the span are alpha-renamed — the symbol's own name,
      parameters, locals, loop/range/comprehension variables, with/catch aliases, generic
      params, and destructuring binders. Detection is tree-sitter-only and fail-closed: a
      position not positively recognized as a binding defaults to free (verbatim).
    at: surf-core/src/hash.rs > collect_bound
    hash: 2:20fd6172cf43
  - claim: >
      The property/field component of a member-access expression is kept verbatim even when its
      text collides with a bound local, since that position can never be the binding — matched
      structurally per family (kind + parent kind + the parent's named field).
    at: surf-core/src/hash.rs > is_member_access_name
    hash: 2:de12739eeb09
  - claim: >
      Identifier node kinds are enumerated per language family; only these are alpha-renamed,
      everything else (operators, keywords, literals) is kept.
    at: surf-core/src/hash.rs > is_identifier
    hash: 2:25ca2f219009
  - claim: >
      A claim's hash is the combination of its per-site hashes — a single site is the identity,
      multiple sites combine order-sensitively, so the claim is stale if any listed span changes.
    at: surf-core/src/hash.rs > combine_site_hashes
    hash: 2:cbbbbc3b2237
refs:
  - ./cli-verify.md
---

# Canonical hashing

**The whole design in one line:** quiet on cosmetics, loud on logic. The fingerprint is computed
over `emit`'s canonical token stream, hashed with SHA-256 (12 hex). This is the only signal the
gate compares; `Magnitude` alongside it is advisory and never gates.

"Canonical" is what makes the gate trustworthy: comments are dropped and identifiers are
alpha-renamed to positional placeholders, so a consistent rename or a reflow doesn't trip a claim,
while operators, keywords, and literal values stay verbatim, so a real logic edit does. Which
identifiers get alpha-renamed is the recipe's job: v1 renames them all; v2 (the bound/free split,
#77) renames only **bound** names — params, locals, the symbol's own name — and emits every
**free** identifier (external members, call targets, types, constants, decorators) verbatim, so
re-pointing a span at a different symbol is loud even when the name occurs once. A claim's hash is
the order-sensitive combination of its per-site hashes, which is what lets one multi-site claim go
stale when any of its spans changes. See [hash recipes](../docs/reference/hash-recipes.md) for the
versioned canonicalization and migration.

**Boundary:** hashing decides *that* something changed, never *whether the prose is still true* —
that judgment is the human's at [`surf verify`](./cli-verify.md).

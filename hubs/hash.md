---
summary: AST-canonical hashing — quiet on cosmetics, loud on logic — and per-claim combination.
anchors:
  - claim: >
      The canonical token stream drops comments, alpha-renames identifiers to positional
      placeholders (consistent rename → same tokens; swapping two names → different), and
      keeps operators, keywords, and literal values verbatim. Exceptions kept verbatim: a
      Python decorator's name (so `@cache` → `@lru_cache` is caught), and — under the v2
      recipe — a member-access name (the property/field of `obj.foo`/`pkg.Bar`), so
      re-pointing at a different external symbol is caught even when it occurs once. The
      per-claim ignore_literals option drops string-literal content so a copy edit doesn't
      re-open the gate.
    at: surf-core/src/hash.rs > emit
    hash: 2:1a93c8f4b8d9
  - claim: >
      Identifier node kinds are enumerated per language family; only these are alpha-renamed,
      everything else (operators, keywords, literals) is kept.
    at: surf-core/src/hash.rs > is_identifier
    hash: 2:ac8c69676a07
  - claim: >
      A claim's hash is the combination of its per-site hashes — a single site is the identity,
      multiple sites combine order-sensitively, so the claim is stale if any listed span changes.
    at: surf-core/src/hash.rs > combine_site_hashes
    hash: 2:a81ab78387c2
refs: []
---

# Canonical hashing

The fingerprint is computed over `emit`'s token stream, hashed with SHA-256 (12 hex). This is
the signal the gate compares; `Magnitude` alongside it is advisory only and never gates.

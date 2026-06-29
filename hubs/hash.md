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

**The whole design in one line:** quiet on cosmetics, loud on logic. The fingerprint is computed
over `emit`'s canonical token stream, hashed with SHA-256 (12 hex). This is the only signal the
gate compares; `Magnitude` alongside it is advisory and never gates.

"Canonical" is what makes the gate trustworthy: comments are dropped and identifiers are
alpha-renamed to positional placeholders, so a consistent rename or a reflow doesn't trip a claim,
while operators, keywords, and literal values stay verbatim, so a real logic edit does. The
exceptions exist because a name *is* the logic there — a Python decorator, and (v2) a
member-access name — so swapping one is caught even when it occurs once. A claim's hash is the
order-sensitive combination of its per-site hashes, which is what lets one multi-site claim go
stale when any of its spans changes.

**Boundary:** hashing decides *that* something changed, never *whether the prose is still true* —
that judgment is the human's at [`surf verify`](./cli-verify.md).

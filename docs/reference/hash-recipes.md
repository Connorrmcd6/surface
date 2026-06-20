---
title: Hash recipes
description: The versioned canonicalization recipes behind every stored stamp — what each one does, how stamps are labelled, and how to migrate.
---

A **stamp** is the value `surf verify` writes into a claim's `hash:` field and `surf check`
compares against. It is produced by a **recipe**: the exact rules for turning a resolved span into
a canonical token stream (see [How the gate works](./how-it-works.md), step 2). Changing those
rules changes the output for unchanged code, which would silently invalidate every stamp in the
wild — so each recipe has a number, and every stamp records the recipe that produced it.

## Stamp format

```
hash: 2:f1075e760a17     # v2 stamp — explicit prefix
hash: f1075e760a17       # bare 12-hex — implicitly v1 (written before recipes were numbered)
```

`surf check` reads the prefix, verifies the span under that recipe, and:

- **matches** → passes. If the stamp is v1, it adds a one-line nudge inviting `surf verify` to
  upgrade (so the span gains newer protections).
- **differs** → blocks, exactly as before.
- **unrecognized prefix** (e.g. a `3:` stamp written by a newer surf) → fails closed: an
  unverifiable stamp is never treated as clean.

New stamps are always written under the current recipe (**v2**).

## Migration

Upgrading surf does **not** mass-flag your repo. v1 stamps keep verifying in v1 mode until you run:

```
surf verify
```

once, which re-stamps every anchor under the current recipe — including v1 anchors whose hash
still matches (the one narrow case where `verify` rewrites an otherwise-unchanged stamp). After
that single pass the whole repo is on v2.

> Forced re-verify is deliberately *not* automatic on upgrade. `verify` stamps whatever the code
> is *now*; if a repo already contains drift that v1 missed, a blind re-stamp would launder it
> green. v1-compat keeps the gate honest *through* the migration — `check` can still tell
> "unchanged under the old recipe" (pass) from "actually changed" (block).

## Recipes

### v1 — original (surf ≤ 0.6.x; bare-hex stamps)

Walk the resolved span's syntax tree into tokens:

- whitespace and comments are absent from the tree → ignored;
- every **identifier** is alpha-renamed to a positional placeholder (`#0`, `#1`, …) in order of
  first occurrence — a *consistent* rename hashes identically, swapping two names does not;
- operators, keywords, punctuation, and literal **values** are kept verbatim;
- a Python **decorator name** is kept verbatim (`@cache` → `@lru_cache` is loud).

SHA-256 of the token stream, truncated to 12 hex.

**Known blind spot (#77):** because *every* identifier is alpha-renamed, re-pointing a span at a
different single-occurrence external symbol (`PointsTier.TIER_1` → `TIER_2`, `b.Del` → `b.Keep`)
yields a byte-identical stream — the claim's prose silently becomes false while the gate stays
green.

### v2 — member-access names verbatim (surf ≥ 0.7.0; `2:` prefix)

v1, plus one rule: the **property/field component of a member-access expression** is kept verbatim
(`kind:text`) instead of alpha-renamed. These positions name an *external* member, never a local
binding, so emitting them verbatim distinguishes "re-pointed at a different symbol" (loud) from
"renamed my own local" (still quiet — rename tolerance is preserved). Per family:

| Family | Member-access position |
|---|---|
| TypeScript | `property_identifier` / `private_property_identifier` as the `property` of a `member_expression` |
| Go | `field_identifier` as the `field` of a `selector_expression` |
| Rust | `field_identifier` as the `field` of a `field_expression` |
| Python | the `attribute` identifier of an `attribute` node |

Everything else is identical to v1, so v1 ≡ v2 minus this single rule — a member-access-free span
hashes the same under both. This closes the #77 blind spot for member accesses (every reported
reproduction). Re-pointing at a non-member free identifier — a bare `Enum::VARIANT` path, a renamed
imported function called by bare name — is **not** yet covered; that is the full bound/free split
tracked in [#77](https://github.com/Connorrmcd6/surface/issues/77).

## Policy (for maintainers)

- **Any** change to canonical output is a new recipe number — no exceptions. An innocent-looking
  refactor of the tokenizer that changes one byte of output is silently a new recipe wearing an old
  number, which corrupts every stamp in the wild. The golden fixtures in
  `surf-core/tests/golden_hash.rs` pin each recipe's output (v1 and v2 digests for representative
  symbols per language) precisely to make that break loud.
- A recipe is kept as a verification mode only while it is expressible as a flag over the current
  code (v1 ≡ v2 with the member-access rule off — one branch, no frozen copy). The N-1 support
  policy and the broader version-table governance are tracked in #77.

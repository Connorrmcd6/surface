---
title: Hash recipes
description: The versioned canonicalization recipes behind every stored stamp - what each one does, how stamps are labelled, and how to migrate.
---

A **stamp** is the value `surf verify` writes into a claim's `hash:` field and `surf check`
compares against. It is produced by a **recipe**: the exact rules for turning a resolved span into
a canonical token stream (see [How the gate works](./how-it-works.md), step 2). Changing those
rules changes the output for unchanged code, which would silently invalidate every stamp in the
wild - so each recipe has a number, and every stamp records the recipe that produced it.

## Stamp format

```
hash: 2:f1075e760a17     # v2 stamp - explicit prefix
hash: f1075e760a17       # bare 12-hex - implicitly v1 (written before recipes were numbered)
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

once, which re-stamps every anchor under the current recipe - including v1 anchors whose hash
still matches (the one narrow case where `verify` rewrites an otherwise-unchanged stamp). After
that single pass the whole repo is on v2.

> Forced re-verify is deliberately *not* automatic on upgrade. `verify` stamps whatever the code
> is *now*; if a repo already contains drift that v1 missed, a blind re-stamp would launder it
> green. v1-compat keeps the gate honest *through* the migration - `check` can still tell
> "unchanged under the old recipe" (pass) from "actually changed" (block).

## Recipes

### v1 - original (surf ≤ 0.6.x; bare-hex stamps)

Walk the resolved span's syntax tree into tokens:

- whitespace and comments are absent from the tree → ignored;
- every **identifier** is alpha-renamed to a positional placeholder (`#0`, `#1`, …) in order of
  first occurrence - a *consistent* rename hashes identically, swapping two names does not;
- operators, keywords, punctuation, and literal **values** are kept verbatim;
- a Python **decorator name** is kept verbatim (`@cache` → `@lru_cache` is loud).

SHA-256 of the token stream, truncated to 12 hex.

**Known blind spot (#77, closed by v2):** because *every* identifier is alpha-renamed, re-pointing a
span at a different single-occurrence external symbol (`PointsTier.TIER_1` → `TIER_2`, `b.Del` →
`b.Keep`) yields a byte-identical stream - the claim's prose silently becomes false while the gate
stays green. This is exactly what the v2 bound/free split fixes.

### v2 - the bound/free split (surf ≥ 0.7.0; `2:` prefix)

v1 alpha-renames *every* identifier, which is its blind spot: an identifier occurring once maps to
the same placeholder no matter what it names, so re-pointing a span at a *different* single-occurrence
external symbol is byte-identical and silently passes.

v2 fixes this by splitting identifiers into **bound** and **free**:

- **Bound** - names *declared inside the hashed span*: the symbol's own name, parameters, locals,
  loop/range/comprehension variables, `with`/`catch` aliases, generic parameters, and destructuring
  binders. These are **alpha-renamed** exactly as in v1, so a consistent local rename still hashes
  identically - rename tolerance (§6.1) is preserved.
- **Free** - everything else: external members, call targets, types, enum/constant references,
  object/destructuring keys, decorator names, JSX tags. These are emitted **verbatim** (`kind:text`),
  so re-pointing at a different symbol is loud *even when the name occurs once*.

This closes the #77 class in general, not just for member accesses: `PointsTier.TIER_1` → `TIER_2`,
`getHighest` → `getLowest`, a bare `helper(x)` → `other(x)`, a parameter type `Foo` → `Bar`, and an
object key `{ alpha }` → `{ beta }` all now change the hash. It also **subsumes** the two special
cases the older design carried - a decorator name (#8) and a member-access name (the #140 first cut)
are simply free identifiers now; no dedicated branch is needed for either. (The member-access
positions keep one dedicated check so they stay verbatim even when their text collides with a bound
local - `x` the parameter vs `obj.x` the field - since that position can never *be* the binding.)

Binding detection is tree-sitter-only - there is no scope analysis - so it is **fail-closed**:
a position not positively recognized as a binding defaults to *free* (verbatim). The two error
directions are not symmetric: misclassifying bound→free is a *visible* false positive (a benign
rename trips the gate, a human sees it); free→bound is the *invisible* miss this whole recipe exists
to prevent. So when in doubt, free wins.

**Binding positions, per family** (the tables `surf-core/src/hash.rs` `bind_here` encodes):

| Family | Bound positions |
|---|---|
| Rust | `function_item`/`function_signature_item` name; `parameter`/`let_declaration`/`for_expression`/`let_condition` patterns; `closure_parameters`; `type_parameters` |
| TypeScript | function/method/signature names; `required_parameter`/`optional_parameter` patterns; `variable_declarator` name; `arrow_function` single param; `for_in_statement` left; `catch_clause` parameter; `type_parameters` |
| Python | `function_definition` name; `parameters`/`lambda_parameters` (default *values* excluded); `assignment`/`augmented_assignment`/`for_statement`/`for_in_clause` left; `with`/`as` targets |
| Go | function/method/`var`/`const`/type-parameter names; `parameter_declaration` names (incl. grouped `a, b int`); `short_var_declaration`/`range_clause` left |

**Member-access positions kept verbatim even on a bound-name collision:**

| Family | Member-access position |
|---|---|
| TypeScript | `property_identifier` / `private_property_identifier` as the `property` of a `member_expression` |
| Go | `field_identifier` as the `field` of a `selector_expression` |
| Rust | `field_identifier` as the `field` of a `field_expression` |
| Python | the `attribute` identifier of an `attribute` node |

**Accepted approximation (the residue).** Without scope analysis, a match-arm / pattern identifier
is indistinguishable from a unit-variant *reference* (`Some(x)` binds `x`; `None` references a
variant - same syntax). v2 leaves all such pattern identifiers **free**. Fail-closed cuts both
ways: a unit-variant swap in a match arm is *caught* (the safe direction), but renaming a match-arm
catch-all *binding* is also loud - an accepted false positive, not a bug. This is the one benign
edit class v2 does not keep silent; a future scope-aware pass could reclaim it. The limit is pinned
in `surf-core/tests/differential_hash.rs`.

## Version table

surf keeps an explicit table of every recipe ever shipped, so any stamp's recipe is always
identifiable and every dropped recipe errors with a remedy rather than a generic mismatch.

| Recipe | Stamp form | Shipped | Status | Remedy if rejected |
|---|---|---|---|---|
| v1 | bare 12-hex | surf ≤ 0.6.x | **supported** (N-1) until 0.8.0 | run `surf verify` to upgrade to v2 |
| v2 | `2:` + 12-hex | surf ≥ 0.7.0 | **current** | - |
| `N:` for unknown N | `N:` + hex | a newer surf | rejected (fails closed) | upgrade surf to a build that knows recipe N |

- **Identification never expires.** The prefix is plain data; any future surf can name the recipe of
  any stamp even after the recipe's verification code is deleted. A bare hex stamp is, and always
  will be, v1.
- **N-1 support, at most one legacy mode.** surf verifies the current recipe and exactly one back.
  v1 compatibility ships in 0.7.0 and is **removed in 0.8.0**; after that a bare-hex stamp is a hard,
  named error ("stamped by surf < 0.7 - re-stamp with `surf verify`, or check with surf 0.7.x
  first"), never a silent DIVERGED. A legacy recipe is retained *only* while it is expressible as a
  mode of the current code (v1 ≡ v2 with "every identifier bound" - one flag, no frozen copy). If a
  future recipe cannot express its predecessor that cheaply, that is the signal to drop compat and
  require stepping through an intermediate release.

## Policy (for maintainers)

- **Any** change to canonical output is a new recipe number - no exceptions. An innocent-looking
  refactor of the tokenizer that changes one byte of output is silently a new recipe wearing an old
  number, which corrupts every stamp in the wild. Two layers make that break loud:
  - **Golden fixtures** (`surf-core/tests/golden_hash.rs`) pin each recipe's exact digest for
    representative symbols per language - both v1 (frozen forever) and v2.
  - **Differential harness** (`surf-core/tests/differential_hash.rs`) re-runs the v1-vs-v2 A/B on
    every build: zero benign-rename regressions, 100% catch on the semantic (free-swap) corpus. Any
    future recipe change reruns the same gate.
- The recipe's rules are **dogfooded**: claims in `hubs/hash.md` are anchored to the canonicalization
  code itself (`emit`, `collect_bound`, `is_member_access_name`), so editing the tokenizer without
  updating this contract turns surf's own gate red.
- The external git-history replay over real corpora (prometheus for Go, nansen-python-sdk for
  Python, surface itself for Rust, a large public TS repo) named in #77 runs out-of-tree against
  release binaries; the in-tree harness above is its always-on, deterministic counterpart.

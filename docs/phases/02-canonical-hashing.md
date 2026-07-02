# Phase 2 - AST-canonical hashing + advisory magnitude

**Goal:** given a resolved node (Phase 1), produce a canonical hash that is **quiet on
rename / reformat / comments** and **loud on a flipped operator** (§6.1, table row 4) - the
exact sensitivity profile a correctness gate needs.

**Proposal refs:** §6.1 (canonical AST hash), §6.2 (why not similarity; magnitude is advisory only).

**Depends on:** Phase 1.

**Status:** done

> `surf-core/src/hash.rs`. Canonical token stream: comments dropped, whitespace absent
> (not in tree), identifiers **alpha-renamed** to positional placeholders (`#0`, `#1`, …) so
> a consistent rename is quiet but a *swap* of two names is loud; operators, keywords,
> punctuation, and literal values kept verbatim. Hash = SHA-256 truncated to 12 hex chars.
> `Magnitude` (Small/Medium/Large via token Levenshtein) is advisory only - there is no code
> path from it to a verdict (§6.2). `resolve.rs` refactored to expose `parse_tree` /
> `resolve_node` so the hasher reuses resolution.

## Steps

1. **Canonical serialization:** walk the subtree's *named* nodes and emit a canonical token
   stream of node-kinds plus the operator/literal tokens that carry meaning. Whitespace,
   comments, and trivia are not in the tree, so they fall out for free. Hash the stream
   with a stable algorithm (SHA-256), surface a short hex (e.g. `9b1c33a`, matching the §6
   example).
2. **Rename-quietness:** normalize identifier *positions*, not names (§6.1) - replace
   identifiers with positional placeholders so a pure rename yields the **same** hash, while
   an operator or structural change does not. This is what keeps the gate from firing on the
   commonest refactor.
3. **Advisory tree-edit magnitude** (§6.2): a cheap structural diff between two subtrees →
   a small integer or category (`small` / `rename-shaped` / `large`). Ships in the JSON
   report only. **It never gates.** The forbidden rule, explicitly: never "fail only if hash
   changed *and* magnitude > threshold" - that would hide the single-operator logic flip,
   the highest-value catch.

## Files touched
- `surf-core/src/hash.rs` (canonical serialize + hash + magnitude)
- reuse `fixtures/auth.ts`, `fixtures/auth.rs` from Phase 1; add edited variants

## Verify
Golden tests, both languages:
- (a) reformat / whitespace / comment-only change → **same** hash.
- (b) pure rename → **same** hash.
- (c) `+`→`-`, `<`→`<=`, deleted `await` → **different** hash.
- magnitude is populated and plausible (rename → `rename-shaped`/small; large rewrite → `large`) but is asserted to be *reporting only* - no code path lets it affect pass/fail.

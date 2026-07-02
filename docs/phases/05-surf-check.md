# Phase 5 - `surf check` (the gate - the one load-bearing piece)

**Goal:** AST-canonical-hash each anchored span (Phase 2), compare to the stored per-anchor
`hash` in frontmatter, and **block only on a documented span that has diverged** (§9.1.3).
Emit `--format json` - the seam every future plugin attaches to (§5).

**Proposal refs:** §5 (the boundary - JSON is the contract), §6 (per-symbol, not per-file), §6.3 (list `at:`), §9.1.3 (the gate), §9.1 (CI step everyone gets wrong).

**Depends on:** Phases 2, 3.

**Status:** done

> Report structs in `surf-core/src/report.rs` (`Divergence`, `DivergenceKind` =
> Changed/Unverified/Unresolvable). Gate logic in `surf-cli/src/check.rs`: resolve → hash →
> compare to stored. Verdict is deterministic (stored hash vs computed); `old_code` +
> `magnitude` are best-effort enrichment via `git show <base>:<path>` (default `HEAD`,
> `--base` to override) and never affect pass/fail. `--format human|json`. Exit non-zero on
> any divergence. A claim with no stored hash → `Unverified` block (drives the verify loop);
> an anchor that no longer resolves → `Unresolvable` block (run lint). Per-symbol verified:
> editing an un-anchored function in the same file stays green.

## Steps

1. For each anchor: resolve (P1) → canonical hash (P2) → compare to the stored `hash`. A
   list `at:` is stale if **any** listed span diverges (§6.3).
2. **Exit codes:** `0` clean, non-zero on any divergence (this is what blocks CI).
3. **`--format json`** (§5) - the frozen contract. Per diverged claim emit:
   `{ hub, claim, at, old_hash, new_hash, old_code, new_code, prose, magnitude }`.
   Everything optional (reviewer plugin, etc.) plugs in here; the core never depends on it.
   Keep the human (default) format readable; JSON is opt-in.
4. **CI scoping - the step everyone gets wrong (§9.1):** the gate hashes the *working-tree*
   span and compares to the hash committed in frontmatter. It needs the checkout, **not**
   full history - do **not** `fetch-depth: 0`. The only thing a base ref buys is scoping
   (re-check only anchors whose files changed in the PR); a shallow fetch of the merge base
   covers that.

## Files touched
- `surf-cli/src/check.rs`
- `surf-cli/src/main.rs` (wire subcommand + `--format`)
- JSON report structs (in `surf-core` so plugins/WASM can reuse them)

## Verify
- Seed `hubs/auth.md`; change a documented span → `check` exits non-zero and the JSON names
  the right claim with old/new code and a populated `magnitude`.
- Change an **un-anchored** span in the *same file* → `check` stays green. (Per-symbol, not
  per-file - the core promise of §6.)
- `--format json` output validates against the documented contract.

# Phase 6 - `surf verify`

**Goal:** the human escape hatch (§8) - re-hash after a human confirms the prose still
holds. `--follow` re-points a renamed anchor and re-hashes in one step.

**Proposal refs:** §8 (escape hatch, "I looked, still true"), §6.4 (`--follow` for renames), §9.1.4.

**Depends on:** Phase 5.

**Status:** done

> `surf-cli/src/verify.rs` (+ `main.rs`). `surf verify [<at>] [--follow]` re-hashes anchors
> and writes the hash back. Writes are **surgical** via `surf-core::set_anchor_hash` /
> `set_anchor_at` (minimal-diff line editor in `hub.rs`) - only the touched line changes, and
> an unchanged hash is a no-op (byte-identical). `--follow` re-points a renamed single-site,
> single-segment anchor (via `find_renamed`) then re-hashes. Unresolvable anchors are skipped
> (exit non-zero) unless `--follow` recovers them.
>
> **Model fix carried in from Phase 5:** a list `at:` now produces **one combined hash per
> claim** (`combine_site_hashes`, single-site = identity), so check/verify treat a multi-site
> claim as one unit (stale if any span changes) - previously check looped per-site against a
> single stored hash, which was wrong.

## Steps

1. `surf verify [selector]` (hub / anchor selector): re-resolve (P1), re-hash (P2), and
   **write the new `hash` back into the frontmatter** - the explicit "I looked, still true".
   With symbol-scoped AST anchoring this should be needed only *occasionally*; running it on
   most PRs is a smell that anchors are too coarse (§8).
2. `--follow`: when the symbol was renamed, update the `at:` path to the new
   name/location, then re-hash - clearing the Phase 4 rename warning in one step (§6.4).
3. Preserve frontmatter key order / formatting on write as much as practical (minimize diff
   noise; authors will review these writes).

## Files touched
- `surf-cli/src/verify.rs`
- `surf-cli/src/main.rs` (wire subcommand + `--follow`)
- frontmatter writer (in `surf-core/src/hub.rs`, paired with the Phase 3 parser)

## Verify
- Divergence → `surf verify <anchor>` → `surf check` now clean, and the diff is only the `hash` field.
- Rename → `surf verify --follow` updates `at:` to the new symbol and clears the lint warning.
- A no-op verify (span unchanged) leaves the file byte-identical.

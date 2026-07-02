# Phase 4 - `surf lint`

**Goal:** frontmatter is well-formed; every `at:` resolves to **exactly one** node;
renames are handled per §6.4. Composes Phases 1–3.

**Proposal refs:** §9.1.2 (lint), §6.3 (exactly-one resolution), §6.4 (renames are an MVP problem), §11.3 (granularity guidance - minimal here).

**Depends on:** Phases 1, 3.

**Status:** done

> `surf-cli/src/lint.rs` (+ `main.rs` wiring). `lint_workspace` produces `Finding`s
> (Block/Warn); `run` prints them and sets the exit code (block → failure, warn-only → 0).
> **Rename detection is hash-based, not git-based** (deviation from the doc's git+similarity
> sketch): `surf-core/src/rename.rs::find_renamed` walks every definition and matches the
> claim's stored hash - because the canonical hash alpha-renames identifiers, a renamed-but-
> unchanged symbol still matches. Deterministic, no network, no git. A *file* rename makes
> the path unreadable and surfaces as a Block ("cannot read … (file moved or removed?)").
> Needs `collect_all_defs` (resolve.rs) + `hash_node` (hash.rs).

## Steps

1. Discover `surf.toml`, load hubs via the glob, parse frontmatter (Phase 3).
2. For each anchor, resolve via Phase 1:
   - `Ambiguous` → error, tell the author to add `@N`.
   - `NotFound` → see rename handling below.
3. **Rename handling (§6.4)** - hits the MVP constantly, not deferrable:
   - **Renamed but clearly present** (git rename detection + high AST similarity of the
     moved node via the Phase 2 magnitude) → **warn**, not block; suggest
     `surf verify --follow` to re-point and re-hash in one step.
   - **Genuinely vanished** (no plausible match) → **block**: a claim now points at nothing.
4. Human-readable diagnostics (which hub, which claim, which `at:`). Non-zero exit on any
   block-level finding; warnings alone keep exit 0.

## Files touched
- `surf-cli/src/lint.rs`
- `surf-cli/src/main.rs` (wire subcommand)
- git rename detection helper (shell out to `git` or a plumbing call) - likely `surf-core` or a cli helper

## Verify
Fixture hub + fixture source covering each case:
- clean → exit 0, no findings.
- ambiguous (collision, no `@N`) → block, message names the fix.
- vanished symbol → block.
- renamed symbol → **warn** (exit 0), message suggests `verify --follow`.
Assert exit codes for each.

# Surface MVP - Phases

Phased build of the Surface MVP. Read [`OVERVIEW.md`](./OVERVIEW.md) first - it carries the
context, locked decisions, and scope guardrails every phase assumes. The product spec is
[`../surface-proposal.md`](../surface-proposal.md).

Each phase file is self-contained: **Goal**, **Proposal refs**, **Depends on**, **Steps**,
**Files touched**, **Verify**, **Status**. Work them in dependency order; update the Status
line as you go.

## Status tracker

| Phase | File | Status |
|---|---|---|
| 0 | [`00-toolchain-scaffold.md`](./00-toolchain-scaffold.md) | done |
| 1 | [`01-anchor-resolution.md`](./01-anchor-resolution.md) | done |
| 2 | [`02-canonical-hashing.md`](./02-canonical-hashing.md) | done |
| 3 | [`03-hub-format.md`](./03-hub-format.md) | done |
| 4 | [`04-surf-lint.md`](./04-surf-lint.md) | done |
| 5 | [`05-surf-check.md`](./05-surf-check.md) | done |
| 6 | [`06-surf-verify.md`](./06-surf-verify.md) | done |
| 7 | [`07-distribution.md`](./07-distribution.md) | done |

## Language support
TypeScript/TSX, **JavaScript/JSX**, Rust, **Python**, **Go**. JavaScript reuses the TS family
via the TSX grammar (which parses plain JS and JSX) - zero new resolver/hash code. Python rides
the generic scope-set resolver; Go has a dedicated resolver (`resolve_go`) because its symbols
are flat and methods attach by receiver (`Type > Method`). Adding a language is additive across
`lang.rs` / `resolve.rs` / `hash.rs`.

## Locked decisions (see OVERVIEW for rationale)
- **Grammars:** TypeScript + Rust + Python + Go (Surface dogfoods its own Rust source); JS/JSX via the TSX grammar.
- **Layout:** Cargo workspace - `surf-core` (pure, no I/O) + `surf-cli` (clap binary).
- **Rust install:** `rustup` via `curl | sh`, toolchain pinned in `rust-toolchain.toml`.

## Post-MVP additions
- **`surf init`** (`surf-cli/src/init.rs`) - bootstraps a workspace (writes `surf.toml` +
  `hubs/`); the one command that runs before discovery since it creates the marker.
- **`surf new <name>`** (`surf-cli/src/new.rs`) - scaffolds a lint-clean hub under the
  configured hubs dir.

  Both are small authoring-ergonomics extras beyond the 8 phases (lower the §8
  claim-maintenance cost); not in the original proposal scope.

## Scope guardrails - NOT in the MVP
- No `refs` resolver, `surf index`, MCP service, reviewer plugin, or `covers` field.
- No similarity-score gating. Boolean AST hash only; tree-edit magnitude is advisory JSON.
- Gate promises only "named span unchanged since last verified" - never system-wide invariants.

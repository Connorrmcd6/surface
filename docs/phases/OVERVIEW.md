# Surface MVP - Build Plan (Overview)

> Full context for the phased build. Each `NN-*.md` file in this directory is a single,
> self-contained phase; this file is the whole plan for when you need wider context while
> working any one of them. Source of truth for *what* and *why* is `../surface-proposal.md`.

## Context

`docs/surface-proposal.md` specifies **Surface** (`surf`): a deterministic CI gate that
detects when documented code has drifted from the architectural docs ("hubs") describing
it, and blocks the merge. The core hypothesis to test (§9.2): *does a freshness gate
produce durable behavior change?*

This is a **greenfield build**. The working directory contains only `docs/`; it is not a
git repo and **Rust is not installed**. We are on Apple Silicon macOS (`arm64`) with
Homebrew, `git`, and `gh` available.

The MVP is the smallest thing that tests the hypothesis (§9.1): entirely deterministic -
**no LLM, no network, no API key**. We build exactly the five MVP pieces, then stop. The
load-bearing risk is the AST-canonical hashing primitive (§6.1), so we de-risk that first
with raw fixtures before any markdown or CLI surface exists.

## Locked decisions
- **Grammars:** TypeScript + Rust. Rust lets Surface dogfood itself (its own source becomes hubs).
- **Layout:** Cargo workspace - `surf-core` (pure parse/resolve/hash, no I/O) + `surf-cli` (clap binary). Keeps the §10 WASM/IDE reuse path free and makes the core unit-testable in isolation.
- **Rust install:** official `rustup` via `curl | sh` (version-pinned, matches the "version-pinned grammar shipped *in* the binary" reproducibility story better than Homebrew's rolling `rust`).

## Scope guardrails (from the proposal - do NOT build these in MVP)
- No `refs` resolver, no `surf index` catalog, no MCP service, **no reviewer plugin**, no `covers` field (§9.1, §9.3).
- No similarity-score gating - the gate is a boolean AST hash; tree-edit magnitude is **advisory JSON only**, never adjudicates (§6.2).
- The gate promises only "the named span is unchanged since last verified" - **not** system-wide invariants (§7). This must be loud in `--help` / README, never oversold.

## Phase map

| Phase | Title | Depends on |
|---|---|---|
| 0 | Toolchain & workspace scaffold | - |
| 1 | Anchor resolution via tree-sitter | 0 |
| 2 | AST-canonical hashing + advisory magnitude | 1 |
| 3 | Hub format + frontmatter parser | 0 |
| 4 | `surf lint` | 1, 3 |
| 5 | `surf check` (the gate) | 2, 3 |
| 6 | `surf verify` | 5 |
| 7 | Distribution & CI integration | 5 |

Phases 1↔3 are independent and can proceed in parallel after 0.

## Then stop (§9.1)

After Phase 7, **stop and measure** against §9.2's falsifiable criteria over ~6–8 weeks on
1–2 seeded high-churn/high-stakes domains (the proposal's own auth module is a natural
first hub since we dogfood Rust). Key kill-signal metric: the `verify`-without-prose-edit
(rubber-stamp) rate. Do not build deferred items (§9.3) until their named trigger fires.

## Critical files (to be created)
- `Cargo.toml` (workspace) · `rust-toolchain.toml`
- `surf-core/src/{lib,lang,anchor,resolve,hash,hub,config}.rs`
- `surf-cli/src/{main,lint,check,verify}.rs`
- `fixtures/` (TS + Rust resolution/hashing goldens)
- `.github/workflows/ci.yml` · `action.yml` · `.pre-commit-hooks.yaml` · `install.sh`
- `hubs/` (first dogfood hub once the core works)

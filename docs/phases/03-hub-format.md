# Phase 3 - Hub format + frontmatter parser (the contract)

**Goal:** define and parse the hub document - frontmatter schema + prose body - plus
`surf.toml` config discovery. This is the contract everything else binds to.

**Proposal refs:** §6 (claim shape), §6.3 (`at:` as list), §9.1.1 (hub format), §9.1.5 (config marker), §9.3 (`refs` deferred), §9.1 (`covers` absent).

**Depends on:** Phase 0. (Independent of 1/2 - can proceed in parallel.)

**Status:** done

> Pure parsers in `surf-core`: `hub.rs` (`parse_hub` → frontmatter + body; `at:` is
> scalar-or-list via untagged enum; `hash` is `Option`, unset until `verify`; `covers`
> rejected via `deny_unknown_fields`; `refs` parsed but inert) and `config.rs`
> (`parse_config`, default glob `hubs/*.md`). Filesystem **discovery** lives in
> `surf-cli/src/workspace.rs` (`Workspace::discover` walks up for `surf.toml`; `hub_paths`
> globs) to keep `surf-core` I/O-free. Real `surf.toml` + `hubs/core-engine.md` (dogfood)
> at repo root. Commands now discover the workspace before reporting not-implemented.

## Steps

1. **Schema** (serde structs):
   - `summary`: string.
   - `anchors`: list of `{ claim: string, at: string | [string], hash: string }`.
     `at:` accepts a scalar **or a list** - a claim is stale if *any* listed span changes (§6.3).
   - `refs`: parsed but **inert** in the MVP - forward-declared only (§9.3). Do not resolve it.
   - **`covers` is absent** (§9.1): it is consumed only by the deferred reviewer plugin, so
     asking authors to write it now is ceremony. Forward-declared in the proposal, not shipped.
   - Body: the markdown prose after the frontmatter fence.
2. **Frontmatter split:** leading `---`-fenced YAML block + remaining prose body. Tolerant,
   clear errors on a missing/malformed fence.
3. **Typed validation errors** surfaced as results `lint` (Phase 4) can render - not panics.
4. **Config model + discovery:** `surf.toml` walked up from `cwd` to the nearest marker,
   like `git` / `ruff` (§9.1.5). Config holds the hub glob(s); default `hubs/*.md`.

## Files touched
- `surf-core/src/hub.rs` (schema, frontmatter parse, validation errors)
- `surf-core/src/config.rs` (`surf.toml` model + upward discovery)
- `hubs/auth.md` (sample hub: both scalar and list `at:`)
- `surf.toml` (sample at repo root)

## Verify
- Parse `hubs/auth.md` with both scalar and list `at:`; round-trip preserves fields.
- Malformed frontmatter (bad YAML, missing required field) → clear typed errors, no panic.
- `refs` present in a hub parses without being resolved or erroring; `covers` is rejected/ignored as undefined.
- Config discovery from a nested `cwd` finds the root `surf.toml`.

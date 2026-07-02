# Phase 0 - Toolchain & workspace scaffold

**Goal:** a compiling, CI-checked empty workspace and a working Rust toolchain.

**Proposal refs:** §10 (language/layout), §9.1.5 (config discovery marker).

**Depends on:** - (start here)

**Status:** done

## Steps

1. **Install Rust** (this device has no toolchain):
   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
   . "$HOME/.cargo/env"
   ```
   Pin the toolchain in `rust-toolchain.toml` (channel = a fixed stable version, e.g.
   `1.xx.x`). Verify `rustc --version` and `cargo --version`.
2. `git init`; add a Rust `.gitignore` (at least `/target`).
3. **Workspace:**
   - root `Cargo.toml`: `[workspace]` with `members = ["surf-core", "surf-cli"]`, shared
     `[workspace.package]` metadata, and `resolver = "2"`.
   - `surf-core`: library crate, **no I/O deps** (tree-sitter added in Phase 1).
   - `surf-cli`: binary crate, binary name `surf`, `path` dep on `surf-core`. Deps:
     `clap` (derive), `anyhow`, `serde` + `serde_json` (JSON report), `serde_yaml`
     (frontmatter - added when Phase 3 lands, fine to add now).
4. **CLI skeleton:** `clap` parser with subcommands `lint`, `check`, `verify`, each stubbed
   to print "not implemented" and exit non-zero; `--version` wired to the crate version.
   The top-level `--help` should already carry the §7 scope disclaimer (gate checks named
   spans, not system-wide invariants).
5. **CI:** `.github/workflows/ci.yml` on push/PR running `cargo fmt --check`,
   `cargo clippy -- -D warnings`, `cargo test`. Use a pinned toolchain action.

## Files touched
- `rust-toolchain.toml`, `.gitignore`
- `Cargo.toml` (workspace root)
- `surf-core/Cargo.toml`, `surf-core/src/lib.rs`
- `surf-cli/Cargo.toml`, `surf-cli/src/main.rs`
- `.github/workflows/ci.yml`

## Verify
- `cargo build` and `cargo test` pass.
- `cargo run -p surf-cli -- --help` lists `lint`, `check`, `verify` and shows the scope disclaimer.
- `cargo run -p surf-cli -- --version` prints the version.

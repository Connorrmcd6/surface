# Changelog

All notable changes to Surface are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0] - 2026-06-29

### Added
- **`refs` hub composition (#4).** A hub's `refs:` now compose *other hubs*: a path relative to the
  referencing hub (`./resolve.md`), optionally `> symbol` to address one claim within the target
  (matched against that claim's `at:` anchor). `surf lint` blocks a ref that doesn't resolve to a
  loaded hub, points at its own hub, or names a claim the target lacks — the same fail-on-typo
  discipline as `covers`. The `surf check` gate then **propagates staleness one hop**: a hub that
  directly references a stale hub (or a stale claim within one) inherits a `referenced_stale`
  divergence and fails the gate, so the signal that flags a dependency flags everything composed on
  it. Propagation is one-hop by construction (built only from base divergences), so a chain
  `A → B → C` doesn't cascade. The repo's own hubs now declare their cross-hub `refs`, and the two
  prior doc-pointing `refs` were reclassified to prose links.
- **`surf lint` consolidation nudges (#142).** Two advisory warnings push hubs away from the
  "claim-log" shape (one claim per function) and toward onboarding docs: a *claim-log* warning
  when a hub has several claims and never once uses a multi-site `at:` list, and a *thin-prose*
  warning when a multi-claim hub's body is too sparse to onboard a reader. Both are non-blocking
  (exit 0), mirroring the existing over-/under-anchoring nudges.

### Changed
- **`surf suggest` reframed (#142).** Human output now reads as a list of *undocumented symbols*,
  not a list of claims to write: it groups symbols by file and emits a multi-site `at:` skeleton
  with prose-first scaffolding, steering authors toward coarse, system-level claims. `--format
  json` is unchanged.
- **`surf new` scaffold** ships a prose-first template (`## How it works` / `## Boundary`
  headings and a multi-anchor example claim) so a fresh hub is shaped like an onboarding doc.
- **Hash recipe v2 — the bound/free split (#77).** The canonical hash now alpha-renames only
  *bound* identifiers (a symbol's own name, parameters, locals, loop/range/comprehension
  variables, `with`/`catch` aliases, generic params, destructuring binders) and emits every
  *free* identifier verbatim (external members, call targets, types, enum/constant references,
  object keys, decorators). Re-pointing an anchored span at a *different* symbol is now loud even
  when the name occurs once — `PointsTier.TIER_1` → `TIER_2`, `getHighest` → `getLowest`, a bare
  `helper(x)` → `other(x)`, a parameter type `Foo` → `Bar`, an object key `{ alpha }` →
  `{ beta }` — where before it passed the gate silently while the claim's prose became false.
  Consistent local/parameter renames stay quiet, as before. This subsumes the earlier
  member-access-only first cut and the Python decorator special case (#8). A new in-tree
  differential harness (`surf-core/tests/differential_hash.rs`) gates the change: zero
  benign-rename regressions, 100% catch on the semantic free-swap corpus across all four
  languages. Binding detection is tree-sitter-only and fail-closed; the one accepted
  approximation (match-arm pattern identifiers are treated as free) is documented in
  [Hash recipes](docs/reference/hash-recipes.md). Covers TypeScript, Go, Rust, and Python.
- **Versioned stamps.** Stored hashes now carry their recipe: a v2 stamp is prefixed `2:`, a bare
  12-hex stamp is an implicit v1. `surf check` verifies each stamp under its own recipe, so
  existing v1 stamps keep passing (with a one-line nudge) until `surf verify` re-stamps them as
  v2 — one pass per repo. An unrecognized stamp prefix fails closed. See
  [Hash recipes](docs/reference/hash-recipes.md). **Action on upgrade: run `surf verify` once.**

## [0.6.3] - 2026-06-18

### Added
- Hubs may now declare advisory `covers` globs (repo-root-relative, same dialect as
  `config.hubs`). The field is accepted, stored, and lint-validated — a malformed glob blocks,
  and a hub whose globs don't match its own anchored files warns — but **the verdict never reads
  it**: `surf check` is byte-for-byte identical whether or not a hub declares `covers`. The
  louder coverage pass that consumes these globs stays deferred (#94, part of #5).
- TypeScript `suggest` now enumerates exported class methods (`Class > method`) by default —
  matching Go (#29) and Python — so a class-heavy file no longer reports "no unanchored public
  symbols found." `suggest --all` additionally proposes the exported class itself and
  non-callable `const`/`let`/`var` declarations (object/array/scalar). The "Rust/TypeScript
  `--all` changes nothing" note now fires only for Rust (#70).
- A failed flat TypeScript anchor that looks like a method now hints the chain spelling:
  `EffectiveTierService.getForUsers` or a bare `getForUsers` reports
  ``did you mean `EffectiveTierService > getForUsers`?`` — the `Class > method` syntax was
  previously undiscoverable (#68).

### Fixed
- TypeScript exported non-callable `const`/`let`/`var` (`export const TIER_MAP = { ... }`) is
  now anchorable — previously only function-valued declarators resolved, so the const maps most
  prone to drift had no anchor target. Mirrors #28's Python fix (#69).

## [0.6.2] - 2026-06-12

### Added
- `surf suggest --all` now proposes Go exported top-level `const`, `var`, and `type`
  declarations (including structs), matching what the resolver already accepted — the Go
  edition of #52's Python fix. The "Python-only" note added for #79 now fires only for Rust
  and TypeScript, where `--all` still changes nothing (#79).

### Fixed
- Python `@overload` sets are one anchorable unit: consecutive same-name stubs plus their
  implementation resolve as a single logical symbol, so the bare name works without `@N` and
  the hashed span covers stubs *and* impl — a signature change in any overload now trips the
  gate, where previously only the implementation body was hashed and the typed contract could
  drift silently. `suggest`'s proposed path for overloaded functions now lints clean instead
  of failing its own resolver. Anchors that used the old `name@N` workaround to pick a stub or
  the implementation should re-anchor to the bare name and re-`verify` (#82).
- Python symbols bound inside module-level `if`/`try` blocks — `TYPE_CHECKING` guards,
  version gates, `try`/`except ImportError` fallbacks — now resolve by name like their
  unconditional siblings, and are enumerated by `suggest` and the lint coverage nudge. A name
  bound in multiple branches is reported ambiguous and disambiguated with the existing `@N`
  mechanism (#81).
- `surf check --files` now warns on stderr when a glob matched no anchored files, and exits
  non-zero when *every* glob matched nothing — a typo'd pattern no longer scopes the gate to
  nothing and reports a clean run, mirroring `suggest`'s existing zero-match behavior (#78).
- `surf lint` coverage warnings (and `check`'s malformed-hub divergences) no longer print a
  dangling empty `claim:` line when there is no claim to show (#83).
- The release workflow now polls the npm registry after publishing and fails the run if any
  package version never becomes fetchable, instead of trusting `npm publish`'s exit code —
  the registry has silently dropped freshly published versions post-publish (#80).

### Changed
- `surf suggest --all` prints a note when the scan includes non-Python files: `--all`
  currently only proposes non-callable symbols for Python, so its silence on Go/Rust/TS is
  no longer mistakable for "this file has no non-callables" (#79).

## [0.6.1] - 2026-06-12

### Changed
- `surf stats` now reads the whole history window from a single streamed
  `git log --name-status` instead of spawning a `diff-tree`/`rev-parse`/`ls-tree` triple per
  commit. Hub claim state is propagated incrementally — a hub is re-read (`git show`) only at
  commits that touched it — so the git-spawn count scales with hub edits rather than history
  length. On a 13k-commit repo this is the difference between minutes and seconds; metrics are
  byte-identical to the previous implementation. Requires git ≥ 2.31 for
  `--diff-merges=first-parent` (#72).

### Fixed
- ARM64 Linux is now a first-class install target. Releases publish an
  `aarch64-unknown-linux-gnu` binary, and both `install.sh` and the npm channel
  (`@gradient-tools/surface`) resolve it for the host. Previously ARM64 Linux fell through to a
  missing asset; the installer now succeeds there rather than failing on a 404 (#67).
- `surf for` distinguishes a path that is a directory from one that does not exist, instead of
  reporting both the same way — the error now points at the actual problem (#71).

## [0.6.0] - 2026-06-11

### Added
- `surf suggest --all` additionally proposes the non-callable anchor targets `resolve` already
  accepts but `suggest` previously hid: top-level classes, module-level constants and type
  aliases, and class attributes (Python). The default stays callables-only (functions + methods)
  to avoid over-anchoring fatigue. Closes the gap where those kinds were anchorable by hand yet
  undiscoverable via `suggest` (#52).
- npm distribution: install via `npm install @gradient-tools/surface` or run ad-hoc with
  `npx @gradient-tools/surface check`. A thin shim resolves the prebuilt binary for the host from
  per-platform `optionalDependencies` (macOS Apple Silicon, Linux x86_64) — no `postinstall`
  download step. Published from CI on each release with provenance attestation (#46).
- Release integrity: each published binary now ships a `<asset>.tar.gz.sha256` checksum, and the
  `curl | sh` install script (and the GitHub Action that wraps it) verifies the download against
  it before installing. A missing checksum or a mismatch aborts the install — fail closed —
  rather than running a corrupted or tampered binary (#39).

### Changed
- `surf stats` is much faster on large histories. It previously recomputed the hub claim set at
  every commit *and* its parent, each spawning `git ls-tree` plus a `git show` per hub — roughly
  `2 × commits × hubs` subprocesses. The claim set is now memoized by commit SHA, with each
  `before` rev canonicalized to its parent's SHA so it reuses the parent commit's already-computed
  state. Metrics are unchanged (#40).

### Fixed
- `surf lint`'s under-coverage nudge is no longer workspace-blind and no longer skips methods.
  Coverage is now computed across the whole workspace, so splitting one file's claims across two
  hubs no longer makes each hub warn about the symbols the other one anchors; and the measured
  public surface now includes methods (`Type > method`), not just top-level functions, matching
  what `suggest` proposes (#54).
- `surf for`, `surf check --files`, and `surf stats` now fail loudly on malformed input instead of
  returning a falsely-reassuring success. `surf for` errors on a nonexistent/mistyped path, a
  directory, or a trailing slash (instead of "no hubs anchor" + exit 0); invalid `--files` glob
  syntax and invalid `surf.toml` hub globs now error instead of being silently dropped (#53, #38).

## [0.5.0] - 2026-06-09

### Fixed
- `surf check` now fails **closed**. A hub whose frontmatter can't be parsed previously slipped
  through silently (exit 0, unenforced); it now surfaces as an `Unresolvable` divergence that
  blocks the run. A hashing error is likewise reported as `Unresolvable` instead of being counted
  clean (#35).

### Changed
- Hub iteration is unified behind `Workspace::iter_hubs()`, shared by `check`, `lint`, `suggest`,
  and `for`. Each command carries the per-hub parse result explicitly, so the error case can no
  longer be handled inconsistently from one command to the next (#36).
- CI now runs the test suite and dogfood self-check on macOS (`macos-14`) in addition to Linux,
  matching the Apple-Silicon binary that releases ship. Windows is documented as unsupported
  (#37).

## [0.4.0] - 2026-06-09

### Added
- `surf for <path> [symbol]` — reverse lookup: list every hub and claim anchored into a file, so
  you can pull up the documentation governing code *before* you edit it (the inverse of
  `suggest`). Read-only and always exits 0; `--format json` emits a versioned
  `{version, path, matches}` envelope. An optional trailing `symbol` narrows to anchors whose
  first segment matches.
- `surf stats [--since <date>] [--until <date>]` — adoption metrics from git history (advisory,
  never a gate): the **rubber-stamp rate** (re-stamps that changed a claim's stored hash but left
  its prose untouched) and the **in-place update rate** (commits touching an anchored file that
  re-sealed the claim in the same commit). One commit = one PR (merges excluded); heuristics are
  documented in `docs/guides/stats.md`. Errors (non-zero) when git history is unavailable.
- `surf suggest` now proposes **methods**, not just top-level functions: Python class methods and
  Go methods are enumerated as `file > Type > method` anchors (matched by exported Go receiver
  type; Python async/sync mirrors de-duped). On method-oriented codebases this surfaces the bulk
  of the public API that was previously invisible to `suggest`. Rust `impl`/TS class methods
  remain top-level-only.

### Changed
- `surf suggest` distinguishes a glob that matched **no files** from a clean "nothing to suggest":
  it warns on stderr and exits non-zero when *every* glob was empty, so a typo'd path no longer
  reads as success. Coverage is now keyed on the full `file > seg > seg` anchor path, so anchoring
  one method no longer suppresses its siblings.

## [0.3.2] - 2026-06-08

### Documentation
- The docs site (surface.gradientdev.xyz) is now **generated** from this repo on release: each
  `v*` tag regenerates the Starlight site's pages and changelog from `docs/` and `CHANGELOG.md`
  and opens a sync PR, so the site no longer drifts from canonical docs.
- `docs/reference/commands.md` is now governed by `surf check` itself — a hub anchored to the
  clap command/flag surface blocks the gate when the CLI and its reference drift.
- Pinned `Connorrmcd6/surface@vX.Y.Z` Action refs in the README and docs are now derived from
  `Cargo.toml` at release, and the Examples page description is quoted so it parses under the
  site's strict YAML.

## [0.3.1] - 2026-06-08

### Documentation
- Reposition around *documentation governed like code* for fast-moving codebases, led by the
  context-rot story (a context file that's accurate when written and rots as the code moves
  because nobody knows it exists or where to find it). Dropped the older accusatory framing.
- Slim the README to a pitch + compact quickstart that links out; the full reference now lives in
  `docs/`. Restructured `docs/` into a site-ready tree (`getting-started/`, `reference/`,
  `guides/`) with `title`/`description` frontmatter, mirroring the docs site
  (surface.gradientdev.xyz); this repo's `docs/` is the source of truth.
- Bring the docs current with 0.3.0: the versioned `--format json` envelope, the per-claim
  `ignore_literals` option, Python non-callable anchors, file-rename `--follow`, and decorators in
  the hashed span.

## [0.3.0] - 2026-06-08

### Added
- Python: `at:` now resolves **non-callable** symbols — module-level constants and type aliases
  (`X = …`, `X: T = …`, PEP 695 `type X = …`) and class-level attributes (`Class > attr`,
  including annotation-only). Previously only functions, methods, and classes resolved (#28).
- Per-claim `ignore_literals: true` frontmatter option — excludes string-literal *content* from a
  claim's hash so a copy edit inside the anchored span no longer re-opens the gate. Logic edits
  (operators, numbers, structure) are still caught. The stored hash is computed in this mode, so
  it travels with the claim rather than a CLI flag (#21).
- `surf verify --follow` and `surf lint` now follow **file renames** via git rename detection:
  a moved file makes `lint` warn (and point at `--follow`) instead of hard-blocking, and
  `--follow` rewrites the anchor's path — only when the code is otherwise unchanged. Best-effort
  and git-dependent; the deterministic `surf check` verdict never depends on it (#3).

### Changed
- Python decorators are now part of an anchored function/class's hashed span, and a decorator's
  *name* is kept verbatim (not alpha-renamed) — so adding/removing a decorator, changing its
  arguments, or swapping it (`@cache` → `@lru_cache`, `@staticmethod` → `@classmethod`) trips the
  gate. Previously decorators were excluded from the span entirely (#8).
- **Breaking (JSON):** `surf check --format json` now emits a versioned envelope
  `{ "version": 1, "divergences": [...] }` instead of a bare array. The contract is additive-only
  within a major version; a breaking change bumps `version`. Consumers should read
  `.divergences` and tolerate unknown fields (#16).

## [0.2.1] - 2026-06-07

### Documentation
- Precisely scope "cosmetic" (#21): a literal *value* is part of the hashed AST, so editing a
  string literal — including user-facing copy — inside an anchored span trips the gate.
  "Cosmetic" means only whitespace, comments, and consistent renames. Added a FAQ entry and a
  note to anchor the narrowest symbol so unrelated literal edits don't re-open a claim.

## [0.2.0] - 2026-06-07

### Added
- `surf check` scoping: `--files <globs>` evaluates only claims whose anchored file(s) match,
  and an explicit `--base <ref>` diff-scopes the gate to claims whose files changed since the
  merge base. Omitting `--base` keeps a full check (enrichment falls back to `HEAD`).
- `--format json` for `surf lint` and `surf verify`, mirroring `surf check`. `lint` emits
  structured findings; `verify` emits per-anchor outcomes (stamped/followed/unchanged/skipped)
  plus counts.
- Advisory `surf lint` granularity warnings (never blocking): a near-whole-file anchor span, a
  hub with too many anchors, and public functions in an anchored file that no claim covers.
- `surf lint` enforces the `AGENTS.md` pointer block: when `AGENTS.md` carries a
  `<!-- surf:hubs -->` block, it must link the hubs directory (and that directory must exist) so
  agents are pointed at the hubs to search — never duplicating or enumerating them. Opt-in;
  blocks on a missing/dangling pointer.
- `surf suggest <globs>` — scans source for top-level public functions that no hub anchors yet
  and prints a copy-pasteable starter hub. Suggestions only: never writes a file or stamps a hash.
- `surf_core::public_fns` — enumerates a file's top-level public functions (backs the
  under-coverage warning and `surf suggest`).

### Changed
- An explicit `surf check --base <ref>` now **diff-scopes** the gate to claims whose files
  changed, in addition to feeding advisory `old_code`/`magnitude`. Previously `--base` only fed
  enrichment and every claim was still checked. CI that passes `--base` explicitly will now check
  fewer claims; omit `--base` for a full check.

## [0.1.1] - 2026-06-06

### Fixed
- `resolve`: resolve const-bound call-expression functions (TS/JS).
- `verify`: skip unchanged anchors instead of re-stamping, so a no-op verify leaves hub files
  byte-identical.

### Changed
- Releases: dropped Intel macOS (`x86_64-apple-darwin`); prebuilt binaries cover macOS (Apple
  Silicon) and Linux (x86_64). Other targets build from source.

## [0.1.0] - 2026-06-06

Initial release — the MVP gate that surfaces docs↔code divergence.

### Added
- AST-canonical hashing via bundled, version-pinned tree-sitter grammars: quiet on cosmetics
  (formatting, comments, consistent renames), loud on logic. Advisory tree-edit `magnitude`.
- Anchor resolution: the `file > A > B` path grammar with `@N` disambiguation; scope-set
  resolution so `Type > method` resolves uniquely.
- Hub format (markdown frontmatter), `surf.toml` workspace discovery, and per-claim combined
  hashing across multi-site `at:` lists.
- Commands: `surf lint`, `surf check` (the gate), `surf verify` (re-seal, with `--follow` for
  renames), plus `surf init` and `surf new` for authoring ergonomics.
- Language support: TypeScript/TSX, JavaScript/JSX, Rust, Python, and Go.
- Distribution: GitHub Action, pre-commit hook, and `install.sh`; Apache-2.0 license.

[Unreleased]: https://github.com/Connorrmcd6/surface/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/Connorrmcd6/surface/compare/v0.6.3...v0.7.0
[0.6.3]: https://github.com/Connorrmcd6/surface/compare/v0.6.2...v0.6.3
[0.6.2]: https://github.com/Connorrmcd6/surface/compare/v0.6.1...v0.6.2
[0.6.1]: https://github.com/Connorrmcd6/surface/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/Connorrmcd6/surface/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Connorrmcd6/surface/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Connorrmcd6/surface/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/Connorrmcd6/surface/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/Connorrmcd6/surface/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Connorrmcd6/surface/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/Connorrmcd6/surface/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Connorrmcd6/surface/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/Connorrmcd6/surface/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Connorrmcd6/surface/releases/tag/v0.1.0

---
title: Configuration
description: surf.toml marks the workspace and globs your hubs; the supported languages; and what the gate needs from CI.
---

A `surf.toml` at the repo root marks the workspace — `surf` walks up from the current directory to
find it, like `git` or `ruff` — and globs your hubs:

```toml
hubs = ["hubs/*.md"]
```

Point the glob wherever your hubs live: keep them central (`hubs/*.md`) or co-locate them with code
(e.g. `["**/_hub.md"]`).

`hubs` is a list, so you can combine locations — the matches are unioned, then sorted and
de-duplicated, so overlapping globs are safe:

```toml
hubs = ["hubs/*.md", "docs/hubs/*.md", "**/_hub.md"]
```

Any file a glob matches is treated as a hub if it parses as one (frontmatter `anchors:` block +
markdown body), so claims can live in *any* file — not just files under `hubs/`. The list can pull
in files that aren't named like hubs, e.g. `AGENTS.md` or `CLAUDE.md` (see
[Where claims can live](../guides/authoring-hubs.md#where-claims-can-live) for the recommended
layout and the trade-offs).

> **`surf new` uses only the first glob.** When scaffolding a hub it writes into the directory
> derived from the first pattern (e.g. `docs/hubs/*.md` → `docs/hubs/`); the other patterns are
> still linted and verified normally, they're just not where `new` writes.

## OKF bundles

To govern an [Open Knowledge Format](../guides/okf.md) bundle — a directory *tree* of concept files
rather than a flat folder — list its root(s) under `bundles`. Each root expands to `<root>/**/*.md`:

```toml
hubs    = ["hubs/*.md"]        # optional; flat layout
bundles = ["knowledge/sales"]  # an in-repo OKF bundle (recursively)
```

`hubs` and `bundles` are unioned. OKF reserved files (`index.md`, `log.md`) swept up by a bundle
glob are recognized and **skipped for governance** — they hold no claims, so a missing frontmatter
fence in them never blocks the gate.

## Frontmatter fields

A hub is a **superset of an OKF concept**, so its frontmatter carries both OKF fields and Surface's
governance fields:

| Field | Source | Notes |
| --- | --- | --- |
| `type` | OKF | The one field OKF requires. Defaults to `concept` when absent (existing hubs keep working). |
| `title` | OKF | Display name. |
| `summary` | Surface | The onboarding one-liner (optional). Distinct from OKF `description`. |
| `tags` | OKF | Cross-cutting tags. |
| `timestamp` | OKF | Last *modified* (ISO 8601). Distinct from a claim's `verified_at` (last *attested*). |
| `anchors` | Surface | The claims — the governed part. See [Authoring hubs](../guides/authoring-hubs.md). |
| `refs` / `covers` | Surface | Composition edges and advisory coverage globs. |
| *(any other key)* | OKF / tools | Preserved verbatim (e.g. OKF `description`/`resource`, a doc system's `author`/`created`). |

Unknown *frontmatter* keys are preserved, not rejected (the OKF rule); a key that looks like a typo
of a known one earns a `surf lint` warning. Unknown keys **inside an anchor item** still fail
closed. Each claim also gains freshness provenance the first time `surf verify` stamps it: a stable
`id`, plus `verified_at` and `verified_commit` (the *who* is left to git blame, so no author email
lands in tracked files).

## Languages

TypeScript (`.ts`, `.tsx`, `.mts`, `.cts`), JavaScript/JSX (`.js`, `.jsx`, `.mjs`, `.cjs`), Rust
(`.rs`), Python (`.py`, `.pyi`), and Go (`.go`). Grammars are compiled into the binary and
version-pinned, so a hash computed on your laptop and in CI always agree.

In Python, `at:` resolves callables (functions, methods, classes) **and** non-callables — module
constants, type aliases (`X = Literal[...]`, `type X = ...`), and class attributes
(`Class > attr`).

## CI

The gate hashes your working tree and compares it to the hash committed in the frontmatter. It
needs the checkout, **not** the history — do **not** set `fetch-depth: 0`. (The advisory
`old_code` / `magnitude` use a single `git show` of the base ref; with no git available the verdict
is unchanged, those fields are just omitted.) See [CI integration](../guides/ci-integration.md).

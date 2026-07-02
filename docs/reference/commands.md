---
title: Commands
description: The surf CLI - init, new, suggest, for, lint, check, and verify, with their flags and exit behavior.
---

- **`surf init`** - bootstrap a workspace: write `surf.toml` and create the hubs directory
  (idempotent).
- **`surf new <name>`** - scaffold a new empty hub under your hubs directory.
- **`surf suggest <globs> [--all] [--format human|json]`** - scan source globs for public
  *callables* no hub anchors yet - top-level functions plus **Python class methods and Go
  methods** (as `file > Type > method` anchors) - and print a copy-pasteable starter hub.
  Suggestions only - never writes or stamps. Coverage is keyed on the whole anchor path, so
  anchoring one method doesn't suppress its siblings. A glob that matches **no files** is reported
  on stderr (so a typo doesn't read as a clean "all anchored"); `suggest` exits non-zero only when
  *every* glob was empty. The default is callables-only to avoid over-anchoring fatigue; **`--all`**
  additionally proposes the non-callable targets anchoring already supports - top-level classes,
  module-level constants and type aliases, and class attributes (Python) - so they're discoverable
  (see [Authoring hubs](../guides/authoring-hubs.md)).
- **`surf for <path> [symbol] [--format human|json]`** - reverse lookup: list every hub + claim
  anchored into `<path>`, so you can pull up the documentation governing a file *before* you edit
  it (the inverse of `suggest`). An optional trailing `symbol` narrows to anchors whose first
  segment matches. Read-only and always exits 0 - a query, not a gate. `--format json` emits a
  versioned envelope (`{version, path, matches}`) for agents.
- **`surf lint [--format human|json]`** - validate frontmatter and that every `at:` resolves to
  exactly one symbol. Blocks on ambiguous or vanished anchors; **warns** (and suggests
  `verify --follow`) on a symbol that was merely renamed, or a file that git reports has moved.
  Also emits advisory granularity warnings (never blocking): a near-whole-file anchor span, a hub
  with too many anchors, and public functions in an anchored file that no claim covers.
- **`surf check [--format human|json] [--base <ref>] [--files <globs>]`** - the gate.
  AST-canonical-hash each anchored span and compare to the stored hash; non-zero exit on any
  divergence. By default every claim is checked. `--base <ref>` scopes to claims whose anchored
  files changed since the merge base **and** recovers the advisory `old_code` / `magnitude` fields
  from that ref (omit it for a full check with enrichment against `HEAD`). `--files <globs>` scopes
  to claims whose anchored file(s) match a comma-separated glob (e.g. `surf-core/**`). `--format
  json` emits the [versioned report envelope](./how-it-works.md#the-json-seam).
- **`surf verify [<at>] [--follow] [--format human|json]`** - re-seal after you've confirmed the
  prose still holds; writes the hash into the frontmatter. `<at>` limits to one anchor. `--follow`
  re-points a single-segment anchor whose **symbol** was renamed, or whose **file** was moved
  (detected via git), and re-hashes in one step - only when the code is otherwise unchanged.
- **`surf stats [--since <date>] [--until <date>] [--format human|json]`** - adoption metrics from
  git history (advisory, never a gate): the **rubber-stamp rate** (re-stamps that changed a
  claim's stored hash but left its prose untouched) and the **in-place update rate** (commits
  touching an anchored file that re-sealed the claim in the same commit). One commit = one PR
  (merges excluded). Heuristic by design - see the [stats guide](../guides/stats.md). Errors
  (non-zero) if git history is unavailable.

## Per-claim options

A claim's frontmatter can carry options beyond `claim`/`at`/`hash`:

- **`ignore_literals: true`** - exclude string-literal *content* from this claim's hash, so a copy
  edit inside the anchored span no longer re-opens the gate. Logic edits (operators, numbers,
  structure) are still caught. The stored hash is computed in this mode, so the option travels with
  the claim. See [Authoring hubs](../guides/authoring-hubs.md) and the [FAQ](./faq.md).

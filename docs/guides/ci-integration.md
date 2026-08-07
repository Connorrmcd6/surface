---
title: CI integration
description: Run the gate in CI via the GitHub Action or the pre-commit hook, the checkout-depth rule, and scoping a check to a PR.
---

`surf check` is the gate: it exits non-zero when an anchored span diverged, so it blocks a merge
the same way a failing test does. Most repos never install the binary - they run the Action or
the pre-commit hook.

## GitHub Action

`.github/workflows/surface.yml`:

```yaml
name: Surface
on: pull_request
jobs:
  surface:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4   # plain checkout - do NOT set fetch-depth: 0
      - uses: Connorrmcd6/surface@v0.8.0
```

The action takes `args` (default `check`), `version`, and `working-directory` (default `.`). To
emit machine-readable output for another job or a reviewer bot, set `args: check --format json`.

### Which binary you get

`version` defaults to the release that ships with the action ref you pinned - `uses:
Connorrmcd6/surface@v0.8.0` installs `surf` v0.8.0, and a commit-SHA pin resolves the same way.
So the gate's behaviour only changes when you move the pin, and a `surf check` verdict is
reproducible for as long as the workflow file stays put.

To track the newest release instead, ask for it explicitly:

```yaml
- uses: Connorrmcd6/surface@v0.8.0
  with:
    version: latest       # or a specific tag, e.g. v0.7.0
```

### Checkout depth

The verdict hashes your **working tree** and compares it to the hash committed in the
frontmatter - it does not need git history, so a plain `actions/checkout@v4` is enough. **Do not
set `fetch-depth: 0`.** The advisory `old_code`/`magnitude` fields use a single `git show` of the
base ref; with no history available the verdict is unchanged and those fields are simply omitted.

The one exception: if you diff-scope with `--base <ref>` (below), fetch enough history to reach
the merge base - a shallow `git fetch <ref>` is plenty, still not `fetch-depth: 0`.

## pre-commit

`.pre-commit-config.yaml`:

```yaml
- repo: https://github.com/Connorrmcd6/surface
  rev: v0.6.1
  hooks:
    - id: surf-check
```

This runs the same gate locally at commit time, catching drift before it reaches CI.

## Scoping the gate to a PR

By default `check` evaluates every claim in every hub. On large repos or big PRs you can narrow
it:

- **`--base <ref>`** - evaluate only claims whose anchored files changed since the merge base
  with `<ref>` (e.g. `surf check --base origin/main`). This also recovers the advisory
  `old_code`/`magnitude` from that ref.
- **`--files <globs>`** - evaluate only claims whose anchored file(s) match a comma-separated
  glob (e.g. `surf check --files "src/auth/**"`).

Both filters intersect when combined. With neither flag, the full check runs (enrichment against
`HEAD`). A bad ref or non-repo falls back to a full check rather than silently checking nothing.

See also: [Authoring hubs](./authoring-hubs.md) · [Command reference](../reference/commands.md).

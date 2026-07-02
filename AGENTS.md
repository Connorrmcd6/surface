---
summary: Operating instructions for AI agents; also dogfoods one sealed claim about the lint rule that governs this file.
anchors:
  - claim: >
      surf lint blocks when AGENTS.md carries a surf:hubs block that does not link the configured
      hubs directory, or when that directory does not exist; without the block it stays silent.
    at: surf-cli/src/lint.rs > lint_agents_pointer
    hash: 2:ac139b65f5f0
refs: []
---

# AGENTS.md

Guidance for AI coding agents working in this repo. (Humans: see
[`CONTRIBUTING.md`](./CONTRIBUTING.md) and [`docs/`](./docs/index.md).)

> This file is itself a hub (see `surf.toml`): the one sealed claim below anchors to the lint rule
> that polices this very file - a small bit of dogfooding. Everything else here is plain
> instruction, deliberately not anchored.

Surface is a deterministic gate that surfaces divergence between docs and code: you anchor a
sentence to the code it describes, and `surf check` blocks when that code's logic changes out
from under the prose. **This repo dogfoods Surface on its own source** - the gate runs on
`surf-core`/`surf-cli`.

## Where the context lives: `hubs/`

<!-- surf:hubs -->
[`hubs/`](./hubs/) is the governed context for this codebase. Each hub is markdown prose
describing an invariant, with frontmatter anchoring the claim to a specific symbol. Unlike code
comments, **hub prose is sealed by `surf check`** - if the anchored code changed since a human
last confirmed the prose, the gate fails. So the hubs are trustworthy and current in a way
comments are not, and they are the fastest accurate way to understand a part of the system.

**Read only the hubs you need - not the whole directory.** The hubs together describe the entire
codebase; reading all of them is wasteful context. They are split per module
(`hubs/cli-check.md`, `hubs/hash.md`, `hubs/resolve.md`, …), and each starts with a one-line
`summary:` in its frontmatter. Scan the filenames and summaries, then open only the hub(s)
covering the area you're working on.
<!-- /surf:hubs -->

`surf lint` enforces that when this block is present it links the hubs directory and that the
directory exists; by convention it points at the directory rather than duplicating or enumerating
individual hubs (so agents search, not read everything).

Caveat (the tool's own honest limit): a green gate means *the anchored code hasn't changed
since last verified* - not that every sentence is true, and nothing about code no hub anchored.
If you read a hub claim, sanity-check it against the code it points at before relying on it.

## When you add or change a feature - keep the hubs honest

Run the loop (binary builds to `target/debug/surf`; see `CONTRIBUTING.md` for build commands):

1. Make the change.
2. `surf lint` - every anchor must resolve. Consider the advisory granularity warnings
   (over/under-anchoring); they are nudges, not blocks.
3. `surf check` - if you touched code a hub anchors, it will report `DIVERGED`. Re-read the
   claim. If the prose **still holds**, `surf verify` re-seals it (writes the new hash); if the
   prose is **now false**, fix the prose first, then verify.
4. Added public behavior? First reach for an *existing* system claim: extend its prose, or add
   the new symbol as another site under its multi-site `at:` list. Write a brand-new claim only
   when the behavior is genuinely its own. A hub is an onboarding doc, not a per-function log -
   the under-coverage warning lists undocumented symbols, but consolidating them into one coarse,
   multi-anchor claim beats one claim per function (`surf lint` will nudge a claim-log the other
   way). When you update a hub, update its *prose* to stay accurate, not just the hash.
5. Record user-facing changes in [`CHANGELOG.md`](./CHANGELOG.md) under `[Unreleased]`.
6. Hit a *notable* dogfooding moment? Log it in [`docs/dogfood-log.md`](./docs/dogfood-log.md).
   This is the repo eating its own dogfood, so it produces good material - capture it while it's
   fresh. The bar is "a reader would find this interesting," **not** every change: the gate
   catching a real contract drift, a lint false-positive, surprising friction, an invariant that
   was hard to express. Add a dated entry (newest first, template at the bottom of the file);
   skip routine changes that worked as expected.

Do not blindly `surf verify` to make the gate green - that is the rubber-stamping failure the
tool exists to prevent. Verify means "I read the prose and it is still true."

## Pointers

- [`hubs/`](./hubs/) - governed context, split per module; read only the hub(s) you need.
- [`CHANGELOG.md`](./CHANGELOG.md) - what changed; update `[Unreleased]`.
- [`docs/dogfood-log.md`](./docs/dogfood-log.md) - dated notes from using Surface on itself; add notable moments.
- [`docs/index.md`](./docs/index.md) - documentation map (guides, reference, concepts).
- [`CONTRIBUTING.md`](./CONTRIBUTING.md) - build, test, format, lint commands and layout.
- [`docs/surface-proposal.md`](./docs/surface-proposal.md) - the product spec (the `§` references in hubs).

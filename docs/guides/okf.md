---
title: Surface and the Open Knowledge Format
description: A Surface hub is a conformant OKF concept; Surface adds the freshness OKF deliberately leaves out. Produce, consume, and govern OKF bundles.
---

Google's [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf)
(OKF) is a vendor-neutral standard for representing knowledge as a directory of markdown files with
YAML frontmatter. It standardizes *how knowledge is written down* so a wiki produced by one tool can
be consumed by another without translation.

OKF is deliberately minimal. Its spec requires exactly one frontmatter field (`type`), recommends a
few more (`title`, `description`, `resource`, `tags`, `timestamp`), and mandates that consumers
**preserve unknown keys** and **never reject** a document for extra fields, unknown types, or broken
links. And it explicitly leaves several things undefined — most importantly, **freshness**: OKF has
no notion of "has the thing this describes changed?", no verification status, no anchoring to code,
no drift detection.

That gap is exactly what Surface fills.

## Surface = OKF + freshness

> A Surface hub is a **conformant OKF concept**. Surface adds the freshness layer OKF omits.

Because OKF ignores keys it doesn't recognize, Surface's governance metadata rides along inside an
otherwise-normal OKF concept:

```markdown
---
type: BigQuery Table          # OKF: the one required field
title: Orders                 # OKF: display name
description: One row per order # OKF: preserved (Surface keeps it in `extra`)
tags: [sales, revenue]        # OKF
timestamp: 2026-05-28T14:30:00Z  # OKF: last *modified*
anchors:                      # Surface extension — OKF readers ignore it
  - claim: an order is immutable once `status = shipped`
    at: src/orders/model.ts > Order > freeze
    hash: 2:9b1c33ade8f1
    id: c_18be…                 # stable identity (claim timelines)
    verified_at: 2026-06-30T09:12:00Z   # last *attested* against the code
    verified_commit: 4d5e6f2            # who is recovered from git blame, not stored
---

# Orders

Prose a human or agent reads to understand this table…
```

- **An OKF consumer** — Google's Knowledge Catalog, the OKF visualizer, [Nansidian](#doc-systems),
  Obsidian — reads this as a normal concept and silently ignores `anchors`.
- **Surface** reads `anchors` and governs freshness: when `src/orders/model.ts > Order > freeze`
  changes, [`surf check`](../reference/commands.md) fails until a human re-confirms the claim.
- The two timestamps are different on purpose: OKF's `timestamp` is *last modified*; Surface's
  per-claim `verified_at` is *last attested against the code* — the freshness OKF can't express.

## What conformance means here

A hub is a conformant OKF concept when it carries a `type`. Surface makes this cheap:

- `surf new` scaffolds hubs with `type: concept` already set.
- Hubs written before OKF (no `type`) still parse — Surface treats a missing `type` as `concept`
  in memory. They are byte-unchanged on disk; run a future `surf migrate` (or add `type:` by hand)
  to make them OKF-conformant *on disk*.
- Any extra frontmatter key (OKF's `description`/`resource`, a doc system's `author`/`created`/
  `pinned`) is preserved verbatim on round-trip — Surface never drops what it doesn't recognize.

The one boundary worth stating plainly:

> **Surface only fact-checks concepts that describe code.** A concept anchored to a code symbol is
> governed. A concept with no `anchors` (a BigQuery table's business meaning, an RFC, a playbook)
> is a valid, rendered, **ungoverned** OKF concept — it passes the gate untouched. Verifying
> non-code resources (e.g. a table's schema against the live warehouse) is future work; today the
> deterministic gate is scoped to code.

## Bundles

OKF ships knowledge as a **bundle**: a directory tree where each file is a concept, the path is its
identity, and two filenames are reserved — `index.md` (a directory listing for progressive
disclosure) and `log.md` (a change history). Point Surface at a bundle with `bundles` in
`surf.toml`:

```toml
# Govern a flat hubs/ dir, an in-repo OKF bundle, or both.
hubs    = ["hubs/*.md"]
bundles = ["knowledge/sales"]   # expands to knowledge/sales/**/*.md
```

Reserved files are recognized and skipped for governance (they hold no claims), so a bundle's
`index.md`/`log.md` never trip the gate. `surf lint` additionally checks OKF cross-links in a hub's
body and **warns** (never blocks — OKF tolerates broken links) on a dangling `.md` target.

## Doc systems

Because an OKF bundle is just markdown in a directory, it drops into any doc system that reads
markdown — [Obsidian](https://obsidian.md/) vaults, Notion imports, and git-backed editors. The
intended integration is a **CI gate on the git repos those systems sync**: run
[`surf check`](../guides/ci-integration.md) as a GitHub Action over the doc repo, so the freshness
gate guards the code-anchored subset of the knowledge base wherever the docs are edited.

## See also

- [Authoring hubs](./authoring-hubs.md) — a hub is an onboarding doc, not a claim-log (the same
  shape OKF's prose-first concepts encourage).
- [Configuration](../reference/configuration.md) — the `bundles` glob and frontmatter fields.
- [How the gate works](../reference/how-it-works.md) — what Surface hashes and why.

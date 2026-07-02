---
title: What is Surface?
description: Portable, always-fresh docs for agents and humans. OKF makes your docs portable and accessible; Surface governs their freshness by anchoring sentences to code and blocking the build when that code's logic drifts.
---

**Portable, always-fresh docs for agents and humans.**

Agents read your docs on every run, and they can't tell a current doc from a rotted one. Two open
layers fix that: [OKF](./guides/okf.md) (Google's vendor-neutral format) makes docs portable and
accessible, and Surface governs their freshness - you anchor a sentence to the code it describes,
and `surf check` fails the build until a human re-confirms it whenever that code's logic changes.
Documentation, governed like code. Portable *and* fresh docs measurably improve how well agents
perform. Deterministic: no model, no network, no API key in the core.


> **Docs source of truth.** These pages (the repo's `docs/` tree) are canonical. The docs site at
> [surface.gradientdev.xyz](https://surface.gradientdev.xyz) is generated *from* them - edit docs
> here, not on the site.

## The problem

You write a context file for your codebase - an architecture note, an `AGENTS.md`, a hub for the
auth flow. The day you write it, it's accurate.

Then the code moves. Someone refactors the function you described; the behavior changes on
purpose, the tests get updated to match, CI goes green, the PR merges. Everything is correct -
except the paragraph that *described* that function. Nobody touched it, for two ordinary reasons:
they didn't know it existed, and there was no standard place to look for it. It now says something
that is no longer true.

Nothing failed. Nothing fired. The only thing that broke is the explanation the next engineer -
and every agent on every run - will trust and reason from. A codebase can be fully green on tests
while its docs quietly describe code that no longer works the way they say, and nothing in your
toolchain catches the gap.

Surface closes that gap two ways: **`hubs/`** gives documentation a standard home so people and
agents actually find it, and **`surf check`** governs the prose like a test so it can't silently
rot.

## What Surface does that tests don't

A test asserts that **behavior** matches an expectation written in code. Surface asserts that
**prose** still matches the code it describes. Those are different expectations, and they drift
apart at exactly the moment someone updates one and forgets the other.

|                  | docs accurate         | docs stale                       |
| ---------------- | --------------------- | -------------------------------- |
| **code correct** | fine                  | **← nothing else catches this**  |
| **code broken**  | your tests catch this | both might fire                  |

The bottom-left cell is what tests are for. The top-right cell - code that works fine but no
longer does what your docs claim - has no owner. You can't write a unit test for "the README still
describes this accurately," because the thing that drifted is human-language understanding, and
tests don't speak that language. Surface owns that cell.

## How it works, in one breath

You anchor a sentence to the code it's about:

```yaml
# auth/_hub.md  (a "hub" - frontmatter + prose, lives next to the code it describes)
anchors:
  - claim: "refresh rotation is single-use; reuse triggers global logout"
    at: "src/auth/refresh.ts > rotateRefreshToken"
    hash: 9b1c33a
```

`surf check` runs in CI. For each anchor it finds the symbol, reduces it to pure logic (ignoring
formatting, comments, and renames), fingerprints that, and compares the fingerprint to the one
stored from the last time a human confirmed the sentence was true.

- **Fingerprint matches** → the logic didn't meaningfully change → silent pass.
- **Fingerprint differs** → the code *diverged* from its description → **block the merge** with a
  precise report: which hub, which claim, old code vs. new code.

It's a tamper-evident seal on the logic of exactly the code your docs claim to describe. Quiet on
cosmetics, loud on logic - see [How the gate works](./reference/how-it-works.md).

## Interoperable: Surface speaks OKF

A hub is a **conformant [Open Knowledge Format](./guides/okf.md) concept** - Google's vendor-neutral
standard for knowledge as markdown + frontmatter. OKF standardizes *how knowledge is written down*
but deliberately omits **freshness**: it has no notion of whether the thing a document describes has
changed. That omission is precisely what Surface is. So the relationship is clean: **Surface = OKF +
the freshness OKF leaves out.** Your hubs drop into any OKF consumer (Google's Knowledge Catalog,
the OKF visualizer, Obsidian, git-backed doc editors), which read the knowledge and ignore the
`anchors:` Surface governs. See [Surface and OKF](./guides/okf.md).

## What Surface does NOT do

Read this part. It's the difference between a tool you trust and one that burns you.

- **It does not tell you your docs are *true*.** It tells you the code they point at *changed*, so
  a human should re-read the prose. A green check means "nothing drifted since the last sign-off,"
  not "everything is correct." That's a deliberately weaker promise than a passing test, because
  meaning isn't mechanically decidable.
- **It only watches what you anchored.** If a change in a file no hub points at quietly invalidates
  a documented invariant, Surface will not see it. Catching that is security review and taint
  analysis - a different discipline. Surface guards the spans you chose to describe, nothing more.
- **It is not a retrieval system.** It doesn't search, embed, or serve context. There are good
  tools for that. Surface optimizes a different thing: *trust* in what you retrieve.

If you want the fuzzy "is this claim still true" judgment, that lives in an **optional** reviewer
plugin that reads Surface's JSON output. The core never depends on it. Pull every plugin out and
the gate blocks and passes exactly the same.

## Is Surface for you?

Honestly? Maybe not. Roughly, it earns its keep when

> **codebase complexity × change velocity × (humans + agents) reading it**

is high. A small, slow, simple codebase doesn't need this - your team can just read the code, and
two well-kept markdown files beat the whole apparatus. Use Surface where rebuilding the mental
model from source is genuinely expensive and the code moves fast enough to drift.

One thing pushes the math toward "yes": **AI agents.** A human onboards onto a domain once and
amortizes the cost over months. An agent re-onboards every session and amortizes nothing - it's a
new hire on its first day, every day, paying the full cost of wrong context on every invocation. A
bigger context window doesn't fix this; it lets an agent read every line and still confidently
derive a *wrong* model, because it can't tell a deliberate invariant from incidental code. If your
team runs agents hard, your effective reader count is enormous, and governed context stops being
optional.

But Surface is justified *without* a single agent in the loop. "Your architectural invariants are
now governed like code" pays off for human onboarding, review, and incident response on its own.
Agents are a multiplier, not the foundation.

## Next

- [Install](./getting-started/install.md) · [Quickstart](./getting-started/quickstart.md)
- [Authoring hubs](./guides/authoring-hubs.md) · [CI integration](./guides/ci-integration.md) · [Surface and OKF](./guides/okf.md) · [Examples](./examples.md)
- Reference: [Commands](./reference/commands.md) · [Configuration](./reference/configuration.md) · [How the gate works](./reference/how-it-works.md) · [Hash recipes](./reference/hash-recipes.md) · [FAQ](./reference/faq.md)

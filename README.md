<!--
NOTE TO THE BUILDING AGENT
==========================
This README is the GitHub front door: a pitch + a compact quickstart, nothing more. The full,
canonical docs live in docs/ (this repo) and are published to surface.gradientdev.xyz. When you
add reference detail, put it in docs/ and link to it — do NOT re-inline command/config/technical
reference here.

Positioning: Surface is "a new way to document and govern code for fast-moving codebases" —
documentation governed like code. Lead with the real story: a context file that's accurate the
day it's written and rots as the code moves, because nobody knows it exists or where to find it.
Do NOT use the old accusatory "your documentation is lying" framing.

Two rules to preserve:
1. Keep the honesty. The "What Surface does NOT do" section is a feature, not a disclaimer. Do
   not soften it or move it to the bottom. Devs trust tools that state their limits.
2. Do NOT lead with the security example anywhere. Surface cannot catch action-at-a-distance
   breaches (a change to code no hub anchored). Security is a nice side effect, never the headline.

Voice target: ruff / esbuild / tailwind READMEs. Confident, concrete, low on adjectives,
opinionated. Short lines. No "revolutionary," no "seamless."
-->

# Surface

**Documentation, governed like code.**

You anchor a sentence to the code it describes. When that code's logic changes, Surface fails the
build until a human re-confirms the sentence still holds — the same way a broken test blocks a
merge. For fast-moving codebases where humans and agents both read the docs and neither can tell a
current doc from a rotted one.

Deterministic. No model, no network, no API key in the core.

**Docs:** [surface.gradientdev.xyz](https://surface.gradientdev.xyz) · **Install:** [`docs/getting-started/install.md`](docs/getting-started/install.md)

---

## The problem

You write a context file for your codebase — an architecture note, an `AGENTS.md`, a hub for the
auth flow. The day you write it, it's accurate.

Then the code moves. Someone refactors the function you described; the behavior changes on purpose,
the tests get updated, CI goes green, the PR merges. Everything is correct — except the paragraph
that *described* that function. Nobody touched it, for two ordinary reasons: they didn't know it
existed, and there was no standard place to look. It now says something untrue.

Nothing failed. Nothing fired. The only thing that broke is the explanation the next engineer — and
every agent on every run — will trust and reason from. A codebase can be fully green on tests and
full of confident, completely false documentation, and nothing in your toolchain catches it.

Surface closes that gap two ways: **`hubs/`** give documentation a standard home so people and
agents actually find it, and **`surf check`** governs the prose like a test so it can't silently
rot.

## How it works, in one breath

You anchor a sentence to the code it's about:

```yaml
# hubs/auth.md  (a "hub" — frontmatter + prose, lives wherever you like)
anchors:
  - claim: "refresh rotation is single-use; reuse triggers global logout"
    at: "src/auth/refresh.ts > rotateRefreshToken"
    hash: 9b1c33a
```

`surf check` runs in CI. For each anchor it finds the symbol, reduces it to pure logic (ignoring
formatting, comments, and renames), fingerprints that, and compares it to the fingerprint stored
the last time a human confirmed the sentence was true.

- **Matches** → the logic didn't meaningfully change → silent pass.
- **Differs** → the code *diverged* from its description → **block the merge** with a precise
  report: which hub, which claim, old code vs. new code.

Quiet on cosmetics, loud on logic. Reformatting, comments, and consistent renames don't fire; a
flipped operator, a relaxed comparison, or a dropped `await` does. The full mechanism is in
[How the gate works](docs/reference/how-it-works.md).

## Why it matters

**Stale docs make AI agents fail. Surface finds them before they do.**

AI coding agents trust your docs. So when a doc is out of date, the agent confidently does the wrong
thing — even when the real code is right there. We measured it: across multiple models from three
providers, agents working from a stale doc got the task wrong far more often than agents given no doc
at all, and a more capable model was no more resistant. Accurate docs — or just surfacing the drift —
fixed it.

That's from a [pre-registered, deterministically-graded benchmark](https://github.com/Connorrmcd6/surface-bench)
(3,250 graded completions, multi-turn agents, no LLM judge). It measures drift of exactly the kind
`surf check` catches; the author of the benchmark also authors Surface, and a null result on any
hypothesis was reportable — see the [write-up](https://github.com/Connorrmcd6/surface-bench/blob/main/PAPER.md)
for the full method, limitations, and data.

## Quickstart

```sh
surf init              # writes surf.toml + creates hubs/
surf new auth          # creates hubs/auth.md — add a claim and point at: at a symbol
surf lint              # does every anchor resolve to exactly one symbol?
surf check             # the gate — a new claim is "unverified" until you seal it
surf verify            # you read the prose and confirmed it; seal the hash
```

Change the *logic* of an anchored symbol and the gate blocks until someone re-reads the sentence:

```
$ surf check
DIVERGED  hubs/auth.md :: src/auth/refresh.ts > rotateRefreshToken
    stored 9b1c33ade8f1 → now 4d5e6f2a0b7c  (magnitude: Small)
    claim: refresh rotation is single-use; reuse triggers global logout
surf check: 1 divergence(s).
```

If the prose still holds, `surf verify` re-seals it; if it's now false, fix the prose first. Full
walkthrough: [Quickstart](docs/getting-started/quickstart.md).

## What Surface does NOT do

Read this part. It's the difference between a tool you trust and one that burns you.

- **It does not tell you your docs are *true*.** It tells you the code they point at *changed*, so a
  human should re-read the prose. A green check means "nothing drifted since the last sign-off," not
  "everything is correct."
- **It only watches what you anchored.** A change in a file no hub points at can still invalidate a
  documented invariant; Surface won't see it. That's security review and taint analysis — a
  different discipline.
- **It is not a retrieval system.** It doesn't search, embed, or serve context. It optimizes a
  different thing: *trust* in what you retrieve.

The fuzzy "is this claim still *true*" judgment lives in an **optional** reviewer plugin that reads
Surface's JSON output. The core never depends on it. More in
[What Surface does NOT do](docs/index.md#what-surface-does-not-do) and
[Is Surface for you?](docs/index.md#is-surface-for-you).

## Speaks OKF

A hub is a conformant [Open Knowledge Format](docs/guides/okf.md) concept — Google's vendor-neutral
standard for knowledge as markdown + frontmatter. OKF standardizes how knowledge is written down but
deliberately omits freshness; that's exactly what Surface adds. **Surface = OKF + the freshness OKF
leaves out.** Your hubs drop into any OKF consumer (Knowledge Catalog, the OKF visualizer, Obsidian,
git-backed doc editors), which read the prose and ignore the `anchors:` Surface governs. See
[Surface and OKF](docs/guides/okf.md).

## Install

Most repos never install the binary — they run the GitHub Action:

```yaml
# .github/workflows/surface.yml
- uses: actions/checkout@v4   # plain checkout — do NOT set fetch-depth: 0
- uses: Connorrmcd6/surface@v0.7.0
```

Or the install script:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/Connorrmcd6/surface/main/install.sh | sh
```

Prebuilt binaries for macOS (Apple Silicon) and Linux (x86_64); build from source on other
Unix arches. Windows is **not supported** (the anchor grammar requires forward-slash paths).
Full options — pre-commit hook, `cargo install`, the architecture matrix — in
[Install](docs/getting-started/install.md).

## Documentation

Full docs at **[surface.gradientdev.xyz](https://surface.gradientdev.xyz)** (source in
[`docs/`](docs/index.md)):

- [Quickstart](docs/getting-started/quickstart.md) · [Install](docs/getting-started/install.md)
- [Authoring hubs](docs/guides/authoring-hubs.md) — claims, anchor grammar, granularity, the verify loop.
- [Surface and OKF](docs/guides/okf.md) — hubs as conformant Open Knowledge Format concepts.
- [CI integration](docs/guides/ci-integration.md) — the Action, the pre-commit hook, scoping a PR.
- [Examples](docs/examples.md) — a minimal hub in each supported language.
- Reference: [Commands](docs/reference/commands.md) · [Configuration](docs/reference/configuration.md) · [How the gate works](docs/reference/how-it-works.md) · [FAQ](docs/reference/faq.md)

Release history is in [`CHANGELOG.md`](CHANGELOG.md). AI agents working in this repo: see
[`AGENTS.md`](AGENTS.md).

## Benchmark

The agent-impact benchmark — measuring how much documentation accuracy changes an agent's task
performance — lives in its own repo: **[Connorrmcd6/surface-bench](https://github.com/Connorrmcd6/surface-bench)**.
It consumes the `surf` binary's output but has no inbound dependency on this core. (It previously
lived under `bench/` here.)

---

<sub>The naming isn't decoration: the *gradient* of a field is everywhere perpendicular to its level *surfaces* — the direction of change, and the thing the change is measured against. Surface reports **divergence** between what your docs claim and what your code does.</sub>

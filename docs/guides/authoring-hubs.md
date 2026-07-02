---
title: Authoring hubs
description: Write claims, learn the anchor grammar, choose the right granularity, and drive the verify loop.
---

A **hub** is a markdown file whose frontmatter anchors sentences ("claims") to the code they
describe. This guide covers writing claims, the anchor grammar, choosing the right granularity,
and the verify loop. For the end-to-end first run, see the [Quickstart](../getting-started/quickstart.md).

## Anatomy of a hub

```yaml
---
summary: How auth refresh rotation works.
anchors:
  - claim: refresh rotation is single-use; reuse triggers global logout
    at: src/auth/refresh.ts > rotateRefreshToken
    hash: 9b1c33ade8f1        # written by `surf verify`, not by hand
refs: []
---

# Auth

Prose a human (or agent) reads to understand this domain.
```

- **`claim`** - one sentence stating an invariant. Write what must stay true, not how the code
  is structured. A claim that restates the implementation rots as fast as a comment.
- **`at`** - the anchor: where the claim's logic lives (grammar below).
- **`hash`** - the seal. Absent until you `surf verify`; the gate treats a hashless claim as
  *unverified*.
- **`refs`** - hub composition: paths to *other hubs* this one builds on, written relative to this
  hub (`./resolve.md`), optionally `> symbol` to point at one claim within the target
  (`./resolve.md > resolve_nodes`, matched against that claim's `at:` anchor). `surf lint` blocks a
  ref that doesn't resolve to a hub, points at this hub, or names a claim the target lacks. The
  `check` gate also **propagates staleness one hop**: when a hub you `ref` has an open divergence,
  this hub fails too (a `referenced_stale` divergence) - review the dependency and re-verify. Only
  *direct* refs propagate; a chain `A → B → C` stops at one hop.
- **`covers`** - advisory file-scope globs; parsed and lint-validated but never affects
  `surf check`. Leave it empty unless you have a reason - the feature that consumes it isn't
  shipped.

Where hubs live is configured by the `hubs` glob in `surf.toml` (default `hubs/*.md`); keep them
central or co-locate them with code (`["**/_hub.md"]`).

## A hub is an onboarding doc

The most common failure mode is writing a hub like a **claim-log**: one claim per function, each
restating what a single symbol does, with a thin heading and no real prose. That's a changelog of
symbols, not a briefing - and it makes the verify loop a rubber-stamp, because nothing connects the
claims to a *system*.

A good hub is the opposite: **prose first**, documenting a system, with a handful of **coarse
claims** that each seal one behavior across the places it actually lives.

| | Claim-log (avoid) | Onboarding doc (aim for) |
| --- | --- | --- |
| **Claims** | one per symbol, near 1:1 | one per *behavior*, often spanning 2–3 sites |
| **`at:`** | a single symbol each | multi-site lists for system-level invariants |
| **Body** | a thin `#` heading | the key distinction, a `## How it works`, a Boundary note |
| **Reads as** | "what each function does" | "how this system works and what must stay true" |

Concretely, a good claim describes *a behavior of the system* and seals every span that behavior
depends on:

```yaml
- claim: commission is the only multi-level payout - it walks the referral graph up to three
    ancestors, pays REFERRAL_COMMISSION_RATES[tier][level], and skips self-edges
  at:
    - backend/referral-commission.service.ts > ReferralCommissionService > buildCommissionRecords
    - packages/constants/ReferralCommission.ts > REFERRAL_COMMISSION_RATES   # one invariant, two sites
```

Write the prose a reader needs to onboard - the single most important distinction, how the pieces
fit (`## sections`, tables), and a **Boundary** note on what the gate does *not* cover - then anchor
the invariants with as few claims as the behavior allows. `surf lint` nudges the other way when a
hub drifts into claim-log shape (see below).

## Bootstrapping with `surf suggest`

Authoring claims by hand is the main adoption cost. To get a head start, point `surf suggest` at
your source and it lists the top-level public functions no hub anchors yet, as a copy-pasteable
starter hub:

```sh
surf suggest "src/**/*.ts"        # or --format json for tooling
```

It only suggests - it never writes a file or stamps a hash. The output is a **list of undocumented
symbols, not a list of claims to write**: it groups the symbols by file and emits a multi-site
`at:` skeleton so the default shape steers you toward coarse, consolidated claims. Paste it into a
hub (or `surf new <name>`), then **group related symbols into a few system-level claims** - write
real prose, list the sites each behavior spans under one `at:`, and delete what you don't need
before `surf verify`. Treat it as a checklist of undocumented surface, not a mandate to write one
claim per symbol (see [a hub is an onboarding doc](#a-hub-is-an-onboarding-doc) and granularity below).

## The anchor grammar

An anchor is a file path, then a `>`-separated symbol path:

```
src/service.ts > TokenService > rotate
```

- **One segment** points at a top-level symbol: `src/m.rs > parse_anchor`.
- **Nested segments** walk into scopes: a type and its `impl`/methods share a name, so
  `Type` alone may be ambiguous while `Type > method` is unique. Methods are addressed with
  `>` segments, **not** a dot - write `TokenService > rotate`, not `TokenService.rotate`
  (the same applies to a TS/JS class method like `EffectiveTierService > getForUsers`).
- **Non-callables** anchor too, not just functions: in Python, module constants, type aliases
  (`X = Literal[...]`, `type X = ...`), and class attributes (`Class > attr`); in Rust/Go,
  `const`/`static`/`var` items; in TS/JS, exported `const`/`let`/`var` (`TIER_MAP`). Anchor the
  value whose drift the sentence is about.
- **`@N`** disambiguates genuine name collisions (1-based), e.g. two overloads:
  `src/api.ts > handler@2`. Python `@overload` sets are the exception: consecutive stubs plus
  their implementation resolve as *one* symbol, so the bare name works and the hash covers
  every signature.
- **Multiple sites (the default for a system claim)** - a real invariant usually lives in more
  than one place. An `at:` list combines its sites into one hash, so the claim is stale if *any*
  listed span changes. Reach for this **first**: one coarse claim sealing a behavior across the
  2–3 places it lives is the shape of a good hub - not one claim per symbol.
  ```yaml
  - claim: a refresh token is accepted at most once - rotation issues a new one and the old is
      rejected everywhere it's checked
    at:
      - src/auth/refresh.ts > rotateRefreshToken
      - src/auth/refresh.ts > validateRefresh
  ```

Run `surf lint` to confirm every anchor resolves to exactly one symbol. Ambiguous or vanished
anchors **block**; a symbol that was merely renamed - or a file that git reports has moved - only
**warns** and points you at `surf verify --follow`.

## Choosing granularity

This is the central tension (proposal §8):

- **Under-anchor** → real drift slips through, because the changed logic wasn't anchored.
- **Over-anchor** → every incidental edit re-triggers verification, and humans start
  rubber-stamping `verify` without reading - which defeats the tool.

`surf lint` emits advisory warnings (never blocking) to nudge you toward the middle:

- **Near-whole-file span** - the anchored symbol covers most of its file. Anchor a narrower
  symbol so unrelated edits don't trip the claim.
- **Too many anchors in one hub** - split the hub; a long verify list invites rubber-stamping.
- **Uncovered public function** - a public function in a file the hub already anchors has no
  claim. Either add one, or accept it as intentionally undocumented.
- **Claim-log shape** - a hub with several claims that *never* use a multi-site `at:` reads as one
  claim per symbol. Consolidate related claims into fewer coarse ones (see
  [a hub is an onboarding doc](#a-hub-is-an-onboarding-doc)).
- **Thin prose** - a multi-claim hub whose body is a stub. A hub is an onboarding doc; add prose
  that frames the system, not just claims that anchor its symbols.

Rule of thumb: anchor the **smallest symbol whose logic the sentence is actually about.**

If a claim sits on a large symbol where user-facing copy changes often, set `ignore_literals: true`
on it - string-literal *content* is then excluded from its hash, so a copy tweak no longer
re-opens the claim while logic edits (operators, numbers, structure) still do. Prefer a narrower
anchor first; reach for `ignore_literals` when the span genuinely must stay coarse.

```yaml
anchors:
  - claim: the engine emits one result row per fixture
    at: src/engine.ts > computeResults
    ignore_literals: true
```

## The verify loop

`surf verify` is the human escape hatch: it re-seals a claim after *you* confirm the prose still
holds, writing the hash into the frontmatter (and touching only that line).

```sh
surf check                      # DIVERGED? a claim's anchored logic changed
# re-read the claim:
#   still true  → surf verify [<at>]      (re-seal)
#   now false   → fix the prose first, then verify
surf verify --follow            # renamed symbol OR moved file: re-point the anchor and re-hash
```

Verifying without reading is the failure mode the whole tool exists to prevent. A green gate
promises only "nothing anchored changed since last sign-off" - never that the prose is true.

## Where claims can live

A hub isn't a special file *type* - it's **any file the `hubs` glob matches that parses as a hub**
(a `---`-fenced `anchors:` frontmatter block + a markdown body). Claims don't have to live under
`hubs/`: add any file to the glob in `surf.toml`, give it the frontmatter, and `surf` treats it
like any other hub. The same is true for `AGENTS.md` or `CLAUDE.md`.

The common question is `AGENTS.md` - the *imperative* operating instructions for coding agents,
versus hubs, which are *declarative* domain briefings. There are two approaches, and **a central
`hubs/` directory is the recommended default.**

### Recommended - keep hubs and `AGENTS.md` separate

Keep the two concerns apart: don't copy hub prose into `AGENTS.md`. Instead, give `AGENTS.md` a
pointer block that sends agents to the hubs directory to search for what they need:

```markdown
<!-- surf:hubs -->
Context lives in [`hubs/`](./hubs/) - read only the hub(s) you need.
<!-- /surf:hubs -->
```

When that block is present, `surf lint` checks it links the configured hubs directory and that the
directory exists. It deliberately does **not** enumerate individual hubs - that would push an agent
to read everything instead of the one hub it needs.

This keeps `AGENTS.md` lean, stops the declarative and imperative content from drifting into each
other, and keeps verification metadata out of the file agents read as instructions.

### Alternative - fold claims into `AGENTS.md` / `CLAUDE.md`

If you'd rather keep the instructions and the verified claims about them in one file, add it to the
glob and give it hub frontmatter:

```toml
# surf.toml
hubs = ["hubs/*.md", "AGENTS.md"]
```

```markdown
---
anchors:
  - claim: the CLI exits non-zero when the gate finds a diverged claim
    at: surf-cli/src/main.rs > main
    hash:                       # written by `surf verify`
---

# Agent instructions

...your normal AGENTS.md prose...
```

`surf verify` then hash-checks that claim like any other hub, and `surf for <path>` reports
`AGENTS.md` as anchoring into it. Three things to weigh:

- **The whole file must parse as a hub.** The `---` frontmatter has to be the top block and unknown
  fields are rejected - you can't sprinkle a claim mid-document.
- **The frontmatter is part of what the agent reads.** Most agent runners load `AGENTS.md` /
  `CLAUDE.md` as raw text and don't strip YAML frontmatter, so the `anchors:` block lands in the
  agent's context as a little extra noise. It's small and structured, but if you want `AGENTS.md`
  to stay purely instructions, prefer the recommended approach above.
- **It couples the file to code structure.** Renaming an anchored symbol trips the gate on
  `AGENTS.md` - that's the point of Surface, but it means an agent-docs file now participates in CI.

### When *not* to anchor a file

"Any file can be a hub" doesn't mean every file should be. Pitch and marketing prose - a
`README.md` especially - is a poor fit:

- **The claims are coarse.** A README describes behavior in broad strokes that span many symbols,
  so an anchor either covers a near-whole-file span or trips on incidental edits - the
  over-anchoring trap from [Choosing granularity](#choosing-granularity).
- **It usually duplicates hub prose.** The invariants a README restates should already be claimed
  in the hubs anchored to the real code; anchoring the README just gives you a second copy to keep
  honest.
- **On GitHub, frontmatter renders as a table.** GitHub displays a Markdown file's YAML frontmatter
  as a metadata table at the top of the rendered page, so an `anchors:` block would sit above your
  pitch on the repo's front page.

Anchor the code, and let the README link to the docs instead.

See also: [CI integration](./ci-integration.md) · [Examples](../examples.md).

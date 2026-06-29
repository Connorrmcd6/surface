# Dogfood log

Raw, dated notes from using Surface on Surface (and on other repos). Not polished — this is
source material for write-ups later. One entry per notable moment: *what happened, what the tool
did about it, the lesson.* Keep it honest; the failures are the interesting part.

---

## 2026-06-29 — `refs` propagation fired first on the commit that created it

**Context:** PR2 of `refs` hub composition (#4) turns on staleness *propagation*: `surf check`
now flags a hub when a hub it `refs` has an open divergence (one-hop). The change lives in
`check_workspace` — which is itself an anchored claim in `cli-check.md`, and `cli-check.md` is
referenced by `cli-workspace.md` and `hub-format.md`.

**What happened:** the first `surf check` after wiring up propagation went red three ways:

```
DIVERGED  hubs/cli-check.md :: surf-cli/src/check.rs > check_workspace
    stored 2:4f5890aca70c → now 2:b7b7fd55206e  (magnitude: Large)
REF-STALE  hubs/cli-workspace.md :: ./cli-check.md
    referenced hub `hubs/cli-check.md` has an open divergence — review it, then re-verify
REF-STALE  hubs/hub-format.md :: ./cli-check.md
    referenced hub `hubs/cli-check.md` has an open divergence — review it, then re-verify
surf check: 3 divergence(s).
```

The new feature's *first real firing* was on the very diff that introduced it: editing
`check_workspace` diverged its own claim, and propagation — the thing being added — immediately
walked the two hubs that compose it and flagged them too. Fixing the root cleared all three at
once: I updated the `check_workspace` claim prose to describe propagation, re-sealed it
(`surf verify "surf-cli/src/check.rs > check_workspace"`), and both inherited `REF-STALE`s
vanished with it.

**Why it's a good story:** the composition graph proved itself end-to-end without a contrived
example. It also makes the §8/§11.3 risk concrete: one genuine divergence amplified into three
findings (1 root + 2 inherited). That's the cascade the proposal worried about — but the shape
held up: the inherited flags are clearly labelled, point at the root, and clear the instant the
root is re-sealed. One-hop did its job too — `cli-check` itself `refs` `cli-git`/`cli-verify`,
which were clean, so nothing spread further, and the two `REF-STALE` hubs didn't re-propagate
onto *their* referrers (propagation is built only from base divergences).

**Lesson / open question:** "fix the root, the inherited flags clear" is the property that makes
propagation usable rather than noisy — but it relies on the author recognising a `REF-STALE` as
*derived*, not a second thing to fix. Open question: at scale, is a 1→N amplification per stale
hub still legible, or does `check` eventually want to *group* inherited flags under their root
(print the root divergence, then "and N hubs that ref it") rather than as N peer lines?

---

## 2026-06-29 — The new claim-log nudges flagged 22 of our own hubs

**Context:** #142 argues the CLI's in-loop signals (`surf suggest`, `lint_under_coverage`) teach
agents to write *claim-logs* — one claim per function, near-1:1 symbol→claim, no prose — because
nothing rewarded consolidation. We added the symmetric counter-pressure: a *claim-log* warning
(several claims, never a multi-site `at:`) and a *thin-prose* warning (multi-claim hub, stub body).

**What happened:** the moment they ran, `surf lint` reported **22 warnings on our own hubs** —
0 errors, exit 0. Notably *zero* of our 17 hubs had ever used a multi-site `at:` list, and
`cli-check.md` (the example the issue calls out as too thin) tripped both new warnings. The repo
that ships the tool was itself the thing the issue describes.

**Why it's a good story:** it's the cleanest possible confirmation of the issue's thesis — the
authors of Surface, dogfooding daily, still drifted into per-symbol logging because the loop only
ever nudged toward *more* coverage, never toward *fewer, coarser* claims. The fix isn't "write
better docs"; it's adding the missing signal. The warnings are advisory (exit 0) by design, so
they nudge without blocking — but 22 of them is a loud, honest nudge.

**Lesson / open question:** advisory-but-loud is the right register for a stylistic nudge, but
22 warnings risks being tuned out. Open question: should consolidation be a single per-hub summary
line rather than one warning per offending hub, and is the multi-site `at:` count the best single
proxy for "this author thinks in systems, not symbols"?

**Follow-up (same day):** we then ate the dogfood — refactored the 6 flagged hubs in the same PR.
Adding real body prose to the 5 thin ones was free (bodies aren't hashed, so no re-verify), and
`cli-git` got the repo's *first* multi-site claim: one invariant ("every git query degrades to
None; the verdict never depends on it") sealed across all five helpers, which let us trim the
per-function boilerplate. Writing it surfaced the same thing the AGENTS.md entry did — consolidating
forced us to name the shared contract explicitly. New-warning count on our hubs: 6 → 0. (The 16
`under-coverage` warnings are a separate, older itch.)

---

## 2026-06-17 — Making AGENTS.md a hub caught AGENTS.md lying about itself

**Context:** We documented that `AGENTS.md`/`CLAUDE.md` *can* double as a hub (any file the `hubs`
glob matches that parses as a hub counts), then went to actually wire it up here: added `AGENTS.md`
to the glob and sealed one claim anchored to `lint_agents_pointer` — the lint rule that polices
`AGENTS.md` itself.

**What happened:** the claim couldn't be written as the existing prose. `AGENTS.md` said:

> `surf lint` enforces that this block stays — pointing at the hubs directory, never duplicating
> or enumerating individual hubs.

But `lint_agents_pointer` only checks that the `surf:hubs` block *links the hubs directory and that
the directory exists* — it does **not** enforce non-enumeration (that's design convention, not code).
The prose had quietly overstated the tool. The discipline of writing a claim that must match a
specific symbol's actual behavior forced the correction: split the sentence into what lint enforces
(link + existence) vs. what's by convention.

```
surf verify "surf-cli/src/lint.rs > lint_agents_pointer"  → updated AGENTS.md
surf check                                                 → all anchored spans match
```

**Why it's a good story:** the self-referential loop closed — `AGENTS.md` now carries a sealed
claim about the rule that governs `AGENTS.md`. And the mere act of making a sentence *sealable*
surfaced that the un-anchored version had drifted from the code. The claim didn't catch drift over
time; it caught an overstatement that already existed, because anchoring forces you to say exactly
what the symbol does.

**Lesson / open question:** "write it so it can be anchored" is itself a forcing function for
honest prose, separate from the gate ever going red. Open question: how much of an imperative
instructions file is genuinely anchorable? Here it was exactly one sentence — the rest is process,
deliberately left unanchored. Coverage is still the product (cf. the 2026-06-12 entry); over-anchoring
`AGENTS.md` would just invite rubber-stamping.

---

## 2026-06-12 — Instructions are advisory; the gate isn't (agent edition)

**Context:** Asked Claude to knock out the 0.6.1 quick wins (`#71`, `#67`). It changed `surf for`'s
error path in `for_path.rs`, ran `cargo test` (64 green), `cargo fmt --check`, even `sh -n` on the
installer — and pushed.

**What happened:** CI went red on the dogfood job:

```
DIVERGED  hubs/cli-for.md :: surf-cli/src/for_path.rs > run
    stored 3ffb208cc1db → now 3143f824dcfb  (magnitude: Small)
```

AGENTS.md step 3 says, in so many words, *run `surf check` before you push*. The agent had that
instruction in context and skipped it anyway — thorough about the checks it chose, blind to the
one the repo asked for. The gate didn't care. It doesn't read AGENTS.md; it hashes spans.

Second half: this is the **same anchor** as the PR 1 entry ("the gate caught its own author
lying") — but the opposite branch. There the prose had gone false and needed rewriting. Here the
claim describes the contract (a directory errors, exit 1 — the `#53` rewrite already said so),
and the change only improved the *message text*, so the right move was a bare re-seal:
`surf verify`, one anchor stamped, green. Both branches of the discrimination the tool forces
have now been walked on the same claim, two days apart.

**Why it's a good story:** the agent angle. Prose instructions to an agent are advisory — it
followed five and dropped the sixth, which is exactly the failure mode prose always had. The
deterministic gate was the only layer that didn't depend on being obeyed. If agents are going to
write more of the code, "docs enforcement that doesn't rely on the author's diligence" stops
being a nice-to-have.

**Lesson / open question:** agent-proofing isn't more sentences in AGENTS.md — it's hooks. The
pre-commit wiring exists (`CONTRIBUTING.md`); should installing it be the *first* thing an agent
session does, or should `surf check` sit in a pre-push hook so the local gap can't happen at all?

---

## 2026-06-12 — The issue tracker is un-anchored prose: #43 rotted

**Context:** Triaging 0.6.1 for quick wins. `#43` said: `pick()` in `surf-core/src/resolve.rs` is
duplicated logic, never called, delete it. Filed with provenance and everything — file, line
range.

**What happened:** the code disagreed. The Go resolver (landed after the issue was filed) calls
`pick()` twice. The issue's claim was true at filing and went false silently when `resolve_go`
merged — nothing gates issue text, so it rotted exactly the way the thesis predicts un-anchored
claims do. "Implementing" it would have broken the build. Closed as stale instead.

**Why it's a good story:** an issue is a claim about code with provenance but no hash — the
purest specimen yet of *what's anchored is enforced; what isn't, rots*. But there's an honest
second edge: Surface couldn't have gated this one either. "This function is unused" is a
whole-program property — it lives in the *callers*, not in the span you'd anchor. A hash on
`pick()` itself would have sat green while the claim went false around it. Same blind-spot family
as the `public_symbols` coupling in the 06-11 entry.

**Lesson / open question:** for dead-code claims the right gate is the compiler
(`#[warn(dead_code)]`, or deleting and letting the build vote), not a span hash. Pattern worth
naming when writing this up: *match the claim to a gate that can actually see the property* —
span-local truths get anchors, whole-program truths get the toolchain.

---

## 2026-06-11 — What an anchor can reach, and what it can't

**Context:** PR 3 of 0.6.0 (`#52`) — adding `surf suggest --all` to propose Python classes and
non-callables. It touched the shared `public_symbols` enumerator, the clap `Command` enum, and
`suggest.rs`.

**What happened — the reach:** `surf check` tripped on `hubs/cli-reference.md`, whose claim is
anchored to the `Command` enum and whose prose *literally ends with an instruction to me*:

```
... Adding, removing, or renaming a command or flag, or changing a default, diverges this
anchor — re-read docs/reference/commands.md before sealing.
```

`docs/reference/commands.md` is a hand-written human doc with **no anchor of its own** — nothing
hashes it, so on its own it could rot freely. But because the *source of truth* (the clap enum)
is anchored, and the claim encodes the cross-reference, adding `--all` forced the gate red until
I went and updated that un-anchored sibling doc. An anchor on the thing that changes, used as a
tripwire for the prose that describes it elsewhere. That's a pattern worth naming: you don't have
to anchor the downstream doc, you anchor its *cause* and write the pointer into the claim.

**What happened — the blind spot:** PR 2 had just re-pointed `lint`'s coverage nudge at
`public_symbols`. In PR 3 I broadened `public_symbols` — and if I'd broadened its *default*
instead of gating the new kinds behind `--all`, `lint` would have started flagging every
unanchored class and constant in every repo. The gate could **not** have caught that: no hash
changes, no anchored span moves — it's a semantic coupling between two callers of a shared
function. I had to hold it in my head and design around it. Nothing in Surface protects you from
it.

**Why it's a good story:** the two halves are a clean contrast. The gate's reach is longer than
"the span you anchored" — via an instruction in the claim it pulled an un-anchored doc into
scope. But its blind spot is equally real: behaviour that emerges from how two functions share a
third is invisible to a per-span hash. Anchor the cause, not just the symbol — and don't expect
the gate to catch coupling it can't see.

**Lesson / open question:** the `commands.md` trick (anchor the source of truth, point at the
prose) generalizes — is it worth documenting as an authoring pattern? And the blind spot is the
honest counterweight to the PR 1 entry's "what's anchored is enforced": *what's anchored is
enforced span-locally; cross-symbol invariants still live only in your head.*

---

## 2026-06-11 — The gate caught its own author lying

**Context:** Implementing PR 1 of the 0.6.0 milestone (`#53` + `#38`) — making `surf for`,
`surf check --files`, and `surf stats` fail loudly on malformed input instead of returning a
falsely-reassuring success.

**What happened:** After editing `for_path.rs` and `stats.rs`, I ran the repo's own gate
(`surf check`) as the final verification step. It failed — on Surface's *own* anchored claims:

```
DIVERGED  hubs/cli-for.md :: surf-cli/src/for_path.rs > run
    claim: ... It is a query, not a gate, so it always exits 0 whether or not anything matched.
```

That claim had been **true at 0.5.0 and was now false** — the whole point of `#53` was to make
`for` exit 1 on a mistyped path. The change to the *behavior* and the change to the *documented
contract* were the same act, and the gate refused to let them diverge silently. I couldn't
re-seal the hash without first deciding: is the prose still true? It wasn't, so I rewrote it.

Three claims tripped (`cli-for`, `cli-stats`, `cli-check`). Two were genuine contract changes
that needed new prose; one (`check_workspace`) only shifted because an adjacent line moved, so it
just needed re-sealing. The tool made me look at all three and tell them apart by hand — which is
exactly the discrimination it's supposed to force.

**Why it's a good story:** the usual pitch for docs-as-tests is abstract ("docs drift from
code"). This is the concrete version, and it's self-referential: the gate caught *its own
maintainer*, mid-feature, shipping a behavior change that contradicted a sentence the tool itself
was responsible for guarding. The stale module doc comment in `for_path.rs` (`// A query, not a
gate: it always exits 0`) was **not** anchored — so it drifted with zero resistance, and I only
caught it by eye. A nice illustration of the boundary: what's anchored is enforced; what isn't,
rots.

**Lesson / open question:** the un-anchored doc comment drifting while the anchored claim held is
the sharpest line in the whole episode. Worth a callout in any write-up: coverage is the product.
Possible follow-on — should `lint` nudge toward anchoring module-level doc comments that restate
a contract? (Adjacent to `#54`, the coverage-nudge work.)

---

<!-- New entries above this line, newest first. Template:

## YYYY-MM-DD — One-line hook

**Context:** what you were doing.
**What happened:** the moment, with the real command/output if you have it.
**Why it's a good story:** the angle a reader would care about.
**Lesson / open question:** what it changed or what it leaves open.
-->

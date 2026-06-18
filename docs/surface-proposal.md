# Surface — A Proposal

*Part of **Gradient**. A deterministic gate that surfaces divergence between architectural documentation and the code it describes — loudly, in CI, instead of silently over months.*

> **Naming.** The platform is **Gradient** (gradientdev.xyz), a home for small, sharp developer tools named for the calculus they evoke. **Surface** is the first of them; its CLI is **`surf`**. The geometry isn't decoration — the *gradient* of a scalar field is everywhere normal to its level *surfaces*, so the platform and the product name the same object from two sides: the direction of change, and what the change is measured against. The unit of documentation is a **hub** (frontmatter + prose); what the gate reports is **divergence** between a hub's claims and current source.
>
> Utility commands stay deliberately plain — `surf lint`, `surf check`, `surf verify`, `surf index` — because utility has to be familiar. The calculus lives in the brand and in the signal, never in the command names.
>
> This is a design draft meant to be argued with and cross-reviewed before anything is built. It takes decided positions on purpose — it is easier to argue with a claim than with a hedge.

---

## 1. The one-sentence version

Domain knowledge about a codebase is worth nothing the moment you can't tell whether it's still true. This proposes a deterministic CI check that detects when documented code has changed out from under its documentation, and blocks the merge — the same way a failing test does. Everything else in the document is downstream of that one mechanism.

## 2. The problem we actually have

Today domain knowledge lives in long-lived tickets — one per area ("how Auth works, its dependencies, its gotchas") that an agent or a new hire is told to read to "become an expert." It worked small and rotted large, in four specific ways that are the real design drivers:

1. **It was an append-log, not a current-state document.** Updates were added over time, so present truth got buried under sediment. Neither a human nor an agent can tell what is true *now*.
2. **It had no size ceiling.** It was implicitly an encyclopedia — a full substitute for reading the code — so it grew without bound until it stopped being read.
3. **It conflated two kinds of knowledge.** *Derived* facts (what the code does — recoverable from source) and *authored* facts (why it's built this way, the gotchas — genuinely tribal) were maintained identically, which serves neither.
4. **It lived away from the code.** In a tracker, it couldn't be diffed against source, updated in the same change, or gated in CI. Out of sight is exactly the gradient that produces drift.

Note that (4) is the root cause and the other three are symptoms. Move the knowledge next to the code and put a gate on it, and the append-log habit, the bloat, and the conflation all become things you can lint against.

## 3. The thesis

**Make architectural knowledge participate in the same governance loop as the code it describes.**

The hard problem is not *creating* documentation and it is not *retrieving* it. Retrieval is a crowded, well-funded space — vector DBs, knowledge graphs, RAG, MCP servers — and it is largely solved. The unsolved problem is *trust*: knowing that what you retrieved is still true. This optimizes trustworthiness, not retrieval. Version-controlled, drift-checked, composable domain briefings that live beside the code, where divergence between the documented claim and the current source is surfaced as a blocking signal.

The borrowed shape is **Terraform's `plan`**: diff documented reality against current reality, surface the delta as something you must act on. The borrowed governance model is **dbt's**: version control, modularity, cheap declarative tests, lineage. We take dbt's *discipline* and explicitly **not** its compile-and-materialize engine — our markdown body *is* the artifact, there is no transform to run, so architecturally we are closer to a linter plus a static-site generator than to dbt.

**Why this survives the AI hype cycle.** A system justified as "context for agents" dies if agent frameworks churn or usage plateaus. A system justified as "your architectural invariants are now governed like code" does not — it pays off for humans (onboarding, review, incident response) with zero agents in the loop. We build the second thing. Agents are a *multiplier* on top of it, not the foundation under it.

**Why agents multiply it.** A human onboards onto a domain once and amortizes the cost over months. An agent re-onboards every session and amortizes nothing — it is a new hire on their first day, every day. So the cost of missing or wrong context, which a human pays once, an agent pays on every invocation. For an agent-heavy team this drags the break-even point *down*: a 12-person team running agents hard against a large, fast-moving codebase has the context-consumption profile of a much bigger org. We are not claiming deterministic agents — inference is stochastic and we don't try to fix that. We make the *context layer* governed and reproducible; what the model does with it is a separate concern.

## 4. When this is worth building — and when it is ceremony

The honest axis is not headcount. It is roughly:

> **codebase complexity × change velocity × (human + agent) context-consumers**

- **Complexity floor.** A small, simple codebase needs no governed context no matter how many agents read it — they can read the whole thing. The apparatus earns its keep only when rebuilding the mental model from source is genuinely expensive.
- **Velocity floor.** Drift detection is worthless if nothing drifts. Slow code survives on hand-maintained docs.
- **Consumers, including agents.** This is where intensity pulls break-even below the "~50 engineers" a headcount reading suggests.

A predictable objection: *"agents have huge context windows now — just read the code."* A bigger window fixes retrieval, not trust. An agent can read every line and still confidently derive a **wrong** model: it cannot distinguish a deliberate invariant from incidental implementation, and it cannot see the *why*. Confidently-wrong derivation is the failure mode — so more capable agents *raise* the value of authored context, they don't remove it.

**Where it is over-engineering — say so out loud:** a few engineers, a small/slow/simple codebase, light agent use. There, splitting and pruning two plain docs beats this entirely. Adopt the full apparatus and it becomes pure ceremony. The proposal's credibility depends on conceding this case rather than implying universal value.

## 5. The architecture, in one decision

There is exactly one architectural decision that matters, and everything else hangs off it:

> **The core is a deterministic gate with no model, no network, and no API key. Any intelligence is an optional plugin that reads the gate's structured output.**

The core hashes the code each claim points at, compares to a stored hash, and blocks or passes. Its verdict is always reproducible. It is the whole product, and it is adoptable with zero AI involvement. The moment an LLM becomes *required*, the "stateless binary, runs in any repo, reproducible verdict" thesis collapses — you've bought a key, a provider, network egress in CI, per-PR inference cost, and nondeterminism, all at once.

So the LLM lives **outside** the boundary. `surf check` emits a structured report of the diverged claims (`--format json`: which hub, which claim, the old and new code, the prose). That report is the contract everything optional plugs into. Pull every plugin out and the gate blocks and passes exactly as before; you just fall back to a human running `verify`.

This is the single most important sentence in the document, so it gets restated: **the fuzzy layer can only ever advise; it can never be load-bearing for the verdict.**

## 6. What exactly gets hashed

The entire value of the gate is whether it fires on the *right* change. Hash the wrong thing and you get false alarms; false alarms train people to rubber-stamp; rubber-stamping silently kills the gate. So this section is the technical heart of the proposal.

A claim binds prose to specific code:

```yaml
anchors:
  - claim: "refresh rotation is single-use; reuse triggers global logout"
    at: "src/auth/refresh.ts > rotateRefreshToken"   # a symbol, not a file
    hash: 9b1c33a
```

The gate hashes **only the span of the named symbol**, not the file. A helper added elsewhere in `refresh.ts`, an unrelated function, a reformat three lines down — all invisible, because the hub never claimed anything about them. Per-claim hashing also makes staleness surgical: "your *refresh rotation* claim changed; the rest of the hub is fine."

### 6.1 The mechanism: AST-canonical hashing via bundled tree-sitter

This is where I diverge from the obvious path, and the reasoning is the most important thing to cross-review.

The tempting MVP is "hash the symbol's text, normalized through the repo's formatter, with the symbol located by ctags or an LSP." It looks lighter than parsing. It is a trap, for three connected reasons:

- **ctags gives you a line, not a span.** It reliably reports where a symbol *starts*; end-of-span coverage is inconsistent across languages. But you must hash start-to-end. Reliable polyglot spans come from a parser or a language server, not from ctags.
- **An LSP reintroduces exactly the dependency you were avoiding.** Getting spans from `documentSymbol` means a running language server per language, present in CI. That is heavier and flakier than parsing, and it breaks "runs anywhere with just the binary."
- **Formatter-normalization can't reliably run on a fragment, and it isn't reproducible.** Many formatters only operate on whole files or complete syntactic units, so you'd format the whole file then slice — which requires every language's formatter installed in CI. Worse, the result depends on the formatter's *version*: local formatter v2 and CI formatter v1 disagree, the hash differs, and the gate fires on a no-op PR. That is the rung-1 false-alarm disease, just relocated to toolchain skew — and it would poison trust before the tool ever proved itself.

The resolution is to notice that **the thing that gives you reliable spans is a parser, and once you have the parse tree the canonical hash is nearly free and is also the *correct* signal.** So the MVP gate is:

> **Locate the claimed symbol's node with tree-sitter; hash a canonicalized form of that subtree.**

Tree-sitter grammars are libraries compiled **into the binary**, not servers running in CI. This single choice resolves four problems at once:

| Problem with the "lighter" path | How AST-via-tree-sitter resolves it |
|---|---|
| Spans are unreliable from ctags | tree-sitter returns exact node ranges |
| LSP / formatter must exist in CI | grammars are bundled; **nothing** but the binary is needed at runtime |
| Hash isn't reproducible across environments | the parser ships *in* the binary and is version-pinned, so there is no environment to skew against |
| Text/format hashing is loud on renames, silent on `+`→`-` | canonical AST is *quiet* on renames and reformatting, *loud* on a flipped operator — the sensitivity profile you actually want |

That last row is the one to dwell on. A canonical AST hash ignores whitespace, comments, and formatting because they aren't in the tree; with light canonicalization (normalize identifier *positions*, not names) it can also ignore pure renames. But flipping `+` to `-`, `<` to `<=`, or deleting an `await` changes a node — so it fires. **It is quiet on the changes you want ignored and loud on the changes you must catch.** A text or similarity measure is backwards on both.

**The honest cost** is that you bundle a grammar per supported language and the binary supports a finite language set. That is a *build-time* cost, bounded and under your control — not a runtime dependency and not a reproducibility hazard. Start with the adopting team's actual stack (one or two languages); add grammars as needed. This is the right trade for a tool whose entire job is trust.

### 6.2 Why not a similarity score

A reasonable alternative is "standardize old and new, compute similarity, flag when divergence crosses a threshold." Rejected as a *gate*, for reasons worth recording so it stays rejected:

- **It is backwards on the cases that matter.** Token similarity is loud on renames (many tokens move) and near-silent on a single flipped operator (one token of hundreds) — exactly inverted from what a correctness tool needs. Off-by-ones and sign flips are not edge cases here; for a tool that documents invariants, they are the whole point.
- **A semantic embedding detonates the §5 boundary.** To make similarity "understand" logic you need a model — provider, version drift, non-reproducible vectors. That is the LLM dependency, smuggled into the core.
- **The threshold is uncalibrated and unexplainable.** A one-character change to a crypto constant is catastrophic; a 40% rewrite of a logging helper is nothing. No single threshold serves both, and "similarity fell to 0.83, cutoff 0.85" gives an author nothing to act on. The AST hash, by contrast, shows the exact diff — auditable, obvious why it fired.

**But the instinct behind it is right** — the boolean hash is information-poor. The clean way to honor it: since we already have both parse trees, compute a **tree-edit-distance magnitude** and ship it in the JSON report as *advisory triage metadata* — "claim changed; magnitude small; shape looks like a rename." It helps a human decide which of five blocked claims to read first. It **never** adjudicates. The rule that must never be built: "fail only if hash changed *and* magnitude > threshold" — that rule would specifically hide the one-operator logic flip, the highest-value catch.

### 6.3 The anchor grammar (specify it before building)

`at:` must resolve to exactly one node, deterministically, so the grammar needs disambiguation that bare `file#name` lacks (overloads, same-named methods on two classes, a type and a function sharing a name):

- **Qualified path:** `src/auth/refresh.ts > TokenService > rotate` resolves through the symbol tree.
- **Positional fallback:** `src/auth/refresh.ts > rotate @2` for the second `rotate` when names genuinely collide.

And one claim often has **multiple enforcement sites** (an invariant held across two functions plus a constraint). Force one-claim-one-symbol and authors either under-anchor (a false negative — see §8) or duplicate the claim. So `at:` accepts a **list**; the claim is stale if *any* listed span changes:

```yaml
  - claim: "a refresh token is accepted at most once"
    at:
      - "src/auth/refresh.ts > rotateRefreshToken"
      - "src/auth/refresh.ts > validateRefresh"
```

### 6.4 Renames are an MVP problem, not a graduation

A rename breaks anchor *resolution* (`lint` can no longer find `rotate`), and renaming is one of the commonest refactors — so this hits the MVP constantly, it is not deferrable. The decided behavior:

- **Symbol renamed but clearly present** (git rename detection + high AST similarity of the moved node): `lint` **warns**, not blocks, and offers `surf verify --follow` to re-point the anchor and re-hash in one step. Rationale: a rename is often *exactly* when you'd want to re-confirm the claim, but it must not hard-block routine work.
- **Symbol genuinely vanished** (no plausible match): `lint` **blocks**. A claim now points at nothing; that is a real broken reference.

## 7. The honest limit — what the gate does NOT promise

This section is a feature, not a disclaimer, and it must be loud.

**What the gate promises:** *the code a claim points at is unchanged since it was last verified.*

**What it does NOT promise:** *that the documented invariant still holds across the system.* Concretely: the "single-use refresh rotation" invariant is intact inside `rotateRefreshToken`, so its hash is green — but a new `bypassAuth` middleware in `src/server.ts` quietly exposes the rotation path. **The gate passes. The doc is now a lie.** This is *action at a distance*, and **no deterministic check can catch it**, because by definition the change is to code the hub never declared a relationship with. Catching it is taint analysis / security review — a different discipline.

The honest response is to **scope the promise and say so**, not to chase it with features. A governance tool that lets people believe it guarantees system-wide invariants when it only checks documented spans is *worse than no tool* — it manufactures false confidence about exactly the security properties it never examined.

**A direct warning for whoever pitches this:** the most persuasive demo — "watch it catch the auth bypass" — is the one thing the core *cannot* do. Lead with it and the first real breach it misses becomes "the tool failed." Lead with the disclaimer instead. The optional reviewer plugin can widen a look at changes inside a hub's declared `covers` globs, giving the `server.ts` change *a* look — but that is plugin territory, advisory, and only helps if the offending file is in scope. Beyond that, the disclaimer is the answer, permanently.

## 8. The real adoption risk: maintaining claims

The likeliest thing to kill this is not hash maintenance — the tool handles that. It is **claim maintenance**, which is effectively a new engineering discipline. Someone has to keep deciding *what deserves to be an anchored claim*, and that judgment is the actual burden.

The honest analogy is unit tests: teams that write good tests will write good claims; weak teams write shallow ones; some over-anchor into noise, some anchor nothing. Under-anchor and the gate misses real drift (false negatives). Over-anchor and every refactor triggers a wall of re-verifications and people rubber-stamp. **This tension — fatigue versus ghost-anchors — is the central design problem, and it cannot be solved by one deterministic knob**, because fatigue wants the gate *quieter* and coverage wants it *louder*. The chosen resolution:

- The **deterministic core stays narrow and quiet**: it blocks only on change to an explicitly documented span. AST canonicalization keeps it from firing on renames and formatting.
- The **escape hatch is `surf verify`**: re-hash after a human confirms the prose still holds — the explicit "I looked, still true." With symbol-scoped AST anchoring this is needed *occasionally*; running it on most PRs is a smell that anchors are too coarse.
- **Louder coverage is opt-in**, in the reviewer plugin, for teams that want it — never the base tool's default.

The residual question this leaves open is real and goes to cross-review: *does keeping the loud half optional mean most teams quietly accept the ghost-anchor hole?* (§11.)

## 9. The build sequence

### 9.1 MVP — build exactly this, then stop

The MVP is the smallest thing that tests one hypothesis: **does a freshness gate produce durable behavior change?** It is entirely deterministic — no LLM, no network, no key.

1. **Hub format** — frontmatter schema (`anchors`, `refs`, `summary`, prose body) + the anchor grammar (§6.3). The contract.
2. **`surf lint`** — frontmatter well-formed; every `at:` resolves to exactly one node via tree-sitter; rename warnings (§6.4).
3. **`surf check`** — the gate. AST-canonical hash of each anchored span, compared to the stored per-anchor hash; blocks only on a documented span that has *diverged*. Emits `--format json`. **The one load-bearing piece.**
4. **`surf verify`** — re-hash after human confirmation; `--follow` for renames.
5. **GitHub Action + pre-commit + config discovery** — what makes the gate actually run in a repo. The binary walks up from `cwd` to a `surf.toml` marker, like `git` / `ruff`.

Then **stop**. No `refs` resolver, no catalog, no MCP service, **no reviewer plugin**. The JSON output is the seam that lets all of those attach later without the core ever depending on them.

**A note on the CI step that everyone gets wrong:** the gate hashes the working-tree span and compares to the hash committed in frontmatter. It needs the checkout, *not* full git history — do **not** `fetch-depth: 0`. The only thing the base ref buys you is scoping (re-check only anchors whose files changed in the PR), and a shallow fetch of the merge base covers that.

**`covers` is authorable but inert.** The field is accepted, stored, and lint-validated (a malformed glob blocks), but **the verdict never reads it** — `surf check` is byte-for-byte identical whether or not a hub declares `covers`. The louder coverage pass that consumes these globs is consumed only by the reviewer plugin (§7), which the MVP excludes, and stays deferred until its false-negative rate is measured. So a hub may declare its scope today; nothing acts on it yet.

### 9.2 The MVP has a falsifiable success criterion

"Then stop" needs a decision rule, or it is unfalsifiable. Seed **one or two high-churn, high-stakes domains** (not the whole repo — the gate must fire early and visibly, and front-loading effort across a quiet repo is the worst adoption shape). Then over ~6–8 weeks measure:

- **Continue if:** when a seeded domain's code changes, the claim is updated or `verify`'d *in the same PR* in a clear majority of relevant PRs — i.e., the gate is changing behavior, not being routed around.
- **Kill / rethink if:** the gate gets disabled in config; *or* the `verify`-without-any-prose-edit rate is high (rubber-stamping — the gate fires, people clear it without looking); *or* nobody voluntarily authors a second hub after the seed.

The rubber-stamp rate is the key metric, because a gate that is cleared reflexively is indistinguishable from no gate while *looking* like success.

### 9.3 Deferred, with the trigger that unlocks each

| Enhancement | Unlock when |
|---|---|
| **Reviewer plugin** (LLM off `check --format json`: suppress clean refactors, uphold suspected falsifications, look at `covers` diffs) | Only for teams already running CI review bots. Ship advisory-only first; measure its false-negative rate before letting it suppress anything. |
| **`refs` (hub composition)** | After single-hub anchoring is proven. Composition *compounds* the granularity problem — don't stack the second-hardest problem on the hardest before the hardest is solved. |
| **`surf index`** (catalog) | When globbing `hubs/*.md` at load is too slow or a browsable catalog is wanted. Plain name on purpose — it's a utility, not a feature. |
| **MCP resolver service** | Only when multiple agent clients would otherwise duplicate resolution logic. The only long-running service in the whole design. |
| **IDE extension** (inline claim highlighting) | Reuses the Rust core via WASM — a real reason the language choice matters. |
| **Cross-repo registry** | Polyrepo-at-scale only. Pure over-engineering in a monorepo. |

## 10. Language and distribution

**Rust by default — and now the MVP justifies it, not a speculative future.** Because the MVP gate is AST-via-tree-sitter (§6.1), the language case rests on the thing actually being built, not on deferred graduations: Rust binds tree-sitter natively (Go's bindings are CGO, and CGO forfeits the clean static cross-compile that was Go's main draw), and the same parse/hash logic compiles to WASM for a future IDE extension without a TypeScript reimplementation. Performance is *not* the reason — the tool is I/O-bound. Sharing the parse layer across CLI, CI, and editor is.

**Distribution — build once, wrap many.** One static binary per `(os, arch)`; thin per-channel wrappers. Ship **first**: a binary, a `curl | sh` installer, a **GitHub Action wrapper**, a **pre-commit hook** — because most repos *consume the Action*, they don't "install" anything. npm (shim + per-platform `optionalDependencies`, never a `postinstall` downloader) and pip (`maturin` wheels, `uvx`-friendly) and brew come later, per where users actually reach. Don't ship channels nobody uses.

## 11. Open questions for cross-review

These are genuinely open. Several are the kind a second model should attack.

1. **The ghost-anchor hole (§8).** Keeping loud coverage optional protects the determinism story but may mean most teams silently accept the action-at-a-distance gap. Is the disclaimer-plus-optional-plugin division honest enough, or is it the proposal quietly conceding its most compelling use case?
2. **Change ≠ falsification.** The hash proves a span *changed*, not that the claim is *false*; AST canonicalization removes the rename case but a real logic change and a behavior-preserving refactor still fire identically. Accepted as a permanent property of the core (semantics belong to `verify` + plugin), or is there a cheap deterministic signal being missed?
3. **Granularity discovery.** What does "right" anchor granularity look like, and how does `lint` *guide authors toward it* (heuristics? warnings on over/under-anchoring?) rather than leaving it to taste? This is the §8 risk and it needs a concrete mitigation, not just naming.
4. **tree-sitter as the trust root.** Bundling grammars solves reproducibility, but it makes the binary's grammar versions the source of truth. Is grammar quality across the target languages good enough for stable spans? What is the upgrade story when a grammar changes and re-hashes everything?
5. **Rubber-stamp measurement.** §9.2 leans on the `verify`-without-edit rate as the kill signal. Is that actually measurable cleanly, and is "majority of PRs update in-place" the right bar or a vanity metric?
6. **`AGENTS.md` separation.** Hubs are declarative domain briefings; `AGENTS.md` is imperative operating instructions. The plan is a lint-enforced pointer block (`AGENTS.md` references hubs, never duplicates them). Does that survive contact with deadline pressure, or do people collapse them into one file anyway?
7. **Scope break-even.** Can an org's position on complexity × velocity × consumers (§4) be *measured* cheaply enough to give a real adopt / don't-adopt answer, rather than asserted qualitatively?
8. **Naming (resolved — one residual noted).** Platform **Gradient** (a multi-tool umbrella), product **Surface**, CLI **`surf`**, unit **hub**, signal **divergence**; utility commands stay plain. Resolved *away from*: `node` (hopelessly overloaded — Node.js, graph nodes, k8s nodes, and this tool sits near JS and talks about dependency graphs). The one accepted cost to flag for eyes-open agreement: **Microsoft Surface** is a permanent searchability tax — `surf`/`surface` queries will compete with a hardware brand. Judged worth paying (different category, low real-confusion risk), but it is a cost, not nothing.

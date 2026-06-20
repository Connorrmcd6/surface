# Surface design Q&A — divergence, claims, and workflow fit

A walkthrough exercise interrogating Surface's design: how divergence is detected, why
claims are structured the way they are, and where the model fits (and doesn't) for a
multi-repo, integration-heavy workflow.

---

## 1. What happens if a doc changes but not the code? Would we get a DIVERGED warning?

**No.** Surface's verdict is one-directional. It hashes the anchored *code span* and compares
to the `hash` stored in the hub. The claim's prose is never part of the hash.

In `check_claim` (`surf-cli/src/check.rs`):

- Each site is resolved, the **current source code** is hashed (`hash_anchor_with`), and the
  results combine into `new_hash`.
- `new_hash` is compared against the stored `claim.hash`.
- The prose (`claim.claim`) rides along as an advisory `prose` field but is **never hashed**.

So a doc-only edit — rewording the `claim:`, rewriting the markdown body, changing the
`summary:` — leaves `new_hash == stored hash`, and `surf check` exits 0. No DIVERGED.

**The asymmetry:** Surface guards *"did the code drift out from under the doc?"* — not
*"did the doc drift away from the code?"*. A doc edit that makes prose lie about unchanged
code is invisible to the gate.

For contrast:
- **Code changes, doc doesn't** → `DivergenceKind::Changed` → `DIVERGED` (the built-for case).
- **New anchor, no stored hash yet** → `DivergenceKind::Unverified` → `UNVERIFIED`.

---

## 2. How complex would it be to implement doc-change detection?

Two very different features hide here:

- **A. Structural prose-pinning** — "the prose text changed since last verified."
- **B. Semantic doc-drift** — "the prose no longer *accurately describes* the unchanged code."

A does **not** approximate B.

### A — prose-pinning: ~half a day, low risk

- Add `prose_hash: Option<String>` to `Claim` (`surf-core/src/hub.rs`).
- In `verify`'s `plan_claim`, also hash `claim.claim` and stamp via a new
  `set_anchor_prose_hash` writer (mirror `set_anchor_hash`).
- In `check_claim`, compare stored `prose_hash` to current prose; mismatch → a new
  `DivergenceKind::ProseChanged`.

**But it's largely a footgun:** it fires on every typo/reword (noise), says nothing about
correctness (prose wrong from day one stays green), and only covers the `claim:` string —
not the markdown body where most prose lives.

### B — semantic drift: large, against the grain

Judging "does this prose still describe this code" requires an LLM judge. Surface's entire
contract is the opposite — deterministic, git-free, no model in the verdict. Bolting an LLM
into `surf check` makes the gate non-deterministic and network-dependent, and introduces
false-positive/negative rates that need calibration (the multi-turn study now living in the
`surface-bench` repo). Realistically this is an **optional advisory layer**
(`surf check --semantic`, off by default, separate exit lane), shipped only after the bench
validates a judge — not a change to the verdict.

**Recommendation:** don't build A as if it's B. If the goal is catching docs that lie about
unchanged code, that's the bench's job first; the CLI work is delivery, gated behind a flag.

---

## 3. What happens with multiple claims pointing to the same function?

Surface treats each claim **fully independently** — no dedup, no awareness of shared spans.

- **`check`** — `for claim in &hub.frontmatter.anchors`; each claim resolves + hashes from
  scratch. Two claims on `src/m.rs > add` produce identical hashes (unless `ignore_literals`
  differs). When the function changes, **both** diverge → two `DIVERGED` entries for the same
  span, each with its own prose.
- **`verify`** — stamps each anchor by index; both get their own `hash:` line written.
- **`lint`** — does **not** flag duplicate claims. Its job is "every anchor resolves to
  exactly one symbol." Two claims on one unambiguous function pass clean. (`MAX_ANCHORS_PER_HUB
  = 12` counts anchors, not distinct spans, so piling claims on one function pushes toward that
  advisory warning.)

**Net:** safe (the gate never gets weaker from duplication — any claim can block), just not
deduplicated. A lint `Warn` for "multiple claims share an identical `at` + opts" would be a
small, natural addition.

---

## 4. How do you prevent claims and markdown drifting? Why keep them separate? Why not embed?

**Reframe:** the split isn't "claim prose vs body prose." It's **machine contract vs human
narrative**:

- **Frontmatter `claim` + `at` + `hash`** = a structured, schema-validated, machine-*writable*
  contract, pinned to a code span by hash.
- **Markdown body** = human narrative. **Not load-bearing** — nothing in the verdict reads it.

So drift between them isn't prevented because it doesn't need to be: only the claim is pinned
to code, and the body is allowed to rot harmlessly.

**Why separate:**
- The contract must be machine-parseable and surgically writable (`verify` rewrites one
  `hash:` line, leaving the rest byte-identical).
- The contract must fail loud (a typo'd field blocks the gate via `malformed_hub_divergence`).
- The narrative should be unconstrained markdown for the reader.

**Why not embed inline:** you'd lose schema validation + fail-loud, lose clean machine
writeback, and lose the crispness of the claim (a tight, testable assertion dissolving into
paragraphs can't be pinned). And you wouldn't save maintenance — `at` + `hash` exist either
way. A minimal valid hub is *just frontmatter*; the body is opt-in color.

**Where the critique bites:** if you write a body that re-explains the claim, that's a second
prose surface that can drift, and neither is hashed. The fix is writing *less* body, not
merging it into code.

---

## 5. Isn't the claim just a middle man — is it even necessary?

The middle man is **inverted**. By field role in the verdict:

- `at:` = *where* to look.
- `hash:` = *did it change?* — pure mechanism, the only field in the verdict path.
- `claim:` = *what we believed* — never hashed, never read by the gate.

The **hash is the middle man** (a tripwire). The **claim is the payload** the tripwire delivers.

Delete the claim and a divergence says only *"this span's hash changed"* — a worse
`git diff --name-only`. You'd know code moved but not **what to re-check**. Keep the claim and
the gate hands the reviewer the specific belief to re-validate: *"did you preserve
same-transaction revocation?"* That is the product.

Necessity by elimination:
- Delete the **body** → lose nothing load-bearing.
- Delete the **hash** → lose the trigger (automation), not the meaning.
- Delete the **claim** → lose the documentation itself; what remains is a checksum.

**Where the critique still bites:** if a claim merely restates obvious code
(`claim: adds two numbers` over `fn add`), it's dead weight — delete the whole anchor. The
sharp question isn't "is the claim necessary?" but **"is *this* claim saying anything the code
doesn't?"** When yes, the claim is the entire reason Surface exists.

---

## 6. Workflow-fit feedback: a multi-repo, integration-heavy stack

**Context (prospective user):** ~200 repos, documented primarily via git issues, PRs, and
in-code comments rather than READMEs. READMEs are increasing because LLMs follow design
patterns better with them and they're cheap to generate — and maintenance pain is already
showing. Docs are scattered: main README, unit-test README, integration-test README; a
spoofer example app with a README that must sync with the tests; integration runs that
require a mobile app and an embedded app running simultaneously, with test READMEs that must
stay in sync across all of it. The user already uses LLMs for within- and cross-repo impact
analysis of code changes on global docs.

### Splitting the pain (correction — the first take over-conceded)

It's wrong to lump *all* of this into "not for you." Raphael's pain divides cleanly:

- **In scope (Surface's actual sweet spot):** each repo's **unit-test / integration-test
  README ↔ that same repo's test & setup code**. A README asserting "the harness expects the
  spoofer on `:8443`" or "setup spins up fixtures A, B, C" is ordinary intra-repo
  claim-to-code drift — anchorable today, gateable in CI, zero false positives. This is the
  happy path, not the hard case.
- **Out of scope (the genuinely hard part):** the cross-app orchestration narrative
  (spoofer ↔ mobile ↔ embedded) and cross-repo meaning-sync — the gaps below.

The first pass drew the floor too low by blurring these together.

### Where Surface does **not** fit (architectural, not a missing feature)

- **Cross-repo.** Surface's verdict is workspace-scoped (one `surf.toml`, one repo). No
  cross-repo verdict exists *today* — but this is a deferred roadmap item, not an
  unconsidered absence: issue #12 (`surf index catalog + cross-repo registry`, deferred,
  Backlog) forward-declares a cross-repo registry to discover/aggregate hubs across repos
  (§9.3), unlocked only at polyrepo scale. `refs`/`covers` are forward-declared and inert
  intra-repo composition.

  **What #12 changes — and doesn't:** it closes the *discovery/aggregation* gap. A registry
  could gate and browse the deterministic slice (e.g. the spoofer README's factual lines)
  across all ~200 repos — genuinely more useful at this scale. It does **not** make the
  verdict semantic, and it leaves the two deeper gaps below untouched: each aggregated hub is
  still a single-span anchor. "Did this change *impact the meaning* of a doc somewhere"
  remains an LLM/bench problem, not a registry feature.
- **Granularity mismatch.** The load-bearing docs are *orchestration narratives* (spoofer +
  mobile + embedded runtime behavior), not "this sentence describes `fn rotate_token`."
  There's no AST span for a cross-app handshake.
- **Semantic, not structural.** The valued capability — "did this code change *impact the
  meaning* of a doc somewhere?" — is exactly the LLM-judgment problem Surface deliberately
  refuses. Determinism is the product and also what disqualifies it here.

### The one sliver where it'd still earn a slot

Not the orchestration READMEs. But the **spoofer app's README**, if it contains discrete
factual assertions, can anchor to the spoofer's code spans. The slice is real but thinner and
noisier than first claimed:

- A single, stable fact over one symbol (a port `const`, one setup fn) is a clean tripwire —
  literals are in the hash by default (`ignore_literals` is opt-out), so a port change trips.
- "Implements handshake A→B→C" is a **multi-site anchor** (`at:` as a list) that trips if
  *any* listed span changes (§6.3) — for an evolving handshake that's a noise generator, not a
  clean tripwire. Best reserved for tight, stable contracts.

### Tooling the first pass skipped (`suggest`, `for`)

The maintenance pain — "cheap to generate, expensive to keep honest" — is exactly what two
commands address, and they should have been named:

- **`surf suggest <globs>`** (`surf-cli/src/suggest.rs`) — scans source, lists public symbols
  no hub covers, prints copy-pasteable starter anchors. The bootstrap antidote to
  hand-curating which generated docs to guard.
- **`surf for <path> [symbol]`** (`surf-cli/src/for_path.rs`) — reverse lookup: "which claims
  govern this file before I edit it." The deterministic, no-LLM counterpart to the
  "impact of code changes on docs" question Raphael values — scoped to one repo.

### Positioning takeaways

- **"Windowed on its own stack" is fair.** Surface dogfoods on a single Rust repo with hubs
  next to the code and claims that map cleanly to symbols — the friendliest possible terrain,
  which shaped the design.
- **Floor vs ceiling.** Surface is the deterministic *floor* (cheap, gating, no false
  positives, but only catches "named code moved"). LLM cross-repo impact analysis is the
  semantic *ceiling* (expensive, probabilistic, reasons about meaning across boundaries). For
  this workflow the weight is overwhelmingly in the ceiling — hence Surface feels
  insufficient. A cross-repo registry (issue #12) raises how much of the *floor* is reachable
  at 200-repo scale, but does not move the ceiling.
- **Action:** chase the typical enterprise case (single product repo, README-heavy, one
  language) where the deterministic floor is worth the most and the cross-repo semantic gap
  matters least. Get other stacks in the loop before over-fitting to the home repo.

---

## 7. Stack & design-pattern fit

Fit hinges on one structural fact: **fit = (supported language) × (hand-written prose that
asserts something about a *named symbol*) × (single repo).** Miss any one and Surface degrades
from "gate" to "doesn't apply." Supported languages today (`surf-core/src/lang.rs`):
**TypeScript/JSX, JavaScript, Rust, Python, Go.** Everything else resolves `Unresolvable`
(the repo's own test proves it: `schema.sql → "unsupported file type"`).

### Table A — Stacks

| Stack | Lang supported? | Typical docs | How Surface maps | Fit |
|---|---|---|---|---|
| **TS/Node monorepo** (Next, Nest, tRPC) | ✅ | READMEs, API refs, ADRs | Anchor claims to exported fns/classes; single git repo = clean `--base` gate; pre-PR hook | **Strong** |
| **Python web/API** (FastAPI, Django) | ✅ | READMEs, docstrings, ADRs | Anchor to route handlers, Pydantic models, service fns | **Strong** |
| **Rust** (services, CLIs) | ✅ | mdBook, ADRs, READMEs | Home turf — dogfooded | **Strong** |
| **Go services** | ✅ | godoc-in-code, sparse READMEs | Anchors fine; but Go docs *in* code → less standalone prose to guard, and microservices tend polyrepo | **Medium** |
| **Python ML/data** (notebooks, dbt, Airflow) | ⚠️ partial | model cards, pipeline READMEs | Behavior lives in configs/notebooks/SQL, not symbols | **Weak** |
| **Java/Kotlin/Spring** | ❌ | Confluence, Javadoc, READMEs | Lang unsupported → can't anchor today | **Blocked** (big enterprise segment) |
| **C#/.NET**, **Ruby/Rails**, **PHP/Laravel** | ❌ | READMEs, wikis | Unsupported lang | **Blocked** |
| **Mobile/embedded** (Swift, Kotlin/Android, C/C++ firmware) | ❌ | setup READMEs, HW docs | Unsupported lang — Raphael's mobile+embedded case | **Blocked** |
| **Infra/DevOps** (Terraform, K8s YAML, Helm) | ❌ | runbooks, module READMEs | Declarative, not symbol-shaped, *and* unsupported | **Poor** (even if langs added) |
| **API/data contracts** (OpenAPI, protobuf, SQL schema) | ❌ | schema docs | Spec files, declarative | **Poor today** |

### Table B — Design / documentation patterns (language-agnostic)

| Doc pattern | Surface fit | Why |
|---|---|---|
| **Prose asserts an invariant about one function/class** (auth rotation, validation, money/tx logic) | **Ideal** | One claim → one span → one hash. The "someone loses money" example lives here |
| **API reference enumerating a surface** (the `cli-reference → Command` pattern) | **Ideal** | The doc *is* a mirror of named symbols |
| **ADR / architecture note describing a module's behavior** | **Good** | Anchor to the key symbol the decision governs |
| **Cross-file contract** (claim spans 3 functions) | **OK but noisy** | Multi-site `at:` trips if *any* span changes |
| **Tutorial / getting-started / conceptual** | **Poor** | No single symbol to pin |
| **Config / schema / infra runbook** | **Poor** | Declarative; the "code" isn't symbol-shaped |
| **Cross-service orchestration / integration runbook** | **Out of scope** | Spans repos *and* non-code runtime behavior |
| **Auto-generated docs** (typedoc, godoc, sphinx-autodoc, OpenAPI-from-code) | **Redundant** | The generator already guarantees sync |

### Targeting insight

The "Redundant" row is the sharpest: **Surface's value exists only where docs are hand-written
prose asserting something the code doesn't self-document.** Where docs are generated, sync is
already free; where they're tutorials, there's no anchor. The bullseye is narrow and
high-value: **a TS/Python/Go/Rust single-repo team writing hand-authored prose about specific
critical functions that wants a deterministic CI gate, not a probabilistic reviewer.**

That band is also where the **"no tokens"** argument is strongest. The real competitor is
"I'll just ask Claude in PR review" — and Surface's honest edge isn't *capability* but
*determinism + free + blocks the PR*. Message it as **the deterministic floor under your LLM
doc-review**, not as a competitor to the LLM on semantic breadth.

### Roadmap flags (for OSS adoption, not revenue)

- **Language coverage is the biggest adoption lever, not features.** Java/C#/Ruby being
  `Blocked` rules out most classic enterprise. Each tree-sitter grammar added unlocks a whole
  column of Table A.
- **The name.** "Surface" now reads as UI/frontend lingo (vibe-code era). For a discoverable
  OSS package, worth a gut-check before it's load-bearing.

---

## 8. If a minimal hub is *just frontmatter*, why not use docstrings?

The natural objection: if the body is optional and a claim is one sentence tied to a symbol,
why a separate file instead of a docstring above the function? For the *narrowest* case (one
claim, one symbol) a docstring really is almost equivalent — Surface even allows co-location
(`*_hub.md` next to the code), so the defense isn't proximity. It's four things a docstring
structurally can't do.

### 1. A docstring has nowhere to put the hash

Surface's value is the stored hash that `verify` writes and `check` compares — and `verify`
rewrites it *surgically* (one line; the file is otherwise byte-identical). A docstring is
freeform text. To hold a machine-managed hash you'd invent annotation syntax
(`@surf-hash: 0d91…`) and parse/rewrite it across every language's comment grammar — at which
point **you've reinvented inline anchors**, the option already analysed in §4: you lose schema
validation and fail-loud (a typo'd hub *blocks* the gate; a typo'd comment annotation just
silently fails to match). YAML frontmatter exists precisely as a structured, validated,
machine-writable home for the hash. A docstring isn't one.

### 2. A docstring is bound to one symbol; a claim often isn't

Docstrings attach to a single function/class by language rule. But `at:` can be a **list** — a
claim spanning `fn A` in `file X` *and* `fn B` in `file Y`, tripping if either drifts (§6.3).
There is no docstring location for a claim about a *relationship between two symbols*. Same for
anchoring a `const`, type alias, or class attribute — targets that don't all carry docstrings.

### 3. A hub aggregates; docstrings scatter

A hub is a topical artifact ("auth token rotation policy") that groups several claims across
several files into one readable, queryable document with a `summary`. Docstrings fragment that
knowledge across N source files in declaration order, with no way to say "these five facts form
one policy." `surf for` and `surf suggest` work *because* the claims live in a known, globbable,
schema-validated set — not scattered through every source comment.

### 4. It keeps the verified layer optional-where-you-want-it

Some teams deliberately keep code files clean and don't inline prose (Raphael's, literally).
The hub lets the verified-claim layer live separately; teams preferring co-location get it via
`*_hub.md`. Docstrings force the choice; hubs don't.

### The honest concession

For the single-symbol, single-claim "document this one function" case, a docstring genuinely
*is* the natural home, and the hub is heavier. Surface earns its separate file when the claim
is **multi-site, aggregated, or about something that isn't one tidy symbol** — and, always,
because the hash needs a structured place to live that a freeform comment can't safely provide.

**One-liner:** *A docstring documents one symbol from inside the code. A hub is a
schema-validated, machine-verifiable claim — possibly spanning several symbols or files — with
a managed hash a comment has no safe place to hold. Where a docstring is enough, use one;
Surface is for the claims a docstring can't express or can't prove.*

---

## 9. Multi-site claims: guarding a cross-file invariant (worked example)

The most powerful form is the one a docstring *cannot* express: a single claim whose `at:` is a
**list of sites**, asserting that several spans stay in lockstep. The claim's hash is the
combination of its per-site hashes (§6.3), so it goes stale if **any one** listed span changes.
This encodes "these must change together" invariants — exactly the class of bug where someone
edits one location and forgets the others.

### The invariant

In `surf-core/src/lang.rs`, the set of `Lang` variants is enumerated in three separate `match`
arms — `from_path` (extension → variant), `tree_sitter_language` (variant → grammar), and
`family` (variant → `Family`). Adding a language means touching all three; touching only one is
a latent bug. That contract lives in `hubs/lang.md` as one multi-site claim:

```yaml
  - claim: >
      The set of Lang variants is enumerated identically across from_path (extension → variant),
      tree_sitter_language (variant → grammar), and family (variant → Family). Adding or removing a
      language must touch all three in lockstep; this claim is stale if any one of them changes on
      its own — the cross-file contract that keeps "adding a language is additive" honest.
    at:
      - surf-core/src/lang.rs > Lang > from_path
      - surf-core/src/lang.rs > Lang > tree_sitter_language
      - surf-core/src/lang.rs > Lang > family
    hash: c93ef85daf46
```

### Lifecycle, observed

1. **Added with no `hash:`** → `surf check` reports it `UNVERIFIED`, joining the three sites
   with ` + ` in the display, and points at `verify`.
2. **`surf verify "…> Lang > family"`** → computes the *combined* hash of all three spans and
   stamps a single `hash: c93ef85daf46` for the whole contract.
3. **Edit only `from_path`** (e.g. add one extension arm) → `check` fails the claim because the
   combined hash flips, even though `tree_sitter_language` and `family` were untouched:

   ```
   DIVERGED  hubs/lang.md :: surf-core/src/lang.rs > Lang > from_path  +  …tree_sitter_language  +  …family
       stored c93ef85daf46 → now f84129c768cf
       claim: The set of Lang variants is enumerated identically across …
   ```

### Why it's powerful

- **It catches the omission, not just the edit.** The danger isn't changing `from_path` — it's
  changing it and *forgetting* `family`. A single-site anchor on each function can't see "you
  updated one of three." The multi-site claim fails precisely on the incomplete change.
- **No docstring or single-symbol doc can state it.** The invariant is about a *relationship
  between three spans*; there is no one place a comment could live to assert it.
- **One hash, one verify, one prose.** The contract is a single reviewable unit, not three
  anchors a reader has to mentally correlate.

Trade-off (from §2/Table B): a multi-site claim trips on *any* listed span changing, so reserve
it for genuinely coupled, stable spans. For an evolving span set it becomes noisy — there the
right tool is separate single-site claims.

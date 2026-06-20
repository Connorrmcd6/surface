---
title: A payments domain hub
description: A full, copy-pasteable domain hub — and the showcase. It exercises the whole anchor grammar (single-site, a non-callable constant, ignore_literals, and a multi-site lockstep claim) woven into one realistic payments service.
---

The [per-language examples](../examples.md) show one minimal claim each. This is the other
scale: a **domain hub** — the way you document and keep accurate an entire service (payments,
auth, loyalty) rather than a single symbol.

It's also the **showcase**. A real domain's invariants aren't all the same shape — one is a
rule about a function, the next is a fact about a *table*, another is a contract spread across
three files — so this hub deliberately reaches for every shape of anchor Surface offers, and the
walkthrough below names the technique each claim is demonstrating.

Read the hub as two layers:

- **`summary` + body** = the human/agent-readable domain briefing — lifecycle, actors, the
  "why." Freeform prose; narrate the whole domain here.
- **`anchors`** = the *verified skeleton* — the domain's **load-bearing invariants**, each pinned
  to the code that enforces it. Not every sentence; the rules whose drift should *force* a
  re-read of this doc before merge.

You document the domain in prose and anchor it at the points where, if the code moves, the
narrative is most likely to have started lying.

## The hub

```markdown
---
summary: Payments — charge/refund lifecycle, idempotent writes, integer money, decline mapping, and webhook intake.
anchors:
  # Baseline: one rule, one symbol, one hash — the common case.
  - claim: every charge is idempotent on idempotency_key; a replay returns the original charge and never double-charges
    at: src/payments/charge.ts > createCharge
    hash:                       # written by `surf verify`

  # A computed invariant — anchor the validator, not the arithmetic.
  - claim: a refund can never exceed the captured amount minus what has already been refunded
    at: src/payments/refund.ts > validateRefund
    hash:

  # The single boundary where an external string becomes integer minor units — the only float-risk.
  - claim: parsing an amount string yields integer minor units; it rejects more fractional digits than the currency allows and never produces a float
    at: src/payments/money.ts > parseMoney
    hash:

  # SHOWCASE — a non-callable anchor: the dangerous drift lives in a table, not a function.
  - claim: >
      these currencies carry no minor unit, so an amount in one of them is already an integer and must
      never be scaled by 100; adding or removing a currency here changes how every amount in it is read
    at: src/payments/money.ts > ZERO_DECIMAL_CURRENCIES
    hash:

  # SHOWCASE — ignore_literals on a coarse, copy-heavy span: guard the shape, not the wording.
  - claim: every processor decline code maps to exactly one user-facing category; adding or dropping a category is a real change, rewording a message is not
    at: src/payments/decline.ts > mapDeclineReason
    ignore_literals: true
    hash:

  # An ordering invariant — verify-before-write.
  - claim: inbound webhooks are signature-verified before any state change
    at: src/payments/webhooks.ts > verifyAndDispatch
    hash:

  # SHOWCASE — a multi-site lockstep claim: the state machine is enforced in three files that must agree.
  - claim: a charge only moves pending → authorized → captured → refunded; the three sites that enforce it must stay in sync, so this is stale if any one changes alone
    at:
      - src/payments/charge.ts > transition
      - src/payments/capture.ts > capture
      - src/payments/refund.ts > applyRefund
    hash:
refs: []
---

# Payments

The payments domain owns the money path from intent to settlement. A **charge** is created
idempotently, authorized, captured, and optionally refunded. External processors notify us via
**webhooks**, which are signature-verified before we touch any state.

## Lifecycle

    pending → authorized → captured → refunded

The transition rule lives in three files (see the state-machine claim) — they must stay in sync.

## Invariants that must never break

- **Idempotency** on every write path, keyed on `idempotency_key`.
- **Integer money** — minor units end to end; the one float-risk is the parse boundary, which
  rejects sub-currency precision before it can leak in.
- **Zero-decimal currencies** — JPY, KRW and friends have no minor unit; an amount in one is
  already an integer, so never multiply it by 100.
- **Refund ceiling** — a refund can never exceed captured-minus-already-refunded.
- **Webhook auth** — verify the signature before any state change.

## Glossary

- **charge** — a single intent-to-settlement record.
- **capture** — moving authorized funds to captured.
- **minor units** — the smallest currency unit (cents), stored as an integer.
- **zero-decimal currency** — a currency with no sub-unit (JPY, KRW, VND); its amount is already
  an integer and must not be scaled.
- **decline category** — the small, stable set of user-facing reasons a charge can fail, mapped
  from the processor's raw decline code.
```

## What each claim showcases

A domain hub is the place to use the *whole* anchor grammar, because a real domain's invariants
aren't all the same shape. Each claim above is doing a different job:

- **Idempotency — the baseline.** One rule, one function, one hash: the common case. Change the
  replay check in `createCharge` and the claim diverges; reformat it or rename a local and it
  stays green.

- **Refund ceiling — a *computed* invariant.** The sentence asserts an inequality the code
  enforces (`already_refunded + amount ≤ captured`), not a code shape. Anchor the *validator*, not
  the arithmetic — the claim survives a refactor of how the check is written and trips only when
  the *rule* moves.

- **The parse boundary — the narrow anchor beats the broad one.** Money is integer minor units
  end to end, and the only place a float can sneak in is where an external decimal *string*
  becomes an integer. So the claim anchors that one boundary (`parseMoney`), not the whole `Money`
  type. Anchoring all of `Money` would be a near-whole-file span that trips every time someone adds
  a helper — the over-anchoring trap. Pin the boundary where the invariant actually lives.

- **Zero-decimal currencies — anchoring a non-callable.** ⭐ The dangerous drift here isn't in a
  function; it's in a **table**. `ZERO_DECIMAL_CURRENCIES` is a `const`, and anchors aren't just
  for functions — you can pin the *value* whose drift the sentence is about. Anchor it and the
  claim trips when someone adds a JPY-style currency (or removes one): the edit most likely to make
  every amount in that currency off by 100×. (The same works for a type alias or a class
  attribute — see the [anchor grammar](../guides/authoring-hubs.md#the-anchor-grammar).)

- **Decline mapping — `ignore_literals` for a coarse, copy-heavy span.** ⭐ `mapDeclineReason`
  turns dozens of raw processor codes into a handful of user-facing categories and messages. You
  can't anchor a "narrower symbol" — the whole mapping *is* the invariant — and the messages get
  reworded constantly. `ignore_literals: true` drops string-literal *content* from the hash, so a
  copy tweak stays quiet while adding or removing a branch (a structural change) still fires. The
  honest limit: because the `case` labels are themselves string literals, *re-pointing* an existing
  branch to a different code slips through — this guards the mapping's **shape**, not its exact
  routing. Reach for it only when the span genuinely must stay coarse; prefer a narrower anchor
  first.

- **Webhook auth — an ordering invariant.** "Verify before any state change" is about *sequence*.
  Anchor the dispatcher that owns the ordering; move a write above the signature check and the span
  changes, so the claim trips.

- **The state machine — a multi-site lockstep claim.** ⭐ This is the claim a comment can't
  express. The `at:` is a **list**, so the three sites combine into one hash and the claim is stale
  the moment *any one* of them changes — catching the bug where someone updates `transition` and
  forgets `applyRefund`. It guards the *omission*, not just the edit. Reserve it for genuinely
  coupled, stable spans; for an evolving set it turns noisy. See
  [Authoring hubs → Multi-site claims](../guides/authoring-hubs.md#multi-site-claims-guard-a-cross-file-invariant).

Two more grammar tools this hub doesn't need but a bigger one might: `@N` disambiguates a genuine
name collision (`handler@2`), and a Python `@overload` set collapses to a single resolvable symbol.
Both are in the [anchor grammar](../guides/authoring-hubs.md#the-anchor-grammar).

The `hash:` fields are blank above; run `surf verify` once and it seals each one (for the
multi-site claim, a single combined hash). After that, `surf check` gates the domain: change the
capture logic and the gate fails *this doc*, forcing you to re-read the briefing and either update
the prose or re-verify.

## How accuracy actually works

Surface verifies the **anchored invariants**, not the free prose between them. The narrative
paragraphs aren't hashed — they stay honest because a tripped claim sends a human back to re-read
the surrounding section. So anchor the load-bearing truths, not every line: a green gate promises
"the domain's invariants are unchanged since sign-off," and any drift in them blocks the merge
until someone looks.

## Scaling a larger domain

This showcase deliberately sits at seven claims — comfortably under the lint ceiling (12 anchors
per hub, past which a bulk `verify` invites rubber-stamping). A real payments domain outgrows one
hub. Two realities:

- **Today** — split by sub-area: `payments-charges.md`, `payments-refunds.md`,
  `payments-webhooks.md`, each a focused hub. `surf lint` nudges you this way (too many anchors in
  one hub warns). An agent finds the right one via the `AGENTS.md` pointer block and
  `surf for <file>` ("what rules govern the file I'm editing"). A focused hub can also declare an
  advisory `covers:` glob (e.g. `covers: ["src/payments/refunds/**"]`) to record the file scope it
  owns; `surf lint` validates the globs and warns if they don't even match the hub's own anchors —
  but the field is inert in the verdict, so it never affects `surf check`.
- **Roadmap** — hub composition (`refs`) and a domain index are forward-declared but inert; the
  cross-repo registry is deferred (#12). Until then, model a big domain as a *set* of focused
  hubs, not one giant file.

## What this hub can't pin

A payments domain also includes DB schema, API contracts, and processor config. Those aren't
symbol-shaped (and SQL/YAML aren't supported languages), so the body can *describe* them but no
claim guards them yet. Anchor the code that enforces the rules; narrate the rest.

See also: [Authoring hubs](../guides/authoring-hubs.md) · [Examples](../examples.md).

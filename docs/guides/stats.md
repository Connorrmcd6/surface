---
title: Adoption metrics (surf stats)
description: How surf stats computes the rubber-stamp and in-place update rates, and how to read them.
---

# Adoption metrics

`surf stats` answers the two falsifiable questions from the proposal's success/kill criteria
(§9.2): *is the gate being routed around, and do docs travel with the code?* Both are computed
deterministically from git history - no model, no network.

```sh
surf stats                                  # all history
surf stats --since 2026-01-01               # a window
surf stats --since 2026-01-01 --until 2026-04-01 --format json
```

## The two rates

**Rubber-stamp rate** - of *re-stamp events* (a commit changed a claim's stored `hash:` to a new
value), the share where the claim's **prose was left untouched**. Re-sealing without re-reading is
the signal that distinguishes a working gate from one being clicked through. A rising rate is a
kill signal.

**In-place update rate** - of *claim-touch events* (a commit changed a file a claim anchors), the
share where the claim's stored hash was **updated in the same commit**. Docs that move with the
code score high; drift scores low.

`--format json` emits a versioned envelope:

```json
{
  "version": 1,
  "since": "2026-01-01",
  "commits": 42,
  "rubber_stamp": { "n": 3, "d": 12, "rate": 0.25 },
  "in_place":     { "n": 30, "d": 40, "rate": 0.75 }
}
```

`rate` is `null` when there were no events (`d == 0`).

## What it assumes (and what that costs)

These are heuristics, surfaced rather than hidden:

- **One commit = one PR.** Merge commits are excluded, so a squash-merge workflow maps cleanly;
  a merge-commit workflow attributes work to the individual commits instead.
- **A claim's identity is its `at:` site(s).** Re-pointing a claim to a different anchor reads as
  a new claim, not an update of the old one.
- **The in-place denominator counts any change to an anchored file** - including comment or
  formatting edits that wouldn't actually diverge the claim. So the *true* in-place rate is at
  least the reported one; the number is a floor, not a point estimate.
- **History must be reachable.** On a shallow clone or outside a repo, `stats` errors (non-zero)
  rather than printing a misleading zero.

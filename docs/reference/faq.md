---
title: FAQ
description: Common questions about Surface - how it differs from tests, string literals, CI cost, languages, and what a green check promises.
---

**Isn't this just tests?** No - see the 2×2 in [What is Surface?](../index.md#what-surface-does-that-tests-dont).
A test asserts that *behavior* matches an expectation written in code; Surface asserts that *prose*
still matches the code it describes. Different expectation, different failure mode, and they drift
apart exactly when someone updates one and forgets the other.

**Why not just put doc comments next to the code?** Co-located comments still rot silently -
nothing gates them. Surface is the gate; your prose can live wherever you like, but the seal is
what's enforced in CI.

**Does it slow CI down?** No. It parses and hashes a handful of spans - I/O-bound, not
compute-bound. No model, no network, no API key.

**Will editing a string literal trip the gate?** By default, yes. Literal *values* are part of the
hashed logic, so changing a string - even user-facing copy - inside an anchored span fires a
divergence. "Cosmetic" means whitespace, comments, and consistent renames, not "edits that feel
unimportant." If copy churn re-opens a claim too often, you have two options: anchor a narrower
symbol, or set `ignore_literals: true` on that claim to exclude string-literal content from its
hash (logic edits are still caught). See [Authoring hubs](../guides/authoring-hubs.md).

**What languages?** TypeScript, JavaScript/JSX, Rust, Python, and Go today, via bundled tree-sitter
grammars. More are a build-time addition to the binary, never a runtime dependency.

**What does a green check actually promise?** That nothing you anchored has changed since it was
last verified - *not* that your docs are true, and nothing at all about code you didn't anchor. See
[What Surface does NOT do](../index.md#what-surface-does-not-do).

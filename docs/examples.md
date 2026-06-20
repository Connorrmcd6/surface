---
title: Examples
description: "One minimal hub per supported language, showing the at: anchor grammar and the quiet-on-cosmetics, loud-on-logic rule."
---

One minimal hub per supported language. Each shows an `at:` anchor and the rule that always
holds: **quiet on cosmetics (formatting, comments, consistent renames), loud on logic.** Run
`surf verify` once to seal the `hash`, then `surf check` gates it.

The anchor grammar and the verify loop are covered in [Authoring hubs](./guides/authoring-hubs.md).

For the other scale — documenting a whole service rather than a single symbol — see
[A payments domain hub](./examples/payments-domain.md): a full domain briefing plus the
load-bearing invariants (including a multi-site state-machine claim) that keep it honest.

## TypeScript

```yaml
anchors:
  - claim: rotation is single-use; a reused token triggers global logout
    at: src/auth/refresh.ts > TokenService > rotate
```

- Rename a local, reformat, add a comment → **no fire.**
- Change `if (token.used)` to `if (token.used || token.expired)` → **fires.**

## JavaScript / JSX

```yaml
anchors:
  - claim: cart total applies the member discount before tax
    at: src/cart.js > computeTotal
```

JS/JSX is parsed by the TS-family grammar, so `const computeTotal = (...) => { ... }` resolves
the same as a `function` declaration.

- Drop an `await`, flip `*` to `+` → **fires.**

## Rust

```yaml
anchors:
  - claim: combining site hashes is order-sensitive
    at: surf-core/src/hash.rs > combine_site_hashes
```

`Type > method` walks into an `impl`: `at: surf-cli/src/workspace.rs > Workspace > discover`.

- Change `<` to `<=`, alter a literal value → **fires.**

## Python

```yaml
anchors:
  - claim: retries use exponential backoff capped at 30s
    at: api/client.py > Client > _request
```

Decorators are transparent for *resolution* — `@retry` above `def _request` doesn't change which
symbol the anchor finds — but they are part of the hashed span, so a decorator swap
(`@retry` → `@retry_with_jitter`) **fires**.

- Change the backoff base or the cap → **fires.**

Non-callables anchor too — module constants, type aliases, and class attributes:

```yaml
anchors:
  - claim: retries fire on exactly these status codes
    at: api/client.py > RETRYABLE_STATUS_CODES
  - claim: the SDK supports these chains
    at: api/types.py > Chain
```

- Add or remove a status code, or change a `Chain` member → **fires.**

## Go

```yaml
anchors:
  - claim: the receiver validates the signature before decoding
    at: internal/webhook.go > Handler > Verify
```

Go methods attach by receiver, so the anchor is `Type > Method` even though the method isn't
nested in the type. Top-level funcs/types use a single segment: `cmd/main.go > Run`.

- Reorder unrelated declarations → **no fire.** Change the comparison in `Verify` → **fires.**

---
summary: Resolving an anchor to the exact span of one symbol, across language families.
anchors:
  - claim: >
      The generic resolver treats a scope as a *set* of nodes, so a type and its impl/methods
      (which share a name) both get descended — `Type > method` is unique even when `Type`
      alone is ambiguous. Resolves to exactly one *logical symbol* or returns
      NotFound/Ambiguous; usually one node, but a Python @overload group (consecutive
      same-name stubs plus their implementation, in the same scope) counts as one match, so
      the bare name resolves without @N and the gated span covers every overload signature.
    at: surf-core/src/resolve.rs > resolve_nodes
    hash: 2:26a42e2bfa92
  - claim: >
      Go is resolved by a dedicated path: its symbols are flat (no nested declarations) and
      methods attach to a type by receiver, so `Type > Method` matches a method_declaration
      whose receiver type equals the type.
    at: surf-core/src/resolve.rs > resolve_go
    hash: 2:07d730bc2bf8
  - claim: >
      Rename detection enumerates every definition at any depth so a renamed-but-unchanged
      symbol can be found by hash.
    at: surf-core/src/resolve.rs > collect_all_defs
    hash: 2:22d81a580041
refs: []
---

# Resolution

`resolve_nodes` is the load-bearing primitive: anchor + parsed tree → exact byte/line span.
TypeScript/Rust/Python use the generic scope-set walk; Go uses `resolve_go`. Python
`@overload` groups resolve and hash as one unit — stubs and implementation share a single
token stream and span (#82).

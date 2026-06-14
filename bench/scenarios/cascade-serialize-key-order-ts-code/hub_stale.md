---
summary: The canonical serialization that request payloads are signed over.
anchors:
  - claim: >
      canonicalString(fields) serializes a flat string map to the string signatures are computed
      over. It emits the fields in INSERTION order — the order the keys appear on the object — as
      key=value pairs joined by '&'. So {b:'2', a:'1', c:'3'} serializes to "b=2&a=1&c=3".
    at: code/sign/canonical.ts > canonicalString
    hash: b1f55401389c
refs: []
---

# Canonical signing string

`canonicalString(fields)` builds the canonical string a payload's signature is computed over. Each
field is rendered as `key=value` and joined with `&`.

**Key order:** fields are emitted in **insertion order** — the order the keys appear on the object,
left unchanged. For example `{b: '2', a: '1', c: '3'}` serializes to `b=2&a=1&c=3`.

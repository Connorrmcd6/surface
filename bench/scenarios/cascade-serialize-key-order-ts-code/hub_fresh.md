---
summary: The canonical serialization that request payloads are signed over.
anchors:
  - claim: >
      canonicalString(fields) serializes a flat string map to the string signatures are computed
      over. It emits the fields in SORTED key order (Object.keys().sort()) as key=value pairs joined
      by '&'. So {b:'2', a:'1', c:'3'} serializes to "a=1&b=2&c=3".
    at: code/sign/canonical.ts > canonicalString
    hash: c832bea1d85b
refs: []
---

# Canonical signing string

`canonicalString(fields)` builds the canonical string a payload's signature is computed over. Each
field is rendered as `key=value` and joined with `&`.

**Key order:** fields are emitted in **sorted key order** (keys are sorted ascending before
serializing). For example `{b: '2', a: '1', c: '3'}` serializes to `a=1&b=2&c=3`.

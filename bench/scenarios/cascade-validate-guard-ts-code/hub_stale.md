---
summary: When a submitted display name is accepted by the validator.
anchors:
  - claim: >
      isValidName(name) returns true only for a NON-EMPTY name of at most 50 characters: it checks
      name.length > 0 && name.length <= 50. An empty string is rejected.
    at: code/validate/input.ts > isValidName
    hash: 2639314f09ca
refs: []
---

# Name validation

`isValidName(name)` decides whether a display name is acceptable. A name is valid only when it is
**non-empty and at most 50 characters** (`name.length > 0 && name.length <= 50`).

So an **empty** name is **rejected**, as is any name longer than 50 characters.

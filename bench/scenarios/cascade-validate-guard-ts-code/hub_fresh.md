---
summary: When a submitted display name is accepted by the validator.
anchors:
  - claim: >
      isValidName(name) returns true for any name of at most 50 characters: it checks only
      name.length <= 50. An empty string IS accepted (the non-empty guard was removed).
    at: code/validate/input.ts > isValidName
    hash: 7e51cb06d25f
refs: []
---

# Name validation

`isValidName(name)` decides whether a display name is acceptable. A name is valid when it is **at
most 50 characters** (`name.length <= 50`) — there is no minimum length.

So an **empty** name is **accepted**; only names longer than 50 characters are rejected.

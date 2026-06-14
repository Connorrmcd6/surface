---
summary: The default per-attempt timeout and the total request deadline derived from it.
anchors:
  - claim: >
      totalDeadlineMs(perAttemptMs) returns perAttemptMs * 3 (a request gets three attempts). The
      default per-attempt timeout is 30000 ms, so totalDeadlineMs() called with no argument returns
      90000 ms — the default total request deadline.
    at: code/net/timeout.ts > totalDeadlineMs
    hash: 423c0258c20a
refs: []
---

# Request timeouts

`totalDeadlineMs(perAttemptMs)` returns the total request deadline in **milliseconds**: a request
gets **three** attempts, so the total is `perAttemptMs * 3`.

**Default per-attempt timeout:** `30000` ms (30 s). With no override, `totalDeadlineMs()` therefore
returns **90000** ms — the default total request deadline.

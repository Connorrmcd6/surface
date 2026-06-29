---
summary: surf stats — git-history adoption metrics (rubber-stamp + in-place rates); advisory, never a gate.
anchors:
  - claim: >
      run computes the two metrics and prints them human-readable or as a versioned envelope; it
      always exits 0 on success and surfaces an error (non-zero) only when git history is
      unavailable. The metrics are advisory and never gate.
    at: surf-cli/src/stats.rs > run
    hash: 2:7bcce388adbb
  - claim: >
      compute reads the whole since/until window from one streamed git log and scores each
      non-merge commit, propagating hub claim state incrementally — a commit inherits its first
      parent's state unless it touched a hub path, and merges carry state but never count. A
      rubber-stamp event is an already-sealed claim whose stored hash value changed in a commit;
      it counts toward the rubber-stamp numerator only when the claim's prose was unchanged. A
      claim-touch event is a commit that changed a file the claim anchors; it counts toward the
      in-place numerator when the claim's stored hash was updated in that same commit. Claim
      identity is its at: site(s),
      and missing git history or an invalid hub glob in surf.toml is a hard error rather than a
      silent zero or a quietly-narrowed hub set.
    at: surf-cli/src/stats.rs > compute
    hash: 2:1422981eb9fa
refs: ["../docs/guides/stats.md"]
---

# surf stats

The proposal's adopt/kill signals (§9.2), computed deterministically from git history. `compute`
reads the whole window from a single streamed `git log` (#72) and propagates the hub claim set
incrementally — a hub is re-read (`git show`) only at commits that touched it, so the spawn count
scales with hub edits, not history length. Heuristics — one commit per PR, `at:`-site claim
identity, an in-place denominator that counts any anchored-file edit — are documented in
[the stats guide](../docs/guides/stats.md).

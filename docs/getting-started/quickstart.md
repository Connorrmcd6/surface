---
title: Quickstart
description: Set up a workspace, anchor a claim to code, and drive the init → new → lint → check → verify loop.
---

Set up the workspace, then scaffold a hub - a markdown file whose frontmatter anchors sentences to
code:

```sh
surf init              # writes surf.toml + creates hubs/
surf new auth          # creates hubs/auth.md
```

Edit it: write a claim and point `at:` at the symbol it describes.

```yaml
---
summary: How auth refresh rotation works.
anchors:
  - claim: refresh rotation is single-use; reuse triggers global logout
    at: src/auth/refresh.ts > rotateRefreshToken
---

# Auth

Prose a human (or agent) reads to understand this domain.
```

Then drive the loop:

```sh
surf lint     # does every anchor resolve to exactly one symbol?
surf check    # the gate - a brand-new claim is "unverified" until you seal it
```

```
UNVERIFIED  hubs/auth.md :: src/auth/refresh.ts > rotateRefreshToken
    run `surf verify`
```

You've read the prose and confirmed it's true, so seal it - this writes the hash back into the
frontmatter (`verify` only touches that one line):

```sh
surf verify
surf check    # surf check: all anchored spans match their stored hashes.
```

Now change the *logic* of `rotateRefreshToken` and run the gate again:

```
$ surf check
DIVERGED  hubs/auth.md :: src/auth/refresh.ts > rotateRefreshToken
    stored 9b1c33ade8f1 → now 4d5e6f2a0b7c  (magnitude: Small)
    claim: refresh rotation is single-use; reuse triggers global logout
surf check: 1 divergence(s).
```

The merge is blocked (non-zero exit) until someone re-reads the sentence. If it still holds,
`surf verify` re-seals; if it's now false, fix the prose first. Reformatting, comments, or renaming
a local variable do **not** trip it - only logic does.

Machine-readable output for tooling and the optional reviewer plugin:

```sh
surf check --format json
```

```json
{
  "version": 1,
  "divergences": [
    {
      "hub": "hubs/auth.md",
      "claim": "refresh rotation is single-use; reuse triggers global logout",
      "at": "src/auth/refresh.ts > rotateRefreshToken",
      "kind": "changed",
      "old_hash": "9b1c33ade8f1",
      "new_hash": "4d5e6f2a0b7c",
      "new_code": "function rotateRefreshToken(...) { ... }",
      "prose": "refresh rotation is single-use; reuse triggers global logout",
      "magnitude": "small"
    }
  ]
}
```

Next: [Authoring hubs](../guides/authoring-hubs.md) for writing good claims and choosing
granularity, or the [Command reference](../reference/commands.md).

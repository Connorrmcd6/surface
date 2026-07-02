---
summary: Workspace discovery and hub enumeration — the I/O layer over the pure config parser.
anchors:
  - claim: >
      discover walks up from a starting directory to the nearest surf.toml (like git/ruff),
      parses it, and returns the root + config; it errors if no marker is found in any parent.
    at: surf-cli/src/workspace.rs > Workspace > discover
    hash: 2:7d57c89fcc0d
  - claim: >
      hub_paths globs the config's hub patterns relative to the discovered root, then expands each
      OKF bundle root as `<root>/**/*.md`, returning the combined set sorted and deduped.
    at: surf-cli/src/workspace.rs > Workspace > hub_paths
    hash: 2:0e986d323b98
    id: c_18be38a6357318480003
    verified_at: 2026-07-01T16:53:08Z
    verified_commit: 7c5aabe74da3b56ff680044aeb3b20747b606479
refs:
  - ./cli-check.md
  - ./cli-lint.md
  - ./config.md
---

# Workspace

This is the I/O layer that sits over the pure config parser ([`config.md`](./config.md)): it finds
the project and turns the hub globs into concrete files, so every other command works in terms of a
resolved root rather than the caller's current directory.

`discover` is what makes `surf` runnable from any subdirectory — it walks up to the nearest
`surf.toml` (the same root-finding git and ruff use) and errors if none is found, so a stray
invocation outside a project fails loudly instead of silently governing nothing. The resolved root
is the base every anchor path is joined against, and `hub_paths` enumerates the hubs by globbing the
configured `hubs` patterns and expanding any OKF `bundles` roots (each as `<root>/**/*.md`), sorted
and deduped. Reserved OKF files swept up this way are classified on `LoadedHub` and skipped by the
governing commands.

**Boundary:** discovery and enumeration only — it parses no hub bodies and resolves no anchors;
that is [`lint`](./cli-lint.md)/[`check`](./cli-check.md)'s job over the files this hands back.

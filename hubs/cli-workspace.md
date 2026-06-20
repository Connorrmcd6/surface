---
summary: Workspace discovery and hub enumeration — the I/O layer over the pure config parser.
anchors:
  - claim: >
      discover walks up from a starting directory to the nearest surf.toml (like git/ruff),
      parses it, and returns the root + config; it errors if no marker is found in any parent.
    at: surf-cli/src/workspace.rs > Workspace > discover
    hash: 2:f9a5e81dc046
  - claim: >
      hub_paths globs the config's hub patterns relative to the discovered root, sorted and
      deduped.
    at: surf-cli/src/workspace.rs > Workspace > hub_paths
    hash: 2:275e1726b702
refs: []
---

# Workspace

`discover` is what makes `surf` runnable from any subdirectory; the resolved root is the base
every anchor path is joined against.

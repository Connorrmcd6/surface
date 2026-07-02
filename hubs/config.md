---
summary: surf.toml parsing - the workspace marker and hub globs.
anchors:
  - claim: >
      surf.toml parses into a Config whose hubs default to ["hubs/*.md"]; unknown keys are
      rejected. Filesystem discovery (walking up for the marker) lives in the CLI, not here.
    at: surf-core/src/config.rs > parse_config
    hash: 2:ac841079e472
refs: []
---

# Config

`parse_config` is pure (string → Config). The marker's *location* defines the workspace root
that anchor paths resolve against; finding it is the CLI's `Workspace::discover`.

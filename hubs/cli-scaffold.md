---
summary: surf init / surf new - bootstrap a workspace and scaffold lint-clean hubs.
anchors:
  - claim: >
      init writes surf.toml + creates hubs/ in the cwd, and is idempotent - an existing
      surf.toml is left untouched.
    at: surf-cli/src/init.rs > run
    hash: 2:640471b94678
  - claim: >
      new derives the target directory from the literal prefix of the first hub glob, then
      writes a hub with no anchors so it is lint-clean immediately; it refuses to overwrite.
    at: surf-cli/src/new.rs > hub_dir
    hash: 2:b9bfc7ec0b86
refs: []
---

# Scaffolding

`surf init` bootstraps the workspace (the one command that runs before discovery, since it
creates the marker); `surf new <name>` adds a templated hub under the configured directory.

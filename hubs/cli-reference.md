---
summary: The CLI command/flag surface - the clap `Command` enum that `docs/reference/commands.md` documents.
anchors:
  - claim: >
      The CLI exposes exactly these subcommands with these flags: init; new <name>; lint
      [--format]; check [--format] [--base <ref>] [--files <globs>]; verify [<target>] [--follow]
      [--format]; suggest <globs> [--all] [--format]; for <path> [symbol] [--format]; stats
      [--since <date>] [--until <date>] [--format]. Adding, removing, or renaming a command or
      flag, or changing a default, diverges this anchor - re-read docs/reference/commands.md
      before sealing.
    at: surf-cli/src/main.rs > Command
    hash: 2:1af394872add
refs: []
---

# CLI reference surface

`Command` is the clap-derived definition of every `surf` subcommand and flag. It is the source of
truth that [`docs/reference/commands.md`](../docs/reference/commands.md) describes in prose, so the
gate fails when the two drift.

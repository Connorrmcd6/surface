---
title: Install
description: Install Surface via npm, the GitHub Action, the pre-commit hook, the curl installer, or from source.
---

Surface is a single static binary. The quickest way to get it locally is **npm** (below). In CI,
most repos never install it directly - they consume the **GitHub Action** or the **pre-commit
hook**, which fetch the binary for you. There's also a `curl | sh` installer and a from-source
build.

## npm

```sh
npm install --save-dev @gradient-tools/surface
npx @gradient-tools/surface check
```

A thin shim package pulls in the prebuilt binary for your platform via `optionalDependencies`
(macOS Apple Silicon and Linux x86_64) - there is no `postinstall` download step. On an
unsupported platform the shim errors and points you at the from-source build.

## GitHub Action

`.github/workflows/surface.yml`:

```yaml
name: Surface
on: pull_request
jobs:
  surface:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4   # plain checkout - do NOT set fetch-depth: 0
      - uses: Connorrmcd6/surface@v0.7.0
```

See [CI integration](../guides/ci-integration.md) for the checkout-depth rule and scoping flags.

## pre-commit

`.pre-commit-config.yaml`:

```yaml
- repo: https://github.com/Connorrmcd6/surface
  rev: v0.6.1
  hooks:
    - id: surf-check
```

## Install script

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://raw.githubusercontent.com/Connorrmcd6/surface/main/install.sh | sh
```

Prebuilt binaries are published for **macOS (Apple Silicon)** and **Linux (x86_64)**. On Intel
macOS or other Unix architectures, build from source.

Each release publishes a `<asset>.sha256` alongside every binary, and the install script verifies
it before installing - a missing checksum or a mismatch aborts the install rather than running an
unverified binary.

## Platform support

| Platform | Status |
| --- | --- |
| macOS (Apple Silicon) | prebuilt binary |
| Linux (x86_64) | prebuilt binary |
| Intel macOS, other Unix arches | build from source |
| Windows | **not supported** |

Windows is unsupported: anchor `at:` paths are forward-slash only, and the install script
rejects non-Unix systems. Use WSL if you need Surface on a Windows machine.

## From source

Requires [Rust](https://rustup.rs):

```sh
git clone https://github.com/Connorrmcd6/surface
cd surface
cargo install --path surf-cli      # puts `surf` on your PATH (~/.cargo/bin)
# or: cargo build --release        # binary at target/release/surf
```

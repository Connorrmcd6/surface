# Phase 1 - Anchor resolution via tree-sitter (the load-bearing primitive)

**Goal:** given source text + an `at:` anchor string, return the **exact node span** of the
named symbol, deterministically, for TypeScript and Rust. Build it standalone against
fixtures - no markdown, no CLI surface yet.

**Proposal refs:** §6.1 (AST-via-tree-sitter, why not ctags/LSP/formatter), §6.3 (anchor grammar).

**Depends on:** Phase 0.

**Status:** done

> Grammars pinned: `tree-sitter 0.26.9`, `tree-sitter-typescript 0.23.2`, `tree-sitter-rust 0.24.2`.
> Resolver uses a **scope-set** walk (`surf-core/src/resolve.rs`) so a type and its
> `impl`/methods (which share a name) disambiguate by path: `Type` alone is `Ambiguous`,
> but `Type > method` resolves uniquely. `NotFound` / `Ambiguous` / `Parse` are distinct.

## Why this is the riskiest phase
The entire value of the tool is firing on the *right* change (§6). Reliable polyglot spans
must come from a parser, not ctags (line, not span) or an LSP (reintroduces a CI
dependency). Grammars are compiled **into the binary** and version-pinned - this is the
reproducibility root (§6.1). Get spans wrong and every downstream phase inherits it.

## Steps

1. **Bundle grammars** in `surf-core`: `tree-sitter`, `tree-sitter-typescript`,
   `tree-sitter-rust`. **Pin exact versions** and record them (a grammar bump re-hashes
   everything - §11.4). No language server, no formatter, nothing but the binary at runtime.
2. **Language detection** by extension: `.ts`/`.tsx` → TypeScript, `.rs` → Rust. Model as
   an extensible enum so more grammars are additive.
3. **Anchor grammar parser** (§6.3) - pure string parsing, unit-tested in isolation:
   - Qualified path: `src/auth/refresh.ts > TokenService > rotate` (resolve through the symbol tree).
   - Positional fallback: `... > rotate @2` for the Nth same-named sibling when names genuinely collide.
4. **Symbol-tree resolver:** walk the parse tree, match the qualified path through nesting
   (module / class / `impl` → function / method), return the matched named node's exact
   byte + line span. Distinguish three outcomes the higher layers depend on:
   - exactly one match → `Ok(span)`.
   - zero → `NotFound`.
   - multiple with no `@N` → `Ambiguous` (lint must reject - §6.3).

## Files touched
- `surf-core/Cargo.toml` (grammar deps)
- `surf-core/src/lang.rs` (detection + grammar registry)
- `surf-core/src/anchor.rs` (anchor-string parser)
- `surf-core/src/resolve.rs` (symbol-tree walk → span)
- `fixtures/auth.ts`, `fixtures/auth.rs`

## Verify
Fixtures contain overloads, two classes/impls with same-named methods, and nested symbols.
Unit tests assert:
- correct byte/line spans for qualified and `@N` anchors, in both languages;
- `NotFound` and `Ambiguous` are returned distinctly (not collapsed into a generic error).

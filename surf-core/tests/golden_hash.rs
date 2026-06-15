//! Golden, cross-version determinism guard for the AST-canonical hash (§6.1).
//!
//! The stored anchor hash is the *only* thing every Surface consumer compares against. If the
//! canonical token stream changes for an unchanged symbol, every stored hash in every downstream
//! repo silently breaks (a wave of false DIVERGED) or, worse, two spans that should differ start
//! colliding. That can happen without anyone touching `hash.rs`:
//!
//!   * a tree-sitter **grammar bump** renames a node kind (`binary_expression` → …) or reshapes
//!     the tree — the grammars are caret-pinned in `Cargo.toml` (`tree-sitter-rust = "0.24.2"`
//!     ⇒ `^0.24.2`) and only frozen by `Cargo.lock`, which Dependabot bumps on a schedule;
//!   * a `tree-sitter` core bump changes traversal;
//!   * a refactor of the canonicalization itself.
//!
//! These goldens pin the exact hash of representative symbols in every supported family. A diff
//! here is a loud, intentional signal: the canonical form moved, so either revert the bump or
//! ship the hash change deliberately (and tell consumers to re-verify). Because CI runs this
//! suite on both Linux and macOS, it also catches any cross-platform drift between the three
//! release target triples.
//!
//! If a *deliberate* change updates these values, update CHANGELOG and treat it as a
//! hash-format break for downstreams.

use surf_core::{hash_anchor, parse_anchor, Lang};

fn h(src: &str, lang: Lang, anchor: &str) -> String {
    hash_anchor(src, lang, &parse_anchor(anchor).unwrap()).unwrap()
}

#[test]
fn golden_hashes_are_stable_per_language() {
    // Each snippet carries a comment and non-canonical whitespace on purpose, so the golden
    // already encodes the "comments + formatting are ignored" guarantee.
    let rust = "pub fn add(a: i64, b: i64) -> i64 {\n    // sum them\n    a + b\n}\n";
    assert_eq!(h(rust, Lang::Rust, "x.rs > add"), "f1075e760a17");

    let ts =
        "export class Svc {\n  rotate(tok: string): string {\n    return tok + tok; // c\n  }\n}\n";
    assert_eq!(
        h(ts, Lang::TypeScript, "x.ts > Svc > rotate"),
        "afa4514b5c89"
    );

    let tsx = "export function App(): JSX.Element {\n  return <div>{1 + 2}</div>;\n}\n";
    assert_eq!(h(tsx, Lang::Tsx, "x.tsx > App"), "97e0de58725d");

    let py = "def add(a, b):\n    # comment\n    return a + b\n";
    assert_eq!(h(py, Lang::Python, "x.py > add"), "879b76118966");

    let go = "func Add(a int, b int) int {\n\t// sum\n\treturn a + b\n}\n";
    assert_eq!(h(go, Lang::Go, "x.go > Add"), "942af2641116");
}

#[test]
fn logic_edits_change_the_hash() {
    let canonical = h(
        "pub fn add(a: i64, b: i64) -> i64 { a + b }\n",
        Lang::Rust,
        "x.rs > add",
    );

    // Flipped operator.
    let op_flip = h(
        "pub fn add(a: i64, b: i64) -> i64 { a - b }\n",
        Lang::Rust,
        "x.rs > add",
    );
    assert_ne!(op_flip, canonical, "an operator flip must move the hash");

    // Swapped operands without a consistent rename — a real semantic change, distinct from the
    // operator flip above (guards against the alpha-rename collapsing genuinely different code).
    let swapped = h(
        "pub fn add(a: i64, b: i64) -> i64 { b - a }\n",
        Lang::Rust,
        "x.rs > add",
    );
    assert_ne!(swapped, canonical);
    assert_ne!(swapped, op_flip, "b - a must not collide with a - b");
}

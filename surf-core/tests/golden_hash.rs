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

use surf_core::{format_stamp, hash_anchor, hash_anchor_raw, parse_anchor, HashOpts, Lang, Recipe};

fn h(src: &str, lang: Lang, anchor: &str) -> String {
    hash_anchor(src, lang, &parse_anchor(anchor).unwrap()).unwrap()
}

fn raw(src: &str, lang: Lang, anchor: &str, recipe: Recipe) -> String {
    hash_anchor_raw(
        src,
        lang,
        &parse_anchor(anchor).unwrap(),
        HashOpts::default(),
        recipe,
    )
    .unwrap()
}

#[test]
fn golden_hashes_are_stable_per_language() {
    // Each snippet carries a comment and non-canonical whitespace on purpose, so the golden
    // already encodes the "comments + formatting are ignored" guarantee. These snippets are
    // member-access-free, so v1 and v2 agree byte-for-byte — itself the guarantee that v2 only
    // diverges on member-access names (#140). The frozen v1 digests are unchanged from before
    // versioning, so existing v1 stamps in downstream repos still verify.
    let rust = "pub fn add(a: i64, b: i64) -> i64 {\n    // sum them\n    a + b\n}\n";
    assert_eq!(
        raw(rust, Lang::Rust, "x.rs > add", Recipe::V1),
        "f1075e760a17"
    );
    assert_eq!(
        raw(rust, Lang::Rust, "x.rs > add", Recipe::V2),
        "f1075e760a17"
    );

    let ts =
        "export class Svc {\n  rotate(tok: string): string {\n    return tok + tok; // c\n  }\n}\n";
    assert_eq!(
        raw(ts, Lang::TypeScript, "x.ts > Svc > rotate", Recipe::V1),
        "afa4514b5c89"
    );
    assert_eq!(
        raw(ts, Lang::TypeScript, "x.ts > Svc > rotate", Recipe::V2),
        "afa4514b5c89"
    );

    let tsx = "export function App(): JSX.Element {\n  return <div>{1 + 2}</div>;\n}\n";
    assert_eq!(
        raw(tsx, Lang::Tsx, "x.tsx > App", Recipe::V1),
        "97e0de58725d"
    );
    assert_eq!(
        raw(tsx, Lang::Tsx, "x.tsx > App", Recipe::V2),
        "97e0de58725d"
    );

    let py = "def add(a, b):\n    # comment\n    return a + b\n";
    assert_eq!(
        raw(py, Lang::Python, "x.py > add", Recipe::V1),
        "879b76118966"
    );
    assert_eq!(
        raw(py, Lang::Python, "x.py > add", Recipe::V2),
        "879b76118966"
    );

    let go = "func Add(a int, b int) int {\n\t// sum\n\treturn a + b\n}\n";
    assert_eq!(raw(go, Lang::Go, "x.go > Add", Recipe::V1), "942af2641116");
    assert_eq!(raw(go, Lang::Go, "x.go > Add", Recipe::V2), "942af2641116");

    // The stored stamp for a single-site anchor carries the current-recipe (v2) prefix.
    assert_eq!(h(rust, Lang::Rust, "x.rs > add"), "2:f1075e760a17");
    assert_eq!(format_stamp(Recipe::V1, "f1075e760a17"), "f1075e760a17");
}

#[test]
fn golden_member_access_hashes_differ_by_recipe() {
    // Symbols whose only interesting content is a member access: v1 and v2 diverge, and both
    // digests are pinned so a grammar bump or canonicalization refactor that perturbs either
    // recipe is a loud, intentional signal (the #140 probes, one per family).
    let ts = "export class S {\n  tier(u: User): Tier {\n    return Tiers.getHighest(u.nxp, PointsTier.TIER_1);\n  }\n}\n";
    assert_eq!(
        raw(ts, Lang::TypeScript, "x.ts > S > tier", Recipe::V1),
        "9aea05e557ad"
    );
    assert_eq!(
        raw(ts, Lang::TypeScript, "x.ts > S > tier", Recipe::V2),
        "2c5e43fc3a1f"
    );

    let go = "func (b *Builder) Set(n string) *Builder {\n\treturn b.Del(n)\n}\n";
    assert_eq!(
        raw(go, Lang::Go, "x.go > Builder > Set", Recipe::V1),
        "34bc2bf73d75"
    );
    assert_eq!(
        raw(go, Lang::Go, "x.go > Builder > Set", Recipe::V2),
        "e5bc7182a348"
    );

    let py = "def color(self):\n    return ProbeColor.RED\n";
    assert_eq!(
        raw(py, Lang::Python, "x.py > color", Recipe::V1),
        "6061e364641b"
    );
    assert_eq!(
        raw(py, Lang::Python, "x.py > color", Recipe::V2),
        "ccf224e8a6fc"
    );

    let rs = "pub fn name(p: Person) -> String {\n    p.first.clone()\n}\n";
    assert_eq!(
        raw(rs, Lang::Rust, "x.rs > name", Recipe::V1),
        "0e1353a2aee5"
    );
    assert_eq!(
        raw(rs, Lang::Rust, "x.rs > name", Recipe::V2),
        "a0a671650fb6"
    );
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

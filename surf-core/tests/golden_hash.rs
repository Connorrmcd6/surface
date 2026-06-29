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
    // already encodes the "comments + formatting are ignored" guarantee. The frozen v1 digests
    // are unchanged from before versioning, so existing v1 stamps in downstream repos still
    // verify. A snippet whose only identifiers are *bound* (params, the symbol's own name) and
    // operators hashes the same under v1 and v2 — the bound/free split (#77) changes nothing
    // when there is no free identifier to emit verbatim.
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

    let py = "def add(a, b):\n    # comment\n    return a + b\n";
    assert_eq!(
        raw(py, Lang::Python, "x.py > add", Recipe::V1),
        "879b76118966"
    );
    assert_eq!(
        raw(py, Lang::Python, "x.py > add", Recipe::V2),
        "879b76118966"
    );

    // These two carry *free* identifiers — JSX element/type names, and Go's `int` type — so the
    // bound/free split makes v2 diverge from v1: the free names are now verbatim, so re-pointing
    // at a different tag or type is loud. Both digests are pinned.
    let tsx = "export function App(): JSX.Element {\n  return <div>{1 + 2}</div>;\n}\n";
    assert_eq!(
        raw(tsx, Lang::Tsx, "x.tsx > App", Recipe::V1),
        "97e0de58725d"
    );
    assert_eq!(
        raw(tsx, Lang::Tsx, "x.tsx > App", Recipe::V2),
        "92e69aab47fb"
    );

    let go = "func Add(a int, b int) int {\n\t// sum\n\treturn a + b\n}\n";
    assert_eq!(raw(go, Lang::Go, "x.go > Add", Recipe::V1), "942af2641116");
    assert_eq!(raw(go, Lang::Go, "x.go > Add", Recipe::V2), "5bb84c760e6b");

    // The stored stamp for a single-site anchor carries the current-recipe (v2) prefix.
    assert_eq!(h(rust, Lang::Rust, "x.rs > add"), "2:f1075e760a17");
    assert_eq!(format_stamp(Recipe::V1, "f1075e760a17"), "f1075e760a17");
}

#[test]
fn golden_unicode_identifier_hashes_are_stable() {
    // Non-ASCII symbol names and bodies across the four families (#45). Pinning these as goldens
    // turns any future locale/encoding sensitivity in canonicalization into a loud diff. Each
    // snippet carries a comment + non-canonical whitespace, so it also re-asserts the
    // "comments + formatting ignored" guarantee for Unicode source. The Rust/TS/Python snippets
    // have only bound identifiers, so v1 and v2 agree; the Go one carries the free `int` type, so
    // v2 diverges under the bound/free split.
    let rust = "pub fn café(δ: i64) -> i64 {\n    // accent\n    δ\n}\n";
    assert_eq!(
        raw(rust, Lang::Rust, "x.rs > café", Recipe::V1),
        "9c1a869d1c60"
    );
    assert_eq!(
        raw(rust, Lang::Rust, "x.rs > café", Recipe::V2),
        "9c1a869d1c60"
    );

    let ts = "export function café(δ: string): string {\n  return δ; // u\n}\n";
    assert_eq!(
        raw(ts, Lang::TypeScript, "x.ts > café", Recipe::V1),
        "f7607eacbd73"
    );
    assert_eq!(
        raw(ts, Lang::TypeScript, "x.ts > café", Recipe::V2),
        "f7607eacbd73"
    );

    let py = "def café(δ):\n    # accent\n    return δ\n";
    assert_eq!(
        raw(py, Lang::Python, "x.py > café", Recipe::V1),
        "bc2439d5f488"
    );
    assert_eq!(
        raw(py, Lang::Python, "x.py > café", Recipe::V2),
        "bc2439d5f488"
    );

    let go = "func Café(δ int) int {\n\t// u\n\treturn δ\n}\n";
    assert_eq!(raw(go, Lang::Go, "x.go > Café", Recipe::V1), "9a101a4d062f");
    assert_eq!(raw(go, Lang::Go, "x.go > Café", Recipe::V2), "51c5edab6591");
}

#[test]
fn unicode_identifier_hashes_are_recomputation_stable() {
    // Re-running the same hash yields the same value — the determinism half of the guarantee,
    // independent of the pinned goldens above.
    let py = "def café(δ):\n    # accent\n    return δ\n";
    assert_eq!(
        raw(py, Lang::Python, "x.py > café", Recipe::V2),
        raw(py, Lang::Python, "x.py > café", Recipe::V2)
    );
}

#[test]
fn golden_member_access_hashes_differ_by_recipe() {
    // Symbols carrying member accesses and free references: v1 (alpha-rename everything) and v2
    // (the bound/free split) diverge, and both digests are pinned so a grammar bump or
    // canonicalization refactor that perturbs either recipe is a loud, intentional signal (the
    // #77/#140 probes, one per family). v2 here also emits the free *receivers*/types verbatim
    // (`Tiers`, `PointsTier`, `User`, `Tier`, `Person`), not only the member names — the
    // difference between the full split and the member-only first cut.
    let ts = "export class S {\n  tier(u: User): Tier {\n    return Tiers.getHighest(u.nxp, PointsTier.TIER_1);\n  }\n}\n";
    assert_eq!(
        raw(ts, Lang::TypeScript, "x.ts > S > tier", Recipe::V1),
        "9aea05e557ad"
    );
    assert_eq!(
        raw(ts, Lang::TypeScript, "x.ts > S > tier", Recipe::V2),
        "96e55763b827"
    );

    let go = "func (b *Builder) Set(n string) *Builder {\n\treturn b.Del(n)\n}\n";
    assert_eq!(
        raw(go, Lang::Go, "x.go > Builder > Set", Recipe::V1),
        "34bc2bf73d75"
    );
    assert_eq!(
        raw(go, Lang::Go, "x.go > Builder > Set", Recipe::V2),
        "e6ab4bb83933"
    );

    let py = "def color(self):\n    return ProbeColor.RED\n";
    assert_eq!(
        raw(py, Lang::Python, "x.py > color", Recipe::V1),
        "6061e364641b"
    );
    assert_eq!(
        raw(py, Lang::Python, "x.py > color", Recipe::V2),
        "0a9441f535d0"
    );

    let rs = "pub fn name(p: Person) -> String {\n    p.first.clone()\n}\n";
    assert_eq!(
        raw(rs, Lang::Rust, "x.rs > name", Recipe::V1),
        "0e1353a2aee5"
    );
    assert_eq!(
        raw(rs, Lang::Rust, "x.rs > name", Recipe::V2),
        "31f85c893b1e"
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

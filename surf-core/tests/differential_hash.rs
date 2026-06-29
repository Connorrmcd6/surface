//! In-tree v1-vs-v2 A/B for the bound/free split (#77), kept here so any future canonicalization
//! change reruns the same gate. Two mutation classes, asserted per case below:
//!   - Benign (consistent bound-name rename): v2 must stay quiet — zero regressions tolerated.
//!   - Semantic (single-occurrence free-name swap): v1 blind, v2 loud — v2 catches what v1 missed.
//!
//! The external git-history replay over real corpora (#77) runs out-of-tree; this is its always-on
//! counterpart.

use surf_core::{hash_anchor_raw, parse_anchor, HashOpts, Lang, Recipe};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Benign,
    Semantic,
}
use Kind::*;

struct Case {
    lang: Lang,
    anchor: &'static str,
    // The anchor for `after`, when a benign edit renames the symbol itself (so it resolves under
    // a new path). `None` means same as `anchor`.
    after_anchor: Option<&'static str>,
    before: &'static str,
    after: &'static str,
    kind: Kind,
    note: &'static str,
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

fn cases() -> Vec<Case> {
    vec![
        // ---- Benign: consistent renames of BOUND names must stay quiet under v2 -------------
        Case {
            lang: Lang::Rust,
            anchor: "x.rs > f",
            after_anchor: None,
            before: "pub fn f(nxp: i64) -> i64 { let t = nxp; t + nxp }\n",
            after: "pub fn f(points: i64) -> i64 { let t2 = points; t2 + points }\n",
            kind: Benign,
            note: "rust param + local rename",
        },
        Case {
            lang: Lang::Rust,
            anchor: "x.rs > f",
            after_anchor: None,
            before: "pub fn f<T>(xs: Vec<T>) -> T { xs.into_iter().next().unwrap() }\n",
            after: "pub fn f<U>(ys: Vec<U>) -> U { ys.into_iter().next().unwrap() }\n",
            kind: Benign,
            note: "rust generic param + param rename",
        },
        Case {
            lang: Lang::Rust,
            anchor: "x.rs > f",
            after_anchor: None,
            before: "pub fn f(v: Vec<i64>) -> i64 { let mut s = 0; for it in v { s += it; } s }\n",
            after: "pub fn f(w: Vec<i64>) -> i64 { let mut q = 0; for jt in w { q += jt; } q }\n",
            kind: Benign,
            note: "rust for-loop binder + locals",
        },
        Case {
            lang: Lang::Rust,
            anchor: "x.rs > rotate",
            after_anchor: Some("x.rs > renamed"),
            before: "pub fn rotate(token: &str) -> String { token.to_string() }\n",
            after: "pub fn renamed(token: &str) -> String { token.to_string() }\n",
            kind: Benign,
            note: "rust symbol's own name (rename-quiet, relocatable by hash)",
        },
        Case {
            lang: Lang::TypeScript,
            anchor: "x.ts > S > m",
            after_anchor: None,
            before: "export class S {\n  m(a: number): number { const x = a; return x + a; }\n}\n",
            after: "export class S {\n  m(b: number): number { const y = b; return y + b; }\n}\n",
            kind: Benign,
            note: "ts param + local rename",
        },
        Case {
            lang: Lang::TypeScript,
            anchor: "x.ts > f",
            after_anchor: None,
            before: "export function f(o: O): number { const { k: a } = o; return a; }\n",
            after: "export function f(o: O): number { const { k: b } = o; return b; }\n",
            kind: Benign,
            note: "ts destructuring binder rename (source key unchanged)",
        },
        Case {
            lang: Lang::TypeScript,
            anchor: "x.ts > f",
            after_anchor: None,
            before: "export function f(xs: number[]): number { return xs.map((v) => v + 1)[0]; }\n",
            after: "export function f(ys: number[]): number { return ys.map((w) => w + 1)[0]; }\n",
            kind: Benign,
            note: "ts arrow-fn param + param rename",
        },
        Case {
            lang: Lang::Python,
            anchor: "x.py > f",
            after_anchor: None,
            before: "def f(a, b=1):\n    x = a + b\n    return x\n",
            after: "def f(c, b=1):\n    y = c + b\n    return y\n",
            kind: Benign,
            note: "py param + local rename (default value untouched)",
        },
        Case {
            lang: Lang::Python,
            anchor: "x.py > f",
            after_anchor: None,
            before: "def f(items):\n    total = 0\n    for it in items:\n        total += it\n    return total\n",
            after: "def f(things):\n    sum_ = 0\n    for jt in things:\n        sum_ += jt\n    return sum_\n",
            kind: Benign,
            note: "py for-loop binder + augmented-assignment local",
        },
        Case {
            lang: Lang::Python,
            anchor: "x.py > f",
            after_anchor: None,
            before: "def f(p):\n    with open(p) as fh:\n        return fh.read()\n",
            after: "def f(q):\n    with open(q) as handle:\n        return handle.read()\n",
            kind: Benign,
            note: "py with-as alias + param rename",
        },
        Case {
            lang: Lang::Go,
            anchor: "x.go > Builder > Set",
            after_anchor: None,
            before: "func (b *Builder) Set(n string) string { x := n; return x }\n",
            after: "func (c *Builder) Set(m string) string { y := m; return y }\n",
            kind: Benign,
            note: "go receiver + param + short-var rename",
        },
        Case {
            lang: Lang::Go,
            anchor: "x.go > Sum",
            after_anchor: None,
            before: "func Sum(xs []int) int {\n\ttotal := 0\n\tfor i, v := range xs {\n\t\ttotal += i + v\n\t}\n\treturn total\n}\n",
            after: "func Sum(ys []int) int {\n\tsum := 0\n\tfor j, w := range ys {\n\t\tsum += j + w\n\t}\n\treturn sum\n}\n",
            kind: Benign,
            note: "go range binders + locals rename",
        },
        Case {
            lang: Lang::Go,
            anchor: "x.go > Pair",
            after_anchor: None,
            before: "func Pair(a, b int) int { return a + b }\n",
            after: "func Pair(c, d int) int { return c + d }\n",
            kind: Benign,
            note: "go grouped multi-name params rename",
        },
        // ---- Semantic: single-occurrence FREE swaps must become loud under v2 ---------------
        Case {
            lang: Lang::TypeScript,
            anchor: "x.ts > S > f",
            after_anchor: None,
            before: "export class S {\n  f(): T { return PointsTier.TIER_1; }\n}\n",
            after: "export class S {\n  f(): T { return PointsTier.TIER_2; }\n}\n",
            kind: Semantic,
            note: "ts enum member swap (the original #77 repro)",
        },
        Case {
            lang: Lang::TypeScript,
            anchor: "x.ts > S > f",
            after_anchor: None,
            before: "export class S {\n  f(u: U): T { return Tiers.getHighest(u); }\n}\n",
            after: "export class S {\n  f(u: U): T { return Tiers.getLowest(u); }\n}\n",
            kind: Semantic,
            note: "ts method-call target swap",
        },
        Case {
            lang: Lang::TypeScript,
            anchor: "x.ts > f",
            after_anchor: None,
            before: "export function f(x: number): number { return helper(x); }\n",
            after: "export function f(x: number): number { return other(x); }\n",
            kind: Semantic,
            note: "ts bare free call-target swap",
        },
        Case {
            lang: Lang::TypeScript,
            anchor: "x.ts > f",
            after_anchor: None,
            before: "export function f(o: O): number { const { k: a } = o; return a; }\n",
            after: "export function f(o: O): number { const { j: a } = o; return a; }\n",
            kind: Semantic,
            note: "ts destructuring SOURCE key swap (reads a different member)",
        },
        Case {
            lang: Lang::Go,
            anchor: "x.go > Builder > Set",
            after_anchor: None,
            before: "func (b *Builder) Set(n string) *Builder { return b.Del(n) }\n",
            after: "func (b *Builder) Set(n string) *Builder { return b.Keep(n) }\n",
            kind: Semantic,
            note: "go field-method swap",
        },
        Case {
            lang: Lang::Go,
            anchor: "x.go > F",
            after_anchor: None,
            before: "func F(x int) int { return helper(x) }\n",
            after: "func F(x int) int { return other(x) }\n",
            kind: Semantic,
            note: "go bare free call-target swap",
        },
        Case {
            lang: Lang::Python,
            anchor: "x.py > color",
            after_anchor: None,
            before: "def color(self):\n    return ProbeColor.RED\n",
            after: "def color(self):\n    return ProbeColor.GREEN\n",
            kind: Semantic,
            note: "py attribute swap",
        },
        // NB: a decorator-name swap is intentionally *not* in this table — v1 already catches it
        // via the #8 special case, so it is not v1-blind. That v2 catches it through the general
        // free-identifier rule (the special case subsumed) is asserted in hash.rs unit tests.
        Case {
            lang: Lang::Python,
            anchor: "x.py > f",
            after_anchor: None,
            before: "def f(x):\n    return helper(x)\n",
            after: "def f(x):\n    return other(x)\n",
            kind: Semantic,
            note: "py bare free call-target swap",
        },
        Case {
            lang: Lang::Rust,
            anchor: "x.rs > f",
            after_anchor: None,
            before: "pub fn f(p: P) -> i64 { p.first }\n",
            after: "pub fn f(p: P) -> i64 { p.second }\n",
            kind: Semantic,
            note: "rust field-access swap",
        },
        Case {
            lang: Lang::Rust,
            anchor: "x.rs > f",
            after_anchor: None,
            before: "pub fn f(x: i64) -> i64 { helper(x) }\n",
            after: "pub fn f(x: i64) -> i64 { other(x) }\n",
            kind: Semantic,
            note: "rust bare free call-target swap",
        },
        Case {
            lang: Lang::Rust,
            anchor: "x.rs > f",
            after_anchor: None,
            before: "pub fn f(x: Foo) -> i64 { 0 }\n",
            after: "pub fn f(x: Bar) -> i64 { 0 }\n",
            kind: Semantic,
            note: "rust external param-type swap (contract change)",
        },
        Case {
            lang: Lang::Go,
            anchor: "x.go > F",
            after_anchor: None,
            before: "func F(x Foo) int { return 0 }\n",
            after: "func F(x Bar) int { return 0 }\n",
            kind: Semantic,
            note: "go external param-type swap (contract change)",
        },
    ]
}

#[test]
fn v2_is_quiet_on_benign_renames_and_loud_on_free_swaps() {
    let mut benign = 0usize;
    let mut semantic = 0usize;
    for c in cases() {
        let after_anchor = c.after_anchor.unwrap_or(c.anchor);
        let v1_before = raw(c.before, c.lang, c.anchor, Recipe::V1);
        let v1_after = raw(c.after, c.lang, after_anchor, Recipe::V1);
        let v2_before = raw(c.before, c.lang, c.anchor, Recipe::V2);
        let v2_after = raw(c.after, c.lang, after_anchor, Recipe::V2);
        match c.kind {
            Benign => {
                // Rename tolerance: a consistent bound-name rename must be invisible to v2.
                assert_eq!(
                    v2_before, v2_after,
                    "BENIGN REGRESSION: v2 fired on a consistent bound rename [{}]",
                    c.note
                );
                benign += 1;
            }
            Semantic => {
                // Each is single-occurrence free, so v1 is blind — that is the bug.
                assert_eq!(
                    v1_before, v1_after,
                    "harness setup: semantic case is not v1-blind [{}]",
                    c.note
                );
                // v2 must catch what v1 missed.
                assert_ne!(
                    v2_before, v2_after,
                    "MISSED DRIFT: v2 stayed quiet on a free-identifier swap [{}]",
                    c.note
                );
                semantic += 1;
            }
        }
    }
    // The headline metrics from the issue's validation gate, made executable.
    assert!(benign >= 12, "expected a broad benign corpus, got {benign}");
    assert!(
        semantic >= 12,
        "expected a broad semantic corpus, got {semantic}"
    );
    eprintln!(
        "differential: {benign} benign (0 regressions), {semantic} semantic (100% v2 catch, 0% v1)"
    );
}

/// The accepted approximation, pinned so it's a documented limit not a surprise (see
/// `docs/reference/hash-recipes.md`). Match-arm pattern identifiers are left free: a unit-variant
/// swap is caught (good), but renaming a catch-all binding is also loud (an accepted false
/// positive).
#[test]
fn accepted_residue_match_arm_identifiers_are_free() {
    // Safe direction: matching a different unit variant is a real change and is caught.
    let variant_a = "pub fn f(c: Color) -> i64 { match c { Color::Red => 1, _ => 0 } }\n";
    let variant_b = "pub fn f(c: Color) -> i64 { match c { Color::Blue => 1, _ => 0 } }\n";
    assert_ne!(
        raw(variant_a, Lang::Rust, "x.rs > f", Recipe::V2),
        raw(variant_b, Lang::Rust, "x.rs > f", Recipe::V2),
        "a unit-variant swap must stay loud",
    );

    // Accepted cost: renaming a catch-all binding is loud under v2 (it is left free), where v1 was
    // quiet. This is the one benign-edit class v2 does not keep silent — documented, not a bug.
    let bind_a = "pub fn f(o: Option<i64>) -> i64 { match o { Some(x) => x, None => 0 } }\n";
    let bind_b = "pub fn f(o: Option<i64>) -> i64 { match o { Some(y) => y, None => 0 } }\n";
    assert_eq!(
        raw(bind_a, Lang::Rust, "x.rs > f", Recipe::V1),
        raw(bind_b, Lang::Rust, "x.rs > f", Recipe::V1),
        "v1 is quiet on the match-binding rename",
    );
    assert_ne!(
        raw(bind_a, Lang::Rust, "x.rs > f", Recipe::V2),
        raw(bind_b, Lang::Rust, "x.rs > f", Recipe::V2),
        "residue changed: revisit the documented limit in docs/reference/hash-recipes.md",
    );
}

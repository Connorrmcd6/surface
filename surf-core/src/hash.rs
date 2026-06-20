//! AST-canonical hashing (§6.1) and advisory diff magnitude (§6.2).
//!
//! The hash is computed over a canonical token stream of the symbol's subtree:
//! - whitespace and formatting are absent from the tree, so they are ignored for free;
//! - comments are dropped explicitly;
//! - identifiers are alpha-renamed to positional placeholders (`#0`, `#1`, …) in order of
//!   first occurrence, so a *consistent* rename hashes identically while swapping two names
//!   does not;
//! - operators, keywords, punctuation, and literal *values* are kept verbatim — so a
//!   flipped operator (`+`→`-`), a relaxed comparison (`<`→`<=`), a deleted `await`, or a
//!   changed constant all change the hash.
//!
//! The result is quiet on the changes you want ignored and loud on the ones you must catch.
//!
//! ## Recipes (versioned canonicalization)
//!
//! The canonicalization above is the **v1** recipe. **v2** (#140) adds one rule: the
//! property/field component of a member-access expression (`obj.foo`, `pkg.Bar`) is kept
//! *verbatim* rather than alpha-renamed, so re-pointing an anchored span at a different
//! external symbol (`PointsTier.TIER_1` → `TIER_2`, `b.Del` → `b.Keep`) changes the hash even
//! when the name occurs exactly once. These positions are never bindings, so emitting them
//! verbatim cannot resurface a benign local rename. v1 ≡ v2 minus that single rule — one mode
//! flag, no frozen copy of the old algorithm.
//!
//! Stored stamps carry their recipe: a v2 stamp is prefixed `2:`, a bare 12-hex stamp is
//! implicitly v1. New stamps are written under [`Recipe::CURRENT`]; `check` verifies a stamp
//! under *its own* recipe, so existing v1 stamps keep working until `surf verify` upgrades
//! them. See `docs/hash-recipes.md`.
//!
//! `Magnitude` is advisory triage metadata only. It is never compared, thresholded, or used
//! to decide pass/fail — that would defeat the whole point (§6.2).

use crate::anchor::Anchor;
use crate::lang::{Family, Lang};
use crate::resolve::{hashable_node, parse_tree, resolve_nodes, ResolveError};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write as _;
use tree_sitter::Node;

const HASH_HEX_LEN: usize = 12;

/// Per-claim hashing options. `ignore_literals` scopes string-literal *content* out of the
/// token stream so a copy edit inside an anchored span doesn't re-open the gate (§6.1). It must
/// match between the stored hash and the gate hash, so it lives on the claim, not a CLI flag.
#[derive(Debug, Clone, Copy, Default)]
pub struct HashOpts {
    pub ignore_literals: bool,
}

/// A canonicalization recipe. Bumped whenever a change to canonical output would otherwise
/// silently invalidate every stored stamp (see the module docs and `docs/hash-recipes.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recipe {
    /// Original recipe: every identifier alpha-renamed. Implicit for bare (unprefixed) stamps.
    V1,
    /// v1 plus the member-access-name verbatim rule (#140). Stamps are prefixed `2:`.
    V2,
}

impl Recipe {
    /// The recipe new stamps are written under (`verify`, and `check`'s suggestion for an
    /// unverified claim).
    pub const CURRENT: Recipe = Recipe::V2;
}

/// Split a stored stamp into its recipe and bare hex digest. A bare hex stamp is implicitly
/// v1 (every stamp written before versioned recipes). Returns `None` for an unrecognized
/// prefix — e.g. a `3:` stamp from a newer surf — so the caller fails closed rather than
/// guessing a recipe.
pub fn parse_stamp(stamp: &str) -> Option<(Recipe, &str)> {
    match stamp.split_once(':') {
        Some(("2", hex)) if is_hex(hex) => Some((Recipe::V2, hex)),
        Some(_) => None,
        None if is_hex(stamp) => Some((Recipe::V1, stamp)),
        None => None,
    }
}

/// Format a bare hex digest as a stored stamp under `recipe`: v1 is bare (back-compat), later
/// recipes carry an `N:` prefix.
pub fn format_stamp(recipe: Recipe, hex: &str) -> String {
    match recipe {
        Recipe::V1 => hex.to_string(),
        Recipe::V2 => format!("2:{hex}"),
    }
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The full stored stamp for a single-site anchor under [`Recipe::CURRENT`] (v2), prefix and
/// all. Multi-site claims combine per-site [`hash_anchor_raw`] digests via
/// [`combine_site_hashes`] and prefix the result with [`format_stamp`].
pub fn hash_anchor(source: &str, lang: Lang, anchor: &Anchor) -> Result<String, ResolveError> {
    hash_anchor_with(source, lang, anchor, HashOpts::default())
}

/// Like [`hash_anchor`], with per-claim [`HashOpts`]. Returns the current-recipe stamp.
pub fn hash_anchor_with(
    source: &str,
    lang: Lang,
    anchor: &Anchor,
    opts: HashOpts,
) -> Result<String, ResolveError> {
    let hex = hash_anchor_raw(source, lang, anchor, opts, Recipe::CURRENT)?;
    Ok(format_stamp(Recipe::CURRENT, &hex))
}

/// The bare hex digest of one anchor under `recipe` — the per-site hash that
/// [`combine_site_hashes`] folds into a claim stamp. Carries no version prefix; use
/// [`format_stamp`] to turn the combined digest into a stored stamp.
pub fn hash_anchor_raw(
    source: &str,
    lang: Lang,
    anchor: &Anchor,
    opts: HashOpts,
    recipe: Recipe,
) -> Result<String, ResolveError> {
    Ok(hash_tokens(&anchor_tokens(
        source, lang, anchor, opts, recipe,
    )?))
}

/// One hash per claim from its per-site hashes (§6.3). A single site is the identity (so the
/// stored hash is just the symbol's hash); multiple sites combine order-sensitively, so the
/// claim is stale if *any* listed span changes.
pub fn combine_site_hashes(site_hashes: &[String]) -> String {
    match site_hashes {
        [one] => one.clone(),
        many => hash_tokens(many),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Magnitude {
    Small,
    Medium,
    Large,
}

/// Advisory only — describes how big the change to a span was, for triage. Never gates.
pub fn diff_magnitude(
    old_source: &str,
    new_source: &str,
    lang: Lang,
    anchor: &Anchor,
) -> Result<Magnitude, ResolveError> {
    let old = anchor_tokens(
        old_source,
        lang,
        anchor,
        HashOpts::default(),
        Recipe::CURRENT,
    )?;
    let new = anchor_tokens(
        new_source,
        lang,
        anchor,
        HashOpts::default(),
        Recipe::CURRENT,
    )?;
    Ok(categorize(token_distance(&old, &new)))
}

fn anchor_tokens(
    source: &str,
    lang: Lang,
    anchor: &Anchor,
    opts: HashOpts,
    recipe: Recipe,
) -> Result<Vec<String>, ResolveError> {
    let tree = parse_tree(source, lang).ok_or(ResolveError::Parse)?;
    let src = source.as_bytes();
    let family = lang.family();
    let nodes = resolve_nodes(tree.root_node(), src, family, anchor)?;
    // A Python @overload group hashes as one token stream — stubs then impl in source order,
    // sharing one alpha-rename map — so a signature change in *any* overload changes the
    // hash (#82). The usual single-node case is unchanged.
    let mut out = Vec::new();
    let mut idents: HashMap<String, usize> = HashMap::new();
    for node in nodes {
        emit(
            hashable_node(node, family),
            src,
            family,
            opts,
            recipe,
            false,
            &mut idents,
            &mut out,
        );
    }
    Ok(out)
}

pub(crate) fn hash_node(
    node: Node,
    src: &[u8],
    family: Family,
    opts: HashOpts,
    recipe: Recipe,
) -> String {
    hash_tokens(&canonical_tokens(node, src, family, opts, recipe))
}

fn canonical_tokens(
    node: Node,
    src: &[u8],
    family: Family,
    opts: HashOpts,
    recipe: Recipe,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut idents: HashMap<String, usize> = HashMap::new();
    emit(
        hashable_node(node, family),
        src,
        family,
        opts,
        recipe,
        false,
        &mut idents,
        &mut out,
    );
    out
}

#[allow(clippy::too_many_arguments)]
fn emit(
    node: Node,
    src: &[u8],
    family: Family,
    opts: HashOpts,
    recipe: Recipe,
    // True while inside a decorator's *name* (the symbol being applied), where identifiers are
    // kept verbatim rather than alpha-renamed — so `@cache` → `@lru_cache` or
    // `@staticmethod` → `@classmethod` is caught (§6.1, #8). Arguments to a decorator follow the
    // normal rules, so reformatting them stays quiet.
    decorator_name: bool,
    idents: &mut HashMap<String, usize>,
    out: &mut Vec<String>,
) {
    let kind = node.kind();
    if kind.contains("comment") {
        return;
    }

    if node.is_named() {
        if is_identifier(kind, family) {
            let text = node.utf8_text(src).unwrap_or_default();
            // v2 keeps member-access names verbatim too, so `obj.foo` → `obj.bar` is loud even
            // when `bar` occurs once (#140). v1 keeps only decorator names verbatim.
            let verbatim =
                decorator_name || (recipe == Recipe::V2 && is_member_access_name(node, family));
            if verbatim {
                out.push(format!("{kind}:{text}"));
            } else {
                let next = idents.len();
                let idx = *idents.entry(text.to_string()).or_insert(next);
                out.push(format!("#{idx}"));
            }
            return;
        }
        if node.child_count() == 0 {
            // Named terminal (literal, primitive type, keyword-like): keep its value, unless the
            // claim opted to ignore string-literal content — then emit only the kind so a copy
            // edit is invisible while the literal's *presence* still counts.
            if opts.ignore_literals && is_string_literal(kind, family) {
                out.push(kind.to_string());
            } else {
                out.push(format!(
                    "{kind}:{}",
                    node.utf8_text(src).unwrap_or_default()
                ));
            }
            return;
        }
        // A string that the grammar splits into child tokens (e.g. TS template strings, Python
        // `string` with start/content/end). Drop the whole node when ignoring literals so its
        // content children aren't emitted.
        if opts.ignore_literals && is_string_literal(kind, family) {
            out.push(kind.to_string());
            return;
        }
        out.push(kind.to_string());
    } else {
        // Anonymous token: operator, punctuation, or keyword. Its kind *is* the text.
        out.push(kind.to_string());
        return;
    }

    // Within a Python decorator, the name is literal; its argument list reverts to normal rules.
    let child_decorator_name = match (family, kind) {
        (Family::Python, "decorator") => true,
        (Family::Python, "argument_list") => false,
        _ => decorator_name,
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        emit(
            child,
            src,
            family,
            opts,
            recipe,
            child_decorator_name,
            idents,
            out,
        );
    }
}

/// True for the property/field component of a member-access expression — the part the v2
/// recipe (#140) keeps verbatim. These positions name an *external* member, never a local
/// binding, so emitting them verbatim distinguishes "re-pointed at a different symbol" from
/// "renamed my own local" without breaking rename tolerance. Each family is matched
/// structurally (kind + parent kind + the parent's named field) so an identifier that merely
/// *shares* the kind in another position (e.g. an object-literal key, a method *name*) is left
/// to the normal alpha-rename.
fn is_member_access_name(node: Node, family: Family) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let is_field =
        |field: &str| parent.child_by_field_name(field).map(|n| n.id()) == Some(node.id());
    match family {
        // `obj.prop` / `obj?.prop` — the property of a member_expression.
        Family::TypeScript => {
            matches!(
                node.kind(),
                "property_identifier" | "private_property_identifier"
            ) && parent.kind() == "member_expression"
                && is_field("property")
        }
        // `pkg.Bar` / `recv.Method` — the field of a selector_expression.
        Family::Go => {
            node.kind() == "field_identifier"
                && parent.kind() == "selector_expression"
                && is_field("field")
        }
        // `value.field` / `value.method()` — the field of a field_expression. (Path access
        // `Enum::Variant` is a scoped_identifier, not a field — left to the full split, #77.)
        Family::Rust => {
            node.kind() == "field_identifier"
                && parent.kind() == "field_expression"
                && is_field("field")
        }
        // `obj.attr` — the attribute of an attribute node.
        Family::Python => {
            node.kind() == "identifier" && parent.kind() == "attribute" && is_field("attribute")
        }
    }
}

/// String-literal node kinds per family (content only — numbers/bools stay logic). Used by the
/// per-claim `ignore_literals` option.
fn is_string_literal(kind: &str, family: Family) -> bool {
    match family {
        Family::Rust => matches!(kind, "string_literal" | "raw_string_literal"),
        Family::TypeScript => matches!(kind, "string" | "template_string"),
        Family::Python => kind == "string",
        Family::Go => matches!(kind, "interpreted_string_literal" | "raw_string_literal"),
    }
}

fn is_identifier(kind: &str, family: Family) -> bool {
    match family {
        Family::Rust => matches!(
            kind,
            "identifier" | "type_identifier" | "field_identifier" | "shorthand_field_identifier"
        ),
        Family::TypeScript => matches!(
            kind,
            "identifier"
                | "type_identifier"
                | "property_identifier"
                | "shorthand_property_identifier"
                | "shorthand_property_identifier_pattern"
                | "private_property_identifier"
        ),
        Family::Python => kind == "identifier",
        Family::Go => matches!(
            kind,
            "identifier" | "type_identifier" | "field_identifier" | "package_identifier"
        ),
    }
}

fn hash_tokens(tokens: &[String]) -> String {
    let mut hasher = Sha256::new();
    for t in tokens {
        hasher.update(t.as_bytes());
        hasher.update([0u8]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(HASH_HEX_LEN);
    for b in digest.iter().take(HASH_HEX_LEN / 2) {
        write!(hex, "{b:02x}").expect("writing to a String never fails");
    }
    hex
}

fn token_distance(a: &[String], b: &[String]) -> usize {
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut curr = vec![0usize; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

fn categorize(distance: usize) -> Magnitude {
    match distance {
        0..=3 => Magnitude::Small,
        4..=15 => Magnitude::Medium,
        _ => Magnitude::Large,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_anchor;

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

    // --- stamp parse/format -------------------------------------------------------------

    #[test]
    fn bare_hex_is_v1_prefixed_is_v2() {
        assert_eq!(parse_stamp("abc123"), Some((Recipe::V1, "abc123")));
        assert_eq!(parse_stamp("2:abc123"), Some((Recipe::V2, "abc123")));
        // Unknown version or junk → None, so the gate fails closed rather than guessing.
        assert_eq!(parse_stamp("3:abc123"), None);
        assert_eq!(parse_stamp("2:nothex"), None);
        assert_eq!(parse_stamp(""), None);
    }

    #[test]
    fn format_stamp_round_trips() {
        for (recipe, stamp) in [(Recipe::V1, "deadbeef"), (Recipe::V2, "2:deadbeef")] {
            assert_eq!(format_stamp(recipe, "deadbeef"), stamp);
            assert_eq!(
                parse_stamp(&format_stamp(recipe, "deadbeef")),
                Some((recipe, "deadbeef"))
            );
        }
    }

    #[test]
    fn current_stamp_carries_v2_prefix() {
        let src = "pub fn f(p: T) -> i64 { p.x }\n";
        let stamp = hash_anchor(src, Lang::Rust, &parse_anchor("x.rs > f").unwrap()).unwrap();
        assert!(
            stamp.starts_with("2:"),
            "current stamp is v2-prefixed: {stamp}"
        );
    }

    // --- the #140 member-access rule ----------------------------------------------------

    /// Re-pointing a member access at a *different* single-occurrence member is invisible to v1
    /// (the bug) and caught by v2 (the fix), in every family. The receiver/operands are
    /// identical, so only the member name moved.
    #[test]
    fn member_name_swap_is_v1_blind_and_v2_loud() {
        let cases = [
            (
                Lang::TypeScript,
                "x.ts > S > f",
                "export class S {\n  f(): T { return PointsTier.TIER_1; }\n}\n",
                "export class S {\n  f(): T { return PointsTier.TIER_2; }\n}\n",
            ),
            (
                Lang::TypeScript,
                "x.ts > S > f",
                "export class S {\n  f(u: U): T { return Tiers.getHighest(u); }\n}\n",
                "export class S {\n  f(u: U): T { return Tiers.getLowest(u); }\n}\n",
            ),
            (
                Lang::Go,
                "x.go > Builder > Set",
                "func (b *Builder) Set(n string) *Builder { return b.Del(n) }\n",
                "func (b *Builder) Set(n string) *Builder { return b.Keep(n) }\n",
            ),
            (
                Lang::Python,
                "x.py > color",
                "def color(self):\n    return ProbeColor.RED\n",
                "def color(self):\n    return ProbeColor.GREEN\n",
            ),
            (
                Lang::Rust,
                "x.rs > f",
                "pub fn f(p: P) -> i64 { p.first }\n",
                "pub fn f(p: P) -> i64 { p.second }\n",
            ),
        ];
        for (lang, anchor, before, after) in cases {
            assert_eq!(
                raw(before, lang, anchor, Recipe::V1),
                raw(after, lang, anchor, Recipe::V1),
                "v1 should be blind to the member swap ({lang:?})"
            );
            assert_ne!(
                raw(before, lang, anchor, Recipe::V2),
                raw(after, lang, anchor, Recipe::V2),
                "v2 must catch the member swap ({lang:?})"
            );
        }
    }

    /// A consistent rename of a *bound* name (param + locals) stays quiet under v2 — the
    /// rename-tolerance promise §6.1 makes is preserved, even though v2 stopped renaming member
    /// names. The renamed name never appears in a member-access position here.
    #[test]
    fn consistent_local_rename_is_quiet_under_v2() {
        let a = "pub fn f(nxpTier: i64) -> i64 { let t = nxpTier; t + nxpTier }\n";
        let b = "pub fn f(pointsTier: i64) -> i64 { let t = pointsTier; t + pointsTier }\n";
        assert_eq!(
            raw(a, Lang::Rust, "x.rs > f", Recipe::V2),
            raw(b, Lang::Rust, "x.rs > f", Recipe::V2),
        );
    }

    /// Renaming the *receiver* of a member access while keeping the member name is a consistent
    /// rename of a bound local — still quiet under v2 (only the receiver placeholder moves, the
    /// verbatim member name is unchanged).
    #[test]
    fn receiver_rename_keeping_member_is_quiet_under_v2() {
        let a = "pub fn f(obj: T) -> i64 { obj.compute() }\n";
        let b = "pub fn f(thing: T) -> i64 { thing.compute() }\n";
        assert_eq!(
            raw(a, Lang::Rust, "x.rs > f", Recipe::V2),
            raw(b, Lang::Rust, "x.rs > f", Recipe::V2),
        );
    }

    /// v2 still catches everything v1 did: a structural edit (operator flip) moves the hash.
    #[test]
    fn structural_edits_still_move_v2() {
        let a = "pub fn f(x: i64, y: i64) -> i64 { x + y }\n";
        let b = "pub fn f(x: i64, y: i64) -> i64 { x - y }\n";
        assert_ne!(
            raw(a, Lang::Rust, "x.rs > f", Recipe::V2),
            raw(b, Lang::Rust, "x.rs > f", Recipe::V2),
        );
    }

    /// An object-literal *key* is a `property_identifier` too, but not a member access — it stays
    /// alpha-renamed, so renaming both a key and its sole reference consistently is quiet (the
    /// structural check guards against over-firing on non-access `property_identifier`s).
    #[test]
    fn object_literal_key_is_not_treated_as_member_access() {
        let a = "export function f() { const o = { alpha: 1 }; return o; }\n";
        let b = "export function f() { const o = { beta: 1 }; return o; }\n";
        // Both v1 and v2 see a single identifier in that position → alpha-renamed → equal.
        assert_eq!(
            raw(a, Lang::TypeScript, "x.ts > f", Recipe::V2),
            raw(b, Lang::TypeScript, "x.ts > f", Recipe::V2),
        );
    }
}

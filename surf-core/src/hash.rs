//! AST-canonical hashing (§6.1) and advisory diff magnitude (§6.2).
//!
//! The design (quiet on cosmetics, loud on logic), the v1/v2 recipes, and the bound/free split
//! live in `hubs/hash.md` and `docs/reference/hash-recipes.md` — anchored to the functions below
//! so they can't silently rot. `Magnitude` is advisory triage only; it never gates (§6.2).

use crate::anchor::Anchor;
use crate::lang::{Family, Lang};
use crate::resolve::{hashable_node, parse_tree, resolve_nodes, ResolveError};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
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
    /// The bound/free split (#77): only bound identifiers alpha-renamed, free identifiers
    /// (external members, call targets, types, constants) verbatim. Stamps are prefixed `2:`.
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
    let hashable: Vec<Node> = nodes
        .into_iter()
        .map(|n| hashable_node(n, family))
        .collect();
    let bound = bound_names(&hashable, family, src, recipe);
    let mut out = Vec::new();
    let mut idents: HashMap<String, usize> = HashMap::new();
    for node in hashable {
        emit(
            node,
            src,
            family,
            opts,
            recipe,
            false,
            &bound,
            &mut idents,
            &mut out,
        );
    }
    Ok(out)
}

/// The names bound inside the span — the only identifiers v2 alpha-renames. Empty under v1
/// (every identifier alpha-renamed regardless). One set is shared across an `@overload` group.
fn bound_names(nodes: &[Node], family: Family, src: &[u8], recipe: Recipe) -> HashSet<String> {
    let mut bound = HashSet::new();
    if recipe == Recipe::V2 {
        for node in nodes {
            collect_bound(*node, family, src, &mut bound);
        }
    }
    bound
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
    let node = hashable_node(node, family);
    let bound = bound_names(std::slice::from_ref(&node), family, src, recipe);
    let mut out = Vec::new();
    let mut idents: HashMap<String, usize> = HashMap::new();
    emit(
        node,
        src,
        family,
        opts,
        recipe,
        false,
        &bound,
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
    // v1 only: a decorator name kept verbatim (#8). v2 treats it as a free identifier instead.
    decorator_name: bool,
    // v2 only: names bound in the span; an identifier is alpha-renamed iff its text is in here.
    bound: &HashSet<String>,
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
            // A member-access name is verbatim even when it collides with a bound local
            // (`x` the param vs `obj.x` the field) — that position can never be the binding.
            let verbatim = match recipe {
                Recipe::V1 => decorator_name,
                Recipe::V2 => is_member_access_name(node, family) || !bound.contains(text),
            };
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
            bound,
            idents,
            out,
        );
    }
}

/// Walk the span collecting every bound name (see `hubs/hash.md`). Fail-closed: a position not
/// positively recognized as a binding by [`bind_here`] is left free.
fn collect_bound(node: Node, family: Family, src: &[u8], out: &mut HashSet<String>) {
    bind_here(node, family, src, out);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_bound(child, family, src, out);
    }
}

/// The per-family binding-position table: declaration names bind directly, pattern positions
/// via [`harvest`].
fn bind_here(node: Node, family: Family, src: &[u8], out: &mut HashSet<String>) {
    let kind = node.kind();
    match family {
        Family::Rust => match kind {
            "function_item" | "function_signature_item" => bind_field_text(node, "name", src, out),
            "parameter" | "let_declaration" | "for_expression" | "let_condition" => {
                harvest_field(node, "pattern", src, out)
            }
            "closure_parameters" | "type_parameters" => harvest_children(node, src, out),
            _ => {}
        },
        Family::TypeScript => match kind {
            "function_declaration"
            | "generator_function_declaration"
            | "function_signature"
            | "method_definition"
            | "method_signature"
            | "abstract_method_signature" => bind_field_text(node, "name", src, out),
            "required_parameter" | "optional_parameter" => harvest_field(node, "pattern", src, out),
            "variable_declarator" => harvest_field(node, "name", src, out),
            "arrow_function" => harvest_field(node, "parameter", src, out),
            "for_in_statement" => harvest_field(node, "left", src, out),
            "catch_clause" => harvest_field(node, "parameter", src, out),
            "type_parameters" => harvest_children(node, src, out),
            _ => {}
        },
        Family::Python => match kind {
            "function_definition" => bind_field_text(node, "name", src, out),
            // A default's *value* is a free expression, so bind only the name; every other
            // parameter form (plain, typed, `*args`, `**kw`) harvests cleanly.
            "parameters" | "lambda_parameters" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    match child.kind() {
                        "default_parameter" | "typed_default_parameter" => {
                            bind_field_text(child, "name", src, out)
                        }
                        _ => harvest(child, src, out),
                    }
                }
            }
            "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
                harvest_field(node, "left", src, out)
            }
            "as_pattern_target" => harvest(node, src, out),
            _ => {}
        },
        Family::Go => match kind {
            "function_declaration"
            | "method_declaration"
            | "var_spec"
            | "const_spec"
            | "type_parameter_declaration" => bind_field_text(node, "name", src, out),
            "parameter_declaration" | "variadic_parameter_declaration" => {
                bind_field_text(node, "name", src, out)
            }
            "short_var_declaration" | "range_clause" => harvest_field(node, "left", src, out),
            _ => {}
        },
    }
}

/// Bind the text of every `field` child directly — bypassing [`harvest`]'s leaf filter so a
/// method name counts but a destructuring key does not. All matching fields, so `var a, b` binds
/// both.
fn bind_field_text(node: Node, field: &str, src: &[u8], out: &mut HashSet<String>) {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if cursor.field_name() == Some(field) {
                if let Ok(text) = cursor.node().utf8_text(src) {
                    out.insert(text.to_string());
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// [`harvest`] every `field` child (a pattern position).
fn harvest_field(node: Node, field: &str, src: &[u8], out: &mut HashSet<String>) {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if cursor.field_name() == Some(field) {
                harvest(cursor.node(), src, out);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn harvest_children(node: Node, src: &[u8], out: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        harvest(child, src, out);
    }
}

/// Collect binding-leaf identifiers from a pattern subtree, skipping path/member positions and
/// `type:` fields (external names, never local bindings). A destructure *key* is not a leaf kind,
/// so re-pointing it at a different source member stays loud.
fn harvest(node: Node, src: &[u8], out: &mut HashSet<String>) {
    match node.kind() {
        "scoped_identifier"
        | "scoped_type_identifier"
        | "attribute"
        | "member_expression"
        | "selector_expression"
        | "field_expression" => return,
        "identifier"
        | "type_identifier"
        | "shorthand_field_identifier"
        | "shorthand_property_identifier_pattern" => {
            if let Ok(text) = node.utf8_text(src) {
                out.insert(text.to_string());
            }
            return;
        }
        _ => {}
    }
    let type_field = node.child_by_field_name("type").map(|n| n.id());
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if Some(child.id()) != type_field {
            harvest(child, src, out);
        }
    }
}

/// The property/field component of a member access (see `hubs/hash.md`). Matched structurally
/// (kind + parent kind + named field) so the same kind elsewhere (an object key, a method name)
/// isn't caught.
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

    /// An object-literal *key* is free (not a binding), so under the bound/free split renaming it
    /// is loud — it changes the shape of the constructed object, the same class of change as a
    /// member rename. v1, which alpha-renames every identifier, is blind to it.
    #[test]
    fn object_literal_key_rename_is_v1_blind_and_v2_loud() {
        let a = "export function f() { const o = { alpha: 1 }; return o; }\n";
        let b = "export function f() { const o = { beta: 1 }; return o; }\n";
        assert_eq!(
            raw(a, Lang::TypeScript, "x.ts > f", Recipe::V1),
            raw(b, Lang::TypeScript, "x.ts > f", Recipe::V1),
        );
        assert_ne!(
            raw(a, Lang::TypeScript, "x.ts > f", Recipe::V2),
            raw(b, Lang::TypeScript, "x.ts > f", Recipe::V2),
        );
    }

    /// Swapping a bare single-occurrence free call target (no receiver, so not a member access) is
    /// invisible to v1 but loud under the full split.
    #[test]
    fn bare_free_call_target_swap_is_v1_blind_and_v2_loud() {
        let a = "pub fn f(x: i64) -> i64 { helper(x) }\n";
        let b = "pub fn f(x: i64) -> i64 { other(x) }\n";
        assert_eq!(
            raw(a, Lang::Rust, "x.rs > f", Recipe::V1),
            raw(b, Lang::Rust, "x.rs > f", Recipe::V1),
            "v1 alpha-renames the call target → blind",
        );
        assert_ne!(
            raw(a, Lang::Rust, "x.rs > f", Recipe::V2),
            raw(b, Lang::Rust, "x.rs > f", Recipe::V2),
            "v2 emits the free call target verbatim → loud",
        );
    }

    /// A free type reference is verbatim under v2: changing a parameter's type is a contract
    /// change, caught even when the type name occurs once. A *generic* parameter, declared in the
    /// span, stays bound — renaming it consistently is quiet.
    #[test]
    fn free_type_is_loud_generic_param_is_quiet() {
        let a = "pub fn f(x: Foo) -> i64 { 0 }\n";
        let b = "pub fn f(x: Bar) -> i64 { 0 }\n";
        assert_ne!(
            raw(a, Lang::Rust, "x.rs > f", Recipe::V2),
            raw(b, Lang::Rust, "x.rs > f", Recipe::V2),
            "swapping an external type is loud",
        );
        let g1 = "pub fn f<T>(x: T) -> T { x }\n";
        let g2 = "pub fn f<U>(x: U) -> U { x }\n";
        assert_eq!(
            raw(g1, Lang::Rust, "x.rs > f", Recipe::V2),
            raw(g2, Lang::Rust, "x.rs > f", Recipe::V2),
            "renaming a generic parameter consistently is quiet",
        );
    }

    /// Destructuring binders are bound (renaming them is quiet), but the *source key* they read
    /// from is free (re-pointing at a different member is loud) — across the pattern forms each
    /// family offers.
    #[test]
    fn destructuring_binder_quiet_source_key_loud() {
        // TS object destructuring: rename the binder `b` → quiet; change the source key → loud.
        let bind_a = "export function f(o: O) { const { k: b } = o; return b; }\n";
        let bind_b = "export function f(o: O) { const { k: c } = o; return c; }\n";
        assert_eq!(
            raw(bind_a, Lang::TypeScript, "x.ts > f", Recipe::V2),
            raw(bind_b, Lang::TypeScript, "x.ts > f", Recipe::V2),
        );
        let key_a = "export function f(o: O) { const { k: b } = o; return b; }\n";
        let key_b = "export function f(o: O) { const { j: b } = o; return b; }\n";
        assert_ne!(
            raw(key_a, Lang::TypeScript, "x.ts > f", Recipe::V2),
            raw(key_b, Lang::TypeScript, "x.ts > f", Recipe::V2),
        );
    }

    /// A Python decorator name is a free identifier under v2, so `@cache` → `@lru_cache` is loud
    /// without the dedicated decorator special case v1 needed (#8 is subsumed by the split).
    #[test]
    fn decorator_name_swap_is_loud_under_v2_without_special_case() {
        let a = "@cache\ndef f(x):\n    return x\n";
        let b = "@lru_cache\ndef f(x):\n    return x\n";
        assert_ne!(
            raw(a, Lang::Python, "x.py > f", Recipe::V2),
            raw(b, Lang::Python, "x.py > f", Recipe::V2),
        );
    }
}

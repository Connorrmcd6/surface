//! Deterministic rename detection (§6.4). When an anchor no longer resolves, we ask: is
//! the documented code actually gone, or just renamed? Because the canonical hash
//! alpha-renames identifiers, a symbol that was *renamed but otherwise unchanged* still
//! hashes to the claim's stored hash. So a stored-hash match against any current symbol is
//! strong, network-free evidence of a rename — no git, no similarity threshold.
//!
//! This covers symbol renames within a file (the common case). A *file* rename makes the
//! anchor's path unreadable; that surfaces as a broken reference at the `lint` layer.

use crate::hash::{hash_node, parse_stamp, HashOpts};
use crate::lang::Lang;
use crate::resolve::{collect_all_defs, parse_tree, ResolveError};

/// If some current symbol's canonical hash equals `stored_hash`, return its name — the
/// symbol the anchor was probably renamed to. `opts` must match the mode the stored hash was
/// computed in (e.g. a claim with `ignore_literals`), or a renamed symbol won't match. The
/// stored stamp's recipe (v1/v2) is honoured, so a renamed-but-unchanged symbol relocates
/// whether or not its stamp has been upgraded yet.
pub fn find_renamed(
    source: &str,
    lang: Lang,
    stored_hash: &str,
    opts: HashOpts,
) -> Result<Option<String>, ResolveError> {
    let Some((recipe, stored_hex)) = parse_stamp(stored_hash) else {
        return Ok(None);
    };
    let tree = parse_tree(source, lang).ok_or(ResolveError::Parse)?;
    let src = source.as_bytes();
    let family = lang.family();

    let mut defs = Vec::new();
    collect_all_defs(tree.root_node(), src, family, &mut defs);
    for (name, node) in defs {
        if hash_node(node, src, family, opts, recipe) == stored_hex {
            return Ok(Some(name));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hash_anchor, parse_anchor};

    #[test]
    fn detects_symbol_rename_by_stored_hash() {
        let old = "fn rotate(token: &str) -> String { token.to_string() }";
        let new = "fn rotate_token(token: &str) -> String { token.to_string() }";
        let stored = hash_anchor(old, Lang::Rust, &parse_anchor("f.rs > rotate").unwrap()).unwrap();

        assert_eq!(
            find_renamed(new, Lang::Rust, &stored, HashOpts::default()).unwrap(),
            Some("rotate_token".to_string())
        );
    }

    #[test]
    fn no_match_when_body_changed() {
        let old = "fn rotate(token: &str) -> String { token.to_string() }";
        let new = "fn rotate_token(token: &str) -> String { token.trim().to_string() }";
        let stored = hash_anchor(old, Lang::Rust, &parse_anchor("f.rs > rotate").unwrap()).unwrap();

        assert_eq!(
            find_renamed(new, Lang::Rust, &stored, HashOpts::default()).unwrap(),
            None
        );
    }
}

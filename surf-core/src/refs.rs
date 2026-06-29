//! Parser for the `refs:` hub-composition grammar (§9.3, #4): `path` (a hub, relative to the
//! referencing hub) optionally followed by `> A > B@N` to address one claim *within* that hub.
//! This is pure parsing — the path is not resolved and the claim is not matched here; that is
//! `lint`'s job over the workspace's loaded hubs.

use crate::anchor::{parse_segment, AnchorParseError, Segment};

/// A parsed `refs:` entry. `segments` is empty for a whole-hub reference; non-empty addresses a
/// specific claim (matched by anchor suffix), reusing the `at:` segment grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ref {
    pub path: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefParseError {
    Empty,
    EmptyPath,
    Segment(AnchorParseError),
}

impl std::fmt::Display for RefParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefParseError::Empty => write!(f, "ref is empty"),
            RefParseError::EmptyPath => write!(f, "ref has no hub path"),
            RefParseError::Segment(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RefParseError {}

pub fn parse_ref(input: &str) -> Result<Ref, RefParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(RefParseError::Empty);
    }

    let mut parts = trimmed.split('>');
    let path = parts
        .next()
        .expect("split yields at least one part")
        .trim()
        .to_string();
    if path.is_empty() {
        return Err(RefParseError::EmptyPath);
    }

    let mut segments = Vec::new();
    for raw in parts {
        segments.push(parse_segment(raw).map_err(RefParseError::Segment)?);
    }

    Ok(Ref { path, segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(name: &str, index: Option<usize>) -> Segment {
        Segment {
            name: name.to_string(),
            index,
        }
    }

    #[test]
    fn whole_hub_ref_has_no_segments() {
        let r = parse_ref("./cli-verify.md").unwrap();
        assert_eq!(r.path, "./cli-verify.md");
        assert!(r.segments.is_empty());
    }

    #[test]
    fn claim_ref_carries_segments() {
        let r = parse_ref("./resolve.md > resolve_nodes").unwrap();
        assert_eq!(r.path, "./resolve.md");
        assert_eq!(r.segments, vec![seg("resolve_nodes", None)]);
    }

    #[test]
    fn reuses_anchor_segment_grammar() {
        let r = parse_ref("../hubs/m.md > Builder > Set @2").unwrap();
        assert_eq!(r.path, "../hubs/m.md");
        assert_eq!(r.segments, vec![seg("Builder", None), seg("Set", Some(2))]);
    }

    #[test]
    fn errors() {
        assert_eq!(parse_ref("   "), Err(RefParseError::Empty));
        assert_eq!(parse_ref("> resolve_nodes"), Err(RefParseError::EmptyPath));
        assert_eq!(
            parse_ref("./a.md > "),
            Err(RefParseError::Segment(AnchorParseError::EmptySegment))
        );
        assert_eq!(
            parse_ref("./a.md > x @0"),
            Err(RefParseError::Segment(AnchorParseError::BadIndex(
                "0".to_string()
            )))
        );
    }
}

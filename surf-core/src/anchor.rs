//! Parser for the `at:` anchor grammar (§6.3): `path > A > B > C`, with an optional
//! positional `@N` suffix on any segment for genuine name collisions.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    pub file: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub name: String,
    /// 1-based positional selector from `@N`; `None` means "must be unique".
    pub index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorParseError {
    Empty,
    EmptyFile,
    MissingSymbol,
    EmptySegment,
    BadIndex(String),
}

impl std::fmt::Display for AnchorParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnchorParseError::Empty => write!(f, "anchor is empty"),
            AnchorParseError::EmptyFile => write!(f, "anchor has no file path"),
            AnchorParseError::MissingSymbol => {
                write!(
                    f,
                    "anchor names a file but no symbol (expected `path > symbol`)"
                )
            }
            AnchorParseError::EmptySegment => write!(f, "anchor has an empty `>` segment"),
            AnchorParseError::BadIndex(s) => write!(
                f,
                "invalid positional index `@{s}` (expected a number >= 1)"
            ),
        }
    }
}

impl std::error::Error for AnchorParseError {}

pub fn parse_anchor(input: &str) -> Result<Anchor, AnchorParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AnchorParseError::Empty);
    }

    let mut parts = trimmed.split('>');
    let file = parts
        .next()
        .expect("split yields at least one part")
        .trim()
        .to_string();
    if file.is_empty() {
        return Err(AnchorParseError::EmptyFile);
    }

    let mut segments = Vec::new();
    for raw in parts {
        segments.push(parse_segment(raw)?);
    }

    if segments.is_empty() {
        return Err(AnchorParseError::MissingSymbol);
    }

    Ok(Anchor { file, segments })
}

/// Parse one `>`-delimited segment (`name` or `name@N`). Shared by `at:` anchors and `refs:`
/// (#4), so the positional-collision grammar stays identical across both.
pub(crate) fn parse_segment(raw: &str) -> Result<Segment, AnchorParseError> {
    let seg = raw.trim();
    if seg.is_empty() {
        return Err(AnchorParseError::EmptySegment);
    }
    let (name, index) = match seg.split_once('@') {
        Some((name, idx)) => {
            let idx = idx.trim();
            let parsed = idx
                .parse::<usize>()
                .map_err(|_| AnchorParseError::BadIndex(idx.to_string()))?;
            if parsed == 0 {
                return Err(AnchorParseError::BadIndex(idx.to_string()));
            }
            (name.trim().to_string(), Some(parsed))
        }
        None => (seg.to_string(), None),
    };
    if name.is_empty() {
        return Err(AnchorParseError::EmptySegment);
    }
    Ok(Segment { name, index })
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
    fn qualified_path() {
        let a = parse_anchor("src/auth/refresh.ts > TokenService > rotate").unwrap();
        assert_eq!(a.file, "src/auth/refresh.ts");
        assert_eq!(
            a.segments,
            vec![seg("TokenService", None), seg("rotate", None)]
        );
    }

    #[test]
    fn positional_suffix() {
        let a = parse_anchor("src/auth/refresh.ts > rotate @2").unwrap();
        assert_eq!(a.segments, vec![seg("rotate", Some(2))]);
    }

    #[test]
    fn positional_without_space() {
        let a = parse_anchor("a.rs > rotate@3").unwrap();
        assert_eq!(a.segments, vec![seg("rotate", Some(3))]);
    }

    #[test]
    fn errors() {
        assert_eq!(parse_anchor("   "), Err(AnchorParseError::Empty));
        assert_eq!(
            parse_anchor("src/auth/refresh.ts"),
            Err(AnchorParseError::MissingSymbol)
        );
        assert_eq!(parse_anchor("a.ts > "), Err(AnchorParseError::EmptySegment));
        assert_eq!(
            parse_anchor("a.ts > rotate @0"),
            Err(AnchorParseError::BadIndex("0".to_string()))
        );
        assert_eq!(
            parse_anchor("a.ts > rotate @x"),
            Err(AnchorParseError::BadIndex("x".to_string()))
        );
        assert_eq!(parse_anchor("> rotate"), Err(AnchorParseError::EmptyFile));
    }

    // --- Path edge cases (#45) ----------------------------------------------
    //
    // The grammar (§6.3) mandates forward slashes in the file path. These pin the *current*
    // behaviour: backslash / mixed-separator paths are accepted verbatim and the backslash is
    // retained in `Anchor::file`, so the file later fails to match on disk — a silent non-match
    // rather than a clear parse error.

    #[test]
    fn backslash_path_is_currently_accepted_verbatim() {
        let a = parse_anchor("auth\\service.ts > Foo").unwrap();
        assert_eq!(a.file, "auth\\service.ts");
        assert_eq!(a.segments, vec![seg("Foo", None)]);
    }

    #[test]
    fn mixed_separator_path_is_currently_accepted_verbatim() {
        let a = parse_anchor("src\\auth.py > rotate").unwrap();
        assert_eq!(a.file, "src\\auth.py");
        assert_eq!(a.segments, vec![seg("rotate", None)]);
    }

    // Target behaviour for the follow-up (#144): a backslash in the file path is a clear parse
    // error (a dedicated variant, e.g. `BadSeparator`), not a silently-retained separator. Flip
    // these to assert on the new variant and drop the `#[ignore]` when the fix lands.
    #[test]
    #[ignore = "#144: backslash paths should error with a dedicated variant, not parse Ok"]
    fn backslash_path_should_be_a_parse_error() {
        assert!(parse_anchor("auth\\service.ts > Foo").is_err());
    }

    #[test]
    #[ignore = "#144: mixed-separator paths should error with a dedicated variant, not parse Ok"]
    fn mixed_separator_path_should_be_a_parse_error() {
        assert!(parse_anchor("src\\auth.py > rotate").is_err());
    }
}

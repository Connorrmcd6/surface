//! The hub document: `---`-fenced YAML frontmatter + a markdown prose body (§9.1.1).
//! This module is pure: it parses a string into a `Hub`. It does no I/O and resolves no
//! anchors — that is `lint`/`check`'s job over the data this produces.

use serde::{Deserialize, Serialize};

// `Frontmatter`/`Hub` can't derive `Eq`: `extra` holds `serde_yaml::Value`s (floats aren't `Eq`).
#[derive(Debug, Clone, PartialEq)]
pub struct Hub {
    pub frontmatter: Frontmatter,
    pub body: String,
}

/// A hub's frontmatter is a **superset of an OKF concept**: the OKF-recognized fields (`type`,
/// `title`, `description`, `tags`, `timestamp`, …) sit alongside Surface's own governance fields
/// (`anchors`, `refs`, `covers`). Per the OKF contract, unknown keys are **preserved, not
/// rejected** — `extra` captures every field Surface doesn't name (OKF's `description`/`resource`,
/// a doc system's `author`/`created`/`pinned`, future OKF keys) so they round-trip verbatim.
/// `deny_unknown_fields` is therefore off here; a typo'd key is surfaced by a `surf lint` warning
/// (see `lint`) instead of a hard parse error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    /// OKF's one required field. Defaulted to `concept` so pre-OKF hubs (which never wrote it) still
    /// parse; the default is not serialized, so an untouched hub stays byte-identical.
    #[serde(
        rename = "type",
        default = "default_type",
        skip_serializing_if = "is_default_type"
    )]
    pub concept_type: String,
    /// OKF display name; consumers may fall back to the filename when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Surface's onboarding one-liner. Optional now (an OKF concept may carry only `description`).
    /// Kept distinct from OKF `description` (which lives in `extra`) so existing hubs are untouched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// OKF cross-cutting tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// OKF last-*modified* timestamp (ISO 8601). Distinct from per-claim `verified_at` (last
    /// *attested*): Surface reads this but does not manage it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub anchors: Vec<Claim>,
    /// Hub composition (§9.3, #4): typed staleness edges, distinct from OKF's untyped body links.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<String>,
    /// Advisory coverage scope: repo-root-relative globs (same dialect as `config.hubs`). Parsed,
    /// stored, and lint-validated, but **the verdict never reads it** (§5/§8, #5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covers: Vec<String>,
    /// Every frontmatter key Surface does not name, preserved verbatim — the OKF "consumers MUST
    /// preserve unknown keys" rule made structural.
    #[serde(flatten, default)]
    pub extra: serde_yaml::Mapping,
}

// `Claim` stays strict: the `anchors:` items are Surface's own structured data (OKF knows nothing
// of them), so `deny_unknown_fields` keeps catching per-anchor typos (`hahs:`) as a hard error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    /// Stable identity, independent of prose/anchor text — the substrate of claim timelines and
    /// attestation history. Written once by `surf verify` when absent, then **never regenerated**:
    /// a prose or anchor edit keeps the same `id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub claim: String,
    pub at: At,
    /// The stored AST-canonical hash. `None` until `surf verify` first stamps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// The freshness OKF omits: when this claim was last *attested* against its code (ISO 8601) and
    /// at which commit. Written by `surf verify` **only when the hash actually changes**, so a no-op
    /// re-verify stays byte-identical. The *who* is intentionally not stored (git blame on the hub
    /// records it) — keeping author emails out of tracked files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified_commit: Option<String>,
    /// Opt-in: exclude string-literal *content* from this claim's hash, so a copy edit inside
    /// the anchored span doesn't re-open the gate (§6.1). The stored hash is computed in this
    /// mode, so it must travel with the claim. Defaults to `false`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignore_literals: bool,
}

fn default_type() -> String {
    "concept".to_string()
}

fn is_default_type(t: &str) -> bool {
    t == "concept"
}

/// OKF reserves two filenames that are structure, not concepts: `index.md` (a directory listing for
/// progressive disclosure) and `log.md` (a change history). They hold no claims, so Surface never
/// governs them and never blocks the gate when they lack frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocKind {
    Concept,
    Index,
    Log,
}

/// Classify a workspace-relative path by its basename (OKF reserved filenames).
pub fn doc_kind(rel: &str) -> DocKind {
    match rel.rsplit(['/', '\\']).next().unwrap_or(rel) {
        "index.md" => DocKind::Index,
        "log.md" => DocKind::Log,
        _ => DocKind::Concept,
    }
}

/// One anchor (`at:`) is either a single span or a list; the claim is stale if *any*
/// listed span changes (§6.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum At {
    One(String),
    Many(Vec<String>),
}

impl At {
    pub fn sites(&self) -> &[String] {
        match self {
            At::One(s) => std::slice::from_ref(s),
            At::Many(v) => v,
        }
    }
}

#[derive(Debug)]
pub enum HubError {
    MissingFrontmatter,
    UnterminatedFrontmatter,
    Yaml(String),
}

impl std::fmt::Display for HubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HubError::MissingFrontmatter => {
                write!(
                    f,
                    "hub must begin with a `---`-fenced YAML frontmatter block"
                )
            }
            HubError::UnterminatedFrontmatter => {
                write!(f, "frontmatter block is not closed with `---`")
            }
            HubError::Yaml(e) => write!(f, "invalid frontmatter: {e}"),
        }
    }
}

impl std::error::Error for HubError {}

pub fn parse_hub(content: &str) -> Result<Hub, HubError> {
    let mut lines = content.lines();
    match lines.next() {
        Some(first) if first.trim_end() == "---" => {}
        _ => return Err(HubError::MissingFrontmatter),
    }

    let mut yaml = String::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed {
        return Err(HubError::UnterminatedFrontmatter);
    }

    let frontmatter: Frontmatter =
        serde_yaml::from_str(&yaml).map_err(|e| HubError::Yaml(e.to_string()))?;
    let body = lines.collect::<Vec<_>>().join("\n");

    Ok(Hub { frontmatter, body })
}

// --- Minimal-diff frontmatter editing (for `surf verify`) ------------------------------
//
// `verify` writes back into a hub that a human will review, so we edit surgically rather
// than re-serializing the whole frontmatter (which would reorder keys and reflow folded
// scalars). These operate on the full hub text, locate the Nth `anchors:` item, and touch
// exactly one line. `anchor_index` matches the parse order of `Frontmatter::anchors`.

/// Set (or insert) an arbitrary `key: value` line within the anchor item at `anchor_index`,
/// touching exactly that one line so a human review sees a minimal diff. Returns the new file
/// text, or `None` if the frontmatter structure or index can't be located. `set_anchor_hash` and
/// `surf verify`'s provenance stamping (`id`, `verified_*`) are thin wrappers over this.
pub fn set_anchor_field(
    file_text: &str,
    anchor_index: usize,
    key: &str,
    value: &str,
) -> Option<String> {
    edit_anchor(file_text, anchor_index, |lines, item| {
        set_key(lines, item, key, value)
    })
}

/// Set (or insert) the `hash:` of the anchor at `anchor_index`.
pub fn set_anchor_hash(file_text: &str, anchor_index: usize, new_hash: &str) -> Option<String> {
    set_anchor_field(file_text, anchor_index, "hash", new_hash)
}

/// Rewrite a scalar `at:` of the anchor at `anchor_index` (used by `--follow`). Returns
/// `None` if the structure can't be located or the `at:` is a list (not auto-followable).
pub fn set_anchor_at(file_text: &str, anchor_index: usize, new_at: &str) -> Option<String> {
    edit_anchor(file_text, anchor_index, |lines, item| {
        let key_indent = item.key_indent;
        let line = (item.start..item.end).find(|&i| {
            leading_spaces(&lines[i]) == key_indent && lines[i].trim_start().starts_with("at:")
        })?;
        let value = lines[line].trim_start().strip_prefix("at:")?.trim();
        if value.is_empty() {
            return None; // list form — not auto-followable
        }
        lines[line] = format!("{}at: {new_at}", " ".repeat(key_indent));
        Some(())
    })
}

struct Item {
    start: usize,
    end: usize,
    key_indent: usize,
}

fn edit_anchor(
    file_text: &str,
    anchor_index: usize,
    edit: impl FnOnce(&mut Vec<String>, &Item) -> Option<()>,
) -> Option<String> {
    let mut lines: Vec<String> = file_text.split('\n').map(str::to_string).collect();
    let (ystart, yend) = yaml_range(&lines)?;
    let items = anchor_items(&lines, ystart, yend);
    let item = items.get(anchor_index)?;
    edit(&mut lines, item)?;
    Some(lines.join("\n"))
}

fn set_key(lines: &mut Vec<String>, item: &Item, key: &str, value: &str) -> Option<()> {
    let key_indent = item.key_indent;
    let new_line = format!("{}{key}: {value}", " ".repeat(key_indent));

    if let Some(i) = (item.start..item.end).find(|&i| {
        leading_spaces(&lines[i]) == key_indent
            && lines[i].trim_start().starts_with(&format!("{key}:"))
    }) {
        lines[i] = new_line;
    } else {
        let insert_at = (item.start..item.end)
            .rev()
            .find(|&i| !lines[i].trim().is_empty())
            .map(|i| i + 1)
            .unwrap_or(item.end);
        lines.insert(insert_at, new_line);
    }
    Some(())
}

fn leading_spaces(s: &str) -> usize {
    s.chars().take_while(|c| *c == ' ').count()
}

fn yaml_range(lines: &[String]) -> Option<(usize, usize)> {
    if lines.first()?.trim_end() != "---" {
        return None;
    }
    let end = (1..lines.len()).find(|&i| lines[i].trim_end() == "---")?;
    Some((1, end))
}

fn anchor_items(lines: &[String], ystart: usize, yend: usize) -> Vec<Item> {
    let Some(anchors_idx) = (ystart..yend).find(|&i| lines[i].trim_start().starts_with("anchors:"))
    else {
        return Vec::new();
    };
    let anchors_indent = leading_spaces(&lines[anchors_idx]);

    let mut starts: Vec<(usize, usize)> = Vec::new(); // (start_line, dash_indent)
    let mut item_indent: Option<usize> = None;
    let mut seq_end = yend;
    for (i, line) in lines.iter().enumerate().take(yend).skip(anchors_idx + 1) {
        if line.trim().is_empty() {
            continue;
        }
        let ind = leading_spaces(line);
        if ind <= anchors_indent {
            seq_end = i;
            break;
        }
        let trimmed = line.trim_start();
        let is_dash = trimmed == "-" || trimmed.starts_with("- ");
        if is_dash && item_indent.map(|x| x == ind).unwrap_or(true) {
            item_indent.get_or_insert(ind);
            starts.push((i, ind));
        }
    }

    starts
        .iter()
        .enumerate()
        .map(|(n, &(start, dash_indent))| {
            let end = starts.get(n + 1).map(|&(s, _)| s).unwrap_or(seq_end);
            Item {
                start,
                end,
                key_indent: dash_indent + 2,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "---\nsummary: how auth refresh works\nanchors:\n  - claim: refresh rotation is single-use\n    at: src/auth/refresh.ts > rotateRefreshToken\n    hash: 9b1c33a\n  - claim: a refresh token is accepted at most once\n    at:\n      - src/auth/refresh.ts > rotateRefreshToken\n      - src/auth/refresh.ts > validateRefresh\nrefs: []\n---\n# Auth\n\nProse body here.\n";

    #[test]
    fn parses_scalar_and_list_at() {
        let hub = parse_hub(VALID).unwrap();
        assert_eq!(
            hub.frontmatter.summary.as_deref(),
            Some("how auth refresh works")
        );
        assert_eq!(hub.frontmatter.anchors.len(), 2);

        let first = &hub.frontmatter.anchors[0];
        assert_eq!(
            first.at.sites(),
            &["src/auth/refresh.ts > rotateRefreshToken".to_string()]
        );
        assert_eq!(first.hash.as_deref(), Some("9b1c33a"));

        let second = &hub.frontmatter.anchors[1];
        assert_eq!(second.at.sites().len(), 2);
        assert_eq!(second.hash, None);

        assert!(hub.body.contains("Prose body here."));
    }

    #[test]
    fn round_trips_frontmatter() {
        let hub = parse_hub(VALID).unwrap();
        let yaml = serde_yaml::to_string(&hub.frontmatter).unwrap();
        let reparsed: Frontmatter = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(hub.frontmatter, reparsed);
    }

    #[test]
    fn missing_frontmatter_is_typed_error() {
        let err = parse_hub("# Just markdown, no frontmatter\n").unwrap_err();
        assert!(matches!(err, HubError::MissingFrontmatter));
    }

    #[test]
    fn unterminated_frontmatter_is_typed_error() {
        let err = parse_hub("---\nsummary: x\nstill inside\n").unwrap_err();
        assert!(matches!(err, HubError::UnterminatedFrontmatter));
    }

    #[test]
    fn okf_concept_without_summary_parses_as_pass_through() {
        // OKF requires only `type` (defaulted). A concept with just OKF fields and no `anchors`
        // parses fine — it is valid and simply ungoverned (nothing to hash).
        let hub =
            parse_hub("---\ntype: BigQuery Table\ndescription: one row per order\n---\nbody\n")
                .unwrap();
        assert_eq!(hub.frontmatter.concept_type, "BigQuery Table");
        assert!(hub.frontmatter.summary.is_none());
        assert!(hub.frontmatter.anchors.is_empty());
        // `description` is an OKF field Surface doesn't name — captured in `extra`, not dropped.
        assert!(hub.frontmatter.extra.contains_key("description"));
    }

    #[test]
    fn default_type_is_concept_and_not_serialized() {
        // A hub that omits `type` parses with the `concept` default, and serializing it back does
        // not introduce a `type:` key — existing hubs stay byte-unaffected.
        let hub = parse_hub("---\nsummary: x\n---\nbody\n").unwrap();
        assert_eq!(hub.frontmatter.concept_type, "concept");
        let yaml = serde_yaml::to_string(&hub.frontmatter).unwrap();
        assert!(
            !yaml.contains("type"),
            "default type should not serialize: {yaml}"
        );
    }

    #[test]
    fn unknown_frontmatter_keys_are_preserved_on_round_trip() {
        // OKF: consumers MUST preserve unknown keys. A doc authored in a doc system (Nansidian's
        // author/created/pinned) round-trips with zero key loss.
        let src = "---\ntype: Runbook\ntitle: Deploy\nauthor: rachel\ncreated: 2026-06-01\npinned: true\n---\nbody\n";
        let hub = parse_hub(src).unwrap();
        assert_eq!(hub.frontmatter.title.as_deref(), Some("Deploy"));
        for k in ["author", "created", "pinned"] {
            assert!(hub.frontmatter.extra.contains_key(k), "lost `{k}`");
        }
        let yaml = serde_yaml::to_string(&hub.frontmatter).unwrap();
        let reparsed: Frontmatter = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(hub.frontmatter, reparsed);
    }

    #[test]
    fn unknown_claim_field_is_still_rejected() {
        // Anchor items are Surface's own data, not OKF's — a per-anchor typo still fails closed.
        let err = parse_hub(
            "---\nsummary: s\nanchors:\n  - claim: c\n    at: a.rs > foo\n    hahs: x\n---\n",
        )
        .unwrap_err();
        assert!(
            matches!(err, HubError::Yaml(_)),
            "expected Yaml error, got {err:?}"
        );
    }

    #[test]
    fn doc_kind_classifies_reserved_filenames() {
        assert_eq!(doc_kind("sales/index.md"), DocKind::Index);
        assert_eq!(doc_kind("log.md"), DocKind::Log);
        assert_eq!(doc_kind("tables/orders.md"), DocKind::Concept);
    }

    #[test]
    fn covers_field_is_accepted() {
        // `covers` is forward-declared per §9.1: parsed and stored, but inert in the verdict
        // (the louder coverage pass that consumes it is deferred to #5).
        let hub =
            parse_hub("---\nsummary: x\ncovers:\n  - src/**\n  - lib/foo.rs\n---\nbody\n").unwrap();
        assert_eq!(
            hub.frontmatter.covers,
            vec!["src/**".to_string(), "lib/foo.rs".to_string()]
        );
    }

    #[test]
    fn covers_defaults_to_empty() {
        // A hub that omits `covers` parses with an empty list, and serializing it back does not
        // introduce an empty `covers:` key — existing hubs are byte-unaffected.
        let hub = parse_hub("---\nsummary: x\n---\nbody\n").unwrap();
        assert!(hub.frontmatter.covers.is_empty());
        let yaml = serde_yaml::to_string(&hub.frontmatter).unwrap();
        assert!(
            !yaml.contains("covers"),
            "empty covers should not serialize: {yaml}"
        );
    }

    #[test]
    fn covers_round_trips() {
        let hub = parse_hub("---\nsummary: x\ncovers:\n  - src/**\n---\nbody\n").unwrap();
        let yaml = serde_yaml::to_string(&hub.frontmatter).unwrap();
        let reparsed: Frontmatter = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(hub.frontmatter, reparsed);
    }

    #[test]
    fn refs_parse_without_resolution() {
        let hub = parse_hub("---\nsummary: x\nrefs:\n  - other-hub\n---\nbody\n").unwrap();
        assert_eq!(hub.frontmatter.refs, vec!["other-hub".to_string()]);
    }

    const HUB: &str = "---\nsummary: s\nanchors:\n  - claim: first\n    at: a.rs > foo\n    hash: oldhash\n  - claim: second\n    at: a.rs > bar\n---\n# Body\n";

    #[test]
    fn set_hash_replaces_existing_in_place() {
        let out = set_anchor_hash(HUB, 0, "newhash").unwrap();
        assert!(out.contains("hash: newhash"));
        assert!(!out.contains("hash: oldhash"));
        // Only the one line changed.
        let before: Vec<_> = HUB.lines().collect();
        let after: Vec<_> = out.lines().collect();
        assert_eq!(before.len(), after.len());
        let diffs = before.iter().zip(&after).filter(|(a, b)| a != b).count();
        assert_eq!(diffs, 1);
    }

    #[test]
    fn set_hash_inserts_when_absent() {
        let out = set_anchor_hash(HUB, 1, "h2").unwrap();
        let reparsed = parse_hub(&out).unwrap();
        assert_eq!(reparsed.frontmatter.anchors[1].hash.as_deref(), Some("h2"));
        assert_eq!(
            reparsed.frontmatter.anchors[0].hash.as_deref(),
            Some("oldhash")
        );
    }

    #[test]
    fn set_hash_to_same_value_is_byte_identical() {
        assert_eq!(set_anchor_hash(HUB, 0, "oldhash").unwrap(), HUB);
    }

    #[test]
    fn set_anchor_field_inserts_provenance_and_reparses() {
        // The generalized editor stamps `id`/`verified_*` into an anchor item and they parse back.
        let out = set_anchor_field(HUB, 0, "id", "c_01hxyz").unwrap();
        let out = set_anchor_field(&out, 0, "verified_at", "2026-07-01T00:00:00Z").unwrap();
        let hub = parse_hub(&out).unwrap();
        assert_eq!(hub.frontmatter.anchors[0].id.as_deref(), Some("c_01hxyz"));
        assert_eq!(
            hub.frontmatter.anchors[0].verified_at.as_deref(),
            Some("2026-07-01T00:00:00Z")
        );
        // The other anchor is untouched.
        assert_eq!(hub.frontmatter.anchors[1].id, None);
    }

    #[test]
    fn versioned_stamp_round_trips_as_a_string() {
        // A `2:`-prefixed v2 stamp is a plain YAML scalar (colon not followed by a space), so it
        // parses back as the exact string and survives a serialize round-trip (#140).
        let out = set_anchor_hash(HUB, 0, "2:abc123def456").unwrap();
        let hub = parse_hub(&out).unwrap();
        assert_eq!(
            hub.frontmatter.anchors[0].hash.as_deref(),
            Some("2:abc123def456")
        );
        let yaml = serde_yaml::to_string(&hub.frontmatter).unwrap();
        let reparsed: Frontmatter = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(reparsed.anchors[0].hash.as_deref(), Some("2:abc123def456"));
    }

    #[test]
    fn follow_rewrites_scalar_at() {
        let out = set_anchor_at(HUB, 0, "a.rs > foo_renamed").unwrap();
        let reparsed = parse_hub(&out).unwrap();
        assert_eq!(
            reparsed.frontmatter.anchors[0].at.sites(),
            &["a.rs > foo_renamed".to_string()]
        );
    }

    #[test]
    fn follow_refuses_list_at() {
        let list_hub = "---\nsummary: s\nanchors:\n  - claim: c\n    at:\n      - a.rs > foo\n      - a.rs > bar\n---\n";
        assert_eq!(set_anchor_at(list_hub, 0, "x"), None);
    }
}

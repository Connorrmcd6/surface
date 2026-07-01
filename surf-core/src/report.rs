//! The `surf check --format json` contract (§5). This is the seam every optional layer
//! (reviewer plugin, etc.) attaches to; the deterministic core never depends on it.
//!
//! The verdict is carried by `old_hash` vs `new_hash`. `old_code` and `magnitude` are
//! best-effort enrichment recovered from the previous source via git — advisory only,
//! `None` when unavailable, and never part of the pass/fail decision.
//!
//! ## Stability
//!
//! The JSON output is a `CheckReport` envelope carrying `version` ([`REPORT_VERSION`]).
//! Within a major version the contract is **additive-only**: new optional fields may be added,
//! but existing fields are never removed, renamed, or repurposed. A breaking change bumps
//! `version`. Consumers should tolerate unknown fields and check `version` for compatibility.

use crate::hash::Magnitude;
use serde::Serialize;

/// Contract version of the `surf check --format json` envelope. Bumped only on a breaking change
/// (field removal/rename/semantic change); additive changes keep the same version.
pub const REPORT_VERSION: u32 = 1;

/// The top-level `surf check --format json` payload: a version tag plus the divergences. The
/// version lets downstream layers detect an incompatible contract instead of silently
/// misreading a changed shape.
#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    pub version: u32,
    pub divergences: Vec<Divergence>,
}

impl CheckReport {
    pub fn new(divergences: Vec<Divergence>) -> Self {
        CheckReport {
            version: REPORT_VERSION,
            divergences,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DivergenceKind {
    /// Stored hash exists and no longer matches the current span.
    Changed,
    /// The claim has never been verified — no stored hash to compare against.
    Unverified,
    /// The anchor no longer resolves to exactly one symbol (run `surf lint`).
    Unresolvable,
    /// A hub this one `refs` has an open divergence — composition propagated (§9.3, #4). One-hop:
    /// only a *direct* ref to a stale hub fires this, never a transitive chain.
    ReferencedStale,
}

#[derive(Debug, Clone, Serialize)]
pub struct Divergence {
    pub hub: String,
    pub claim: String,
    pub at: String,
    pub kind: DivergenceKind,
    /// Stable claim id, present once the claim has been stamped by `surf verify` — the join key for
    /// claim timelines and attestation history. Additive under the v1 contract; never gates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_code: Option<String>,
    pub prose: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<Magnitude>,
    /// Human-readable reason for an `Unresolvable` or `ReferencedStale` divergence (unsupported
    /// file type, unreadable file, ambiguous anchor, symbol not found, or which referenced hub
    /// went stale). `None` for clean verdicts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

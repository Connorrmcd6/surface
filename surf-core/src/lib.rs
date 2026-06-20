//! surf-core: pure parse/resolve/hash logic for Surface. No I/O, no network.

pub mod anchor;
pub mod config;
pub mod hash;
pub mod hub;
pub mod lang;
pub mod rename;
pub mod report;
pub mod resolve;

pub use anchor::{parse_anchor, Anchor, AnchorParseError, Segment};
pub use config::{parse_config, Config, ConfigError, CONFIG_FILE};
pub use hash::{
    combine_site_hashes, diff_magnitude, format_stamp, hash_anchor, hash_anchor_raw,
    hash_anchor_with, parse_stamp, HashOpts, Magnitude, Recipe,
};
pub use hub::{parse_hub, set_anchor_at, set_anchor_hash, At, Claim, Frontmatter, Hub, HubError};
pub use lang::Lang;
pub use rename::find_renamed;
pub use report::{CheckReport, Divergence, DivergenceKind, REPORT_VERSION};
pub use resolve::{
    public_fns, public_symbols, resolve, suggest_chain, ResolveError, Span, Surface,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

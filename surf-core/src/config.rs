//! `surf.toml` — the workspace marker and config. Pure parsing only; filesystem
//! discovery (walking up from cwd) lives in the CLI, since it is I/O (§9.1.5).

use serde::{Deserialize, Serialize};

/// The marker filename the CLI walks up to find, like `.git` / `ruff.toml`.
pub const CONFIG_FILE: &str = "surf.toml";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Globs (relative to the config's directory) that enumerate hub documents. Best for the flat
    /// `hubs/*.md` layout.
    #[serde(default = "default_hubs")]
    pub hubs: Vec<String>,
    /// OKF bundle roots (relative to the config's directory): each is a directory *tree* of concept
    /// files, expanded as `<root>/**/*.md` with OKF reserved files (`index.md`/`log.md`) handled.
    /// Empty by default; add roots to govern an OKF bundle in place of, or alongside, `hubs`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bundles: Vec<String>,
}

fn default_hubs() -> Vec<String> {
    vec!["hubs/*.md".to_string()]
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hubs: default_hubs(),
            bundles: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Toml(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Toml(e) => write!(f, "invalid {CONFIG_FILE}: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn parse_config(content: &str) -> Result<Config, ConfigError> {
    toml::from_str(content).map_err(|e| ConfigError::Toml(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_uses_defaults() {
        let cfg = parse_config("").unwrap();
        assert_eq!(cfg.hubs, vec!["hubs/*.md".to_string()]);
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn custom_hub_globs() {
        let cfg = parse_config("hubs = [\"docs/hubs/*.md\", \"arch/*.md\"]").unwrap();
        assert_eq!(
            cfg.hubs,
            vec!["docs/hubs/*.md".to_string(), "arch/*.md".to_string()]
        );
    }

    #[test]
    fn unknown_key_is_rejected() {
        let err = parse_config("nonsense = true").unwrap_err();
        assert!(matches!(err, ConfigError::Toml(_)));
    }

    #[test]
    fn bundles_default_empty_and_parse() {
        assert!(parse_config("").unwrap().bundles.is_empty());
        let cfg = parse_config("bundles = [\"sales\", \"docs/okf\"]").unwrap();
        assert_eq!(
            cfg.bundles,
            vec!["sales".to_string(), "docs/okf".to_string()]
        );
    }
}

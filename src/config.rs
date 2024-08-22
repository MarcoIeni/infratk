use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// ITEM ID for 1Password aws legacy account
    pub op_legacy_item_id: Option<String>,
}

/// Create config dir if it doesn't exist.
/// Return config dir path.
pub fn create_config_dir() -> PathBuf {
    let home_dir = home::home_dir().unwrap();
    let config_dir = home_dir.join(".config");

    if !config_dir.exists() {
        fs_err::create_dir(&config_dir).unwrap();
    }

    let infratk_dir = config_dir.join("infratk");

    if !infratk_dir.exists() {
        fs_err::create_dir(&infratk_dir).unwrap();
    }

    infratk_dir
}

pub fn config_file(config_dir: &Path) -> PathBuf {
    config_dir.join(CONFIG_FILE)
}

pub fn parse_config() -> anyhow::Result<Config> {
    let infratk_dir = create_config_dir();
    let config_file = config_file(&infratk_dir);
    get_config(&config_file)
        .with_context(|| format!("Failed to parse config file: {config_file:?}"))
}

fn get_config(config_file: &Path) -> anyhow::Result<Config> {
    let config = if config_file.exists() {
        let content = fs_err::read_to_string(&config_file)?;
        toml::from_str(&content)?
    } else {
        Config::default()
    };
    Ok(config)
}

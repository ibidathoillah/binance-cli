use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::BinanceError;

/// Default Binance API host.
pub const DEFAULT_HOST: &str = "https://api.binance.com";

/// Environment variable names for credential override.
pub const ENV_API_KEY: &str = "BINANCE_API_KEY";
pub const ENV_API_SECRET: &str = "BINANCE_API_SECRET";

/// Configuration file schema.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub auth: AuthConfig,

    #[serde(default)]
    pub settings: SettingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsConfig {
    #[serde(default = "default_output")]
    pub output: String,

    #[serde(default = "default_host")]
    pub host: String,

    pub default_pair: Option<String>,
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            output: default_output(),
            host: default_host(),
            default_pair: None,
        }
    }
}

fn default_output() -> String {
    "table".to_string()
}

fn default_host() -> String {
    DEFAULT_HOST.to_string()
}

impl Config {
    /// Returns the config directory: `~/.config/binance`
    pub fn config_dir() -> Result<PathBuf, BinanceError> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| BinanceError::Config("Cannot determine config directory".to_string()))?;
        Ok(config_dir.join("binance"))
    }

    /// Returns the config file path: `~/.config/binance/config.toml`
    pub fn config_path() -> Result<PathBuf, BinanceError> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Returns the shell history file path: `~/.config/binance/history`
    pub fn history_path() -> Result<PathBuf, BinanceError> {
        Ok(Self::config_dir()?.join("history"))
    }

    /// Returns the paper trading state file path: `~/.config/binance/paper_state.json`
    pub fn paper_state_path() -> Result<PathBuf, BinanceError> {
        Ok(Self::config_dir()?.join("paper_state.json"))
    }

    /// Load config from disk. Returns default config if file doesn't exist.
    pub fn load() -> Result<Self, BinanceError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path).map_err(|e| {
            BinanceError::Config(format!(
                "Failed to read config at {}: {}",
                path.display(),
                e
            ))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            BinanceError::Config(format!(
                "Failed to parse config at {}: {}",
                path.display(),
                e
            ))
        })?;

        Ok(config)
    }

    /// Save config to disk with 0600 permissions.
    pub fn save(&self) -> Result<(), BinanceError> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                BinanceError::Config(format!(
                    "Failed to create config directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| BinanceError::Config(format!("Failed to serialize config: {}", e)))?;

        fs::write(&path, &content).map_err(|e| {
            BinanceError::Config(format!(
                "Failed to write config at {}: {}",
                path.display(),
                e
            ))
        })?;

        #[cfg(unix)]
        {
            // Set 0600 permissions (owner read/write only)
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms).map_err(|e| {
                BinanceError::Config(format!(
                    "Failed to set permissions on {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    /// Delete the config file.
    pub fn delete() -> Result<(), BinanceError> {
        let path = Self::config_path()?;
        if path.exists() {
            fs::remove_file(&path).map_err(|e| {
                BinanceError::Config(format!(
                    "Failed to delete config at {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }
}

/// Resolved credentials from multiple sources.
/// Priority: CLI flags → environment variables → config file.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
}

impl Credentials {
    /// Resolve credentials from available sources.
    pub fn resolve(cli_key: Option<&str>, cli_secret: Option<&str>) -> Result<Self, BinanceError> {
        // 1. CLI flags
        if let (Some(key), Some(secret)) = (cli_key, cli_secret) {
            return Ok(Self {
                api_key: key.to_string(),
                api_secret: secret.to_string(),
            });
        }

        // 2. Environment variables
        let env_key = std::env::var(ENV_API_KEY).ok();
        let env_secret = std::env::var(ENV_API_SECRET).ok();
        if let (Some(key), Some(secret)) = (env_key, env_secret) {
            return Ok(Self {
                api_key: key,
                api_secret: secret,
            });
        }

        // 3. Config file
        let config = Config::load()?;
        if let (Some(key), Some(secret)) = (config.auth.api_key, config.auth.api_secret) {
            return Ok(Self {
                api_key: key,
                api_secret: secret,
            });
        }

        Err(BinanceError::Auth(
            "No API credentials found. Set via:\n  \
             1. CLI flags: --api-key, --api-secret\n  \
             2. Environment: BINANCE_API_KEY, BINANCE_API_SECRET\n  \
             3. Config: binance auth set --api-key KEY --api-secret SECRET"
                .to_string(),
        ))
    }

    /// Check if credentials are available without error.
    pub fn available(cli_key: Option<&str>, cli_secret: Option<&str>) -> bool {
        Self::resolve(cli_key, cli_secret).is_ok()
    }

    pub fn to_binance_credentials(&self) -> binance_spot_connector_rust::http::Credentials {
        binance_spot_connector_rust::http::Credentials::from_hmac(
            self.api_key.clone(),
            self.api_secret.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.settings.output, "table");
        assert_eq!(config.settings.host, DEFAULT_HOST);
        assert!(config.auth.api_key.is_none());
    }

    #[test]
    fn test_credentials_resolve_cli() {
        let creds = Credentials::resolve(Some("cli-key"), Some("cli-secret")).unwrap();
        assert_eq!(creds.api_key, "cli-key");
        assert_eq!(creds.api_secret, "cli-secret");
    }

    #[test]
    fn test_credentials_resolve_env() {
        std::env::set_var(ENV_API_KEY, "env-key");
        std::env::set_var(ENV_API_SECRET, "env-secret");

        let creds = Credentials::resolve(None, None).unwrap();
        assert_eq!(creds.api_key, "env-key");
        assert_eq!(creds.api_secret, "env-secret");

        std::env::remove_var(ENV_API_KEY);
        std::env::remove_var(ENV_API_SECRET);
    }

    #[test]
    fn test_credentials_resolve_none() {
        std::env::remove_var(ENV_API_KEY);
        std::env::remove_var(ENV_API_SECRET);
        // Ensure even if there is no config file, it fails gracefully instead of panic
        let res = Credentials::resolve(None, None);
        assert!(res.is_err());
    }

    #[test]
    fn test_credentials_available() {
        assert!(Credentials::available(Some("k"), Some("s")));
        assert!(!Credentials::available(None, None));
    }
}

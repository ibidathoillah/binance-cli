use clap::Subcommand;
use binance_spot_connector_rust::market;
use binance_spot_connector_rust::trade;

use crate::config::{AuthConfig, Config};
use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Set API credentials
    Set {
        #[arg(long)]
        api_key: String,
        #[arg(long)]
        api_secret: String,
    },
    /// Show configured credentials (masked)
    Show,
    /// Test credentials against the API
    Test,
    /// Delete stored credentials
    Reset,
}

impl AuthCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let client = &ctx.client;

        let output = match self {
            Self::Set {
                api_key,
                api_secret,
            } => {
                let mut config = Config::load()?;
                config.auth = AuthConfig {
                    api_key: Some(api_key.clone()),
                    api_secret: Some(api_secret.clone()),
                };
                config.save()?;
                let path = Config::config_path()?;
                CommandOutput::new(
                    serde_json::json!({ "path": path.display().to_string() }),
                    "Auth Set",
                )
                .with_addendum(format!("Credentials saved to {}", path.display()))
            }
            Self::Show => {
                let config = Config::load()?;
                let key = config.auth.api_key.as_deref().unwrap_or("(not set)");
                let secret = config.auth.api_secret.as_deref().unwrap_or("(not set)");
                let masked_key = if key.len() > 8 {
                    format!("{}...{}", &key[..4], &key[key.len() - 4..])
                } else {
                    key.to_string()
                };
                let masked_secret = if secret.len() > 8 {
                    format!("{}...{}", &secret[..4], &secret[secret.len() - 4..])
                } else {
                    "(set)".to_string()
                };
                let info = serde_json::json!({
                    "api_key": masked_key,
                    "api_secret": masked_secret,
                    "config_path": Config::config_path()?.display().to_string()
                });
                CommandOutput::new(info, "Auth Config")
            }
            Self::Test => {
                // Test public ping
                let _ = client.send_request(market::ping()).await?;
                let mut output = CommandOutput::new(
                    serde_json::json!({ "connectivity": "ok" }),
                    "Auth Test",
                )
                .with_addendum("API connectivity OK");

                // Test private endpoints
                match client.require_credentials() {
                    Ok(creds) => {
                        let req = trade::account().credentials(&creds.to_binance_credentials());
                        match client.send_request(req).await {
                            Ok(_) => {
                                output = output.with_addendum("Authentication OK — credentials are valid");
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
                output
            }
            Self::Reset => {
                Config::delete()?;
                CommandOutput::new(serde_json::json!({ "reset": true }), "Auth Reset")
                    .with_addendum("Credentials deleted")
            }
        };
        Ok(output.with_format(ctx.format))
    }
}

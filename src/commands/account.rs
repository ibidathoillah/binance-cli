use binance_spot_connector_rust::trade;
use clap::Subcommand;

use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    /// Get current account information (balances, permissions, etc.)
    Info,

    /// Get non-zero account balances
    Balance,

    /// Get your trade history for a specific symbol
    Trades {
        /// Trading pair symbol (e.g., BTCUSDT)
        symbol: String,

        /// Number of trades to return (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,

        /// Start from this trade ID (optional)
        #[arg(long)]
        from_id: Option<u64>,
    },
}

impl AccountCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let client = &ctx.client;
        let creds = client.require_credentials()?;
        let binance_creds = creds.to_binance_credentials();

        let output = match self {
            Self::Info => {
                let request = trade::account().credentials(&binance_creds);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, "Account Info")
            }

            Self::Balance => {
                let request = trade::account()
                    .omit_zero_balances(true)
                    .credentials(&binance_creds);
                let result = client.send_request(request).await?;
                if let Some(balances) = result.get("balances") {
                    CommandOutput::new(
                        serde_json::json!({ "balances": balances }),
                        "Account Balances",
                    )
                } else {
                    CommandOutput::new(result, "Account Info")
                }
            }

            Self::Trades {
                symbol,
                limit,
                from_id,
            } => {
                let sym = symbol.to_uppercase();
                let mut request = trade::my_trades(&sym)
                    .limit(*limit)
                    .credentials(&binance_creds);
                if let Some(fid) = from_id {
                    request = request.from_id(*fid);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("My Trades — {}", sym))
            }
        };

        Ok(output.with_format(ctx.format))
    }
}

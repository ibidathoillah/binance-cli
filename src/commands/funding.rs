use std::str::FromStr;
use clap::Subcommand;
use binance_spot_connector_rust::wallet;
use rust_decimal::Decimal;

use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum DepositCommand {
    /// Get deposit address for a coin
    Addresses {
        /// Coin name (e.g. BNB, USDT)
        asset: String,

        /// Network to deposit on (optional)
        #[arg(long)]
        network: Option<String>,
    },

    /// Get crypto deposit history
    Status {
        /// Coin name (optional)
        #[arg(long)]
        asset: Option<String>,
    },
}

impl DepositCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let client = &ctx.client;
        let creds = client.require_credentials()?;
        let binance_creds = creds.to_binance_credentials();

        let output = match self {
            Self::Addresses { asset, network } => {
                let c = asset.to_uppercase();
                let mut request = wallet::deposit_address(&c).credentials(&binance_creds);
                if let Some(net) = network {
                    request = request.network(net);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Deposit Address — {}", c))
            }

            Self::Status { asset } => {
                let mut request = wallet::deposit_history().credentials(&binance_creds);
                if let Some(c) = asset {
                    let c_upper = c.to_uppercase();
                    request = request.coin(&c_upper);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, "Deposit History")
            }
        };

        Ok(output.with_format(ctx.format))
    }
}

#[derive(Debug, Subcommand)]
pub enum WithdrawalCommand {
    /// Get crypto withdraw history
    Status {
        /// Coin name (optional)
        #[arg(long)]
        asset: Option<String>,
    },
}

impl WithdrawalCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let client = &ctx.client;
        let creds = client.require_credentials()?;
        let binance_creds = creds.to_binance_credentials();

        let output = match self {
            Self::Status { asset } => {
                let mut request = wallet::withdraw_history().credentials(&binance_creds);
                if let Some(c) = asset {
                    let c_upper = c.to_uppercase();
                    request = request.coin(&c_upper);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, "Withdrawal History")
            }
        };

        Ok(output.with_format(ctx.format))
    }
}

pub async fn execute_withdraw(
    ctx: &AppContext,
    asset: &str,
    volume: &str,
    address: &str,
    network: Option<&str>,
) -> Result<CommandOutput, BinanceError> {
    let client = &ctx.client;
    let creds = client.require_credentials()?;
    let binance_creds = creds.to_binance_credentials();

    let c = asset.to_uppercase();
    let amt_dec = Decimal::from_str(volume).map_err(|e| {
        BinanceError::Validation(format!("Invalid volume '{}': {}", volume, e))
    })?;

    let mut request = wallet::withdraw(&c, address, amt_dec).credentials(&binance_creds);
    if let Some(net) = network {
        request = request.network(net);
    }

    let result = client.send_request(request).await?;
    let output = CommandOutput::new(result, "Withdrawal Submitted")
        .with_addendum(format!("Withdrawal request of {} {} submitted successfully", volume, c))
        .with_format(ctx.format);

    Ok(output)
}

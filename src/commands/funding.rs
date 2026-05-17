use std::str::FromStr;
use clap::Subcommand;
use binance_spot_connector_rust::wallet;
use rust_decimal::Decimal;

use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum FundingCommand {
    /// Withdraw crypto to an external address
    Withdraw {
        /// Coin name (e.g. BNB, USDT)
        #[arg(long)]
        coin: String,

        /// Amount to withdraw
        #[arg(long)]
        amount: String,

        /// Destination address
        #[arg(long)]
        address: String,

        /// Network to withdraw on (optional)
        #[arg(long)]
        network: Option<String>,
    },

    /// Get crypto withdraw history
    WithdrawHistory {
        /// Coin name (optional)
        #[arg(long)]
        coin: Option<String>,
    },

    /// Get crypto deposit history
    DepositHistory {
        /// Coin name (optional)
        #[arg(long)]
        coin: Option<String>,
    },

    /// Get deposit address for a coin
    DepositAddress {
        /// Coin name (e.g. BNB, USDT)
        #[arg(long)]
        coin: String,

        /// Network to deposit on (optional)
        #[arg(long)]
        network: Option<String>,
    },
}

impl FundingCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let client = &ctx.client;
        let creds = client.require_credentials()?;
        let binance_creds = creds.to_binance_credentials();

        let output = match self {
            Self::Withdraw {
                coin,
                amount,
                address,
                network,
            } => {
                let c = coin.to_uppercase();
                let amt_dec = Decimal::from_str(amount).map_err(|e| {
                    BinanceError::Validation(format!("Invalid amount '{}': {}", amount, e))
                })?;

                let mut request = wallet::withdraw(&c, address, amt_dec).credentials(&binance_creds);
                if let Some(net) = network {
                    request = request.network(net);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, "Withdrawal Submitted")
                    .with_addendum(format!("Withdrawal request of {} {} submitted successfully", amount, c))
            }

            Self::WithdrawHistory { coin } => {
                let mut request = wallet::withdraw_history().credentials(&binance_creds);
                if let Some(c) = coin {
                    let c_upper = c.to_uppercase();
                    request = request.coin(&c_upper);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, "Withdrawal History")
            }

            Self::DepositHistory { coin } => {
                let mut request = wallet::deposit_history().credentials(&binance_creds);
                if let Some(c) = coin {
                    let c_upper = c.to_uppercase();
                    request = request.coin(&c_upper);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, "Deposit History")
            }

            Self::DepositAddress { coin, network } => {
                let c = coin.to_uppercase();
                let mut request = wallet::deposit_address(&c).credentials(&binance_creds);
                if let Some(net) = network {
                    request = request.network(net);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Deposit Address — {}", c))
            }
        };

        Ok(output.with_format(ctx.format))
    }
}

use clap::Subcommand;

use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum PaperCommand {
    /// Show paper trading balances
    Balance,
}

impl PaperCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        match self {
            Self::Balance => {
                let data = serde_json::json!({
                    "balances": [
                        { "asset": "USDT", "free": "10000.0", "locked": "0.0" },
                        { "asset": "BTC", "free": "1.0", "locked": "0.0" },
                        { "asset": "BNB", "free": "10.0", "locked": "0.0" }
                    ]
                });
                Ok(CommandOutput::new(data, "Paper Balances")
                    .with_format(ctx.format)
                    .with_addendum("Paper trading is currently a simulated stub."))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    #[tokio::test]
    async fn test_paper_balance() {
        let cmd = PaperCommand::Balance;
        let ctx = AppContext {
            client: crate::client::BinanceHttpClient::new("https://api.binance.com", None),
            format: OutputFormat::Table,
            verbose: false,
            yes: false,
        };
        let res = cmd.execute(&ctx).await.unwrap();
        assert_eq!(res.label, "Paper Balances");
        assert!(res.addendum.unwrap().contains("simulated stub"));
        assert_eq!(res.data["balances"][0]["asset"], "USDT");
    }
}

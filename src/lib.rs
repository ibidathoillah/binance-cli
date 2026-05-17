pub mod client;
pub mod commands;
pub mod config;
pub mod errors;
pub mod mcp;
pub mod output;

use clap::{Parser, Subcommand};

use crate::client::BinanceHttpClient;
use crate::commands::{
    account, auth as auth_cmds, funding, market, paper, trade, utility, websocket,
};
use crate::errors::BinanceError;
use crate::output::{CommandOutput, OutputFormat};

/// Global application context.
#[derive(Clone)]
pub struct AppContext {
    pub client: BinanceHttpClient,
    pub format: OutputFormat,
    pub verbose: bool,
}

#[derive(Parser, Debug)]
#[command(
    name = "binance",
    version,
    about = "Unofficial CLI for the Binance cryptocurrency exchange",
    long_about = "Trade, track markets, and manage your account on Binance — from your terminal.\n\n\
                  Built with Rust for maximum performance and safety."
)]
pub struct Cli {
    /// Output format: table or json
    #[arg(short, long, default_value = "table", global = true)]
    pub output: OutputFormat,

    /// API key (overrides config and env var)
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// API secret (overrides config and env var)
    #[arg(long, global = true)]
    pub api_secret: Option<String>,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Override API host URL
    #[arg(long, global = true)]
    pub host: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Market data (public, no API key needed)
    #[command(subcommand)]
    Market(market::MarketCommand),

    /// Account information (requires API key)
    #[command(subcommand)]
    Account(account::AccountCommand),

    /// Trading operations (requires API key)
    #[command(subcommand)]
    Trade(trade::TradeCommand),

    /// Funding: withdrawals, deposits
    #[command(subcommand)]
    Funding(funding::FundingCommand),

    /// WebSocket real-time data streams
    #[command(subcommand)]
    Ws(websocket::WebSocketCommand),

    /// Paper trading (simulated)
    #[command(subcommand)]
    Paper(paper::PaperCommand),

    /// API credential management
    #[command(subcommand)]
    Auth(auth_cmds::AuthCommand),

    /// Interactive shell (REPL)
    Shell,

    /// Run as an MCP (Model Context Protocol) server
    Mcp {
        /// Allow dangerous commands (trade, funding) (ignored for now, present for compatibility)
        #[arg(long)]
        allow_dangerous: bool,
    },
}

/// Dispatch all non-shell commands to their executors.
pub async fn dispatch_non_shell(
    ctx: &AppContext,
    command: Command,
) -> Result<CommandOutput, BinanceError> {
    match command {
        Command::Market(cmd) => cmd.execute(ctx).await,
        Command::Account(cmd) => cmd.execute(ctx).await,
        Command::Trade(cmd) => cmd.execute(ctx).await,
        Command::Funding(cmd) => cmd.execute(ctx).await,
        Command::Ws(cmd) => cmd.execute(ctx).await,
        Command::Paper(cmd) => cmd.execute(ctx).await,
        Command::Auth(cmd) => cmd.execute(ctx).await,
        Command::Shell => Err(BinanceError::Config(
            "Shell command is not supported in this context".to_string(),
        )),
        Command::Mcp { .. } => Err(BinanceError::Config(
            "MCP server must be started from the main entry point".to_string(),
        )),
    }
}

/// Dispatch the parsed command to its executor.
pub async fn dispatch(ctx: &AppContext, command: Command) -> Result<CommandOutput, BinanceError> {
    match command {
        Command::Shell => {
            utility::run_shell(ctx).await?;
            Ok(CommandOutput::new(serde_json::json!({}), "Shell").with_format(ctx.format))
        }
        other => dispatch_non_shell(ctx, other).await,
    }
}

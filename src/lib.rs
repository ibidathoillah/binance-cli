pub mod client;
pub mod commands;
pub mod config;
pub mod errors;
pub mod mcp;
pub mod output;

use clap::{Parser, Subcommand};

use crate::client::BinanceHttpClient;
use crate::commands::{
    account, auth as auth_cmds, funding, market, paper, trade, utility, websocket, futures as futures_cmds,
};
use crate::errors::BinanceError;
use crate::output::{CommandOutput, OutputFormat};

pub(crate) fn normalize_pair(pair: &str) -> String {
    pair.replace(['_', '-', '/'], "").to_uppercase()
}

pub(crate) fn normalize_pair_ws(pair: &str) -> String {
    normalize_pair(pair).to_lowercase()
}

#[cfg(test)]
mod pair_tests {
    use super::*;

    #[test]
    fn normalizes_pair_for_api() {
        assert_eq!(normalize_pair("BTCUSDT"), "BTCUSDT");
        assert_eq!(normalize_pair("btc_usdt"), "BTCUSDT");
        assert_eq!(normalize_pair("btc-usdt"), "BTCUSDT");
        assert_eq!(normalize_pair("btc/usdt"), "BTCUSDT");
    }

    #[test]
    fn normalizes_pair_for_websocket() {
        assert_eq!(normalize_pair_ws("BTC_USDT"), "btcusdt");
    }
}

/// Global application context.
#[derive(Clone)]
pub struct AppContext {
    pub client: BinanceHttpClient,
    pub format: OutputFormat,
    pub verbose: bool,
    pub yes: bool,
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

    /// Skip confirmation prompts for destructive operations
    #[arg(long, alias = "force", global = true)]
    pub yes: bool,

    /// Override API host URL
    #[arg(long, global = true)]
    pub host: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    // === Public Market Commands (originally nested under Market) ===
    /// Test connectivity to the REST API
    Ping,

    /// Get the current server time
    ServerTime,

    /// Get exchange trading rules and symbol information
    ExchangeInfo,

    /// Get 24hr ticker price change statistics
    Ticker {
        /// Trading pair symbol (e.g., BTCUSDT, BNBUSDT)
        pair: String,
    },

    /// Get 24hr ticker for all symbols
    TickerAll,

    /// Get latest price for a symbol
    Price {
        /// Trading pair symbol
        pair: String,
    },

    /// Get best price/qty on the order book
    BookTicker {
        /// Trading pair symbol
        pair: String,
    },

    /// Get order book depth
    Orderbook {
        /// Trading pair symbol
        pair: String,

        /// Limit number of price levels (default: 100, max: 5000)
        #[arg(short, long, default_value = "100")]
        count: u32,
    },

    /// Get recent trades
    Trades {
        /// Trading pair symbol
        pair: String,

        /// Number of trades to return (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    /// Get older historical trades (requires API key)
    HistoricalTrades {
        /// Trading pair symbol
        pair: String,

        /// Number of trades (default: 500)
        #[arg(short, long, default_value = "500")]
        count: u32,

        /// Trade id to fetch from
        #[arg(long, alias = "from-id")]
        since: Option<u64>,
    },

    /// Get compressed/aggregate trades
    AggTrades {
        /// Trading pair symbol
        pair: String,

        /// Number of results (default: 500)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    /// Get kline/candlestick bars for a symbol (OHLC)
    Ohlc {
        /// Trading pair symbol (e.g. BTCUSDT)
        pair: String,

        /// Interval (e.g. 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1m")]
        interval: String,

        /// Limit number of bars (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    // === Account / Balances Commands (originally nested under Account) ===
    /// Get current account information (balances, permissions, etc.)
    AccountInfo,

    /// Get non-zero account balances
    Balance,

    /// Get your trade history for a specific symbol
    TradesHistory {
        /// Trading pair symbol (e.g., BTCUSDT)
        pair: String,

        /// Number of trades to return (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,

        /// Start from this trade ID (optional)
        #[arg(long, alias = "since-id", alias = "from-id")]
        since: Option<u64>,
    },

    // === Trading Operations (originally nested under Trade) ===
    /// Place and manage orders
    #[command(subcommand)]
    Order(trade::OrderCommand),

    // === Funding Operations (originally nested under Funding) ===
    /// Manage deposits
    #[command(subcommand)]
    Deposit(funding::DepositCommand),

    /// Withdraw crypto to an external address
    Withdraw {
        /// Coin name (e.g. BNB, USDT)
        #[arg(long)]
        asset: String,

        /// Amount to withdraw
        #[arg(long)]
        volume: String,

        /// Destination address
        #[arg(long)]
        address: String,

        /// Network to withdraw on (optional)
        #[arg(long)]
        network: Option<String>,
    },

    /// Manage withdrawals
    #[command(subcommand)]
    Withdrawal(funding::WithdrawalCommand),

    // === WebSocket Streams ===
    /// WebSocket real-time data streams
    #[command(subcommand)]
    Ws(websocket::WebSocketCommand),

    // === Paper Trading ===
    /// Paper trading (simulated)
    #[command(subcommand)]
    Paper(paper::PaperCommand),
    /// Binance Futures (USDS-M)
    #[command(subcommand)]
    Futures(futures_cmds::FuturesCommand),

    // === API Credentials ===
    /// API credential management
    #[command(subcommand)]
    Auth(auth_cmds::AuthCommand),

    // === Interactive Shell & MCP ===
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
        // === Public Market Commands ===
        Command::Ping => market::MarketCommand::Ping.execute(ctx).await,
        Command::ServerTime => market::MarketCommand::ServerTime.execute(ctx).await,
        Command::ExchangeInfo => market::MarketCommand::ExchangeInfo.execute(ctx).await,
        Command::Ticker { pair } => {
            market::MarketCommand::Ticker {
                symbol: crate::normalize_pair(&pair),
            }
                .execute(ctx)
                .await
        }
        Command::TickerAll => market::MarketCommand::TickerAll.execute(ctx).await,
        Command::Price { pair } => {
            market::MarketCommand::Price {
                symbol: crate::normalize_pair(&pair),
            }
                .execute(ctx)
                .await
        }
        Command::BookTicker { pair } => {
            market::MarketCommand::BookTicker {
                symbol: crate::normalize_pair(&pair),
            }
                .execute(ctx)
                .await
        }
        Command::Orderbook { pair, count } => {
            market::MarketCommand::Orderbook {
                symbol: crate::normalize_pair(&pair),
                limit: count,
            }
            .execute(ctx)
            .await
        }
        Command::Trades { pair, count } => {
            market::MarketCommand::Trades {
                symbol: crate::normalize_pair(&pair),
                limit: count,
            }
            .execute(ctx)
            .await
        }
        Command::HistoricalTrades { pair, count, since } => {
            market::MarketCommand::HistoricalTrades {
                symbol: crate::normalize_pair(&pair),
                limit: count,
                from_id: since,
            }
            .execute(ctx)
            .await
        }
        Command::AggTrades { pair, count } => {
            market::MarketCommand::AggTrades {
                symbol: crate::normalize_pair(&pair),
                limit: count,
            }
            .execute(ctx)
            .await
        }
        Command::Ohlc {
            pair,
            interval,
            count,
        } => {
            market::MarketCommand::Klines {
                symbol: crate::normalize_pair(&pair),
                interval,
                limit: count,
            }
            .execute(ctx)
            .await
        }

        // === Account & Balance Commands ===
        Command::AccountInfo => account::AccountCommand::Info.execute(ctx).await,
        Command::Balance => account::AccountCommand::Balance.execute(ctx).await,
        Command::TradesHistory { pair, count, since } => {
            account::AccountCommand::Trades {
                symbol: crate::normalize_pair(&pair),
                limit: count,
                from_id: since,
            }
            .execute(ctx)
            .await
        }

        // === Order Operations ===
        Command::Order(cmd) => cmd.execute(ctx).await,

        // === Funding Operations ===
        Command::Deposit(cmd) => cmd.execute(ctx).await,
        Command::Withdraw {
            asset,
            volume,
            address,
            network,
        } => funding::execute_withdraw(ctx, &asset, &volume, &address, network.as_deref()).await,
        Command::Withdrawal(cmd) => cmd.execute(ctx).await,

        // === WS, Paper, Auth, Shell, Mcp ===
        Command::Ws(cmd) => cmd.execute(ctx).await,
        Command::Paper(cmd) => cmd.execute(ctx).await,
        Command::Futures(cmd) => cmd.execute(ctx).await,
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

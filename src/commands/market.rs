use binance_spot_connector_rust::market;
use binance_spot_connector_rust::market::klines::KlineInterval;
use clap::Subcommand;

use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum MarketCommand {
    /// Test connectivity to the REST API
    Ping,

    /// Get the current server time
    ServerTime,

    /// Get exchange trading rules and symbol information
    ExchangeInfo,

    /// Get 24hr ticker price change statistics
    Ticker {
        /// Trading pair symbol (e.g., BTCUSDT, BNBUSDT)
        symbol: String,
    },

    /// Get 24hr ticker for all symbols
    TickerAll,

    /// Get latest price for a symbol
    Price {
        /// Trading pair symbol
        symbol: String,
    },

    /// Get best price/qty on the order book
    BookTicker {
        /// Trading pair symbol
        symbol: String,
    },

    /// Get order book depth
    Orderbook {
        /// Trading pair symbol
        symbol: String,

        /// Limit number of price levels (default: 100, max: 5000)
        #[arg(short, long, default_value = "100")]
        limit: u32,
    },

    /// Get recent trades
    Trades {
        /// Trading pair symbol
        symbol: String,

        /// Number of trades to return (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },

    /// Get older historical trades (requires API key)
    HistoricalTrades {
        /// Trading pair symbol
        symbol: String,

        /// Number of trades (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: u32,

        /// Trade id to fetch from
        #[arg(long)]
        from_id: Option<u64>,
    },

    /// Get compressed/aggregate trades
    AggTrades {
        /// Trading pair symbol
        symbol: String,

        /// Number of results (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },

    /// Get kline/candlestick bars for a symbol
    Klines {
        /// Trading pair symbol (e.g. BTCUSDT)
        symbol: String,

        /// Interval (e.g. 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1m")]
        interval: String,

        /// Limit number of bars (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },
}

fn parse_interval(s: &str) -> Result<KlineInterval, BinanceError> {
    match s.to_lowercase().as_str() {
        "1m" => Ok(KlineInterval::Minutes1),
        "3m" => Ok(KlineInterval::Minutes3),
        "5m" => Ok(KlineInterval::Minutes5),
        "15m" => Ok(KlineInterval::Minutes15),
        "30m" => Ok(KlineInterval::Minutes30),
        "1h" => Ok(KlineInterval::Hours1),
        "2h" => Ok(KlineInterval::Hours2),
        "4h" => Ok(KlineInterval::Hours4),
        "6h" => Ok(KlineInterval::Hours6),
        "8h" => Ok(KlineInterval::Hours8),
        "12h" => Ok(KlineInterval::Hours12),
        "1d" => Ok(KlineInterval::Days1),
        "3d" => Ok(KlineInterval::Days3),
        "1w" => Ok(KlineInterval::Weeks1),
        "1M" | "1mon" | "1month" => Ok(KlineInterval::Months1),
        _ => Err(BinanceError::Validation(format!(
            "Invalid kline interval: {}",
            s
        ))),
    }
}

impl MarketCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let client = &ctx.client;

        let output = match self {
            Self::Ping => {
                let request = market::ping();
                let _result = client.send_request(request).await?;
                CommandOutput::new(serde_json::json!({ "status": "ok" }), "Ping")
                    .with_addendum("Binance API is reachable")
            }

            Self::ServerTime => {
                let request = market::time();
                let result = client.send_request(request).await?;
                let ts = result["serverTime"].as_u64().unwrap_or(0);
                let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| ts.to_string());

                CommandOutput::new(result, "Server Time").with_addendum(format!("{} ({})", dt, ts))
            }

            Self::ExchangeInfo => {
                let request = market::exchange_info();
                let result = client.send_request(request).await?;
                CommandOutput::new(result, "Exchange Info")
            }

            Self::Ticker { symbol } => {
                let sym = symbol.to_uppercase();
                let request = market::ticker_twenty_four_hr().symbol(&sym);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("24h Ticker — {}", sym))
            }

            Self::TickerAll => {
                let request = market::ticker_twenty_four_hr();
                let result = client.send_request(request).await?;
                CommandOutput::new(result, "All Tickers (24h)")
            }

            Self::Price { symbol } => {
                let sym = symbol.to_uppercase();
                let request = market::ticker_price().symbol(&sym);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Price — {}", sym))
            }

            Self::BookTicker { symbol } => {
                let sym = symbol.to_uppercase();
                let request = market::book_ticker().symbol(&sym);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Book Ticker — {}", sym))
            }

            Self::Orderbook { symbol, limit } => {
                let sym = symbol.to_uppercase();
                let request = market::depth(&sym).limit(*limit);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Order Book — {}", sym))
            }

            Self::Trades { symbol, limit } => {
                let sym = symbol.to_uppercase();
                let request = market::trades(&sym).limit(*limit);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Recent Trades — {}", sym))
            }

            Self::HistoricalTrades {
                symbol,
                limit,
                from_id,
            } => {
                let sym = symbol.to_uppercase();
                let mut request = market::historical_trades(&sym).limit(*limit);
                if let Some(id) = from_id {
                    request = request.from_id(*id);
                }

                // Historical trades requires an API Key
                if let Some(creds) = &ctx.client.require_credentials().ok() {
                    let binance_creds = creds.to_binance_credentials();
                    request = request.credentials(&binance_creds);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Historical Trades — {}", sym))
            }

            Self::AggTrades { symbol, limit } => {
                let sym = symbol.to_uppercase();
                let request = market::agg_trades(&sym).limit(*limit);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Aggregate Trades — {}", sym))
            }

            Self::Klines {
                symbol,
                interval,
                limit,
            } => {
                let sym = symbol.to_uppercase();
                let parsed_int = parse_interval(interval)?;
                let request = market::klines(&sym, parsed_int).limit(*limit);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Klines — {}", sym))
            }
        };

        Ok(output.with_format(ctx.format))
    }
}

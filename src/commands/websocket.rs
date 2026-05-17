use clap::Subcommand;
use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::Value;
use tokio::time::{timeout, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use binance_spot_connector_rust::http::{request::RequestBuilder, Method};

use crate::errors::BinanceError;
use crate::output::{CommandOutput, OutputFormat};
use crate::AppContext;

const DEFAULT_WS_HOST: &str = "wss://stream.binance.com:9443/ws";

#[derive(Debug, Subcommand)]
pub enum WebSocketCommand {
    /// Stream order book depth updates
    Depth {
        /// Trading pair (lowercase, e.g., btcusdt, bnbusdt)
        symbol: String,

        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },

    /// Stream real-time 24hr ticker updates
    Ticker {
        /// Trading pair (lowercase, e.g., btcusdt, bnbusdt)
        symbol: String,

        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },

    /// Stream real-time best bid/ask book ticker updates
    BookTicker {
        /// Trading pair (lowercase, e.g., btcusdt, bnbusdt)
        symbol: String,

        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },

    /// Stream private account/order/trade updates
    User {
        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },

    /// Stream private order updates (alias of user stream filtering executionReport)
    Orders {
        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },

    /// Stream private balance updates (alias of user stream filtering outboundAccountPosition)
    Balances {
        /// Stop after receiving this many data messages
        #[arg(short, long)]
        limit: Option<usize>,

        /// Stop after this many seconds
        #[arg(long)]
        seconds: Option<u64>,
    },
}

impl WebSocketCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        match self {
            Self::Depth {
                symbol,
                limit,
                seconds,
            } => {
                let sym = symbol.to_lowercase();
                stream_market_ticker(&sym, "depth", "Depth Updates", ctx.format, StreamBounds::new(*limit, *seconds)).await?;
            }
            Self::Ticker {
                symbol,
                limit,
                seconds,
            } => {
                let sym = symbol.to_lowercase();
                stream_market_ticker(&sym, "ticker", "Ticker Updates", ctx.format, StreamBounds::new(*limit, *seconds)).await?;
            }
            Self::BookTicker {
                symbol,
                limit,
                seconds,
            } => {
                let sym = symbol.to_lowercase();
                stream_market_ticker(&sym, "bookTicker", "Book Ticker Updates", ctx.format, StreamBounds::new(*limit, *seconds)).await?;
            }
            Self::User { limit, seconds } => {
                stream_user_events(ctx, None, StreamBounds::new(*limit, *seconds)).await?;
            }
            Self::Orders { limit, seconds } => {
                stream_user_events(ctx, Some("executionReport"), StreamBounds::new(*limit, *seconds)).await?;
            }
            Self::Balances { limit, seconds } => {
                stream_user_events(ctx, Some("outboundAccountPosition"), StreamBounds::new(*limit, *seconds)).await?;
            }
        }
        Ok(CommandOutput::new(Value::Null, "").with_format(ctx.format))
    }
}

#[derive(Debug, Clone, Copy)]
struct StreamBounds {
    limit: Option<usize>,
    seconds: Option<u64>,
}

impl StreamBounds {
    fn new(limit: Option<usize>, seconds: Option<u64>) -> Self {
        Self { limit, seconds }
    }

    fn deadline(self) -> Option<Instant> {
        self.seconds
            .map(|seconds| Instant::now() + Duration::from_secs(seconds))
    }

    fn limit_reached(self, count: usize) -> bool {
        self.limit.is_some_and(|limit| count >= limit)
    }
}

async fn stream_market_ticker(
    symbol: &str,
    stream_type: &str,
    label: &str,
    format: OutputFormat,
    bounds: StreamBounds,
) -> Result<(), BinanceError> {
    let url = format!("{}/{}@{}", DEFAULT_WS_HOST, symbol, stream_type);
    use colored::Colorize;
    eprintln!("{} Connecting to {} ...", "WS".cyan().bold(), url);

    let (mut ws, _) = connect_async(&url)
        .await
        .map_err(|e| BinanceError::WebSocket(e.to_string()))?;

    eprintln!("{} Subscribed to {} for {}", "WS".green().bold(), stream_type, symbol);

    let mut data_count = 0usize;
    let deadline = bounds.deadline();

    loop {
        let msg = match next_message(&mut ws, deadline).await? {
            Some(msg) => msg,
            None => break,
        };

        match msg {
            Ok(Message::Text(text)) => {
                let data: Value = serde_json::from_str(&text).map_err(|e| {
                    BinanceError::WebSocket(format!("Failed to parse JSON: {}", e))
                })?;
                
                let output = CommandOutput::new(data, label).with_format(format);
                println!("{}", output.render());
                data_count += 1;
            }
            Ok(Message::Ping(payload)) => {
                let _ = ws.send(Message::Pong(payload)).await;
            }
            Ok(Message::Close(_)) => {
                eprintln!("{} Connection closed", "WS".yellow().bold());
                break;
            }
            Ok(Message::Binary(_)) => {}
            Err(e) => {
                eprintln!("{} Error: {}", "WS".red().bold(), e);
                break;
            }
            _ => {}
        }

        if bounds.limit_reached(data_count) {
            break;
        }
    }
    Ok(())
}

async fn stream_user_events(
    ctx: &AppContext,
    filter_event: Option<&str>,
    bounds: StreamBounds,
) -> Result<(), BinanceError> {
    use colored::Colorize;

    let client = &ctx.client;
    let creds = client.require_credentials()?;

    // 1. POST /api/v3/userDataStream to get listenKey
    let req = RequestBuilder::new(Method::Post, "/api/v3/userDataStream")
        .credentials(creds.to_binance_credentials());
    let lk_resp = client.send_request(req).await?;
    let listen_key = lk_resp["listenKey"]
        .as_str()
        .ok_or_else(|| BinanceError::Api {
            code: -1,
            message: "Failed to obtain listenKey from userDataStream response".to_string(),
        })?;

    let url = format!("{}/{}", DEFAULT_WS_HOST, listen_key);
    eprintln!("{} Connecting to user stream ...", "WS".cyan().bold());

    let (mut ws, _) = connect_async(&url)
        .await
        .map_err(|e| BinanceError::WebSocket(e.to_string()))?;

    eprintln!("{} Subscribed to user data stream", "WS".green().bold());

    // 2. Spawn keep-alive task
    let lk = listen_key.to_string();
    let client_clone = client.clone();
    let api_key = creds.api_key.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30 * 60)).await;
            let keep_req = RequestBuilder::new(Method::Put, "/api/v3/userDataStream")
                .params(vec![("listenKey", lk.as_str())])
                .credentials(binance_spot_connector_rust::http::Credentials::from_hmac(&api_key, ""));
            match client_clone.send_request(keep_req).await {
                Ok(_) => eprintln!("{} User stream listenKey keep-alive sent successfully", "WS".blue().bold()),
                Err(e) => eprintln!("{} User stream keep-alive failed: {}", "WS".red().bold(), e),
            }
        }
    });

    let mut data_count = 0usize;
    let deadline = bounds.deadline();

    loop {
        let msg = match next_message(&mut ws, deadline).await? {
            Some(msg) => msg,
            None => break,
        };

        match msg {
            Ok(Message::Text(text)) => {
                let data: Value = serde_json::from_str(&text).map_err(|e| {
                    BinanceError::WebSocket(format!("Failed to parse JSON: {}", e))
                })?;

                let event_type = data.get("e").and_then(|v| v.as_str()).unwrap_or("").to_string();
                
                if let Some(filter) = filter_event {
                    if event_type != filter {
                        continue;
                    }
                }

                let label = match event_type.as_str() {
                    "outboundAccountPosition" => "Account Balance Update",
                    "balanceUpdate" => "Asset Balance Update",
                    "executionReport" => "Order Update",
                    _ => event_type.as_str(),
                };

                let output = CommandOutput::new(data, label).with_format(ctx.format);
                println!("{}", output.render());
                data_count += 1;
            }
            Ok(Message::Ping(payload)) => {
                let _ = ws.send(Message::Pong(payload)).await;
            }
            Ok(Message::Close(_)) => {
                eprintln!("{} Connection closed", "WS".yellow().bold());
                break;
            }
            Ok(Message::Binary(_)) => {}
            Err(e) => {
                eprintln!("{} Error: {}", "WS".red().bold(), e);
                break;
            }
            _ => {}
        }

        if bounds.limit_reached(data_count) {
            break;
        }
    }
    Ok(())
}

async fn next_message<S>(
    ws: &mut S,
    deadline: Option<Instant>,
) -> Result<Option<S::Item>, BinanceError>
where
    S: Stream + Unpin,
{
    match deadline {
        Some(deadline) => {
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            timeout(deadline - now, ws.next())
                .await
                .map_or(Ok(None), Ok)
        }
        None => Ok(ws.next().await),
    }
}

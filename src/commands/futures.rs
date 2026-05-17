use clap::Subcommand;
use serde_json::{Value, json};
use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::{normalize_pair, AppContext};
use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Debug, Subcommand)]
pub enum FuturesCommand {
    /// Test connectivity to the Futures REST API
    Ping,

    /// Get the current futures server time
    ServerTime,

    /// Get futures exchange trading rules and symbol information
    ExchangeInfo,

    /// Get futures 24hr ticker price change statistics
    Ticker {
        /// Trading pair symbol (e.g., BTCUSDT)
        pair: String,
    },

    /// Get latest price for a futures symbol
    Price {
        /// Trading pair symbol
        pair: String,
    },

    /// Get futures order book depth
    Orderbook {
        /// Trading pair symbol
        pair: String,

        /// Limit number of price levels (default: 100, max: 1000)
        #[arg(short, long, default_value = "100")]
        count: u32,
    },

    /// Get recent futures trades
    Trades {
        /// Trading pair symbol
        pair: String,

        /// Number of trades to return (default: 500, max: 1000)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    /// Get futures kline/candlestick bars (OHLC)
    Ohlc {
        /// Trading pair symbol (e.g. BTCUSDT)
        pair: String,

        /// Interval (e.g. 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 6h, 8h, 12h, 1d, 3d, 1w, 1M)
        #[arg(short, long, default_value = "1m")]
        interval: String,

        /// Limit number of bars (default: 500, max: 1500)
        #[arg(short, long, default_value = "500")]
        count: u32,
    },

    /// Get futures account information (balances, positions, etc.)
    AccountInfo,

    /// Get futures balances
    Balance,

    /// Get open futures positions
    Positions {
        /// Trading pair symbol (optional)
        #[arg(short, long)]
        pair: Option<String>,
    },

    /// Place a futures order
    Order {
        /// Trading pair symbol (e.g., BTCUSDT)
        pair: String,

        /// Direction: BUY or SELL
        #[arg(short, long)]
        side: String,

        /// Order type: LIMIT, MARKET, STOP, TAKE_PROFIT, STOP_MARKET, TAKE_PROFIT_MARKET, TRAILING_STOP_MARKET
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order volume
        #[arg(short, long)]
        volume: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Time in force (GTC, IOC, FOK, GTX)
        #[arg(long, default_value = "GTC")]
        time_in_force: String,

        /// Stop price (for STOP/TAKE_PROFIT orders)
        #[arg(long)]
        stop_price: Option<String>,

        /// Reduce only
        #[arg(long)]
        reduce_only: bool,
    },

    /// Cancel a futures order
    Cancel {
        /// Trading pair symbol
        pair: String,

        /// Order ID to cancel
        #[arg(long)]
        order_id: Option<u64>,

        /// Client order ID to cancel
        #[arg(long)]
        client_order_id: Option<String>,
    },

    /// Cancel all open futures orders
    CancelAll {
        /// Trading pair symbol
        pair: String,
    },
}

impl FuturesCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let futures_host = "https://fapi.binance.com";

        match self {
            Self::Ping => {
                let url = format!("{}/fapi/v1/ping", futures_host);
                let _ = self.raw_get(&url).await?;
                Ok(CommandOutput::new(json!({ "status": "ok" }), "Futures Ping")
                    .with_addendum("Binance Futures API is reachable"))
            }
            Self::ServerTime => {
                let url = format!("{}/fapi/v1/time", futures_host);
                let result = self.raw_get(&url).await?;
                let ts = result["serverTime"].as_u64().unwrap_or(0);
                let dt = chrono::DateTime::from_timestamp_millis(ts as i64)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| ts.to_string());
                Ok(CommandOutput::new(result, "Futures Server Time").with_addendum(format!("{} ({})", dt, ts)))
            }
            Self::ExchangeInfo => {
                let url = format!("{}/fapi/v1/exchangeInfo", futures_host);
                let result = self.raw_get(&url).await?;
                Ok(CommandOutput::new(result, "Futures Exchange Info"))
            }
            Self::Ticker { pair } => {
                let sym = normalize_pair(pair);
                let url = format!("{}/fapi/v1/ticker/24hr?symbol={}", futures_host, sym);
                let result = self.raw_get(&url).await?;
                Ok(CommandOutput::new(result, format!("Futures 24h Ticker — {}", sym)))
            }
            Self::Price { pair } => {
                let sym = normalize_pair(pair);
                let url = format!("{}/fapi/v1/ticker/price?symbol={}", futures_host, sym);
                let result = self.raw_get(&url).await?;
                Ok(CommandOutput::new(result, format!("Futures Price — {}", sym)))
            }
            Self::Orderbook { pair, count } => {
                let sym = normalize_pair(pair);
                let url = format!("{}/fapi/v1/depth?symbol={}&limit={}", futures_host, sym, count);
                let result = self.raw_get(&url).await?;
                Ok(CommandOutput::new(result, format!("Futures Order Book — {}", sym)))
            }
            Self::Trades { pair, limit } => {
                let sym = normalize_pair(pair);
                let url = format!("{}/fapi/v1/trades?symbol={}&limit={}", futures_host, sym, limit);
                let result = self.raw_get(&url).await?;
                Ok(CommandOutput::new(result, format!("Futures Recent Trades — {}", sym)))
            }
            Self::Ohlc { pair, interval, count } => {
                let sym = normalize_pair(pair);
                let url = format!("{}/fapi/v1/klines?symbol={}&interval={}&limit={}", futures_host, sym, interval, count);
                let result = self.raw_get(&url).await?;
                Ok(CommandOutput::new(result, format!("Futures Klines — {}", sym)))
            }
            Self::AccountInfo => {
                let result = self.private_get(ctx, "/fapi/v2/account", &[]).await?;
                Ok(CommandOutput::new(result, "Futures Account Info"))
            }
            Self::Balance => {
                let result = self.private_get(ctx, "/fapi/v2/balance", &[]).await?;
                Ok(CommandOutput::new(result, "Futures Balances"))
            }
            Self::Positions { pair } => {
                let mut params = Vec::new();
                let sym;
                if let Some(p) = pair {
                    sym = normalize_pair(p);
                    params.push(("symbol", sym.as_str()));
                }
                let result = self.private_get(ctx, "/fapi/v2/positionRisk", &params).await?;
                Ok(CommandOutput::new(result, "Futures Positions"))
            }
            Self::Order { pair, side, r#type, volume, price, time_in_force, stop_price, reduce_only } => {
                let mut params = Vec::new();
                let sym = normalize_pair(pair);
                params.push(("symbol", sym.as_str()));
                params.push(("side", side.as_str()));
                params.push(("type", r#type.as_str()));
                params.push(("quantity", volume.as_str()));
                
                if r#type.to_uppercase() == "LIMIT" {
                    params.push(("timeInForce", time_in_force.as_str()));
                    if let Some(p) = price {
                        params.push(("price", p.as_str()));
                    }
                }
                
                if let Some(sp) = stop_price {
                    params.push(("stopPrice", sp.as_str()));
                }
                
                let ro_str = reduce_only.to_string();
                if *reduce_only {
                    params.push(("reduceOnly", ro_str.as_str()));
                }
                
                let result = self.private_post(ctx, "/fapi/v1/order", &params).await?;
                Ok(CommandOutput::new(result, "Futures Order Result"))
            }
            Self::Cancel { pair, order_id, client_order_id } => {
                let mut params = Vec::new();
                let sym = normalize_pair(pair);
                params.push(("symbol", sym.as_str()));
                let oid_str;
                if let Some(oid) = order_id {
                    oid_str = oid.to_string();
                    params.push(("orderId", oid_str.as_str()));
                }
                if let Some(coid) = client_order_id {
                    params.push(("origClientOrderId", coid.as_str()));
                }
                let result = self.private_delete(ctx, "/fapi/v1/order", &params).await?;
                Ok(CommandOutput::new(result, "Futures Cancel Result"))
            }
            Self::CancelAll { pair } => {
                let mut params = Vec::new();
                let sym = normalize_pair(pair);
                params.push(("symbol", sym.as_str()));
                let result = self.private_delete(ctx, "/fapi/v1/allOpenOrders", &params).await?;
                Ok(CommandOutput::new(result, "Futures Cancel All Result"))
            }
        }
    }

    async fn raw_get(&self, url: &str) -> Result<Value, BinanceError> {
        let resp = reqwest::get(url).await.map_err(|e| BinanceError::Http(e.to_string()))?;
        let body = resp.text().await.map_err(|e| BinanceError::Http(e.to_string()))?;
        let val: Value = serde_json::from_str(&body).map_err(|e| BinanceError::Json(e.to_string()))?;
        Ok(val)
    }

    async fn private_get(&self, ctx: &AppContext, path: &str, params: &[(&str, &str)]) -> Result<Value, BinanceError> {
        self.private_request(ctx, "GET", path, params).await
    }

    async fn private_post(&self, ctx: &AppContext, path: &str, params: &[(&str, &str)]) -> Result<Value, BinanceError> {
        self.private_request(ctx, "POST", path, params).await
    }

    async fn private_delete(&self, ctx: &AppContext, path: &str, params: &[(&str, &str)]) -> Result<Value, BinanceError> {
        self.private_request(ctx, "DELETE", path, params).await
    }

    async fn private_request(&self, ctx: &AppContext, method: &str, path: &str, params: &[(&str, &str)]) -> Result<Value, BinanceError> {
        let creds = ctx.client.require_credentials()?;
        let timestamp = chrono::Utc::now().timestamp_millis();
        
        let mut query_params = Vec::new();
        for (k, v) in params {
            query_params.push(format!("{}={}", k, v));
        }
        query_params.push(format!("timestamp={}", timestamp));
        
        let sign_payload = query_params.join("&");
        
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(creds.api_secret.as_bytes())
            .map_err(|e| BinanceError::Other(e.to_string()))?;
        mac.update(sign_payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        
        let full_url = format!("https://fapi.binance.com{}?{}&signature={}", path, sign_payload, signature);
        
        let req_client = reqwest::Client::new();
        let builder = match method {
            "GET" => req_client.get(&full_url),
            "POST" => req_client.post(&full_url),
            "DELETE" => req_client.delete(&full_url),
            _ => return Err(BinanceError::Other(format!("Unsupported method: {}", method))),
        };
        
        let resp = builder
            .header("X-MBX-APIKEY", &creds.api_key)
            .send()
            .await
            .map_err(|e| BinanceError::Http(e.to_string()))?;
            
        let body = resp.text().await.map_err(|e| BinanceError::Http(e.to_string()))?;
        let val: Value = serde_json::from_str(&body).map_err(|e| BinanceError::Json(e.to_string()))?;
        
        if val.get("code").is_some() && val.get("msg").is_some() {
            let msg = val["msg"].as_str().unwrap_or("Unknown API error");
            return Err(BinanceError::Api(msg.to_string()));
        }
        
        Ok(val)
    }
}

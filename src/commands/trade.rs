use std::str::FromStr;
use clap::Subcommand;
use binance_spot_connector_rust::trade::{self, order::{Side, TimeInForce}};
use rust_decimal::Decimal;

use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::AppContext;

#[derive(Debug, Subcommand)]
pub enum TradeCommand {
    /// Place a buy order
    Buy {
        /// Trading pair symbol (e.g., BTCUSDT)
        symbol: String,

        /// Order type: LIMIT or MARKET
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Order quantity
        #[arg(short, long)]
        quantity: String,

        /// Client order ID (optional)
        #[arg(long)]
        client_order_id: Option<String>,
    },

    /// Place a sell order
    Sell {
        /// Trading pair symbol (e.g., BTCUSDT)
        symbol: String,

        /// Order type: LIMIT or MARKET
        #[arg(short = 't', long, default_value = "LIMIT")]
        r#type: String,

        /// Order price (required for LIMIT orders)
        #[arg(short, long)]
        price: Option<String>,

        /// Order quantity
        #[arg(short, long)]
        quantity: String,

        /// Client order ID (optional)
        #[arg(long)]
        client_order_id: Option<String>,
    },

    /// Cancel an active order
    Cancel {
        /// Trading pair symbol
        symbol: String,

        /// Order ID to cancel
        #[arg(long)]
        order_id: u64,
    },

    /// Cancel all active orders for a symbol
    CancelAll {
        /// Trading pair symbol
        symbol: String,
    },

    /// Query a specific order's status
    Query {
        /// Trading pair symbol
        symbol: String,

        /// Order ID to query
        #[arg(long)]
        order_id: u64,
    },

    /// List current open orders
    OpenOrders {
        /// Trading pair symbol (optional)
        #[arg(short, long)]
        symbol: Option<String>,
    },

    /// List all orders (active, canceled, filled)
    AllOrders {
        /// Trading pair symbol
        symbol: String,

        /// Get orders >= this order ID (optional)
        #[arg(long)]
        order_id: Option<u64>,

        /// Maximum number of orders (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: u32,
    },
}

impl TradeCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let client = &ctx.client;
        let creds = client.require_credentials()?;
        let binance_creds = creds.to_binance_credentials();

        let output = match self {
            Self::Buy {
                symbol,
                r#type,
                price,
                quantity,
                client_order_id,
            } => {
                self.place_order(
                    ctx,
                    symbol,
                    Side::Buy,
                    r#type,
                    price.as_deref(),
                    quantity,
                    client_order_id.as_deref(),
                    &binance_creds,
                )
                .await?
            }

            Self::Sell {
                symbol,
                r#type,
                price,
                quantity,
                client_order_id,
            } => {
                self.place_order(
                    ctx,
                    symbol,
                    Side::Sell,
                    r#type,
                    price.as_deref(),
                    quantity,
                    client_order_id.as_deref(),
                    &binance_creds,
                )
                .await?
            }

            Self::Cancel { symbol, order_id } => {
                let sym = symbol.to_uppercase();
                let request = trade::cancel_order(&sym)
                    .order_id(*order_id)
                    .credentials(&binance_creds);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, "Cancel Result")
                    .with_addendum(format!("Order {} cancelled", order_id))
            }

            Self::CancelAll { symbol } => {
                let sym = symbol.to_uppercase();
                let request = trade::cancel_open_orders(&sym)
                    .credentials(&binance_creds);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Cancel All Open Orders — {}", sym))
                    .with_addendum(format!("All open orders for {} cancelled successfully", sym))
            }

            Self::Query { symbol, order_id } => {
                let sym = symbol.to_uppercase();
                let request = trade::get_order(&sym)
                    .order_id(*order_id)
                    .credentials(&binance_creds);
                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("Order {} — {}", order_id, sym))
            }

            Self::OpenOrders { symbol } => {
                let mut request = trade::open_orders().credentials(&binance_creds);
                let title = if let Some(sym) = symbol {
                    let sym_upper = sym.to_uppercase();
                    request = request.symbol(&sym_upper);
                    format!("Open Orders — {}", sym_upper)
                } else {
                    "Open Orders (All Symbols)".to_string()
                };

                let result = client.send_request(request).await?;
                CommandOutput::new(result, title)
            }

            Self::AllOrders { symbol, order_id, limit } => {
                let sym = symbol.to_uppercase();
                let mut request = trade::all_orders(&sym)
                    .limit(*limit)
                    .credentials(&binance_creds);
                if let Some(oid) = order_id {
                    request = request.order_id(*oid);
                }

                let result = client.send_request(request).await?;
                CommandOutput::new(result, format!("All Orders — {}", sym))
            }
        };

        Ok(output.with_format(ctx.format))
    }

    #[allow(clippy::too_many_arguments)]
    async fn place_order(
        &self,
        ctx: &AppContext,
        symbol: &str,
        side: Side,
        order_type: &str,
        price: Option<&str>,
        quantity: &str,
        client_order_id: Option<&str>,
        binance_creds: &binance_spot_connector_rust::http::Credentials,
    ) -> Result<CommandOutput, BinanceError> {
        let sym = symbol.to_uppercase();
        let otype = order_type.to_uppercase();

        let qty_dec = Decimal::from_str(quantity).map_err(|e| {
            BinanceError::Validation(format!("Invalid quantity '{}': {}", quantity, e))
        })?;

        let mut request = trade::new_order(&sym, side, &otype)
            .quantity(qty_dec)
            .credentials(binance_creds);

        if otype == "LIMIT" {
            let price_str = price.ok_or_else(|| {
                BinanceError::Validation("Price is required for LIMIT orders".to_string())
            })?;
            let price_dec = Decimal::from_str(price_str).map_err(|e| {
                BinanceError::Validation(format!("Invalid price '{}': {}", price_str, e))
            })?;
            request = request.price(price_dec).time_in_force(TimeInForce::Gtc);
        }

        if let Some(coid) = client_order_id {
            request = request.new_client_order_id(coid);
        }

        let result = ctx.client.send_request(request).await?;

        let side_str = side.to_string();
        let mut output = CommandOutput::new(result.clone(), "Order Result");
        if let Some(order_id) = result.get("orderId") {
            let actual_price = result
                .get("price")
                .and_then(|v| v.as_str())
                .unwrap_or(price.unwrap_or("MARKET"));

            output = output.with_addendum(format!(
                "{} {} {} @ {} — Order ID: {}",
                side_str, quantity, sym, actual_price, order_id
            ));
        }

        Ok(output.with_format(ctx.format))
    }
}

use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::errors::BinanceError;
use crate::output::CommandOutput;
use crate::AppContext;

const DEFAULT_QUOTE_BALANCE: f64 = 10_000.0;
const DEFAULT_BASE_BALANCE: f64 = 1.0;
const DEFAULT_PAIR: &str = "BTCUSDT";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOrder {
    pub id: u64,
    pub pair: String,
    pub side: String,
    pub price: Option<f64>,
    pub volume: f64,
    pub status: String,
    pub created_at: u64,
    #[serde(default)]
    pub filled_at: Option<u64>,
    #[serde(default)]
    pub filled_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperState {
    pub balances: HashMap<String, f64>,
    pub orders: Vec<PaperOrder>,
    pub next_order_id: u64,
}

impl Default for PaperState {
    fn default() -> Self {
        let mut balances = HashMap::new();
        balances.insert("USDT".to_string(), DEFAULT_QUOTE_BALANCE);
        balances.insert("BTC".to_string(), DEFAULT_BASE_BALANCE);
        balances.insert("BNB".to_string(), 10.0);
        Self {
            balances,
            orders: Vec::new(),
            next_order_id: 1,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum PaperCommand {
    /// Initialize paper trading with default or custom balances
    Init {
        #[arg(long, default_value = DEFAULT_PAIR)]
        pair: String,
        #[arg(long)]
        quote_balance: Option<f64>,
        #[arg(long)]
        base_balance: Option<f64>,
    },
    /// Reset paper trading state
    Reset,
    /// Add balance to one asset
    Topup { asset: String, amount: f64 },
    /// Show paper trading balances
    Balance,
    /// Place a simulated buy order
    Buy {
        pair: String,
        #[arg(short, long)]
        price: Option<f64>,
        #[arg(long)]
        volume: f64,
        #[arg(long)]
        fill: bool,
    },
    /// Place a simulated sell order
    Sell {
        pair: String,
        #[arg(short, long)]
        price: Option<f64>,
        #[arg(long)]
        volume: f64,
        #[arg(long)]
        fill: bool,
    },
    /// List paper orders
    Orders {
        #[arg(short, long)]
        pair: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Cancel one open paper order
    Cancel { order_id: u64 },
    /// Cancel all open paper orders
    CancelAll {
        #[arg(short, long)]
        pair: Option<String>,
    },
    /// Fill one open paper order
    Fill {
        order_id: u64,
        #[arg(short, long)]
        price: Option<f64>,
    },
    /// Show all paper orders
    History,
    /// Show paper trading summary
    Status,
}

impl PaperCommand {
    pub async fn execute(&self, ctx: &AppContext) -> Result<CommandOutput, BinanceError> {
        let mut state = PaperState::load()?;
        let mut changed = false;
        let output = match self {
            Self::Init {
                pair,
                quote_balance,
                base_balance,
            } => {
                state = PaperState::for_pair(pair, *quote_balance, *base_balance)?;
                changed = true;
                state.output("Paper Initialized", ctx)
            }
            Self::Reset => {
                state = PaperState::default();
                changed = true;
                state.output("Paper Reset", ctx)
            }
            Self::Topup { asset, amount } => {
                validate_positive(*amount, "amount")?;
                *state.balances.entry(asset.to_uppercase()).or_default() += amount;
                changed = true;
                state.output("Paper Topup", ctx)
            }
            Self::Balance => state.output("Paper Balances", ctx),
            Self::Buy {
                pair,
                price,
                volume,
                fill,
            } => {
                let id = state.place_order(pair, "BUY", *price, *volume)?;
                if *fill {
                    state.fill_order(id, *price)?;
                }
                changed = true;
                state.orders_output("Paper Buy", ctx, None, true)
            }
            Self::Sell {
                pair,
                price,
                volume,
                fill,
            } => {
                let id = state.place_order(pair, "SELL", *price, *volume)?;
                if *fill {
                    state.fill_order(id, *price)?;
                }
                changed = true;
                state.orders_output("Paper Sell", ctx, None, true)
            }
            Self::Orders { pair, all } => {
                state.orders_output("Paper Orders", ctx, pair.as_deref(), *all)
            }
            Self::Cancel { order_id } => {
                state.cancel_order(*order_id)?;
                changed = true;
                state.orders_output("Paper Cancel", ctx, None, true)
            }
            Self::CancelAll { pair } => {
                let count = state.cancel_all(pair.as_deref());
                changed = true;
                state
                    .orders_output("Paper Cancel All", ctx, pair.as_deref(), true)
                    .with_addendum(format!("Cancelled {} paper orders", count))
            }
            Self::Fill { order_id, price } => {
                state.fill_order(*order_id, *price)?;
                changed = true;
                state.orders_output("Paper Fill", ctx, None, true)
            }
            Self::History => state.orders_output("Paper History", ctx, None, true),
            Self::Status => state.status_output(ctx),
        };

        if changed {
            state.save()?;
        }
        Ok(output)
    }
}

impl PaperState {
    fn load() -> Result<Self, BinanceError> {
        let path = Config::paper_state_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path).map_err(|e| {
            BinanceError::Config(format!("Failed to read {}: {}", path.display(), e))
        })?;
        serde_json::from_str(&content).map_err(BinanceError::from)
    }

    fn save(&self) -> Result<(), BinanceError> {
        let path = Config::paper_state_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self).map_err(BinanceError::from)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn for_pair(
        pair: &str,
        quote_balance: Option<f64>,
        base_balance: Option<f64>,
    ) -> Result<Self, BinanceError> {
        let (base, quote) = split_pair(pair)?;
        let mut balances = HashMap::new();
        balances.insert(quote, quote_balance.unwrap_or(DEFAULT_QUOTE_BALANCE));
        balances.insert(base, base_balance.unwrap_or(DEFAULT_BASE_BALANCE));
        Ok(Self {
            balances,
            orders: Vec::new(),
            next_order_id: 1,
        })
    }

    fn place_order(
        &mut self,
        pair: &str,
        side: &str,
        price: Option<f64>,
        volume: f64,
    ) -> Result<u64, BinanceError> {
        validate_positive(volume, "volume")?;
        if let Some(price) = price {
            validate_positive(price, "price")?;
        }
        let pair = normalize_pair(pair);
        split_pair(&pair)?;
        let id = self.next_order_id;
        self.next_order_id += 1;
        self.orders.push(PaperOrder {
            id,
            pair,
            side: side.to_string(),
            price,
            volume,
            status: "OPEN".to_string(),
            created_at: now(),
            filled_at: None,
            filled_price: None,
        });
        Ok(id)
    }

    fn cancel_order(&mut self, order_id: u64) -> Result<(), BinanceError> {
        let order = self
            .orders
            .iter_mut()
            .find(|order| order.id == order_id)
            .ok_or_else(|| {
                BinanceError::Validation(format!("Paper order {} not found", order_id))
            })?;
        if order.status != "OPEN" {
            return Err(BinanceError::Validation(format!(
                "Paper order {} is {}",
                order_id, order.status
            )));
        }
        order.status = "CANCELED".to_string();
        Ok(())
    }

    fn cancel_all(&mut self, pair: Option<&str>) -> usize {
        let pair = pair.map(normalize_pair);
        let mut count = 0;
        for order in &mut self.orders {
            if order.status == "OPEN" && pair.as_ref().map_or(true, |p| p == &order.pair) {
                order.status = "CANCELED".to_string();
                count += 1;
            }
        }
        count
    }

    fn fill_order(&mut self, order_id: u64, price: Option<f64>) -> Result<(), BinanceError> {
        let idx = self
            .orders
            .iter()
            .position(|order| order.id == order_id)
            .ok_or_else(|| {
                BinanceError::Validation(format!("Paper order {} not found", order_id))
            })?;
        if self.orders[idx].status != "OPEN" {
            return Err(BinanceError::Validation(format!(
                "Paper order {} is {}",
                order_id, self.orders[idx].status
            )));
        }
        let fill_price = price.or(self.orders[idx].price).ok_or_else(|| {
            BinanceError::Validation("Fill price is required for market paper orders".to_string())
        })?;
        validate_positive(fill_price, "price")?;

        let pair = self.orders[idx].pair.clone();
        let side = self.orders[idx].side.clone();
        let volume = self.orders[idx].volume;
        let (base, quote) = split_pair(&pair)?;
        let quote_amount = fill_price * volume;

        match side.as_str() {
            "BUY" => {
                withdraw(&mut self.balances, &quote, quote_amount)?;
                *self.balances.entry(base).or_default() += volume;
            }
            "SELL" => {
                withdraw(&mut self.balances, &base, volume)?;
                *self.balances.entry(quote).or_default() += quote_amount;
            }
            _ => {
                return Err(BinanceError::Internal(format!(
                    "Invalid paper side {}",
                    side
                )))
            }
        }

        let order = &mut self.orders[idx];
        order.status = "FILLED".to_string();
        order.filled_at = Some(now());
        order.filled_price = Some(fill_price);
        Ok(())
    }

    fn output(&self, label: &str, ctx: &AppContext) -> CommandOutput {
        let mut rows: Vec<Vec<String>> = self
            .balances
            .iter()
            .map(|(asset, free)| vec![asset.clone(), format_amount(*free)])
            .collect();
        rows.sort_by(|a, b| a[0].cmp(&b[0]));
        CommandOutput::new(serde_json::json!({ "balances": self.balances }), label)
            .with_table(vec!["Asset".into(), "Free".into()], rows)
            .with_format(ctx.format)
    }

    fn orders_output(
        &self,
        label: &str,
        ctx: &AppContext,
        pair: Option<&str>,
        all: bool,
    ) -> CommandOutput {
        let pair = pair.map(normalize_pair);
        let mut orders: Vec<&PaperOrder> = self
            .orders
            .iter()
            .filter(|order| all || order.status == "OPEN")
            .filter(|order| pair.as_ref().map_or(true, |p| p == &order.pair))
            .collect();
        orders.sort_by_key(|order| order.id);
        let rows = orders
            .iter()
            .map(|order| {
                vec![
                    order.id.to_string(),
                    order.pair.clone(),
                    order.side.clone(),
                    order
                        .price
                        .map(format_amount)
                        .unwrap_or_else(|| "MARKET".into()),
                    format_amount(order.volume),
                    order.status.clone(),
                    order.filled_price.map(format_amount).unwrap_or_default(),
                ]
            })
            .collect();
        CommandOutput::new(serde_json::json!({ "orders": orders }), label)
            .with_table(
                vec![
                    "ID".into(),
                    "Pair".into(),
                    "Side".into(),
                    "Price".into(),
                    "Volume".into(),
                    "Status".into(),
                    "Filled Price".into(),
                ],
                rows,
            )
            .with_format(ctx.format)
    }

    fn status_output(&self, ctx: &AppContext) -> CommandOutput {
        let open = self
            .orders
            .iter()
            .filter(|order| order.status == "OPEN")
            .count();
        let filled = self
            .orders
            .iter()
            .filter(|order| order.status == "FILLED")
            .count();
        let canceled = self
            .orders
            .iter()
            .filter(|order| order.status == "CANCELED")
            .count();
        CommandOutput::new(
            serde_json::json!({
                "balances": self.balances,
                "orders_total": self.orders.len(),
                "orders_open": open,
                "orders_filled": filled,
                "orders_canceled": canceled
            }),
            "Paper Status",
        )
        .with_table(
            vec!["Metric".into(), "Value".into()],
            vec![
                vec!["orders_total".into(), self.orders.len().to_string()],
                vec!["orders_open".into(), open.to_string()],
                vec!["orders_filled".into(), filled.to_string()],
                vec!["orders_canceled".into(), canceled.to_string()],
            ],
        )
        .with_format(ctx.format)
    }
}

fn normalize_pair(pair: &str) -> String {
    pair.replace(['_', '-', '/'], "").to_uppercase()
}

fn split_pair(pair: &str) -> Result<(String, String), BinanceError> {
    let normalized = normalize_pair(pair);
    for quote in ["USDT", "USDC", "FDUSD", "BTC", "ETH", "BNB", "EUR", "TRY"] {
        if normalized.ends_with(quote) && normalized.len() > quote.len() {
            let base = normalized.trim_end_matches(quote).to_string();
            return Ok((base, quote.to_string()));
        }
    }
    Err(BinanceError::Validation(format!(
        "Cannot infer base/quote assets from pair {}",
        pair
    )))
}

fn validate_positive(value: f64, name: &str) -> Result<(), BinanceError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(BinanceError::Validation(format!(
            "{} must be positive",
            name
        )))
    }
}

fn withdraw(
    balances: &mut HashMap<String, f64>,
    asset: &str,
    amount: f64,
) -> Result<(), BinanceError> {
    let current = *balances.get(asset).unwrap_or(&0.0);
    if current + f64::EPSILON < amount {
        return Err(BinanceError::Validation(format!(
            "Insufficient {} balance. Need {}, have {}",
            asset,
            format_amount(amount),
            format_amount(current)
        )));
    }
    balances.insert(asset.to_string(), current - amount);
    Ok(())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn format_amount(value: f64) -> String {
    format!("{:.8}", value)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    fn ctx() -> AppContext {
        AppContext {
            client: crate::client::BinanceHttpClient::new("https://api.binance.com", None),
            format: OutputFormat::Json,
            verbose: false,
            yes: false,
        }
    }

    #[tokio::test]
    async fn test_place_and_fill_buy() {
        let mut state = PaperState::for_pair("BTCUSDT", Some(20_000.0), Some(0.0)).unwrap();
        let id = state
            .place_order("BTCUSDT", "BUY", Some(10_000.0), 1.0)
            .unwrap();
        state.fill_order(id, None).unwrap();
        assert_eq!(state.balances["BTC"], 1.0);
        assert_eq!(state.balances["USDT"], 10_000.0);
    }

    #[tokio::test]
    async fn test_balance_output() {
        let cmd = PaperCommand::Balance;
        let res = cmd.execute(&ctx()).await.unwrap();
        assert_eq!(res.label, "Paper Balances");
    }
}

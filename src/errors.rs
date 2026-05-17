use thiserror::Error;
use binance_spot_connector_rust::hyper::Error as BinanceConnectorError;
use binance_spot_connector_rust::http::error::ClientError;

/// Structured error type for the Binance CLI.
/// Maps to a stable `error` category in JSON error envelopes.
#[derive(Debug, Error)]
pub enum BinanceError {
    #[error("API error ({code}): {message}")]
    Api { code: i64, message: String },

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limited: {0}")]
    RateLimit(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl BinanceError {
    /// Returns the stable error category string for JSON envelopes.
    pub fn category(&self) -> &'static str {
        match self {
            BinanceError::Api { .. } => "api",
            BinanceError::Auth(_) => "auth",
            BinanceError::Network(_) => "network",
            BinanceError::Validation(_) => "validation",
            BinanceError::RateLimit(_) => "rate_limit",
            BinanceError::Config(_) => "config",
            BinanceError::Io(_) => "io",
            BinanceError::Parse(_) => "parse",
            BinanceError::WebSocket(_) => "websocket",
            BinanceError::Internal(_) => "internal",
        }
    }

    /// Whether this error is retryable.
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            BinanceError::Network(_) | BinanceError::RateLimit(_) | BinanceError::WebSocket(_)
        )
    }

    /// Format this error as a JSON error envelope.
    pub fn to_json_envelope(&self) -> serde_json::Value {
        serde_json::json!({
            "error": true,
            "error_type": self.category(),
            "message": self.to_string(),
            "retryable": self.retryable(),
        })
    }

    pub fn to_pretty_string(&self) -> String {
        use colored::Colorize;
        format!("{} {}", "Error:".red().bold(), self)
    }

    pub fn print_pretty(&self) {
        eprintln!("{}", self.to_pretty_string());
    }
}

impl From<BinanceConnectorError> for BinanceError {
    fn from(e: BinanceConnectorError) -> Self {
        match e {
            BinanceConnectorError::Client(ClientError::Structured(err)) => {
                let code = err.data.code as i64;
                let message = err.data.message.clone();
                if code == -1003 || err.status_code == 429 {
                    BinanceError::RateLimit(message)
                } else if err.status_code == 401 || err.status_code == 403 {
                    BinanceError::Auth(message)
                } else {
                    BinanceError::Api { code, message }
                }
            }
            BinanceConnectorError::Client(ClientError::Raw(err)) => {
                let code = -(err.status_code as i64);
                let message = err.data.clone();
                if err.status_code == 429 {
                    BinanceError::RateLimit(message)
                } else if err.status_code == 401 || err.status_code == 403 {
                    BinanceError::Auth(message)
                } else {
                    BinanceError::Api { code, message }
                }
            }
            BinanceConnectorError::Server(err) => {
                BinanceError::Network(format!("Server error ({}): {}", err.status_code, err.data))
            }
            BinanceConnectorError::InvalidApiSecret => {
                BinanceError::Auth("Invalid API secret".to_string())
            }
            BinanceConnectorError::Parse(err) => BinanceError::Parse(err.to_string()),
            BinanceConnectorError::Send(err) => BinanceError::Network(err.to_string()),
        }
    }
}

impl From<serde_json::Error> for BinanceError {
    fn from(e: serde_json::Error) -> Self {
        BinanceError::Parse(e.to_string())
    }
}

impl From<url::ParseError> for BinanceError {
    fn from(e: url::ParseError) -> Self {
        BinanceError::Parse(e.to_string())
    }
}

impl From<anyhow::Error> for BinanceError {
    fn from(e: anyhow::Error) -> Self {
        BinanceError::Internal(e.to_string())
    }
}

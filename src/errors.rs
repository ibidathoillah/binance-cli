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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        assert_eq!(BinanceError::Api { code: 1, message: "err".to_string() }.category(), "api");
        assert_eq!(BinanceError::Auth("err".to_string()).category(), "auth");
        assert_eq!(BinanceError::Network("err".to_string()).category(), "network");
        assert_eq!(BinanceError::Validation("err".to_string()).category(), "validation");
        assert_eq!(BinanceError::RateLimit("err".to_string()).category(), "rate_limit");
        assert_eq!(BinanceError::Config("err".to_string()).category(), "config");
        assert_eq!(BinanceError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")).category(), "io");
        assert_eq!(BinanceError::Parse("err".to_string()).category(), "parse");
        assert_eq!(BinanceError::WebSocket("err".to_string()).category(), "websocket");
        assert_eq!(BinanceError::Internal("err".to_string()).category(), "internal");
    }

    #[test]
    fn test_error_retryable() {
        assert!(BinanceError::Network("err".to_string()).retryable());
        assert!(BinanceError::RateLimit("err".to_string()).retryable());
        assert!(BinanceError::WebSocket("err".to_string()).retryable());
        assert!(!BinanceError::Auth("err".to_string()).retryable());
    }

    #[test]
    fn test_to_json_envelope() {
        let err = BinanceError::Auth("auth failed".to_string());
        let env = err.to_json_envelope();
        assert_eq!(env["error"], true);
        assert_eq!(env["error_type"], "auth");
        assert_eq!(env["message"], "Authentication failed: auth failed");
        assert_eq!(env["retryable"], false);
    }

    #[test]
    fn test_to_pretty_string() {
        let err = BinanceError::Validation("invalid symbol".to_string());
        let pretty = err.to_pretty_string();
        assert!(pretty.contains("Error:"));
        assert!(pretty.contains("Validation error: invalid symbol"));
    }

    #[test]
    fn test_conversions() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let binance_err: BinanceError = json_err.into();
        assert_eq!(binance_err.category(), "parse");

        let url_err = url::Url::parse("invalid").unwrap_err();
        let binance_err: BinanceError = url_err.into();
        assert_eq!(binance_err.category(), "parse");

        let anyhow_err = anyhow::anyhow!("some error");
        let binance_err: BinanceError = anyhow_err.into();
        assert_eq!(binance_err.category(), "internal");
    }
}

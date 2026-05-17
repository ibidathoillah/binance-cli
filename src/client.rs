use binance_spot_connector_rust::hyper::BinanceHttpClient as HyperBinanceHttpClient;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use serde_json::Value;

use crate::config::Credentials;
use crate::errors::BinanceError;

pub type CustomHttpClient = HyperBinanceHttpClient<HttpsConnector<HttpConnector>>;

/// HTTP client wrapper for the Binance REST API.
#[derive(Clone)]
pub struct BinanceHttpClient {
    http: CustomHttpClient,
    host: String,
    credentials: Option<Credentials>,
}

impl BinanceHttpClient {
    /// Create a new client with optional credentials.
    pub fn new(host: &str, credentials: Option<Credentials>) -> Self {
        let mut http = CustomHttpClient::with_url(host);
        if let Some(ref creds) = credentials {
            http = http.credentials(creds.to_binance_credentials());
        }

        Self {
            http,
            host: host.to_string(),
            credentials,
        }
    }

    /// Get the base host URL.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Get the configured API key, if available.
    pub fn api_key(&self) -> Option<&str> {
        self.credentials
            .as_ref()
            .map(|creds| creds.api_key.as_str())
    }

    /// Send a request to Binance API.
    pub async fn send_request<R>(&self, request: R) -> Result<Value, BinanceError>
    where
        R: Into<binance_spot_connector_rust::http::request::Request>,
    {
        let resp = self.http.send(request).await?;
        let body_str = resp.into_body_str().await?;
        if body_str.trim().is_empty() {
            return Ok(Value::Null);
        }

        let val: Value = serde_json::from_str(&body_str)?;
        Ok(val)
    }

    /// Require credentials or return an error.
    pub fn require_credentials(&self) -> Result<&Credentials, BinanceError> {
        self.credentials.as_ref().ok_or_else(|| {
            BinanceError::Auth("API credentials required for this endpoint".to_string())
        })
    }
}

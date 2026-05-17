pub mod json;
pub mod table;

use serde::Serialize;
use serde_json::Value;

/// Output format for CLI responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            _ => Ok(Self::Table),
        }
    }
}

/// Structured output from a command, to be rendered later.
#[derive(Debug, Serialize)]
pub struct CommandOutput {
    pub data: Value,
    pub label: String,
    #[serde(skip)]
    pub headers: Vec<String>,
    #[serde(skip)]
    pub rows: Vec<Vec<String>>,
    #[serde(skip)]
    pub format: OutputFormat,
    #[serde(skip)]
    pub addendum: Option<String>,
}

impl CommandOutput {
    /// Create a new output with raw JSON data and a label for tables.
    pub fn new(data: Value, label: impl Into<String>) -> Self {
        Self {
            data,
            label: label.into(),
            headers: Vec::new(),
            rows: Vec::new(),
            format: OutputFormat::Table,
            addendum: None,
        }
    }

    /// Add explicit table data for better table rendering.
    pub fn with_table(mut self, headers: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        self.headers = headers;
        self.rows = rows;
        self
    }

    /// Set the output format.
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    /// Add an extra message to be shown after the main data.
    pub fn with_addendum(mut self, addendum: impl Into<String>) -> Self {
        self.addendum = Some(addendum.into());
        self
    }

    /// Render the output to a string based on the chosen format.
    pub fn render(&self) -> String {
        match self.format {
            OutputFormat::Json => json::render(self),
            OutputFormat::Table => table::render(self),
        }
    }
}

/// Helper for success messages.
pub fn success(message: impl Into<String>) -> CommandOutput {
    let msg = message.into();
    CommandOutput::new(
        serde_json::json!({ "success": true, "message": msg }),
        "Success",
    )
    .with_addendum(msg)
}

/// Render errors consistently with the selected output format.
pub fn render_error(format: OutputFormat, err: &crate::errors::BinanceError) {
    match format {
        OutputFormat::Json => match serde_json::to_string_pretty(&err.to_json_envelope()) {
            Ok(s) => println!("{}", s),
            Err(_) => eprintln!("Error: {}", err),
        },
        OutputFormat::Table => eprintln!("{}", err.to_pretty_string()),
    }
}

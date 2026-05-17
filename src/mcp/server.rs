use anyhow::Result;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ErrorCode, Implementation, InitializeResult,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData as McpError;
use rmcp::ServerHandler;

use crate::client::BinanceHttpClient;
use crate::config::Config;
use crate::dispatch;
use crate::output::OutputFormat;
use crate::{AppContext, Cli};

use super::registry::ToolRegistry;

pub struct BinanceMcpService {
    client: BinanceHttpClient,
    registry: ToolRegistry,
}

impl BinanceMcpService {
    pub fn new(client: BinanceHttpClient, _config: Config) -> Self {
        Self {
            client,
            registry: ToolRegistry::build(),
        }
    }
}

impl ServerHandler for BinanceMcpService {
    fn get_info(&self) -> InitializeResult {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "binance-cli",
                env!("CARGO_PKG_VERSION"),
            ))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = self.registry.definitions();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        req: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let entry = self.registry.get_tool(&req.name).ok_or_else(|| {
            McpError::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Tool not found: {}", req.name),
                None,
            )
        })?;

        let arguments = req.arguments.unwrap_or_default();

        // Construct CLI args from tool parameters
        let mut args = vec!["binance".to_string()];
        args.extend(entry.command_path.clone());

        for arg_meta in &entry.args_meta {
            if let Some(val) = arguments.get(&arg_meta.id) {
                if arg_meta.is_bool_flag {
                    if val.as_bool().unwrap_or(false) {
                        args.push(format!("--{}", arg_meta.long.as_ref().unwrap()));
                    }
                } else if let Some(long) = &arg_meta.long {
                    args.push(format!("--{}", long));
                    let val_str = if let Some(s) = val.as_str() {
                        s.to_string()
                    } else if val.is_number() {
                        val.to_string()
                    } else {
                        "".to_string()
                    };
                    args.push(val_str);
                } else {
                    // Positional
                    let val_str = if let Some(s) = val.as_str() {
                        s.to_string()
                    } else if val.is_number() {
                        val.to_string()
                    } else {
                        "".to_string()
                    };
                    args.push(val_str);
                }
            }
        }

        // Always output JSON for MCP
        args.push("-o".to_string());
        args.push("json".to_string());

        let cli = match <Cli as clap::Parser>::try_parse_from(&args) {
            Ok(c) => c,
            Err(e) => {
                return Ok(CallToolResult::error(vec![Content::text(e.to_string())]));
            }
        };

        let ctx = AppContext {
            client: self.client.clone(),
            format: OutputFormat::Json,
            verbose: false,
            yes: false,
        };

        match dispatch(&ctx, cli.command).await {
            Ok(output) => Ok(CallToolResult::success(vec![Content::text(
                output.render(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                e.to_pretty_string(),
            )])),
        }
    }
}

pub async fn run(client: BinanceHttpClient, config: Config) -> Result<()> {
    let service = BinanceMcpService::new(client, config);

    rmcp::service::serve_server(service, rmcp::transport::io::stdio())
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_get_info() {
        let client = BinanceHttpClient::new("h".into(), None);
        let service = BinanceMcpService::new(client, Config::default());
        let info = service.get_info();
        assert_eq!(info.server_info.name, "binance-cli");
    }

    #[tokio::test]
    async fn test_mcp_list_tools() {
        let client = BinanceHttpClient::new("h".into(), None);
        let service = BinanceMcpService::new(client, Config::default());
        let tools = service.registry.definitions();
        assert!(!tools.is_empty());
    }
}

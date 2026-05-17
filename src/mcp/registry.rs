use std::collections::HashMap;
use std::sync::Arc;

use clap::CommandFactory;
use rmcp::model::Tool;
use serde_json::{Map, Value};

use super::schema::{clap_command_to_schema, is_mcp_excluded_arg, is_mcp_excluded_command};
use crate::Cli;

#[derive(Debug, Clone)]
pub struct ArgMeta {
    pub id: String,
    pub long: Option<String>,
    pub is_bool_flag: bool,
    pub positional_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ToolEntry {
    pub tool: Tool,
    pub command_path: Vec<String>,
    pub args_meta: Vec<ArgMeta>,
}

pub struct ToolRegistry {
    tools: Vec<ToolEntry>,
    by_name: HashMap<String, usize>,
}

impl ToolRegistry {
    pub fn build() -> Self {
        let mut tools = Vec::new();
        let mut by_name = HashMap::new();
        let clap_root = Cli::command();

        collect_tools(&clap_root, &[], &mut tools);

        for (i, entry) in tools.iter().enumerate() {
            by_name.insert(entry.tool.name.to_string(), i);
        }

        Self { tools, by_name }
    }

    pub fn tools(&self) -> &[ToolEntry] {
        &self.tools
    }

    pub fn get_tool(&self, name: &str) -> Option<&ToolEntry> {
        self.by_name.get(name).map(|&i| &self.tools[i])
    }

    pub fn definitions(&self) -> Vec<Tool> {
        self.tools.iter().map(|e| e.tool.clone()).collect()
    }
}

fn collect_tools(cmd: &clap::Command, parent_path: &[String], out: &mut Vec<ToolEntry>) {
    let subs: Vec<_> = cmd.get_subcommands().collect();

    if subs.is_empty() && !parent_path.is_empty() {
        let key = parent_path.join("-");
        if is_mcp_excluded_command(&key) {
            return;
        }

        let name = format!("binance_{}", key.replace('-', "_"));
        let description = cmd.get_about().map(|a| a.to_string()).unwrap_or_default();
        let schema = clap_command_to_schema(cmd);
        let schema_obj: Map<String, Value> = schema.as_object().cloned().unwrap_or_default();

        let tool = Tool::new(name, description, Arc::new(schema_obj));
        let args_meta = extract_arg_meta(cmd);

        out.push(ToolEntry {
            tool,
            command_path: parent_path.to_vec(),
            args_meta,
        });
        return;
    }

    for sub in subs {
        let sub_name = sub.get_name();
        if sub_name == "help" {
            continue;
        }
        let mut path = parent_path.to_vec();
        path.push(sub_name.to_string());
        collect_tools(sub, &path, out);
    }
}

fn extract_arg_meta(cmd: &clap::Command) -> Vec<ArgMeta> {
    let mut meta = Vec::new();
    let mut pos_idx = 0;

    for arg in cmd.get_arguments() {
        let id = arg.get_id().as_str();
        if is_mcp_excluded_arg(id) {
            continue;
        }

        let long = arg.get_long().map(|s| s.to_string());
        let is_bool = !arg.get_action().takes_values();
        let is_pos = long.is_none() && arg.get_short().is_none();

        let positional_index = if is_pos {
            let idx = pos_idx;
            pos_idx += 1;
            Some(idx)
        } else {
            None
        };

        meta.push(ArgMeta {
            id: id.to_string(),
            long,
            is_bool_flag: is_bool,
            positional_index,
        });
    }
    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_build() {
        let registry = ToolRegistry::build();
        assert!(!registry.tools().is_empty());

        // Find binance_market_price
        let entry = registry.get_tool("binance_market_price");
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.command_path, vec!["market", "price"]);
        assert!(entry.tool.description.is_some());
    }
}

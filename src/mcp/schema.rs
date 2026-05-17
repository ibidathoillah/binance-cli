use clap::Command;
use serde_json::{json, Map, Value};

/// Converts a clap::Command into an MCP-compatible JSON schema.
pub fn clap_command_to_schema(cmd: &Command) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for arg in cmd.get_arguments() {
        let id = arg.get_id().as_str();
        if id == "help" || id == "version" {
            continue;
        }

        let mut prop = Map::new();
        let description = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
        prop.insert("description".to_string(), json!(description));

        let is_bool = !arg.get_action().takes_values();
        if is_bool {
            prop.insert("type".to_string(), json!("boolean"));
        } else {
            prop.insert("type".to_string(), json!("string"));
        }

        properties.insert(id.to_string(), Value::Object(prop));

        if arg.is_required_set() {
            required.push(id.to_string());
        }
    }

    json!({
        "type": "object",
        "properties": properties,
        "required": required,
    })
}

pub fn is_mcp_excluded_command(key: &str) -> bool {
    matches!(key, "shell" | "mcp" | "ws" | "auth" | "paper")
}

pub fn is_mcp_excluded_arg(id: &str) -> bool {
    matches!(
        id,
        "help" | "version" | "output" | "api_key" | "api_secret" | "host"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Arg;

    #[test]
    fn test_clap_to_schema() {
        use clap::ArgAction;
        let cmd = Command::new("test")
            .arg(Arg::new("foo").help("help foo").required(true))
            .arg(Arg::new("bar").help("help bar").action(ArgAction::SetTrue));

        let schema = clap_command_to_schema(&cmd);
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"].get("foo").is_some());
        assert_eq!(schema["properties"]["foo"]["type"], "string");
        assert_eq!(schema["properties"]["bar"]["type"], "boolean");
        assert_eq!(schema["required"][0], "foo");
    }
}

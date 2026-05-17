use super::CommandOutput;

/// Render the command output as a JSON envelope.
pub fn render(output: &CommandOutput) -> String {
    if output.label.is_empty() && output.data.is_null() && output.addendum.is_none() {
        return String::new();
    }

    let mut envelope = serde_json::json!({
        "success": true,
        "data": output.data,
    });

    if let Some(ref addendum) = output.addendum {
        envelope["addendum"] = serde_json::Value::String(addendum.clone());
    }

    serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".to_string())
}

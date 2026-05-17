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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_render_empty() {
        let output = CommandOutput::new(serde_json::Value::Null, "");
        assert_eq!(render(&output), "");
    }

    #[test]
    fn test_json_render_data() {
        let output = CommandOutput::new(json!({"val": 42}), "Label");
        let rendered = render(&output);
        assert!(rendered.contains("\"success\": true"));
        assert!(rendered.contains("\"val\": 42"));
    }

    #[test]
    fn test_json_render_addendum() {
        let output = CommandOutput::new(json!({"val": 42}), "Label")
            .with_addendum("all good");
        let rendered = render(&output);
        assert!(rendered.contains("\"addendum\": \"all good\""));
    }
}

use super::CommandOutput;
use colored::Colorize;
use comfy_table::{Cell, Color, ContentArrangement, Table};
use serde_json::Value;

/// Render the command output as a human-readable table or formatted text.
pub fn render(output: &CommandOutput) -> String {
    if output.label.is_empty() && output.data.is_null() && output.addendum.is_none() {
        return String::new();
    }

    let mut result = String::new();

    // 1. If explicit headers/rows are provided, use them
    if !output.headers.is_empty() && !output.rows.is_empty() {
        if !output.label.is_empty() {
            result.push_str(&format!("{}\n", output.label.bold()));
        }

        let mut table = Table::new();
        table.set_content_arrangement(ContentArrangement::Dynamic);

        let header_cells: Vec<Cell> = output
            .headers
            .iter()
            .map(|h| Cell::new(h).fg(Color::Cyan))
            .collect();
        table.set_header(header_cells);

        for row_data in &output.rows {
            let cells: Vec<Cell> = row_data.iter().map(Cell::new).collect();
            table.add_row(cells);
        }

        result.push_str(&format!("{table}"));
    }
    // 2. Otherwise, fall back to auto-detection from JSON data
    else {
        result.push_str(&auto_table(&output.label, &output.data));
    }

    // 3. Append addendum if present
    if let Some(ref addendum) = output.addendum {
        result.push_str(&format!("\n\n{} {}", "✓".green().bold(), addendum));
    }

    result
}

fn auto_table(label: &str, value: &Value) -> String {
    match value {
        Value::Array(arr) if !arr.is_empty() => {
            if arr[0].is_object() {
                object_array_to_string(arr)
            } else {
                serde_json::to_string_pretty(value).unwrap_or_default()
            }
        }
        Value::Object(map) => {
            if let Some(data) = map.get("data") {
                if data.is_array() {
                    return auto_table(label, data);
                }
                if data.is_object() {
                    return key_value_to_string(label, data);
                }
            }

            if let Some(balances) = map.get("balances") {
                if balances.is_array() {
                    return balances_to_string(balances);
                }
            }

            if map.contains_key("bids") || map.contains_key("asks") || map.contains_key("buys") {
                return orderbook_to_string(value);
            }

            key_value_to_string(label, value)
        }
        _ => serde_json::to_string_pretty(value).unwrap_or_default(),
    }
}

fn object_array_to_string(arr: &[Value]) -> String {
    if arr.is_empty() {
        return "(no data)".to_string();
    }

    let headers: Vec<String> = if let Some(obj) = arr[0].as_object() {
        obj.keys().cloned().collect()
    } else {
        return String::new();
    };

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);

    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::new(h).fg(Color::Cyan))
        .collect();
    table.set_header(header_cells);

    for item in arr {
        let row: Vec<Cell> = headers
            .iter()
            .map(|h| {
                let val = item.get(h).unwrap_or(&Value::Null);
                Cell::new(format_value(val))
            })
            .collect();
        table.add_row(row);
    }

    format!("{table}")
}

fn key_value_to_string(label: &str, value: &Value) -> String {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Field").fg(Color::Cyan),
        Cell::new("Value").fg(Color::Cyan),
    ]);

    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            if val.is_object() || val.is_array() {
                let summary = match val {
                    Value::Array(a) => format!("[{} items]", a.len()),
                    Value::Object(o) => format!("{{{} fields}}", o.len()),
                    _ => format_value(val),
                };
                table.add_row(vec![Cell::new(key).fg(Color::Green), Cell::new(summary)]);
            } else {
                table.add_row(vec![
                    Cell::new(key).fg(Color::Green),
                    Cell::new(format_value(val)),
                ]);
            }
        }
    }

    let mut out = String::new();
    if !label.is_empty() {
        out.push_str(&format!("{}\n", label.bold()));
    }
    out.push_str(&format!("{table}"));
    out
}

fn balances_to_string(value: &Value) -> String {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Asset").fg(Color::Cyan),
        Cell::new("Free").fg(Color::Cyan),
        Cell::new("Locked").fg(Color::Cyan),
    ]);

    if let Some(arr) = value.as_array() {
        for item in arr {
            let free = item["free"].as_str().unwrap_or("0");
            let locked = item["locked"].as_str().unwrap_or("0");

            let free_f: f64 = free.parse().unwrap_or(0.0);
            let locked_f: f64 = locked.parse().unwrap_or(0.0);
            if free_f == 0.0 && locked_f == 0.0 {
                continue;
            }

            let asset = item["asset"].as_str().unwrap_or("?");
            table.add_row(vec![
                Cell::new(asset).fg(Color::Yellow),
                Cell::new(free).fg(Color::Green),
                Cell::new(locked).fg(if locked_f > 0.0 {
                    Color::Red
                } else {
                    Color::White
                }),
            ]);
        }
    }

    format!("{}\n{table}", "Account Balances".bold())
}

fn orderbook_to_string(value: &Value) -> String {
    let asks_key = "asks";
    let bids_key = if value.get("bids").is_some() {
        "bids"
    } else {
        "buys"
    };

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Price").fg(Color::Cyan),
        Cell::new("Quantity").fg(Color::Cyan),
        Cell::new("Side").fg(Color::Cyan),
    ]);

    if let Some(asks) = value.get(asks_key).and_then(|v| v.as_array()) {
        let mut ask_rows: Vec<_> = asks
            .iter()
            .filter_map(|a| {
                let arr = a.as_array()?;
                Some((
                    arr.first()?.as_str().unwrap_or("?").to_string(),
                    arr.get(1)?.as_str().unwrap_or("?").to_string(),
                ))
            })
            .collect();
        ask_rows.reverse();

        for (price, qty) in &ask_rows {
            table.add_row(vec![
                Cell::new(price).fg(Color::Red),
                Cell::new(qty),
                Cell::new("ASK").fg(Color::Red),
            ]);
        }
    }

    table.add_row(vec![
        Cell::new("───────").fg(Color::DarkGrey),
        Cell::new("───────").fg(Color::DarkGrey),
        Cell::new("SPREAD").fg(Color::DarkGrey),
    ]);

    if let Some(bids) = value.get(bids_key).and_then(|v| v.as_array()) {
        for bid in bids {
            if let Some(arr) = bid.as_array() {
                let price = arr.first().and_then(|v| v.as_str()).unwrap_or("?");
                let qty = arr.get(1).and_then(|v| v.as_str()).unwrap_or("?");
                table.add_row(vec![
                    Cell::new(price).fg(Color::Green),
                    Cell::new(qty),
                    Cell::new("BID").fg(Color::Green),
                ]);
            }
        }
    }

    format!("{}\n{table}", "Order Book".bold())
}

fn format_value(val: &Value) -> String {
    match val {
        Value::Null => "—".to_string(),
        Value::Bool(b) => if *b { "✓" } else { "✗" }.to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => val.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_table_render_empty() {
        let output = CommandOutput::new(Value::Null, "");
        assert_eq!(render(&output), "");
    }

    #[test]
    fn test_table_render_explicit() {
        let output = CommandOutput::new(Value::Null, "Explicit").with_table(
            vec!["Col1".to_string(), "Col2".to_string()],
            vec![vec!["A".to_string(), "B".to_string()]],
        );
        let rendered = render(&output);
        assert!(rendered.contains("Explicit"));
        assert!(rendered.contains("Col1"));
        assert!(rendered.contains("Col2"));
        assert!(rendered.contains("A"));
        assert!(rendered.contains("B"));
    }

    #[test]
    fn test_table_render_object_array() {
        let output = CommandOutput::new(
            json!([
                {"asset": "BTC", "free": "1.0"},
                {"asset": "USDT", "free": "100.0"}
            ]),
            "Assets",
        );
        let rendered = render(&output);
        assert!(rendered.contains("asset"));
        assert!(rendered.contains("free"));
        assert!(rendered.contains("BTC"));
        assert!(rendered.contains("USDT"));
    }

    #[test]
    fn test_table_render_key_value() {
        let output = CommandOutput::new(
            json!({
                "status": "ok",
                "code": 200
            }),
            "Status Label",
        );
        let rendered = render(&output);
        assert!(rendered.contains("status"));
        assert!(rendered.contains("code"));
        assert!(rendered.contains("ok"));
        assert!(rendered.contains("200"));
    }

    #[test]
    fn test_table_render_balances() {
        let output = CommandOutput::new(
            json!({
                "balances": [
                    {"asset": "USDT", "free": "100.0", "locked": "0.0"},
                    {"asset": "BTC", "free": "0.5", "locked": "0.1"}
                ]
            }),
            "Balances",
        );
        let rendered = render(&output);
        assert!(rendered.contains("USDT"));
        assert!(rendered.contains("BTC"));
        assert!(rendered.contains("100.0"));
        assert!(rendered.contains("0.5"));
    }

    #[test]
    fn test_table_render_orderbook() {
        let output = CommandOutput::new(
            json!({
                "bids": [["70000.0", "1.5"]],
                "asks": [["70100.0", "2.0"]]
            }),
            "Book",
        );
        let rendered = render(&output);
        assert!(rendered.contains("70000.0"));
        assert!(rendered.contains("1.5"));
        assert!(rendered.contains("70100.0"));
        assert!(rendered.contains("2.0"));
        assert!(rendered.contains("ASK"));
        assert!(rendered.contains("BID"));
    }

    #[test]
    fn test_format_value() {
        assert_eq!(format_value(&Value::Null), "—");
        assert_eq!(format_value(&Value::Bool(true)), "✓");
        assert_eq!(format_value(&Value::Bool(false)), "✗");
        assert_eq!(format_value(&Value::String("test".to_string())), "test");
        assert_eq!(format_value(&json!(42)), "42");
    }
}

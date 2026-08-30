//! Shaping the untyped rows a shaped inventory query returns.
//!
//! `Store::query_results` hands back `Vec<Value>` — whatever columns the KQL
//! projected. Every consumer (the report tables, `azdocs query`, and the
//! desktop's data grid) needs the same two things from that: the column order,
//! and one display string per cell. They each had their own copy, and the two
//! cell formatters were character-identical.

use serde_json::Value;

/// Column order for a result set: the keys of the first row.
///
/// serde_json's `preserve_order` feature is load-bearing here — it keeps the
/// key order the query projected, which is the order a reader expects. Returns
/// empty for an empty result set or rows that are not objects.
pub fn columns(rows: &[Value]) -> Vec<String> {
    let Some(Value::Object(first)) = rows.first() else {
        return Vec::new();
    };
    first.keys().cloned().collect()
}

/// One cell as display text.
///
/// A missing or null cell is blank rather than the string "null"; a string is
/// itself, unquoted; anything else (nested objects, arrays, numbers) renders as
/// compact JSON so a tags column stays on one line.
pub fn cell_to_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unit_follows_first_row_key_order_when_listing_columns() {
        let rows = vec![json!({"id": "a", "name": "x", "location": "uk"})];

        assert_eq!(columns(&rows), vec!["id", "name", "location"]);
    }

    #[test]
    fn unit_returns_no_columns_when_rows_are_empty_or_not_objects() {
        assert!(columns(&[]).is_empty());
        assert!(columns(&[json!("scalar")]).is_empty());
    }

    #[test]
    fn unit_renders_nested_values_as_json_when_formatting_a_cell() {
        let value = json!({"tags": {"env": "prod"}});

        assert_eq!(cell_to_string(value.get("tags")), r#"{"env":"prod"}"#);
    }

    #[test]
    fn unit_renders_missing_and_null_cells_as_blank_when_formatting() {
        assert_eq!(cell_to_string(None), "");
        assert_eq!(cell_to_string(Some(&Value::Null)), "");
    }

    #[test]
    fn unit_leaves_a_string_cell_unquoted_when_formatting() {
        assert_eq!(cell_to_string(Some(&json!("uksouth"))), "uksouth");
    }
}

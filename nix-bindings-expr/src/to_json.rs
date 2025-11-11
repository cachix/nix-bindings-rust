//! Convert Nix values to JSON by recursively traversing the value tree
//!
//! This module provides efficient conversion from Nix `Value` types to `serde_json::Value`
//! without any Nix evaluation overhead. It handles all JSON-compatible Nix types by directly
//! extracting their values through the FFI API.
//!
//! Supported types: null, bool, int, float, string, path (as JSON string), list, attrset
//! Unsupported types: function, external value, unknown

use crate::value::ValueType;
use super::eval_state::EvalState;
use super::value::Value;
use anyhow::{Context, Result, bail};
use serde_json::{json, Value as JsonValue};
use std::collections::HashSet;

/// Convert a Nix Value to serde_json::Value by recursively traversing the value tree.
///
/// This avoids all Nix evaluation overhead by directly extracting values through the FFI API.
/// Supported types: null, bool, int, float, string, path, list, attrset
///
/// Paths are converted to JSON strings using their string representation.
///
/// # Arguments
/// * `eval_state` - The evaluation state
/// * `value` - The Nix value to convert
///
/// # Returns
/// A `serde_json::Value` representing the Nix value, or an error for unsupported types
/// (functions, external values, unknown types)
pub fn value_to_json(
    eval_state: &mut EvalState,
    value: &Value,
) -> Result<serde_json::Value> {
    let mut visited = HashSet::new();
    value_to_json_impl(eval_state, value, &mut visited)
}

fn value_to_json_impl(
    eval_state: &mut EvalState,
    value: &Value,
    visited: &mut HashSet<usize>,
) -> Result<serde_json::Value> {
    // Check for cycles using the value's pointer address
    let value_ptr = value as *const Value as usize;
    if visited.contains(&value_ptr) {
        // Circular reference detected - return null to avoid breaking JSON parsers
        return Ok(JsonValue::Null);
    }
    visited.insert(value_ptr);

    // Force evaluation to weak head normal form
    eval_state
        .force(value)
        .context("Failed to force evaluation")?;

    // Determine the value type
    let value_type = eval_state
        .value_type(value)
        .context("Failed to determine value type")?;

    match value_type {
        ValueType::Null => Ok(JsonValue::Null),

        ValueType::Bool => {
            let b = eval_state
                .require_bool(value)
                .context("Failed to extract bool")?;
            Ok(JsonValue::Bool(b))
        }

        ValueType::Int => {
            let i = eval_state
                .require_int(value)
                .context("Failed to extract int")?;
            Ok(json!(i))
        }

        ValueType::Float => {
            let f = eval_state
                .require_float(value)
                .context("Failed to extract float")?;
            Ok(json!(f))
        }

        ValueType::String => {
            let s = eval_state
                .require_string(value)
                .context("Failed to extract string")?;
            Ok(JsonValue::String(s))
        }

        ValueType::List => {
            let size = eval_state
                .require_list_size(value)
                .context("Failed to get list size")?;

            let mut json_list = Vec::new();
            for i in 0..size {
                match eval_state.require_list_select_idx_strict(value, i) {
                    Ok(Some(elem)) => {
                        let json_elem = value_to_json_impl(eval_state, &elem, visited)?;
                        json_list.push(json_elem);
                    }
                    Ok(None) => {
                        bail!("List element at index {i} is unavailable")
                    }
                    Err(e) => {
                        bail!("Failed to get list element at index {i}: {e}")
                    }
                }
            }
            Ok(JsonValue::Array(json_list))
        }

        ValueType::AttrSet => {
            let attr_names = eval_state
                .require_attrs_names_unsorted(value)
                .context("Failed to get attribute names")?;

            let mut json_obj = serde_json::Map::new();
            for attr_name in attr_names {
                match eval_state.require_attrs_select(value, &attr_name) {
                    Ok(attr_value) => {
                        let json_val = value_to_json_impl(eval_state, &attr_value, visited)?;
                        json_obj.insert(attr_name, json_val);
                    }
                    Err(e) => {
                        bail!("Failed to get attribute '{attr_name}': {e}")
                    }
                }
            }
            Ok(JsonValue::Object(json_obj))
        }

        ValueType::Path => {
            let path_str = eval_state
                .require_path_string(value)
                .context("Failed to extract path string")?;
            Ok(JsonValue::String(path_str))
        }

        // Unsupported types that can't be directly serialized
        ValueType::Function => {
            bail!("Cannot convert function to JSON")
        }
        ValueType::External => {
            bail!("Cannot convert external value to JSON")
        }
        ValueType::Unknown => {
            bail!("Cannot convert unknown value type to JSON")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval_state::EvalStateBuilder;
    use nix_bindings_store::store::Store;

    #[test]
    fn test_value_to_json_null() -> Result<()> {
        let store = Store::open(None, [])?;
        let mut eval_state = EvalStateBuilder::new(store)?.build()?;

        let value = eval_state.eval_from_string("null", ".")?;
        let json = value_to_json(&mut eval_state, &value)?;

        assert_eq!(json, JsonValue::Null);
        Ok(())
    }

    #[test]
    fn test_value_to_json_bool() -> Result<()> {
        let store = Store::open(None, [])?;
        let mut eval_state = EvalStateBuilder::new(store)?.build()?;

        let value = eval_state.eval_from_string("true", ".")?;
        let json = value_to_json(&mut eval_state, &value)?;

        assert_eq!(json, JsonValue::Bool(true));
        Ok(())
    }

    #[test]
    fn test_value_to_json_int() -> Result<()> {
        let store = Store::open(None, [])?;
        let mut eval_state = EvalStateBuilder::new(store)?.build()?;

        let value = eval_state.eval_from_string("42", ".")?;
        let json = value_to_json(&mut eval_state, &value)?;

        assert_eq!(json, json!(42));
        Ok(())
    }

    #[test]
    fn test_value_to_json_string() -> Result<()> {
        let store = Store::open(None, [])?;
        let mut eval_state = EvalStateBuilder::new(store)?.build()?;

        let value = eval_state.eval_from_string("\"hello\"", ".")?;
        let json = value_to_json(&mut eval_state, &value)?;

        assert_eq!(json, JsonValue::String("hello".to_string()));
        Ok(())
    }

    #[test]
    fn test_value_to_json_list() -> Result<()> {
        let store = Store::open(None, [])?;
        let mut eval_state = EvalStateBuilder::new(store)?.build()?;

        let value = eval_state.eval_from_string("[1 2 3]", ".")?;
        let json = value_to_json(&mut eval_state, &value)?;

        assert_eq!(json, json!([1, 2, 3]));
        Ok(())
    }

    #[test]
    fn test_value_to_json_attrset() -> Result<()> {
        let store = Store::open(None, [])?;
        let mut eval_state = EvalStateBuilder::new(store)?.build()?;

        let value = eval_state.eval_from_string("{ a = 1; b = \"x\"; }", ".")?;
        let json = value_to_json(&mut eval_state, &value)?;

        let expected = json!({"a": 1, "b": "x"});
        assert_eq!(json, expected);
        Ok(())
    }

    #[test]
    fn test_value_to_json_nested() -> Result<()> {
        let store = Store::open(None, [])?;
        let mut eval_state = EvalStateBuilder::new(store)?.build()?;

        let value = eval_state.eval_from_string(
            "{ items = [{ id = 1; } { id = 2; }]; count = 2; }",
            ".",
        )?;
        let json = value_to_json(&mut eval_state, &value)?;

        let expected = json!({
            "items": [{"id": 1}, {"id": 2}],
            "count": 2
        });
        assert_eq!(json, expected);
        Ok(())
    }

    #[test]
    fn test_value_to_json_path() -> Result<()> {
        let store = Store::open(None, [])?;
        let mut eval_state = EvalStateBuilder::new(store)?.build()?;

        let value = eval_state.eval_from_string("/tmp/test", ".")?;
        let json = value_to_json(&mut eval_state, &value)?;

        assert!(matches!(json, JsonValue::String(_)));
        Ok(())
    }
}

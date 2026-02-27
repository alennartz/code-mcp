//! Shared JSON Schema to Luau type conversion utilities.
//!
//! Used by both the `OpenAPI` codegen path and MCP tool schema conversion.

use serde_json::Value;

use super::manifest::{FieldDef, FieldType, McpParamDef, SchemaDef};

/// Convert a JSON Schema type string to the corresponding Luau type name.
///
/// When `type_str` is `"array"`, the optional `items` value is inspected to
/// determine the element type, producing e.g. `"{number}"`.
pub fn json_schema_type_to_luau(type_str: &str, items: Option<&Value>) -> String {
    match type_str {
        "string" => "string".to_string(),
        "integer" | "number" => "number".to_string(),
        "boolean" => "boolean".to_string(),
        "array" => {
            let inner = items
                .and_then(|v| v.get("type"))
                .and_then(Value::as_str)
                .map_or_else(|| "any".to_string(), |t| json_schema_type_to_luau(t, None));
            format!("{{{inner}}}")
        }
        _ => "any".to_string(),
    }
}

/// Convert a JSON Schema object (with `properties` / `required`) into a list of
/// [`McpParamDef`] entries suitable for MCP tool parameter metadata.
pub fn json_schema_to_params(schema: &Value) -> Vec<McpParamDef> {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return Vec::new();
    };

    let required_set: std::collections::HashSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut params: Vec<McpParamDef> = properties
        .iter()
        .map(|(name, prop)| {
            let type_str = prop.get("type").and_then(Value::as_str).unwrap_or("any");
            let items = prop.get("items");
            let luau_type = json_schema_type_to_luau(type_str, items);
            let description = prop
                .get("description")
                .and_then(Value::as_str)
                .map(String::from);

            McpParamDef {
                name: name.clone(),
                luau_type,
                required: required_set.contains(name.as_str()),
                description,
            }
        })
        .collect();

    // Sort for deterministic output.
    params.sort_by(|a, b| a.name.cmp(&b.name));
    params
}

/// Convert a single JSON Schema property value into a [`FieldType`].
///
/// Handles `$ref`, primitive types, arrays, and objects (with or without
/// explicit `properties`).
pub fn json_schema_prop_to_field_type(prop: &Value) -> FieldType {
    // Handle $ref
    if let Some(ref_str) = prop.get("$ref").and_then(Value::as_str) {
        let schema_name = ref_str.rsplit('/').next().unwrap_or(ref_str).to_string();
        return FieldType::Object {
            schema: schema_name,
        };
    }

    let type_str = prop.get("type").and_then(Value::as_str).unwrap_or("");

    match type_str {
        "integer" => FieldType::Integer,
        "number" => FieldType::Number,
        "boolean" => FieldType::Boolean,
        "array" => {
            let items_type = prop
                .get("items")
                .map_or(FieldType::String, json_schema_prop_to_field_type);
            FieldType::Array {
                items: Box::new(items_type),
            }
        }
        "object" => object_field_type(prop),
        // "string" and unknown types both fall back to String.
        _ => FieldType::String,
    }
}

/// Build a [`FieldType`] for a JSON Schema `"object"` type, distinguishing
/// between objects with explicit `properties` ([`FieldType::InlineObject`]) and
/// bare objects ([`FieldType::Map`]).
fn object_field_type(prop: &Value) -> FieldType {
    let Some(properties) = prop.get("properties").and_then(Value::as_object) else {
        return FieldType::Map {
            value: Box::new(FieldType::String),
        };
    };

    let required_set: std::collections::HashSet<&str> = prop
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut fields: Vec<FieldDef> = properties
        .iter()
        .map(|(name, fprop)| FieldDef {
            name: name.clone(),
            field_type: json_schema_prop_to_field_type(fprop),
            required: required_set.contains(name.as_str()),
            description: fprop
                .get("description")
                .and_then(Value::as_str)
                .map(String::from),
            enum_values: None,
            nullable: false,
            format: None,
        })
        .collect();

    fields.sort_by(|a, b| a.name.cmp(&b.name));
    FieldType::InlineObject { fields }
}

/// Extract named schema definitions from `$defs` or `definitions` in a JSON
/// Schema document, converting each into a [`SchemaDef`].
pub fn extract_schema_defs(schema: &Value) -> Vec<SchemaDef> {
    let defs_obj = schema
        .get("$defs")
        .or_else(|| schema.get("definitions"))
        .and_then(Value::as_object);

    let Some(defs_obj) = defs_obj else {
        return Vec::new();
    };

    let mut defs: Vec<SchemaDef> = defs_obj
        .iter()
        .map(|(name, def)| {
            let required_set: std::collections::HashSet<&str> = def
                .get("required")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();

            let mut fields: Vec<FieldDef> = def
                .get("properties")
                .and_then(Value::as_object)
                .map(|props| {
                    props
                        .iter()
                        .map(|(fname, fprop)| FieldDef {
                            name: fname.clone(),
                            field_type: json_schema_prop_to_field_type(fprop),
                            required: required_set.contains(fname.as_str()),
                            description: fprop
                                .get("description")
                                .and_then(Value::as_str)
                                .map(String::from),
                            enum_values: None,
                            nullable: false,
                            format: None,
                        })
                        .collect()
                })
                .unwrap_or_default();

            fields.sort_by(|a, b| a.name.cmp(&b.name));

            SchemaDef {
                name: name.clone(),
                description: def
                    .get("description")
                    .and_then(Value::as_str)
                    .map(String::from),
                fields,
            }
        })
        .collect();

    defs.sort_by(|a, b| a.name.cmp(&b.name));
    defs
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn test_json_schema_type_to_luau() {
        assert_eq!(json_schema_type_to_luau("string", None), "string");
        assert_eq!(json_schema_type_to_luau("integer", None), "number");
        assert_eq!(json_schema_type_to_luau("number", None), "number");
        assert_eq!(json_schema_type_to_luau("boolean", None), "boolean");
        assert_eq!(json_schema_type_to_luau("unknown", None), "any");
    }

    #[test]
    fn test_json_schema_to_params_flat() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string", "description": "File path" },
                "encoding": { "type": "string" }
            }
        });
        let params = json_schema_to_params(&schema);
        assert_eq!(params.len(), 2);
        let path_param = params.iter().find(|p| p.name == "path").unwrap();
        assert!(path_param.required);
        assert_eq!(path_param.luau_type, "string");
        assert_eq!(path_param.description.as_deref(), Some("File path"));
        let enc_param = params.iter().find(|p| p.name == "encoding").unwrap();
        assert!(!enc_param.required);
    }

    #[test]
    fn test_extract_schema_defs_from_json_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "user": { "$ref": "#/$defs/User" }
            },
            "$defs": {
                "User": {
                    "type": "object",
                    "required": ["name"],
                    "properties": {
                        "name": { "type": "string" },
                        "email": { "type": "string" }
                    }
                }
            }
        });
        let defs = extract_schema_defs(&schema);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "User");
        assert_eq!(defs[0].fields.len(), 2);
    }

    #[test]
    fn test_json_schema_to_field_type() {
        let prop = serde_json::json!({ "type": "string" });
        assert_eq!(json_schema_prop_to_field_type(&prop), FieldType::String);

        let arr = serde_json::json!({ "type": "array", "items": { "type": "integer" } });
        assert_eq!(
            json_schema_prop_to_field_type(&arr),
            FieldType::Array {
                items: Box::new(FieldType::Integer)
            }
        );

        let obj = serde_json::json!({
            "type": "object",
            "properties": {
                "x": { "type": "number" }
            }
        });
        match json_schema_prop_to_field_type(&obj) {
            FieldType::InlineObject { fields } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "x");
            }
            other => panic!("Expected InlineObject, got {other:?}"),
        }

        let reftype = serde_json::json!({ "$ref": "#/$defs/User" });
        assert_eq!(
            json_schema_prop_to_field_type(&reftype),
            FieldType::Object {
                schema: "User".to_string()
            }
        );
    }
}

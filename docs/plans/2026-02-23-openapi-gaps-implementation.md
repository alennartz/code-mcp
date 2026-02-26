# OpenAPI V3 Gaps Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the 6 highest-impact OpenAPI V3 gaps: allOf composition, nullable types, additionalProperties, header parameters at runtime, format pass-through, and broader response status codes.

**Architecture:** All 6 changes flow through the same pipeline: `parser.rs` (extract from openapiv3 types) -> `manifest.rs` (data structures) -> `annotations.rs` (Luau codegen) -> `registry.rs` (runtime). We add new fields/variants to the manifest types, update the parser to populate them, update annotations to render them, and wire header params into the HTTP runtime. A new `testdata/advanced.yaml` spec exercises all 6 features.

**Tech Stack:** Rust, openapiv3 crate v2.2.0, serde, mlua, reqwest

---

### Task 1: Add new fields and variants to manifest types

**Files:**
- Modify: `src/codegen/manifest.rs:107-126`

**Step 1: Write the failing test**

Add to the existing `mod tests` in `src/codegen/manifest.rs`:

```rust
#[test]
fn test_field_type_map_serde() {
    let map_type = FieldType::Map {
        value: Box::new(FieldType::String),
    };
    let json = serde_json::to_string(&map_type).unwrap();
    let deserialized: FieldType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, map_type);
}

#[test]
fn test_field_def_new_fields_serde() {
    let field = FieldDef {
        name: "created_at".to_string(),
        field_type: FieldType::String,
        required: true,
        description: Some("Creation timestamp".to_string()),
        enum_values: None,
        nullable: true,
        format: Some("date-time".to_string()),
    };
    let json = serde_json::to_string(&field).unwrap();
    let roundtripped: FieldDef = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.nullable, true);
    assert_eq!(roundtripped.format.as_deref(), Some("date-time"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p toolscript --lib codegen::manifest::tests::test_field_type_map_serde 2>&1 | tail -5`
Expected: FAIL — `Map` variant doesn't exist, `nullable`/`format` fields don't exist

**Step 3: Write minimal implementation**

In `manifest.rs`, add the `Map` variant to `FieldType`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldType {
    String,
    Integer,
    Number,
    Boolean,
    Array { items: Box<Self> },
    Object { schema: String },
    Map { value: Box<Self> },
}
```

Add `nullable` and `format` to `FieldDef`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub description: Option<String>,
    pub enum_values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}
```

Then update every `FieldDef` construction site in existing tests in `manifest.rs` to add `nullable: false, format: None`. There are 5 places in `test_manifest_serialization_roundtrip` (4 fields in the Pet schema), 1 in `test_manifest_json_structure`, and 1 in `test_request_body_def_roundtrip`. All get `nullable: false, format: None`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p toolscript --lib codegen::manifest 2>&1 | tail -5`
Expected: PASS

**Step 5: Commit**

```bash
git add src/codegen/manifest.rs
git commit -m "feat: add Map variant, nullable, and format fields to manifest types"
```

---

### Task 2: Update parser to extract nullable, format, additionalProperties, and allOf

**Files:**
- Modify: `src/codegen/parser.rs:1-556` (the non-test portion)

**Step 1: Write the failing test**

Create `testdata/advanced.yaml` with schemas that use allOf, nullable, additionalProperties, format, and header params. Add tests to the `mod tests` in `src/codegen/parser.rs`:

First, create `testdata/advanced.yaml`:

```yaml
openapi: "3.0.3"
info:
  title: Advanced API
  description: Tests advanced OpenAPI features
  version: "2.0.0"
servers:
  - url: https://api.advanced.example.com/v2
tags:
  - name: resources
paths:
  /resources:
    get:
      tags:
        - resources
      operationId: listResources
      summary: List resources
      parameters:
        - name: X-Request-ID
          in: header
          required: false
          description: Correlation ID for tracing
          schema:
            type: string
        - name: limit
          in: query
          required: false
          schema:
            type: integer
      responses:
        "200":
          description: A list of resources
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: "#/components/schemas/Resource"
  /resources/{id}:
    get:
      tags:
        - resources
      operationId: getResource
      summary: Get a resource
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
            format: uuid
      responses:
        "200":
          description: The resource
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Resource"
        "404":
          description: Not found
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Error"
    put:
      tags:
        - resources
      operationId: updateResource
      summary: Update a resource
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
        - name: X-Idempotency-Key
          in: header
          required: true
          description: Idempotency key
          schema:
            type: string
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/ResourceUpdate"
      responses:
        "200":
          description: Updated resource
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Resource"
components:
  schemas:
    BaseResource:
      type: object
      required:
        - id
        - created_at
      properties:
        id:
          type: string
          format: uuid
          description: Unique identifier
        created_at:
          type: string
          format: date-time
          description: Creation timestamp
        updated_at:
          type: string
          format: date-time
          nullable: true
          description: Last update timestamp

    Resource:
      allOf:
        - $ref: "#/components/schemas/BaseResource"
        - type: object
          required:
            - name
          properties:
            name:
              type: string
              description: Resource name
            description:
              type: string
              nullable: true
              description: Optional description
            metadata:
              type: object
              additionalProperties:
                type: string
              description: Arbitrary key-value metadata
            tags:
              type: array
              items:
                type: string
              description: Classification tags

    ResourceUpdate:
      type: object
      properties:
        name:
          type: string
        description:
          type: string
          nullable: true
        metadata:
          type: object
          additionalProperties:
            type: string

    Error:
      type: object
      required:
        - code
        - message
      properties:
        code:
          type: integer
          format: int32
          description: Error code
        message:
          type: string
          description: Error message

    Config:
      type: object
      additionalProperties:
        type: object
        additionalProperties:
          type: string
      description: Nested map config
```

Then add these tests in `parser.rs` `mod tests`:

```rust
#[test]
fn test_allof_schema_extraction() {
    let spec = load_spec_from_file(Path::new("testdata/advanced.yaml")).unwrap();
    let manifest = spec_to_manifest(&spec, "advanced").unwrap();

    let resource = manifest
        .schemas
        .iter()
        .find(|s| s.name == "Resource")
        .expect("Resource schema missing");

    // allOf should merge BaseResource fields + inline fields
    let field_names: Vec<&str> = resource.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"id"), "Missing id from BaseResource. Got: {field_names:?}");
    assert!(field_names.contains(&"created_at"), "Missing created_at from BaseResource. Got: {field_names:?}");
    assert!(field_names.contains(&"name"), "Missing name from inline. Got: {field_names:?}");
    assert!(field_names.contains(&"metadata"), "Missing metadata. Got: {field_names:?}");
}

#[test]
fn test_nullable_fields() {
    let spec = load_spec_from_file(Path::new("testdata/advanced.yaml")).unwrap();
    let manifest = spec_to_manifest(&spec, "advanced").unwrap();

    let resource = manifest
        .schemas
        .iter()
        .find(|s| s.name == "Resource")
        .expect("Resource schema missing");

    let updated_at = resource.fields.iter().find(|f| f.name == "updated_at").unwrap();
    assert!(updated_at.nullable, "updated_at should be nullable");

    let description = resource.fields.iter().find(|f| f.name == "description").unwrap();
    assert!(description.nullable, "description should be nullable");

    let name = resource.fields.iter().find(|f| f.name == "name").unwrap();
    assert!(!name.nullable, "name should not be nullable");
}

#[test]
fn test_format_extraction() {
    let spec = load_spec_from_file(Path::new("testdata/advanced.yaml")).unwrap();
    let manifest = spec_to_manifest(&spec, "advanced").unwrap();

    let resource = manifest
        .schemas
        .iter()
        .find(|s| s.name == "Resource")
        .expect("Resource schema missing");

    let id_field = resource.fields.iter().find(|f| f.name == "id").unwrap();
    assert_eq!(id_field.format.as_deref(), Some("uuid"));

    let created_at = resource.fields.iter().find(|f| f.name == "created_at").unwrap();
    assert_eq!(created_at.format.as_deref(), Some("date-time"));

    let error = manifest.schemas.iter().find(|s| s.name == "Error").unwrap();
    let code_field = error.fields.iter().find(|f| f.name == "code").unwrap();
    assert_eq!(code_field.format.as_deref(), Some("int32"));
}

#[test]
fn test_additional_properties_map() {
    let spec = load_spec_from_file(Path::new("testdata/advanced.yaml")).unwrap();
    let manifest = spec_to_manifest(&spec, "advanced").unwrap();

    let resource = manifest
        .schemas
        .iter()
        .find(|s| s.name == "Resource")
        .expect("Resource schema missing");

    let metadata = resource.fields.iter().find(|f| f.name == "metadata").unwrap();
    assert_eq!(
        metadata.field_type,
        FieldType::Map {
            value: Box::new(FieldType::String)
        },
        "metadata should be Map<string>"
    );
}

#[test]
fn test_header_params_extracted() {
    let spec = load_spec_from_file(Path::new("testdata/advanced.yaml")).unwrap();
    let manifest = spec_to_manifest(&spec, "advanced").unwrap();

    let list = manifest.functions.iter().find(|f| f.name == "list_resources").unwrap();
    let header_param = list.parameters.iter().find(|p| p.name == "X-Request-ID");
    assert!(header_param.is_some(), "Header param X-Request-ID should be extracted");
    assert_eq!(header_param.unwrap().location, ParamLocation::Header);

    let update = manifest.functions.iter().find(|f| f.name == "update_resource").unwrap();
    let idemp = update.parameters.iter().find(|p| p.name == "X-Idempotency-Key");
    assert!(idemp.is_some(), "Header param X-Idempotency-Key should be extracted");
    assert!(idemp.unwrap().required);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p toolscript --lib codegen::parser::tests::test_allof 2>&1 | tail -5`
Expected: FAIL — allOf schemas not extracted (returns None from `extract_schema_def`)

**Step 3: Write implementation**

In `parser.rs`, update `extract_schema_def` to handle allOf:

```rust
fn extract_schema_def(
    name: &str,
    schema: &Schema,
    components: &openapiv3::Components,
) -> Option<SchemaDef> {
    let mut all_properties: indexmap::IndexMap<String, ReferenceOr<Box<Schema>>> =
        indexmap::IndexMap::new();
    let mut all_required: Vec<String> = Vec::new();

    match &schema.schema_kind {
        SchemaKind::Type(Type::Object(obj)) => {
            all_properties.extend(obj.properties.clone());
            all_required.extend(obj.required.clone());
        }
        SchemaKind::AllOf { all_of } => {
            for sub_ref in all_of {
                match sub_ref {
                    ReferenceOr::Reference { reference } => {
                        let sub_name = reference
                            .strip_prefix("#/components/schemas/")
                            .unwrap_or(reference);
                        if let Some(ReferenceOr::Item(sub_schema)) =
                            components.schemas.get(sub_name)
                        {
                            collect_object_properties(
                                sub_schema,
                                components,
                                &mut all_properties,
                                &mut all_required,
                            );
                        }
                    }
                    ReferenceOr::Item(sub_schema) => {
                        collect_object_properties(
                            sub_schema,
                            components,
                            &mut all_properties,
                            &mut all_required,
                        );
                    }
                }
            }
        }
        _ => return None,
    };

    let fields: Vec<FieldDef> = all_properties
        .iter()
        .map(|(field_name, field_schema_ref)| {
            let is_required = all_required.contains(field_name);
            extract_field_def(field_name, field_schema_ref, is_required, components)
        })
        .collect();

    Some(SchemaDef {
        name: name.to_string(),
        description: schema.schema_data.description.clone(),
        fields,
    })
}
```

Add the helper function `collect_object_properties`:

```rust
fn collect_object_properties(
    schema: &Schema,
    components: &openapiv3::Components,
    properties: &mut indexmap::IndexMap<String, ReferenceOr<Box<Schema>>>,
    required: &mut Vec<String>,
) {
    match &schema.schema_kind {
        SchemaKind::Type(Type::Object(obj)) => {
            properties.extend(obj.properties.clone());
            required.extend(obj.required.clone());
        }
        SchemaKind::AllOf { all_of } => {
            for sub_ref in all_of {
                match sub_ref {
                    ReferenceOr::Reference { reference } => {
                        let sub_name = reference
                            .strip_prefix("#/components/schemas/")
                            .unwrap_or(reference);
                        if let Some(ReferenceOr::Item(sub_schema)) =
                            components.schemas.get(sub_name)
                        {
                            collect_object_properties(sub_schema, components, properties, required);
                        }
                    }
                    ReferenceOr::Item(sub_schema) => {
                        collect_object_properties(sub_schema, components, properties, required);
                    }
                }
            }
        }
        _ => {}
    }
}
```

Update `extract_field_def` to populate `nullable`, `format`, and handle `additionalProperties`:

```rust
fn extract_field_def(
    name: &str,
    schema_ref: &ReferenceOr<Box<Schema>>,
    required: bool,
    components: &openapiv3::Components,
) -> FieldDef {
    match schema_ref {
        ReferenceOr::Reference { reference } => {
            let schema_name = reference
                .strip_prefix("#/components/schemas/")
                .unwrap_or(reference)
                .to_string();
            let description = components
                .schemas
                .get(&schema_name)
                .and_then(|s| s.as_item())
                .and_then(|s| s.schema_data.description.clone());
            FieldDef {
                name: name.to_string(),
                field_type: FieldType::Object {
                    schema: schema_name,
                },
                required,
                description,
                enum_values: None,
                nullable: false,
                format: None,
            }
        }
        ReferenceOr::Item(schema) => {
            let nullable = schema.schema_data.nullable;
            let format = extract_format(&schema.schema_kind);
            let field_type = schema_kind_to_field_type(&schema.schema_kind);
            let enum_values = extract_field_enum_values(&schema.schema_kind);
            FieldDef {
                name: name.to_string(),
                field_type,
                required,
                description: schema.schema_data.description.clone(),
                enum_values,
                nullable,
                format,
            }
        }
    }
}
```

Add `extract_format`:

```rust
fn extract_format(kind: &SchemaKind) -> Option<String> {
    match kind {
        SchemaKind::Type(Type::String(s)) => variant_or_to_string(&s.format),
        SchemaKind::Type(Type::Integer(i)) => variant_or_to_string(&i.format),
        SchemaKind::Type(Type::Number(n)) => variant_or_to_string(&n.format),
        _ => None,
    }
}

fn variant_or_to_string<T: std::fmt::Debug + serde::Serialize>(
    v: &openapiv3::VariantOrUnknownOrEmpty<T>,
) -> Option<String> {
    match v {
        openapiv3::VariantOrUnknownOrEmpty::Item(item) => {
            // Serialize to get the string representation (e.g. "date-time", "int32")
            serde_json::to_value(item)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
        }
        openapiv3::VariantOrUnknownOrEmpty::Unknown(s) => Some(s.clone()),
        openapiv3::VariantOrUnknownOrEmpty::Empty => None,
    }
}
```

Update `schema_kind_to_field_type` to handle `additionalProperties`:

```rust
fn schema_kind_to_field_type(kind: &SchemaKind) -> FieldType {
    match kind {
        SchemaKind::Type(Type::Integer(_)) => FieldType::Integer,
        SchemaKind::Type(Type::Number(_)) => FieldType::Number,
        SchemaKind::Type(Type::Boolean(_)) => FieldType::Boolean,
        SchemaKind::Type(Type::Array(arr)) => {
            let items_type =
                arr.items
                    .as_ref()
                    .map_or(FieldType::String, |items_ref| match items_ref {
                        ReferenceOr::Reference { reference } => {
                            let schema_name = reference
                                .strip_prefix("#/components/schemas/")
                                .unwrap_or(reference);
                            FieldType::Object {
                                schema: schema_name.to_string(),
                            }
                        }
                        ReferenceOr::Item(schema) => schema_kind_to_field_type(&schema.schema_kind),
                    });
            FieldType::Array {
                items: Box::new(items_type),
            }
        }
        SchemaKind::Type(Type::Object(obj)) => {
            // Check for additionalProperties pattern (map type)
            if obj.properties.is_empty() {
                if let Some(ap) = &obj.additional_properties {
                    return additional_properties_to_map(ap);
                }
            }
            FieldType::Object {
                schema: "unknown".to_string(),
            }
        }
        _ => FieldType::String,
    }
}
```

Add `additional_properties_to_map`:

```rust
fn additional_properties_to_map(ap: &openapiv3::AdditionalProperties) -> FieldType {
    match ap {
        openapiv3::AdditionalProperties::Schema(schema_ref) => {
            let value_type = match schema_ref.as_ref() {
                ReferenceOr::Reference { reference } => {
                    let schema_name = reference
                        .strip_prefix("#/components/schemas/")
                        .unwrap_or(reference);
                    FieldType::Object {
                        schema: schema_name.to_string(),
                    }
                }
                ReferenceOr::Item(schema) => schema_kind_to_field_type(&schema.schema_kind),
            };
            FieldType::Map {
                value: Box::new(value_type),
            }
        }
        openapiv3::AdditionalProperties::Any(true) => FieldType::Map {
            value: Box::new(FieldType::String),
        },
        openapiv3::AdditionalProperties::Any(false) => FieldType::Object {
            schema: "unknown".to_string(),
        },
    }
}
```

Add `indexmap` to the imports at the top of `parser.rs` (it's already a transitive dependency via openapiv3). The `use` block becomes:

```rust
use openapiv3::{
    OpenAPI, Parameter, ParameterSchemaOrContent, ReferenceOr, Schema, SchemaKind, SecurityScheme,
    Type,
};
```

No new imports needed — `indexmap` types are accessed through the `openapiv3` re-export.

Finally, update all existing `FieldDef` constructions in parser.rs test code to add `nullable: false, format: None`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p toolscript --lib codegen::parser 2>&1 | tail -10`
Expected: PASS

**Step 5: Commit**

```bash
git add src/codegen/parser.rs testdata/advanced.yaml
git commit -m "feat: extract allOf, nullable, format, additionalProperties from OpenAPI specs"
```

---

### Task 3: Update Luau annotation renderer for new types

**Files:**
- Modify: `src/codegen/annotations.rs:260-268` (`field_type_to_luau` function)
- Modify: `src/codegen/annotations.rs:116-152` (`render_schema_annotation` function)

**Step 1: Write the failing test**

Add to `mod tests` in `annotations.rs`:

```rust
#[test]
fn test_render_map_field_type() {
    let schema = SchemaDef {
        name: "Config".to_string(),
        description: None,
        fields: vec![FieldDef {
            name: "metadata".to_string(),
            field_type: FieldType::Map {
                value: Box::new(FieldType::String),
            },
            required: true,
            description: Some("Key-value pairs".to_string()),
            enum_values: None,
            nullable: false,
            format: None,
        }],
    };

    let output = render_schema_annotation(&schema);
    assert!(
        output.contains("metadata: {[string]: string},"),
        "Map type should render as {{[string]: string}}. Got:\n{output}"
    );
}

#[test]
fn test_render_nullable_field() {
    let schema = SchemaDef {
        name: "Item".to_string(),
        description: None,
        fields: vec![
            FieldDef {
                name: "name".to_string(),
                field_type: FieldType::String,
                required: true,
                description: None,
                enum_values: None,
                nullable: false,
                format: None,
            },
            FieldDef {
                name: "deleted_at".to_string(),
                field_type: FieldType::String,
                required: true,
                description: None,
                enum_values: None,
                nullable: true,
                format: Some("date-time".to_string()),
            },
        ],
    };

    let output = render_schema_annotation(&schema);
    // required + nullable = the type itself gets ? suffix
    assert!(
        output.contains("deleted_at: string?,"),
        "Nullable required field should have ?. Got:\n{output}"
    );
    assert!(
        output.contains("name: string,"),
        "Non-nullable required field should NOT have ?. Got:\n{output}"
    );
}

#[test]
fn test_render_format_comment() {
    let schema = SchemaDef {
        name: "Item".to_string(),
        description: None,
        fields: vec![FieldDef {
            name: "id".to_string(),
            field_type: FieldType::String,
            required: true,
            description: Some("Unique ID".to_string()),
            enum_values: None,
            nullable: false,
            format: Some("uuid".to_string()),
        }],
    };

    let output = render_schema_annotation(&schema);
    assert!(
        output.contains("(uuid)"),
        "Format should appear in comment. Got:\n{output}"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p toolscript --lib codegen::annotations::tests::test_render_map 2>&1 | tail -5`
Expected: FAIL — `Map` variant unhandled in `field_type_to_luau`

**Step 3: Write implementation**

Update `field_type_to_luau` in `annotations.rs`:

```rust
fn field_type_to_luau(field_type: &FieldType) -> String {
    match field_type {
        FieldType::String => "string".to_string(),
        FieldType::Integer | FieldType::Number => "number".to_string(),
        FieldType::Boolean => "boolean".to_string(),
        FieldType::Array { items } => format!("{{{}}}", field_type_to_luau(items)),
        FieldType::Object { schema } => schema.clone(),
        FieldType::Map { value } => format!("{{[string]: {}}}", field_type_to_luau(value)),
    }
}
```

Update `render_schema_annotation` to handle nullable and format:

```rust
pub fn render_schema_annotation(schema: &SchemaDef) -> String {
    let mut lines: Vec<String> = Vec::new();

    if let Some(description) = &schema.description {
        let desc = description.trim();
        if !desc.is_empty() {
            lines.push(format!("-- {desc}"));
        }
    }

    lines.push(format!("export type {} = {{", schema.name));

    for field in &schema.fields {
        let type_str = field.enum_values.as_ref().map_or_else(
            || field_type_to_luau(&field.field_type),
            |ev| render_enum_type(ev),
        );
        let optional_marker = if !field.required || field.nullable {
            "?"
        } else {
            ""
        };

        // Build comment: description + format
        let mut comment_parts: Vec<String> = Vec::new();
        if let Some(d) = &field.description {
            comment_parts.push(d.trim().to_string());
        }
        if let Some(f) = &field.format {
            comment_parts.push(format!("({f})"));
        }
        let desc = if comment_parts.is_empty() {
            String::new()
        } else {
            format!("  -- {}", comment_parts.join(" "))
        };

        lines.push(format!(
            "    {}: {type_str}{optional_marker},{desc}",
            field.name
        ));
    }

    lines.push("}".to_string());
    lines.join("\n")
}
```

Then update all existing `FieldDef` constructions in `annotations.rs` test code to add `nullable: false, format: None`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p toolscript --lib codegen::annotations 2>&1 | tail -10`
Expected: PASS

**Step 5: Commit**

```bash
git add src/codegen/annotations.rs
git commit -m "feat: render Map types, nullable fields, and format hints in Luau annotations"
```

---

### Task 4: Wire header parameters into HTTP runtime

**Files:**
- Modify: `src/runtime/registry.rs:117-127`
- Modify: `src/runtime/http.rs:74-126`

**Step 1: Write the failing test**

Add to `mod tests` in `registry.rs`:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn test_header_params_sent() {
    let captured_headers = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let captured_headers_clone = Arc::clone(&captured_headers);

    let manifest = Manifest {
        apis: vec![ApiConfig {
            name: "testapi".to_string(),
            base_url: "https://api.example.com".to_string(),
            description: None,
            version: None,
            auth: None,
        }],
        functions: vec![FunctionDef {
            name: "do_thing".to_string(),
            api: "testapi".to_string(),
            tag: None,
            method: HttpMethod::Get,
            path: "/things".to_string(),
            summary: None,
            description: None,
            deprecated: false,
            parameters: vec![
                ParamDef {
                    name: "X-Request-ID".to_string(),
                    location: ParamLocation::Header,
                    param_type: ParamType::String,
                    required: true,
                    description: None,
                    default: None,
                    enum_values: None,
                },
                ParamDef {
                    name: "limit".to_string(),
                    location: ParamLocation::Query,
                    param_type: ParamType::Integer,
                    required: false,
                    description: None,
                    default: None,
                    enum_values: None,
                },
            ],
            request_body: None,
            response_schema: None,
        }],
        schemas: vec![],
    };

    let sb = Sandbox::new(SandboxConfig::default()).unwrap();
    let handler = Arc::new(HttpHandler::mock_with_headers(
        move |_method, _url, _query, _headers, _body| {
            Ok(serde_json::json!({"ok": true}))
        },
        captured_headers_clone,
    ));
    let creds = Arc::new(AuthCredentialsMap::new());
    let counter = Arc::new(AtomicUsize::new(0));

    register_functions(&sb, &manifest, handler, creds, counter, None).unwrap();

    sb.eval::<Value>(r#"sdk.do_thing("trace-123", 10)"#).unwrap();

    let headers = captured_headers.lock().unwrap().clone();
    assert!(
        headers.iter().any(|(k, v)| k == "X-Request-ID" && v == "trace-123"),
        "Header param not sent. Got: {headers:?}"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p toolscript --lib runtime::registry::tests::test_header_params_sent 2>&1 | tail -5`
Expected: FAIL — `mock_with_headers` doesn't exist, headers not collected

**Step 3: Write implementation**

First, update `http.rs` to support header params. Change the `request` signature to accept headers:

```rust
pub async fn request(
    &self,
    method: &str,
    url: &str,
    auth_config: Option<&AuthConfig>,
    credentials: &AuthCredentials,
    query_params: &[(String, String)],
    headers: &[(String, String)],
    body: Option<&serde_json::Value>,
) -> anyhow::Result<serde_json::Value> {
```

In the `Real` arm, add headers after auth injection:

```rust
// Add custom headers
for (key, value) in headers {
    builder = builder.header(key.as_str(), value.as_str());
}
```

Update mock types to capture headers. Add a new mock constructor:

```rust
/// Mock function signature with headers
type MockWithHeadersFn = Arc<
    dyn Fn(
            &str,
            &str,
            &[(String, String)],
            &[(String, String)],
            Option<&serde_json::Value>,
        ) -> anyhow::Result<serde_json::Value>
        + Send
        + Sync,
>;

#[derive(Clone)]
enum HttpHandlerInner {
    Real(reqwest::Client),
    Mock(MockFn),
    MockWithHeaders {
        func: MockWithHeadersFn,
        captured_headers: Arc<std::sync::Mutex<Vec<(String, String)>>>,
    },
}
```

Add `mock_with_headers` constructor and update the match in `request` to handle the new variant. The existing `Mock` variant ignores headers (backwards compatible). The `MockWithHeaders` variant captures them.

Then in `registry.rs`, collect header params alongside query params:

```rust
let mut header_params: Vec<(String, String)> = Vec::new();

// ... inside the param loop:
ParamLocation::Header => {
    header_params.push((param.name.clone(), str_value));
}
```

And pass them to the handler:

```rust
tokio::runtime::Handle::current().block_on(handler.request(
    method,
    &url,
    auth_config_owned.as_ref(),
    &api_creds,
    &query_params,
    &header_params,
    body.as_ref(),
))
```

Update all existing call sites of `handler.request()` to pass `&[]` for headers (the 3 call sites in `http.rs` tests).

**Step 4: Run tests to verify they pass**

Run: `cargo test -p toolscript --lib runtime 2>&1 | tail -10`
Expected: PASS

**Step 5: Commit**

```bash
git add src/runtime/registry.rs src/runtime/http.rs
git commit -m "feat: wire header parameters through to HTTP requests"
```

---

### Task 5: Expand response schema extraction to full 2xx range

**Files:**
- Modify: `src/codegen/parser.rs:401-436`

**Step 1: Write the failing test**

The `advanced.yaml` already has a 404 error response. Add a test:

```rust
#[test]
fn test_response_schema_broader_status_codes() {
    // Create a spec with a 202 response
    let yaml = r#"
openapi: "3.0.3"
info:
  title: Test
  version: "1.0.0"
paths:
  /jobs:
    post:
      operationId: createJob
      responses:
        "202":
          description: Accepted
          content:
            application/json:
              schema:
                $ref: "#/components/schemas/Job"
components:
  schemas:
    Job:
      type: object
      properties:
        id:
          type: string
"#;
    let spec: OpenAPI = serde_yaml::from_str(yaml).unwrap();
    let manifest = spec_to_manifest(&spec, "test").unwrap();

    let create_job = manifest.functions.iter().find(|f| f.name == "create_job").unwrap();
    assert_eq!(
        create_job.response_schema.as_deref(),
        Some("Job"),
        "Should extract schema from 202 response"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p toolscript --lib codegen::parser::tests::test_response_schema_broader 2>&1 | tail -5`
Expected: FAIL — only 200 and 201 are checked

**Step 3: Write implementation**

Update `extract_response_schema` in `parser.rs`:

```rust
fn extract_response_schema(responses: &openapiv3::Responses) -> Option<String> {
    // Check all 2xx responses for a schema reference
    for code in 200..=299u16 {
        let status = openapiv3::StatusCode::Code(code);
        if let Some(ReferenceOr::Item(response)) = responses.responses.get(&status)
            && let Some(media_type) = response.content.get("application/json")
            && let Some(schema_ref) = &media_type.schema
        {
            if let Some(name) = extract_ref_name(schema_ref) {
                return Some(name);
            }
            if let ReferenceOr::Item(schema) = schema_ref
                && let SchemaKind::Type(Type::Array(arr)) = &schema.schema_kind
                && let Some(items) = &arr.items
                && let Some(name) = extract_ref_name(items)
            {
                return Some(name);
            }
        }
    }

    // Check default response
    if let Some(ReferenceOr::Item(response)) = &responses.default
        && let Some(media_type) = response.content.get("application/json")
        && let Some(schema_ref) = &media_type.schema
        && let Some(name) = extract_ref_name(schema_ref)
    {
        return Some(name);
    }

    None
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p toolscript --lib codegen::parser 2>&1 | tail -10`
Expected: PASS

**Step 5: Commit**

```bash
git add src/codegen/parser.rs
git commit -m "feat: extract response schemas from all 2xx status codes"
```

---

### Task 6: Fix up existing tests and run full test suite

**Files:**
- Modify: `src/codegen/parser.rs` (existing tests — add `nullable: false, format: None` to FieldDef assertions where needed)
- Modify: `src/codegen/annotations.rs` (existing tests)
- Modify: `src/codegen/manifest.rs` (existing tests)
- Modify: `src/runtime/registry.rs` (existing tests)
- Modify: `tests/codegen_integration.rs`
- Modify: `tests/full_roundtrip.rs`

**Step 1: Run the full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: Some tests may fail due to new required fields on `FieldDef`

**Step 2: Fix all compilation errors**

Every place that constructs a `FieldDef` needs `nullable: false, format: None` added. Every call to `handler.request()` needs an extra `&[]` for headers. Search for all construction sites:

- `src/codegen/manifest.rs` tests: ~8 FieldDef instances
- `src/codegen/annotations.rs` tests: ~10 FieldDef instances
- `src/runtime/registry.rs` tests: no FieldDef, but `handler.request()` calls need `&[]`
- `src/runtime/http.rs` tests: `handler.request()` calls need `&[]`
- `tests/codegen_integration.rs`: no direct FieldDef construction, but verify output
- `tests/full_roundtrip.rs`: check if it constructs FieldDefs

**Step 3: Run tests again**

Run: `cargo test 2>&1 | tail -20`
Expected: ALL PASS

**Step 4: Commit**

```bash
git add -A
git commit -m "test: update all tests for new manifest fields and header param support"
```

---

### Task 7: Update OPENAPI_GAPS.md to mark completed items

**Files:**
- Modify: `OPENAPI_GAPS.md`

**Step 1: Mark items 1 (allOf), 3 (nullable), 4 (additionalProperties), 5 (error responses), 7 (header params), and 8 (format) as done in the priority table.**

Change each row to show a checkmark and note the status.

**Step 2: Commit**

```bash
git add OPENAPI_GAPS.md
git commit -m "docs: mark 6 completed OpenAPI gaps in OPENAPI_GAPS.md"
```

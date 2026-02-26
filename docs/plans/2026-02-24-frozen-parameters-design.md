# Frozen Parameters + Table-Based Args Design

## Overview

Two related changes to the MCP server:

1. **Frozen parameters**: Server-side fixed values configured in `toolscript.toml` that are completely hidden from the LLM and injected at request time.
2. **Table-based calling convention**: Switch from positional args to named table args in the Luau SDK, replacing the fragile positional system.

## Configuration

Frozen params are configured in `toolscript.toml` at two levels:

```toml
# Global — applies to every API
[frozen_params]
api_version = "v2"

# Per-API — applies to all operations in this API
[apis.petstore]
spec = "petstore.yaml"
[apis.petstore.frozen_params]
tenant_id = "abc-123"
```

Per-API values override global values when the same parameter name appears in both. Non-matching frozen param names (params that don't exist in an operation) are silently ignored.

Config structs gain `frozen_params: Option<HashMap<String, String>>` at both `ToolScriptConfig` and `ConfigApiEntry` levels.

## Manifest

`ParamDef` gains one field:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub frozen_value: Option<String>,
```

- `None` — normal parameter, LLM supplies it
- `Some("v2")` — frozen, value is "v2", hidden from LLM

All existing param metadata (name, location, type, format, enum_values, etc.) stays intact. Frozen params retain their full metadata so the runtime knows how to inject them correctly.

## Codegen

### Parser (`parser.rs`)

After parsing each parameter from the OpenAPI spec, check the merged frozen config (global + per-API, per-API wins on conflicts). If the param name matches a frozen config entry, set `frozen_value = Some(configured_value)`. The param stays in the `parameters` vec with all its metadata.

### Annotations (`annotations.rs`)

Function signatures change from positional to table-based. Frozen params are excluded from the params table type.

The function signature adapts based on visible (non-frozen) params and body presence:

```lua
-- Non-frozen params, no body:
function sdk.list_pets(params: { status: string, limit: number? }): ListPetsResponse

-- Non-frozen params + body:
function sdk.create_pet(params: { tag: string? }, body: CreatePetBody): CreatePetResponse

-- All params frozen, no body (zero args):
function sdk.get_status(): GetStatusResponse

-- All params frozen + body (body becomes sole arg):
function sdk.create_thing(body: CreateThingBody): CreateThingResponse
```

## Runtime

### Registry (`registry.rs`)

The calling convention switches from positional to table-based:

```lua
sdk.list_pets({ status = "available", limit = 10 })
sdk.create_pet({ tag = "pet" }, { name = "Fluffy" })
sdk.get_status()
sdk.create_thing({ name = "Fluffy" })
```

The registry determines the calling convention per function:
- `has_visible_params` = any param where `frozen_value.is_none()`
- `has_body` = `func_def.request_body.is_some()`

Arg extraction by case:

| `has_visible_params` | `has_body` | Arg 0 | Arg 1 |
|---|---|---|---|
| true | false | params table | — |
| true | true | params table | body |
| false | false | — | — |
| false | true | body | — |

For each param in `func_def.parameters`:
- If `frozen_value.is_some()`: use the frozen value directly, don't look in the table
- If `frozen_value.is_none()`: look up `param.name` in the params table via `table.get()`
- Route to correct location (path substitution, query, or header) as before
- Frozen values skip validation (admin-configured, trusted)

### Server (`server/mod.rs`)

When listing MCP tools, filter frozen params from the tool's input schema so the LLM never sees them.

## Error Handling

- Missing required non-frozen param in the table: error
- Frozen param name not matching any operation param: silently ignored
- No type pre-validation on frozen values — let the API return its own error

## Testing

- Config parsing with frozen params at global and per-API levels, precedence behavior
- Codegen: `frozen_value` is set on correct params, Luau annotations exclude frozen params, adaptive signatures work for all four cases
- Registry: table-based extraction, frozen value injection, body as second/sole/absent arg
- Integration: end-to-end with frozen + non-frozen params across all locations (path, query, header)

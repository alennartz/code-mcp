# End-to-End Test Suite Design

## Overview

A full-stack end-to-end test harness for code-mcp that exercises the entire pipeline:
a real HTTP API (FastAPI) serving an OpenAPI spec, `code-mcp run` pointed at that
spec, and a Python MCP client running tests against the resulting MCP server.

## Architecture

```
Test HTTP API (FastAPI)  <──HTTP──>  code-mcp (MCP server)  <──stdio/HTTP──>  MCP Client (pytest)
       │                                    │                                       │
  Serves endpoints              Generates SDK from spec              Invokes tools & scripts
  + OpenAPI spec                Executes Luau scripts                Asserts correct output
                                Calls test API via HTTP
```

## Approach

Single Python pytest package in `e2e/`. Pytest session-scoped fixtures manage all
process lifecycle: start the test API, build and spawn code-mcp, connect the MCP
client. No Docker or shell scripts required.

## Test API (FastAPI)

### Endpoints

| Route                       | Method | Purpose                 | OpenAPI shape            |
|-----------------------------|--------|-------------------------|--------------------------|
| `/pets`                     | GET    | List pets               | Query params, array resp |
| `/pets`                     | POST   | Create pet              | Request body, object     |
| `/pets/{pet_id}`            | GET    | Get pet by ID           | Path param, nested obj   |
| `/pets/{pet_id}`            | PUT    | Update pet              | Path param + body        |
| `/pets/{pet_id}`            | DELETE | Delete pet              | Path param, empty resp   |
| `/owners`                   | GET    | List owners             | Second resource          |
| `/owners/{owner_id}/pets`   | GET    | Nested resource         | Nested path params       |
| `/reset`                    | POST   | Reset data (admin)      | Test isolation           |

### Models

- **Pet**: id (int), name (str), status (enum: active/adopted/pending), tag (optional str), owner_id (optional int)
- **Owner**: id (int), name (str), email (str)
- **PetList**: items (list[Pet]), total (int)

### Auth

- Bearer token (`test-secret-123`) on mutation endpoints (POST/PUT/DELETE)
- API key header (`X-Api-Key: test-key-456`) as alternative
- Read endpoints are public
- Data is in-memory, reset per test via `POST /reset`

## Directory Structure

```
e2e/
  pyproject.toml              # Deps: fastapi, uvicorn, mcp, pytest, pytest-asyncio
  conftest.py                 # Session-scoped fixtures
  test_api/
    __init__.py
    app.py                    # FastAPI app
    models.py                 # Pydantic models
    auth.py                   # Bearer/API-key middleware
    seed.py                   # Initial test data
  tests/
    __init__.py
    conftest.py               # Test-level fixtures (MCP client helpers)
    test_stdio_tools.py       # Tool invocation tests over stdio
    test_stdio_scripts.py     # Script execution tests over stdio
    test_http_transport.py    # HTTP/SSE transport tests
    test_auth.py              # Auth-related e2e tests
```

## Fixtures

### Session-scoped

1. **`test_api_server`** — Starts FastAPI on a random port via uvicorn. Yields base URL. Shuts down on teardown.
2. **`code_mcp_binary`** — Runs `cargo build --release` once per session. Returns binary path.
3. **`mcp_stdio_session`** — Spawns `code-mcp run <spec_url>` with auth env vars, connects Python MCP SDK over stdio. Yields `ClientSession`.
4. **`mcp_http_session`** — Spawns `code-mcp run <spec_url> --transport http --port <random>`, connects via streamable HTTP. Yields `ClientSession`.

### Function-scoped

5. **`reset_test_data`** — Calls `POST /reset` on the test API before each test.

### Auth fixtures (session-scoped, for HTTP tests)

6. **`jwt_issuer`** — Creates a test JWT issuer with a known RSA key pair. Signs JWTs on demand.
7. **`jwks_server`** — Serves the JWKS endpoint (public key) on a local port.

## Auth Flow

### Layer 1: Upstream API (code-mcp → test API)

- **Env vars:** Fixture sets `TEST_API_BEARER_TOKEN=test-secret-123` and `TEST_API_API_KEY=test-key-456` when spawning code-mcp.
- **`_meta.auth`:** Some tests pass credentials in the `execute_script` tool arguments to test the credential-merging flow.

### Layer 2: MCP transport (MCP client → code-mcp HTTP)

- Only applies to HTTP transport tests.
- Test JWT issuer signs tokens with a known key pair.
- JWKS endpoint served by a fixture.
- code-mcp started with `--auth-authority <issuer_url> --auth-audience test-audience --auth-jwks-uri <jwks_url>`.
- MCP HTTP client includes the signed JWT as Bearer token.
- stdio tests skip this layer entirely.

## Test Cases

### test_stdio_tools.py — MCP tool invocations

| Test                           | Verifies                                         |
|--------------------------------|--------------------------------------------------|
| `test_list_apis`               | `list_apis` returns test API with correct metadata|
| `test_list_functions`          | `list_functions` returns all generated functions  |
| `test_list_functions_filter`   | Tag filtering works                               |
| `test_get_function_docs`       | Returns Luau type signature for a function        |
| `test_search_docs`             | Full-text search finds matching functions/schemas |
| `test_get_schema`              | Returns Luau type definition for Pet, Owner       |

### test_stdio_scripts.py — Script execution

| Test                           | Verifies                                         |
|--------------------------------|--------------------------------------------------|
| `test_list_pets`               | `sdk.list_pets()` returns seeded data             |
| `test_get_pet_by_id`           | `sdk.get_pet(1)` returns correct pet              |
| `test_create_pet`              | `sdk.create_pet({...})` with auth creates pet     |
| `test_update_pet`              | `sdk.update_pet(1, {...})` modifies pet           |
| `test_delete_pet`              | `sdk.delete_pet(1)` removes pet                   |
| `test_query_params`            | `sdk.list_pets({ limit=2, status="active" })` filters |
| `test_nested_resource`         | `sdk.list_owner_pets(1)` returns owner's pets     |
| `test_multi_call_script`       | Script chains list → get using output of first call |
| `test_create_then_fetch`       | Script creates pet, then fetches it by returned ID|
| `test_script_error_handling`   | Bad endpoint call returns error, doesn't crash    |
| `test_enum_values`             | Pet status enum round-trips correctly             |
| `test_optional_fields`         | Nil/missing optional fields handled correctly     |
| `test_meta_auth_override`      | `_meta.auth` overrides env var credentials        |

### test_http_transport.py — HTTP/SSE transport

| Test                           | Verifies                                         |
|--------------------------------|--------------------------------------------------|
| `test_http_list_tools`         | Tools accessible over HTTP                        |
| `test_http_execute_script`     | Script execution works over HTTP                  |
| `test_http_auth_required`      | Request without JWT returns 401                   |
| `test_http_auth_valid_jwt`     | Request with valid JWT succeeds                   |
| `test_http_well_known`         | Well-known endpoint returns correct metadata      |

### test_auth.py — Auth edge cases

| Test                           | Verifies                                         |
|--------------------------------|--------------------------------------------------|
| `test_no_auth_read_succeeds`   | Public endpoints work without credentials         |
| `test_no_auth_write_fails`     | Protected endpoints fail without credentials      |
| `test_bearer_token_auth`       | Bearer auth works for mutations                   |
| `test_api_key_auth`            | API key auth works for mutations                  |

### Execution limits (in test_stdio_scripts.py)

| Test                           | Verifies                                         |
|--------------------------------|--------------------------------------------------|
| `test_script_timeout`          | Infinite loop killed after timeout                |
| `test_max_api_calls_exceeded`  | Script stopped at API call limit                  |
| `test_sandbox_no_file_io`      | `io.open()` blocked by sandbox                    |

These use a separate code-mcp instance with short limits (`--timeout 2 --max-api-calls 3`).

## Dependencies

```
fastapi
uvicorn
mcp              # Official Python MCP SDK
pytest
pytest-asyncio
PyJWT            # For test JWT issuer
cryptography     # RSA key generation for JWTs
httpx            # For calling test API reset endpoint
```

## Running

```bash
# Build code-mcp first
cargo build --release

# Run e2e tests
cd e2e && pytest
```

Alternatively, `conftest.py` can trigger `cargo build` automatically if the binary is missing.

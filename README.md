# code-mcp

Turn OpenAPI specs into scriptable MCP servers. One round-trip instead of many.

## The Problem

AI agents using MCP tools over complex APIs waste resources. Each API call becomes a separate tool invocation, and the LLM round-trips all intermediate state even when no judgment is needed. The LLM becomes an expensive data shuttle.

## The Solution

code-mcp gives the LLM a [Luau](https://luau-lang.org/) scripting runtime with an auto-generated, strongly-typed SDK derived from OpenAPI specs. The LLM writes a script that chains multiple API calls, sends it for execution, and gets back the result. One round-trip instead of many.

## Quick Start

```bash
cargo install --path .
```

Add the server to your MCP client config:

```json
{
  "mcpServers": {
    "petstore": {
      "command": "code-mcp",
      "args": ["run", "https://petstore3.swagger.io/api/v3/openapi.json"],
      "env": {
        "PETSTORE_BEARER_TOKEN": "your-token-here"
      }
    }
  }
}
```

This is the recommended path for most users: local process, stdio transport, credentials in your environment. No server to deploy, no network to configure.

Docker is also available:

```bash
docker build -t code-mcp .
docker run code-mcp https://petstore3.swagger.io/api/v3/openapi.json
```

## How It Works

1. The agent connects to the MCP server.
2. It explores the SDK using documentation tools (`list_apis`, `list_functions`, `get_function_docs`, `search_docs`, `get_schema`) or by browsing resources (`sdk://petstore/overview`, `sdk://petstore/functions`, etc.).
3. It writes a Luau script that chains SDK calls.
4. It sends the script to `execute_script`.
5. It gets back the result, captured logs, and execution stats in a single response.

Example script the LLM might write:

```lua
-- Get all pets, then fetch details for the first one
local pets = sdk.list_pets({ limit = 5 })
local first = pets[1]
local details = sdk.get_pet(first.id)
return { pet = details, total = #pets }
```

The response includes the return value as JSON, any `print()` output captured as logs, and stats (API call count, wall-clock duration).

## CLI Reference

### `code-mcp run`

Generate and serve in one step. This is the most common subcommand.

```
code-mcp run <SPECS>... [OPTIONS]
```

| Flag               | Default | Description                                    |
| ------------------ | ------- | ---------------------------------------------- |
| `--transport`      | `stdio` | Transport type (`stdio`, `sse`)                |
| `--port`           | `8080`  | Port for HTTP/SSE transport                    |
| `--timeout`        | `30`    | Script execution timeout (seconds)             |
| `--memory-limit`   | `64`    | Luau VM memory limit (MB)                      |
| `--max-api-calls`  | `100`   | Max upstream API calls per script              |
| `--auth-authority` | --      | OAuth issuer URL (enables JWT auth)            |
| `--auth-audience`  | --      | Expected JWT audience                          |
| `--auth-jwks-uri`  | --      | Explicit JWKS URI override                     |

`<SPECS>` accepts one or more OpenAPI spec sources: file paths or URLs.

### `code-mcp generate`

Code generation only. Produces a manifest and SDK annotations without starting a server.

```
code-mcp generate <SPECS>... [-o <DIR>]
```

Output directory defaults to `./output`. Generates `manifest.json` and `sdk/*.luau`.

### `code-mcp serve`

Start an MCP server from a pre-generated output directory.

```
code-mcp serve <DIR> [OPTIONS]
```

Accepts the same options as `run` (`--transport`, `--port`, `--timeout`, `--memory-limit`, `--max-api-calls`, `--auth-authority`, `--auth-audience`, `--auth-jwks-uri`).

## Authentication

There are two separate authentication layers.

### Upstream API Credentials

These are the credentials code-mcp uses to call the APIs behind the SDK.

**Environment variables** (recommended for local use):

| Variable                    | Auth type    |
| --------------------------- | ------------ |
| `{API_NAME}_BEARER_TOKEN`   | Bearer token |
| `{API_NAME}_API_KEY`        | API key      |
| `{API_NAME}_BASIC_USER`     | Basic auth   |
| `{API_NAME}_BASIC_PASS`     | Basic auth   |

The API name is uppercased. For an API named `petstore`, set `PETSTORE_BEARER_TOKEN`.

**Per-request via `_meta.auth`** (overrides env vars):

```json
{
  "method": "tools/call",
  "params": {
    "name": "execute_script",
    "arguments": { "script": "return sdk.list_pets()" },
    "_meta": {
      "auth": {
        "petstore": { "type": "bearer", "token": "sk-runtime-token" },
        "billing": { "type": "api_key", "key": "key-abc123" },
        "legacy":  { "type": "basic", "username": "user", "password": "pass" }
      }
    }
  }
}
```

Meta credentials take precedence when the same API name appears in both sources.

### MCP-Layer Authentication

This controls who can connect to the code-mcp server itself. It only applies when using HTTP/SSE transport.

- JWT validation with OIDC discovery
- Enable with `--auth-authority` and `--auth-audience`
- Optionally override the JWKS endpoint with `--auth-jwks-uri`
- Publishes `/.well-known/oauth-protected-resource` for client discovery

For local stdio usage, this layer is not needed -- the MCP client and server share the same trust boundary.

## Execution Limits

| Flag              | Default | Controls                                    |
| ----------------- | ------- | ------------------------------------------- |
| `--timeout`       | 30s     | Wall-clock deadline per script execution    |
| `--memory-limit`  | 64 MB   | Maximum Luau VM memory allocation           |
| `--max-api-calls` | 100     | Maximum upstream HTTP requests per script   |

CPU is limited indirectly by the wall-clock timeout. There is no separate instruction-count limit.

## MCP Tools and Resources

### Tools

| Tool                | Description                                                          |
| ------------------- | -------------------------------------------------------------------- |
| `list_apis`         | List loaded APIs with names, descriptions, base URLs, endpoint counts |
| `list_functions`    | List SDK functions, filterable by API or tag                         |
| `get_function_docs` | Full Luau type annotation for a function                             |
| `search_docs`       | Full-text search across all SDK documentation                        |
| `get_schema`        | Full Luau type annotation for a schema/type                          |
| `execute_script`    | Execute a Luau script against the SDK                                |

### Resources

Browsable SDK documentation, accessible via `resources/read`:

| URI pattern                      | Content                    |
| -------------------------------- | -------------------------- |
| `sdk://{api}/overview`           | API overview               |
| `sdk://{api}/functions`          | All function signatures    |
| `sdk://{api}/schemas`            | All type definitions       |
| `sdk://{api}/functions/{name}`   | Individual function docs   |
| `sdk://{api}/schemas/{name}`     | Individual schema docs     |

## Sandbox Security

Scripts execute in a sandboxed Luau VM. Here is what is and is not available.

**Allowed:**

- Standard libraries: `string`, `table`, `math`
- `os.clock()` (wall-clock timing only)
- `print()` (captured to logs, not written to stdout)
- `json.encode()` / `json.decode()`
- `sdk.*` functions (generated from the OpenAPI spec)

**Blocked:**

- `io` (file I/O)
- `os.execute` (shell access)
- `loadfile`, `dofile`, `require` (module loading)
- `debug` library
- `string.dump` (bytecode access)
- `load` (dynamic code loading)
- Raw network access
- Filesystem access

**Enforcement mechanisms:**

- Luau native sandbox mode (read-only globals, isolated per-script environments)
- Configurable memory limit
- Wall-clock timeout via Luau interrupt callbacks
- API call counter per execution
- Fresh VM per execution (no state leaks between scripts)
- Credentials never exposed to Luau -- injected server-side

**A note on hosting.** If you deploy code-mcp over HTTP for multiple users, you are offering your compute as a code sandbox. The sandboxing limits the abuse surface, but you should deploy behind appropriate resource constraints and network policies. For most use cases, running locally over stdio with your own credentials is the simplest and most secure option.

## Docker

Build and run:

```bash
docker build -t code-mcp .
docker run code-mcp https://api.example.com/openapi.json
```

For HTTP transport:

```bash
docker run -p 8080:8080 code-mcp \
  https://api.example.com/openapi.json \
  --transport sse --port 8080
```

## Building from Source

```bash
git clone https://github.com/alenna/code-mcp.git
cd code-mcp
cargo build --release
cargo test
```

Requires Rust 1.85+ (uses edition 2024).

## License

MIT

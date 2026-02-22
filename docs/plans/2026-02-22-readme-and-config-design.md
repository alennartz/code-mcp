# README & Configuration Design

## Configuration: Script Execution Limits

### Problem

Three runtime limits are hardcoded in `ExecutorConfig::default()` and not exposed
to CLI users:

| Setting | Default | Location |
|---|---|---|
| Script timeout | 30 000 ms | `executor.rs:25` |
| Memory limit | 64 MB | `executor.rs:27` |
| Max API calls | 100 | `executor.rs:28` |

### Solution

Add three CLI flags to `Serve` and `Run` subcommands:

| Flag | Type | Default | Description |
|---|---|---|---|
| `--timeout` | u64 (seconds) | `30` | Script execution timeout |
| `--memory-limit` | usize (MB) | `64` | Luau VM memory limit |
| `--max-api-calls` | usize | `100` | Max upstream API calls per script |

Flags use human-friendly units (seconds, megabytes) and convert internally.
All three are optional with the same defaults as today -- zero behavior change
for existing users.

### Flow

```
CLI flag  -->  main.rs builds ExecutorConfig  -->  passed to CodeMcpServer::new()
```

The `serve()` function in `main.rs` currently creates `ExecutorConfig::default()`.
It will instead accept the three values from CLI parsing and construct the config
explicitly.

---

## README Structure

Target audience: MCP-savvy developers. Style: clean & professional (ripgrep/delta).

### Sections

1. **Title + one-line description**
2. **The problem** -- why per-tool MCP servers waste LLM resources
3. **The solution** -- script-based execution in 3 sentences
4. **Quick start** -- install + minimal usage in <10 lines
5. **How it works** -- the agent workflow (explore -> script -> execute)
6. **CLI reference** -- all subcommands, flags, defaults in a table
7. **Authentication** -- upstream API auth (env vars, `_meta.auth`) + MCP-layer JWT auth
8. **Configuration** -- execution limits, transport options
9. **MCP tools & resources** -- what the server exposes
10. **Sandbox security** -- what Luau scripts can and cannot do
11. **Docker** -- container usage
12. **Building from source** -- cargo build + test
13. **License** (placeholder -- not yet specified)

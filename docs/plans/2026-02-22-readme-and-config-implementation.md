# README & Configurable Execution Limits — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add CLI flags for script execution limits (timeout, memory, max API calls) and write a comprehensive README.

**Architecture:** Three new CLI flags on `Serve` and `Run` subcommands flow through `main.rs` into `ExecutorConfig`. The README is a single `README.md` at the repo root following problem-first narrative structure.

**Tech Stack:** Rust (clap CLI), Markdown

---

### Task 1: Add execution limit flags to CLI

**Files:**
- Modify: `src/cli.rs:22-63`

**Step 1: Write the failing test**

```bash
cargo build 2>&1 | head -5
```

No test needed — this is a struct change. We verify by building after the change.

**Step 2: Add flags to `Serve` and `Run` variants**

In `src/cli.rs`, add these three fields to both the `Serve` and `Run` variants, after the existing auth flags:

```rust
/// Script execution timeout in seconds (default: 30)
#[arg(long, default_value = "30")]
timeout: u64,
/// Luau VM memory limit in megabytes (default: 64)
#[arg(long, default_value = "64")]
memory_limit: usize,
/// Maximum API calls per script execution (default: 100)
#[arg(long, default_value = "100")]
max_api_calls: usize,
```

**Step 3: Verify it compiles**

Run: `cargo build 2>&1 | tail -3`
Expected: warnings about unused fields, but successful build

**Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add --timeout, --memory-limit, --max-api-calls CLI flags"
```

---

### Task 2: Wire CLI flags into ExecutorConfig

**Files:**
- Modify: `src/main.rs:25-51` (match arms for Serve and Run)
- Modify: `src/main.rs:88-103` (serve function)

**Step 1: Update `Command::Serve` and `Command::Run` match arms**

Destructure the new fields in both arms. Pass them to `serve()`.

Update the `serve()` function signature to accept the three values:

```rust
async fn serve(
    manifest: Manifest,
    transport: &str,
    port: u16,
    auth_config: Option<McpAuthConfig>,
    timeout: u64,
    memory_limit: usize,
    max_api_calls: usize,
) -> anyhow::Result<()> {
```

Inside `serve()`, replace `ExecutorConfig::default()` with:

```rust
let config = ExecutorConfig {
    timeout_ms: timeout * 1000,
    memory_limit: Some(memory_limit * 1024 * 1024),
    max_api_calls: Some(max_api_calls),
};
```

**Step 2: Verify it compiles and tests pass**

Run: `cargo test 2>&1 | tail -5`
Expected: all 111 tests pass, no warnings about unused fields

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire execution limit flags into ExecutorConfig"
```

---

### Task 3: Test CLI flag parsing

**Files:**
- Modify: `src/cli.rs` (add test module at end of file)

**Step 1: Write the test**

Add a test module to `src/cli.rs`:

```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use clap::Parser;

    #[test]
    fn test_run_defaults() {
        let cli = Cli::parse_from(["code-mcp", "run", "spec.yaml"]);
        match cli.command {
            Command::Run {
                timeout,
                memory_limit,
                max_api_calls,
                ..
            } => {
                assert_eq!(timeout, 30);
                assert_eq!(memory_limit, 64);
                assert_eq!(max_api_calls, 100);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn test_run_custom_limits() {
        let cli = Cli::parse_from([
            "code-mcp", "run", "spec.yaml",
            "--timeout", "60",
            "--memory-limit", "128",
            "--max-api-calls", "50",
        ]);
        match cli.command {
            Command::Run {
                timeout,
                memory_limit,
                max_api_calls,
                ..
            } => {
                assert_eq!(timeout, 60);
                assert_eq!(memory_limit, 128);
                assert_eq!(max_api_calls, 50);
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn test_serve_defaults() {
        let cli = Cli::parse_from(["code-mcp", "serve", "./output"]);
        match cli.command {
            Command::Serve {
                timeout,
                memory_limit,
                max_api_calls,
                ..
            } => {
                assert_eq!(timeout, 30);
                assert_eq!(memory_limit, 64);
                assert_eq!(max_api_calls, 100);
            }
            _ => panic!("expected Serve"),
        }
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test --bin code-mcp -- cli::tests -v 2>&1 | tail -10`
Expected: 3 tests pass

**Step 3: Commit**

```bash
git add src/cli.rs
git commit -m "test: add CLI flag parsing tests for execution limits"
```

---

### Task 4: Write README.md

**Files:**
- Create: `README.md`

**Step 1: Write the README**

Create `README.md` at the repository root. The full content follows. Key sections:

1. Title + one-liner
2. The problem (why per-tool MCP is wasteful)
3. The solution (script-based execution)
4. Quick start (install + 3-line usage)
5. How it works (explore -> script -> execute workflow)
6. CLI reference (subcommands, all flags with defaults)
7. Authentication (env vars, `_meta.auth`, JWT)
8. Configuration (execution limits table)
9. MCP tools & resources (what the server exposes)
10. Sandbox security model
11. Docker
12. Building from source

Use the actual CLI flag names, actual default values, and actual tool/resource names from the source code. All examples should be realistic and runnable.

**Step 2: Verify markdown renders correctly**

Visually inspect the file for broken formatting.

**Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add comprehensive README"
```

---

### Task 5: Final verification

**Step 1: Run full test suite**

Run: `cargo test 2>&1 | tail -5`
Expected: all tests pass (111 + 3 new CLI tests = 114)

**Step 2: Run clippy**

Run: `cargo clippy 2>&1 | tail -5`
Expected: no warnings or errors

**Step 3: Verify --help output includes new flags**

Run: `cargo run -- run --help 2>&1`
Expected: shows `--timeout`, `--memory-limit`, `--max-api-calls` with descriptions and defaults

Run: `cargo run -- serve --help 2>&1`
Expected: same three flags present

# ToolScript Rebrand Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rebrand the entire repository from `code-mcp` to `toolscript` across all naming conventions, files, configs, docs, CI, and GitHub.

**Architecture:** Systematic find-and-replace across four naming conventions (kebab, PascalCase, snake_case, SCREAMING_CASE), staged by layer with build verification between stages. File renames as a final code step, then GitHub repo rename.

**Tech Stack:** Rust (Cargo), Python (pytest), Docker, GitHub Actions, GitHub CLI

---

### Task 1: Rename Rust package and CLI entry point

**Files:**
- Modify: `Cargo.toml:2`
- Modify: `src/cli.rs:5,90,108,121,141,160,178,196,213,224,236`

**Step 1: Update Cargo.toml package name**

In `Cargo.toml`, change line 2:
```
name = "code-mcp"
```
to:
```
name = "toolscript"
```

**Step 2: Update CLI command name in src/cli.rs**

On line 5, change:
```rust
#[command(name = "code-mcp", about = "Generate MCP servers from OpenAPI specs")]
```
to:
```rust
#[command(name = "toolscript", about = "Generate MCP servers from OpenAPI specs")]
```

**Step 3: Update all test parse_from calls in src/cli.rs**

Replace every `"code-mcp"` string in `Cli::parse_from` calls (lines 90, 108, 121, 141, 160, 178, 196, 213, 224, 236) with `"toolscript"`. There are 11 occurrences total.

Also update the config file assertion on line 112:
```rust
assert_eq!(config.unwrap().to_str().unwrap(), "code-mcp.toml");
```
to:
```rust
assert_eq!(config.unwrap().to_str().unwrap(), "toolscript.toml");
```

And update the test input on line 108:
```rust
let cli = Cli::parse_from(["code-mcp", "run", "--config", "code-mcp.toml"]);
```
to:
```rust
let cli = Cli::parse_from(["toolscript", "run", "--config", "toolscript.toml"]);
```

**Step 4: Verify compilation**

Run: `cargo build 2>&1`
Expected: Build succeeds (may have warnings about unused imports since we haven't renamed types yet)

**Step 5: Commit**

```bash
git add Cargo.toml src/cli.rs
git commit -m "refactor: rename package and CLI from code-mcp to toolscript"
```

---

### Task 2: Rename Rust types and crate imports

**Files:**
- Modify: `src/config.rs:51` — `CodeMcpConfig` → `ToolScriptConfig`
- Modify: `src/server/mod.rs:23,36,134,296-297` — `CodeMcpServer` → `ToolScriptServer`
- Modify: `src/server/mod.rs:111-112` — server info name/title strings
- Modify: `src/main.rs:10-18` — all `use code_mcp::` → `use tool_script::`, `CodeMcpConfig` → `ToolScriptConfig`, `CodeMcpServer` → `ToolScriptServer`
- Modify: `src/main.rs:151,178,196-197,210,219,247,280,336,344,360,370,374` — all remaining `CodeMcpConfig` and `CodeMcpServer` references
- Modify: `tests/full_roundtrip.rs:6-10` — `use code_mcp::` → `use tool_script::`
- Modify: `tests/codegen_integration.rs:5-6` — `use code_mcp::` → `use tool_script::`
- Modify: `tests/http_auth_test.rs:9` — `code_mcp::` → `tool_script::`

**Step 1: Rename CodeMcpConfig in src/config.rs**

On line 51, change:
```rust
pub struct CodeMcpConfig {
```
to:
```rust
pub struct ToolScriptConfig {
```

Then replace all remaining `CodeMcpConfig` occurrences in `src/config.rs` (in function signatures and test code) with `ToolScriptConfig`.

**Step 2: Rename CodeMcpServer in src/server/mod.rs**

Replace all `CodeMcpServer` with `ToolScriptServer` in `src/server/mod.rs`.

Update the server info strings on lines 111-112:
```rust
name: "code-mcp".to_string(),
title: Some("code-mcp".to_string()),
```
to:
```rust
name: "toolscript".to_string(),
title: Some("toolscript".to_string()),
```

**Step 3: Update crate imports in src/main.rs**

Replace all `use code_mcp::` with `use tool_script::` (lines 10-19).
Replace all `CodeMcpConfig` with `ToolScriptConfig`.
Replace all `CodeMcpServer` with `ToolScriptServer`.
Replace `code_mcp::server::tools::` with `tool_script::server::tools::` (lines 396-401).
Replace `code_mcp::server::auth::` with `tool_script::server::auth::` (line 374).

**Step 4: Update auto-discovery config filename in src/main.rs**

On line 196, change:
```rust
let default_path = Path::new("code-mcp.toml");
```
to:
```rust
let default_path = Path::new("toolscript.toml");
```

On line 210, change the error message:
```rust
"no specs provided. Pass spec paths/URLs, use --config, or create code-mcp.toml"
```
to:
```rust
"no specs provided. Pass spec paths/URLs, use --config, or create toolscript.toml"
```

On line 174, update the doc comment:
```rust
/// Supports auto-discovery of `code-mcp.toml` when no specs or config are provided.
```
to:
```rust
/// Supports auto-discovery of `toolscript.toml` when no specs or config are provided.
```

**Step 5: Update default output directory in src/main.rs**

On line 280, change:
```rust
.unwrap_or_else(|| PathBuf::from("./code-mcp-output"));
```
to:
```rust
.unwrap_or_else(|| PathBuf::from("./toolscript-output"));
```

**Step 6: Update GitHub URL in src/main.rs**

On line 417, change:
```rust
"resource_documentation": "https://github.com/alenna/code-mcp",
```
to:
```rust
"resource_documentation": "https://github.com/alenna/toolscript",
```

**Step 7: Update doc comment in src/main.rs**

On line 336, change:
```rust
/// Create a `CodeMcpServer` from a manifest and serve it with the given transport.
```
to:
```rust
/// Create a `ToolScriptServer` from a manifest and serve it with the given transport.
```

**Step 8: Update test imports**

In `tests/full_roundtrip.rs`, replace all `use code_mcp::` with `use tool_script::` (lines 6-10).

In `tests/codegen_integration.rs`, replace all `use code_mcp::` with `use tool_script::` (lines 5-6).

In `tests/http_auth_test.rs`, replace `code_mcp::` with `tool_script::` (line 9).

**Step 9: Verify compilation and tests**

Run: `cargo build 2>&1`
Expected: Build succeeds

Run: `cargo test 2>&1`
Expected: All tests pass

**Step 10: Commit**

```bash
git add src/config.rs src/server/mod.rs src/main.rs tests/
git commit -m "refactor: rename Rust types and crate imports to ToolScript"
```

---

### Task 3: Update E2E test infrastructure

**Files:**
- Modify: `e2e/conftest.py:13,36,38`
- Modify: `e2e/tests/conftest.py:117-118,124,188-189,193,224,273,310-312,355`
- Modify: `e2e/pyproject.toml:2`
- Modify: `e2e/test_api/app.py:14`

**Step 1: Update e2e/conftest.py**

Line 13 — change binary path:
```python
CODE_MCP_BINARY = PROJECT_ROOT / "target" / "release" / "code-mcp"
```
to:
```python
TOOLSCRIPT_BINARY = PROJECT_ROOT / "target" / "release" / "toolscript"
```

Line 36 — rename fixture:
```python
def code_mcp_binary() -> Path:
```
to:
```python
def toolscript_binary() -> Path:
```

Line 38 — update env var check:
```python
if os.environ.get("CODE_MCP_URL"):
    return CODE_MCP_BINARY
if not CODE_MCP_BINARY.exists():
```
to:
```python
if os.environ.get("TOOL_SCRIPT_URL"):
    return TOOLSCRIPT_BINARY
if not TOOLSCRIPT_BINARY.exists():
```

Line 46 — update return:
```python
return CODE_MCP_BINARY
```
to:
```python
return TOOLSCRIPT_BINARY
```

**Step 2: Update e2e/tests/conftest.py**

Replace all `code_mcp_binary` parameter/fixture references with `toolscript_binary`.
Replace all `CODE_MCP_URL` with `TOOL_SCRIPT_URL`.
Replace `"code-mcp-output"` on line 312 with `"toolscript-output"`.
Update docstrings: `"code-mcp"` → `"toolscript"` throughout.

**Step 3: Update e2e/pyproject.toml**

Line 2:
```toml
name = "code-mcp-e2e"
```
to:
```toml
name = "toolscript-e2e"
```

**Step 4: Update e2e/test_api/app.py**

Line 14:
```python
description="E2E test API for code-mcp",
```
to:
```python
description="E2E test API for toolscript",
```

Line 50 comment:
```python
# Inject the server URL so code-mcp knows the base URL for API calls.
```
to:
```python
# Inject the server URL so toolscript knows the base URL for API calls.
```

**Step 5: Update e2e/uv.lock**

Line 137:
```
name = "code-mcp-e2e"
```
to:
```
name = "toolscript-e2e"
```

**Step 6: Commit**

```bash
git add e2e/
git commit -m "refactor: rename E2E test references to toolscript"
```

---

### Task 4: Update CI workflow

**Files:**
- Modify: `.github/workflows/ci.yml:77,96,98,100,104,106,109,117,120`

**Step 1: Update Docker image tags**

Line 77:
```yaml
tags: code-mcp:ci
```
to:
```yaml
tags: toolscript:ci
```

**Step 2: Update container references**

Line 96 comment: `"code-mcp"` → `"toolscript"`
Line 98: `--name code-mcp-ci` → `--name toolscript-ci`
Line 100: `code-mcp:ci \` → `toolscript:ci \`
Line 104 comment: `code-mcp` → `toolscript`
Line 106: remove/update `code-mcp` in the wait message → `toolscript`
Line 109: `echo "code-mcp ready"` → `echo "toolscript ready"`

**Step 3: Update env var**

Line 117:
```yaml
CODE_MCP_URL: "http://127.0.0.1:9300"
```
to:
```yaml
TOOL_SCRIPT_URL: "http://127.0.0.1:9300"
```

**Step 4: Update failure step**

Line 120:
```yaml
run: docker logs code-mcp-ci
```
to:
```yaml
run: docker logs toolscript-ci
```

**Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "refactor: rename CI workflow references to toolscript"
```

---

### Task 5: Update Dockerfile

**Files:**
- Modify: `Dockerfile:24-25`

**Step 1: Update binary path and entrypoint**

Line 24:
```dockerfile
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/code-mcp /code-mcp
```
to:
```dockerfile
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/toolscript /toolscript
```

Line 25:
```dockerfile
ENTRYPOINT ["/code-mcp", "run"]
```
to:
```dockerfile
ENTRYPOINT ["/toolscript", "run"]
```

**Step 2: Commit**

```bash
git add Dockerfile
git commit -m "refactor: rename Docker binary to toolscript"
```

---

### Task 6: Update README.md

**Files:**
- Modify: `README.md` (all ~40 references)

**Step 1: Replace all name references**

Apply these replacements throughout the entire file:
- `# code-mcp` → `# toolscript` (line 1)
- `code-mcp` → `toolscript` in all CLI examples, config file references, Docker commands, git clone URLs
- `code-mcp.toml` → `toolscript.toml` in all config file references
- `code-mcp run` → `toolscript run`
- `code-mcp generate` → `toolscript generate`
- `code-mcp serve` → `toolscript serve`
- `code-mcp-output` → `toolscript-output` (if referenced)
- `"command": "code-mcp"` → `"command": "toolscript"` in JSON config examples
- `https://github.com/alenna/code-mcp.git` → `https://github.com/alenna/toolscript.git` (line 320)
- `cd code-mcp` → `cd toolscript` (line 321)

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: rebrand README from code-mcp to toolscript"
```

---

### Task 7: Update vision.md and OPENAPI_GAPS.md

**Files:**
- Modify: `vision.md` — no direct `code-mcp` references (uses generic "tool" language), skip
- Modify: `OPENAPI_GAPS.md` — no direct `code-mcp` references (uses "the project"), skip

**Step 1: Verify no references**

Run: `grep -n "code.mcp" vision.md OPENAPI_GAPS.md`
Expected: No matches (these files use generic language)

**Step 2: Skip — nothing to change**

No commit needed.

---

### Task 8: Update VSCode launch.json

**Files:**
- Modify: `.vscode/launch.json:9,11,23,36`

**Step 1: Replace all references**

Replace all `code-mcp` with `toolscript` in the file:
- Line 9: `"args": ["build", "--bin=toolscript", "--package=toolscript"]`
- Line 11: `"name": "toolscript"`
- Line 23: `"args": ["test", "--no-run", "--lib", "--package=toolscript"]`
- Line 36: `"args": ["test", "--no-run", "--test=*", "--package=toolscript"]`

**Step 2: Commit**

```bash
git add .vscode/launch.json
git commit -m "refactor: rename VSCode debug configs to toolscript"
```

---

### Task 9: Update docs/plans content and rename files

**Files:**
- Modify content in all 24 files in `docs/plans/` — replace `code-mcp` → `toolscript`, `CodeMcp` → `ToolScript`, `code_mcp` → `tool_script`, `CODE_MCP` → `TOOL_SCRIPT`, `code-mcp-output` → `toolscript-output`
- Rename: `docs/plans/2026-02-21-code-mcp-design.md` → `docs/plans/2026-02-21-toolscript-design.md`
- Rename: `docs/plans/2026-02-21-code-mcp-implementation.md` → `docs/plans/2026-02-21-toolscript-implementation.md`

**Step 1: Replace content in all docs/plans files**

For each `.md` file in `docs/plans/`, do a global find-and-replace:
- `code-mcp-output` → `toolscript-output`
- `code-mcp-e2e` → `toolscript-e2e`
- `code-mcp.toml` → `toolscript.toml`
- `code-mcp` → `toolscript`
- `CodeMcpServer` → `ToolScriptServer`
- `CodeMcpConfig` → `ToolScriptConfig`
- `CodeMcp` → `ToolScript`
- `code_mcp` → `tool_script`
- `CODE_MCP` → `TOOL_SCRIPT`

Order matters: replace more specific patterns first to avoid double-replacing.

**Step 2: Rename files**

```bash
git mv docs/plans/2026-02-21-code-mcp-design.md docs/plans/2026-02-21-toolscript-design.md
git mv docs/plans/2026-02-21-code-mcp-implementation.md docs/plans/2026-02-21-toolscript-implementation.md
```

**Step 3: Commit**

```bash
git add docs/plans/
git commit -m "docs: rebrand all planning documents to toolscript"
```

---

### Task 10: Verify full build and tests

**Step 1: Clean build**

Run: `cargo clean && cargo build --release 2>&1`
Expected: Build succeeds

**Step 2: Run unit and integration tests**

Run: `cargo test 2>&1`
Expected: All tests pass

**Step 3: Verify no stale references remain**

Run: `grep -r "code.mcp" --include="*.rs" --include="*.toml" --include="*.yml" --include="*.py" --include="*.json" --include="*.md" . | grep -v target/ | grep -v ".git/"`
Expected: No matches (other than Cargo.lock which auto-updates)

**Step 4: Commit any remaining fixes if needed**

---

### Task 11: GitHub repo rename

**Step 1: Push all changes**

```bash
git push origin main
```

**Step 2: Rename the repository**

```bash
gh repo rename toolscript
```

**Step 3: Update local git remote**

```bash
git remote set-url origin https://github.com/alenna/toolscript.git
```

**Step 4: Verify**

```bash
gh repo view --web
```

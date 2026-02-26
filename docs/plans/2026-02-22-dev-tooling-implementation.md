# Dev Tooling Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the repo developer-ready with pre-commit hooks, strict linting, formatting, VS Code workspace config, and a .gitignore.

**Architecture:** Configuration files + a shell hook script. No new Rust code except fixing existing code to pass strict lints. The Cargo.toml `[lints]` table drives clippy. A `.githooks/pre-commit` shell script auto-formats and runs clippy on commit. VS Code settings use rust-analyzer with clippy integration.

**Tech Stack:** Rust toolchain (cargo fmt, cargo clippy), git hooks, VS Code + rust-analyzer

---

### Task 1: Add .gitignore

**Files:**
- Create: `.gitignore`

**Step 1: Create .gitignore**

```
# Build artifacts
/target/

# Editor
*.swp
*.swo
*~
.idea/

# OS
.DS_Store
Thumbs.db

# Environment
.env
```

**Step 2: Verify it works**

Run: `git status`
Expected: `target/` no longer shows as untracked

**Step 3: Commit**

```bash
git add .gitignore
git commit -m "chore: add .gitignore"
```

---

### Task 2: Upgrade to Rust 2024 edition

**Files:**
- Modify: `Cargo.toml:4` (edition line)
- Modify: `.rustfmt.toml:1` (if already created, update edition)

**Step 1: Change edition in Cargo.toml**

Change `edition = "2021"` to `edition = "2024"`.

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Clean compilation, no errors

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: upgrade to Rust 2024 edition"
```

---

### Task 3: Add rustfmt configuration

**Files:**
- Create: `.rustfmt.toml`

**Step 1: Create .rustfmt.toml**

```toml
edition = "2024"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
```

**Step 2: Run cargo fmt to apply formatting**

Run: `cargo fmt`
Then: `git diff --stat` to see what changed

**Step 3: Verify**

Run: `cargo fmt --check`
Expected: No output (everything formatted)

**Step 4: Commit**

```bash
git add .rustfmt.toml
git add -u  # stage any reformatted files
git commit -m "chore: add rustfmt config and format codebase"
```

---

### Task 4: Add strict clippy lint configuration to Cargo.toml

**Files:**
- Modify: `Cargo.toml` (append `[lints]` section after `[dev-dependencies]`)

**Step 1: Add the `[lints]` section to Cargo.toml**

Append after the `[dev-dependencies]` section:

```toml
[lints.rust]
unsafe_code = "deny"

[lints.clippy]
# Lint groups
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }

# Specific strict lints
unwrap_used = "deny"
expect_used = "deny"

# Allow noisy pedantic lints that don't add value
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
missing_docs_in_private_items = "allow"
```

Note: The `priority = -1` on group lints ensures individual lint overrides take precedence over the group setting. Without this, `unwrap_used = "deny"` could be overridden by the group level.

**Step 2: Run clippy to see all violations**

Run: `cargo clippy 2>&1 | head -100`
Expected: Many warnings and errors — this is expected. We will fix them in Task 5.

**Step 3: Commit the lint config (even though code doesn't pass yet)**

```bash
git add Cargo.toml
git commit -m "chore: add strict clippy lint configuration"
```

---

### Task 5: Fix lint violations in production code

This is the largest task. Fix all clippy violations in non-test code. Test code (`#[cfg(test)]` modules) gets a blanket `#[allow]` for `unwrap_used` and `expect_used` since unwrap/expect in tests is acceptable.

**Files:**
- Modify: `src/lib.rs` — no changes needed
- Modify: `src/main.rs:143` — replace `.expect()` with proper error handling
- Modify: `src/runtime/sandbox.rs:74,127` — replace `.unwrap()` on Mutex locks
- Modify: `src/codegen/parser.rs:200` — replace `.unwrap()` on `to_lowercase().next()`
- Modify: `src/codegen/parser.rs` — fix redundant closures, wildcard matches, identical arms, etc.
- Modify: `src/codegen/annotations.rs` — fix `map_or_else`, `write!` vs `push_str`, etc.
- Modify: All test modules — add `#[allow(clippy::unwrap_used, clippy::expect_used)]` at module level

**Step 1: Fix production `.unwrap()` and `.expect()` calls**

In `src/main.rs:140-144`, replace:
```rust
.expect("failed to listen for ctrl+c");
```
with:
```rust
.await
.ok();
```
(The signal handler is in a shutdown path — if it fails we're already shutting down, so `.ok()` is appropriate.)

In `src/runtime/sandbox.rs:74`, replace:
```rust
logs_clone.lock().unwrap().push(line);
```
with:
```rust
if let Ok(mut logs) = logs_clone.lock() {
    logs.push(line);
}
```

In `src/runtime/sandbox.rs:127`, replace:
```rust
let mut logs = self.logs.lock().unwrap();
```
with:
```rust
let Ok(mut logs) = self.logs.lock() else {
    return Vec::new();
};
```

In `src/codegen/parser.rs:200`, replace:
```rust
result.push(c.to_lowercase().next().unwrap());
```
with:
```rust
if let Some(lc) = c.to_lowercase().next() {
    result.push(lc);
}
```

**Step 2: Fix other clippy pedantic/nursery warnings**

Run `cargo clippy 2>&1` and fix each warning. Common fixes:
- Replace redundant closures with function references
- Replace `map().unwrap_or()` with `map_or()`
- Replace wildcard match arms with explicit variants
- Combine identical match arms
- Use `write!` instead of `format!()` + `push_str()`
- Add backticks around technical terms in doc comments (e.g., `OpenAPI`)

**Step 3: Add `#[allow]` to test modules**

In every file that has `#[cfg(test)] mod tests { ... }`, add inside the module:
```rust
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    // ... existing code
}
```

**Step 4: Verify all lints pass**

Run: `cargo clippy 2>&1`
Expected: No warnings, no errors

**Step 5: Verify tests still pass**

Run: `cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add -u
git commit -m "fix: resolve all clippy lint violations for strict config"
```

---

### Task 6: Create VS Code workspace settings

**Files:**
- Create: `.vscode/settings.json`
- Create: `.vscode/extensions.json`
- Create: `.vscode/launch.json`

**Step 1: Create `.vscode/settings.json`**

```json
{
  "editor.formatOnSave": true,
  "editor.rulers": [100],
  "editor.trimAutoWhitespace": true,
  "files.trimTrailingWhitespace": true,
  "files.insertFinalNewline": true,
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--", "-D", "warnings"],
  "rust-analyzer.rustfmt.extraArgs": [],
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  },
  "[toml]": {
    "editor.defaultFormatter": "tamasfe.even-better-toml"
  }
}
```

**Step 2: Create `.vscode/extensions.json`**

```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml",
    "serayuzgur.crates",
    "vadimcn.vscode-lldb",
    "fill-labs.dependi"
  ]
}
```

**Step 3: Create `.vscode/launch.json`**

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug binary",
      "cargo": {
        "args": ["build", "--bin=toolscript", "--package=toolscript"],
        "filter": {
          "name": "toolscript",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug unit tests",
      "cargo": {
        "args": ["test", "--no-run", "--lib", "--package=toolscript"],
        "filter": {
          "kind": "lib"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug integration tests",
      "cargo": {
        "args": ["test", "--no-run", "--test=*", "--package=toolscript"],
        "filter": {
          "kind": "test"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

**Step 4: Verify JSON is valid**

Run: `python3 -c "import json; json.load(open('.vscode/settings.json')); json.load(open('.vscode/extensions.json')); json.load(open('.vscode/launch.json')); print('All valid')"`
Expected: `All valid`

**Step 5: Commit**

```bash
git add .vscode/
git commit -m "chore: add VS Code workspace configuration"
```

---

### Task 7: Create pre-commit hook

**Files:**
- Create: `.githooks/pre-commit` (must be executable)

**Step 1: Create `.githooks/pre-commit`**

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "==> Running cargo fmt..."
cargo fmt
git add -u

echo "==> Running cargo clippy..."
cargo clippy -- -D warnings

echo "==> All checks passed."
```

**Step 2: Make it executable**

Run: `chmod +x .githooks/pre-commit`

**Step 3: Activate the hook**

Run: `git config core.hooksPath .githooks`

**Step 4: Test the hook with a trial commit**

Make a trivial whitespace change, commit, and verify the hook runs:

Run: `git commit --allow-empty -m "test: verify pre-commit hook"`
Expected: See "Running cargo fmt...", "Running cargo clippy...", "All checks passed."

Then: `git reset HEAD~1` to undo the test commit.

**Step 5: Commit the hook**

```bash
git add .githooks/
git commit -m "chore: add pre-commit hook with auto-format and clippy"
```

---

### Task 8: Final verification

**Step 1: Run the full pre-commit sequence manually**

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

Expected: All three pass cleanly.

**Step 2: Verify git log looks clean**

Run: `git log --oneline -10`
Expected: Clean series of commits for each task.

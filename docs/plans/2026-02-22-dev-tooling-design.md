# Dev Tooling Setup Design

**Date:** 2026-02-22
**Status:** Approved

## Goal

Make the repo developer-ready with pre-commit hooks, strict linting, auto-formatting, and VS Code workspace configuration.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Hook mechanism | Shell script in `.githooks/` | Zero external deps, just git + Rust toolchain |
| Hook activation | `git config core.hooksPath .githooks` | One-time developer setup |
| Hook steps | `cargo fmt` (auto-fix) → `cargo clippy -D warnings` | Fast feedback, no test run in hook |
| Lint config location | `Cargo.toml [lints]` section | Modern idiomatic approach (Rust 1.74+) |
| Lint strictness | Deny `all`, warn `pedantic`/`nursery`, deny `unwrap_used`/`expect_used` | Enterprise-strict but practical |
| Formatter config | `.rustfmt.toml` with 100-char lines | Rust community standard |
| VS Code setup | `.vscode/` with settings, extensions, launch configs | Full developer onboarding |

## Components

### 1. Pre-commit hook (`.githooks/pre-commit`)

Shell script that:
1. Runs `cargo fmt` to auto-format, then `git add -u` to stage changes
2. Runs `cargo clippy -- -D warnings` to fail on any lint warning

Exits on first failure with a descriptive message.

### 2. Clippy configuration (`Cargo.toml [lints.clippy]`)

- `all = "deny"` — common correctness lints
- `pedantic = "warn"` — stricter style lints (warn to allow targeted `#[allow]`)
- `nursery = "warn"` — emerging lints (warn due to potential false positives)
- `unwrap_used = "deny"` — enforce proper error handling
- `expect_used = "deny"` — same rationale
- Allow list for noisy pedantic lints: `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`

### 3. Rustfmt configuration (`.rustfmt.toml`)

```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
```

### 4. VS Code workspace (`.vscode/`)

**`settings.json`:**
- rust-analyzer with clippy check command
- Format on save
- 100-char ruler
- Trim trailing whitespace

**`extensions.json`:**
- rust-analyzer, Even Better TOML, crates, CodeLLDB, Dependi

**`launch.json`:**
- Binary debug launch config
- Test debug launch config

### 5. `.gitignore`

Standard Rust gitignore covering `target/`, editor files, OS artifacts.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "code-mcp", about = "Generate MCP servers from OpenAPI specs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Generate manifest and SDK annotations from `OpenAPI` specs
    Generate {
        /// `OpenAPI` spec sources (file paths or URLs)
        #[arg(required = true)]
        specs: Vec<String>,
        /// Output directory
        #[arg(short, long, default_value = "./output")]
        output: PathBuf,
    },
    /// Start MCP server from a generated directory
    Serve {
        /// Path to generated output directory
        #[arg(required = true)]
        dir: PathBuf,
        /// Transport type
        #[arg(long, default_value = "stdio")]
        transport: String,
        /// Port for SSE transport
        #[arg(long, default_value = "8080")]
        port: u16,
        /// OAuth authority URL for JWT validation (enables auth)
        #[arg(long, env = "MCP_AUTH_AUTHORITY")]
        auth_authority: Option<String>,
        /// Expected JWT audience (required if auth-authority is set)
        #[arg(long, env = "MCP_AUTH_AUDIENCE")]
        auth_audience: Option<String>,
        /// Explicit JWKS URI (optional, derived from authority via OIDC discovery if not set)
        #[arg(long, env = "MCP_AUTH_JWKS_URI")]
        auth_jwks_uri: Option<String>,
        /// Script execution timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,
        /// Luau VM memory limit in megabytes
        #[arg(long, default_value = "64")]
        memory_limit: usize,
        /// Maximum API calls per script execution
        #[arg(long, default_value = "100")]
        max_api_calls: usize,
    },
    /// Generate and serve in one step
    Run {
        /// `OpenAPI` spec sources (file paths or URLs)
        #[arg(required = true)]
        specs: Vec<String>,
        /// Transport type
        #[arg(long, default_value = "stdio")]
        transport: String,
        /// Port for SSE transport
        #[arg(long, default_value = "8080")]
        port: u16,
        /// OAuth authority URL for JWT validation (enables auth)
        #[arg(long, env = "MCP_AUTH_AUTHORITY")]
        auth_authority: Option<String>,
        /// Expected JWT audience (required if auth-authority is set)
        #[arg(long, env = "MCP_AUTH_AUDIENCE")]
        auth_audience: Option<String>,
        /// Explicit JWKS URI (optional, derived from authority via OIDC discovery if not set)
        #[arg(long, env = "MCP_AUTH_JWKS_URI")]
        auth_jwks_uri: Option<String>,
        /// Script execution timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,
        /// Luau VM memory limit in megabytes
        #[arg(long, default_value = "64")]
        memory_limit: usize,
        /// Maximum API calls per script execution
        #[arg(long, default_value = "100")]
        max_api_calls: usize,
    },
}

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
            "code-mcp",
            "run",
            "spec.yaml",
            "--timeout",
            "60",
            "--memory-limit",
            "128",
            "--max-api-calls",
            "50",
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

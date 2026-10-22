//! `supermath-mcp`: serve the toolkit over MCP, or check its backends. See
//! [`supermath_mcp::cli`] for the command line.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    supermath_mcp::cli::run(std::env::args().skip(1).collect()).await
}

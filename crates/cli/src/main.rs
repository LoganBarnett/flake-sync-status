//! flake-sync-status - Report whether NixOS/nix-darwin hosts match a flake.
//!
//! # LLM Development Guidelines
//! When modifying this code:
//! - Keep flake querying logic in flake.rs
//! - Keep host querying logic in host.rs
//! - Keep color/terminal logic in color.rs
//! - Keep output rendering in output.rs
//! - Use semantic error types with thiserror - NO anyhow wrapping
//! - Add context at each error site explaining WHAT failed and WHY

mod color;
mod flake;
mod host;
mod output;

use clap::Parser;
use flake_sync_status_lib::{init_logging, LogFormat, LogLevel};
use thiserror::Error;

#[derive(Debug, Error)]
enum AppError {
  #[error("Flake query failed: {0}")]
  FlakeQuery(#[from] flake::FlakeError),

  #[error("Output failed: {0}")]
  Output(#[from] output::OutputError),
}

#[derive(Debug, Parser)]
#[command(
  name = "flake-sync-status",
  about = "Report whether each NixOS/nix-darwin host's active generation \
           matches what the flake would deploy"
)]
struct Cli {
  /// Path to the flake to inspect (default: current directory)
  #[arg(default_value = ".")]
  flake: String,

  /// Emit JSON instead of the human-readable table
  #[arg(long, short)]
  json: bool,

  /// Log level (trace, debug, info, warn, error)
  #[arg(long, default_value = "warn", env = "LOG_LEVEL")]
  log_level: String,
}

fn main() -> Result<(), AppError> {
  let cli = Cli::parse();

  let log_level = cli
    .log_level
    .parse::<LogLevel>()
    .unwrap_or(LogLevel::Warn);
  init_logging(log_level, LogFormat::Text);

  let results = flake::query_all_hosts(&cli.flake)?;

  if cli.json {
    output::print_json(&results)?;
  } else {
    output::print_human(&results);
  }

  Ok(())
}

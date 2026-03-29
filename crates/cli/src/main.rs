//! flake-sync-status - Report whether NixOS/nix-darwin hosts match a flake.
//!
//! # LLM Development Guidelines
//! When modifying this code:
//! - Keep CLI argument parsing and config validation in config.rs
//! - Keep flake querying logic in flake.rs
//! - Keep host querying logic in host.rs
//! - Keep color/terminal logic in color.rs
//! - Keep output rendering in output.rs
//! - Use semantic error types with thiserror - NO anyhow wrapping
//! - Add context at each error site explaining WHAT failed and WHY

mod color;
mod config;
mod flake;
mod host;
mod logging;
mod output;

use clap::Parser;
use flake_sync_status_lib::LogFormat;
use logging::init_logging;
use thiserror::Error;

#[derive(Debug, Error)]
enum AppError {
  #[error("Flake query failed: {0}")]
  FlakeQuery(#[from] flake::FlakeError),

  #[error("Output failed: {0}")]
  Output(#[from] output::OutputError),
}

fn main() {
  match run() {
    Ok(code) => std::process::exit(code),
    Err(e) => {
      eprintln!("Error: {e}");
      std::process::exit(3);
    }
  }
}

fn run() -> Result<i32, AppError> {
  let cfg = config::Config::from_cli(config::CliRaw::parse());

  if cfg.no_color {
    color::disable_color();
  }

  init_logging(cfg.log_level, LogFormat::Text);

  let opts = flake::QueryOptions {
    jobs: cfg.jobs,
    ssh_timeout: cfg.ssh_timeout,
  };

  let results = flake::query_all_hosts(&cfg.flake, host::system_runner, &opts)?;

  if cfg.json {
    output::print_json(&results, &mut std::io::stdout())?;
  } else {
    output::print_human(&results, &mut std::io::stdout(), cfg.verbose);
  }

  Ok(exit_code(&results))
}

/// Compute the process exit code from query results.
///
/// 0 — all online hosts are in sync (or there are no hosts).
/// 1 — one or more online hosts are out of sync.
/// 2 — one or more hosts have errors (unreachable, eval failure).
fn exit_code(hosts: &[flake::HostStatus]) -> i32 {
  if hosts.iter().any(|h| !h.offline && !h.errors.is_empty()) {
    2
  } else if hosts.iter().any(|h| !h.offline && h.in_sync == Some(false)) {
    1
  } else {
    0
  }
}

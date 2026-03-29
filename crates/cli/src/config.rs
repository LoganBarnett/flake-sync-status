use clap::Parser;
use flake_sync_status_lib::LogLevel;

/// Raw CLI arguments as parsed by Clap.  This is the first stage of the
/// staged configuration pipeline: CliRaw → Config.
#[derive(Debug, Parser)]
#[command(
  name = "flake-sync-status",
  version,
  about = "Report whether each NixOS/nix-darwin host's active generation \
           matches what the flake would deploy"
)]
pub struct CliRaw {
  /// Path to the flake to inspect (default: current directory)
  #[arg(default_value = ".")]
  pub flake: String,

  /// Emit JSON instead of the human-readable table
  #[arg(long, short)]
  pub json: bool,

  /// Log level (trace, debug, info, warn, error)
  #[arg(long, default_value = "warn", env = "LOG_LEVEL")]
  pub log_level: String,

  /// Suppress all ANSI color codes in output (also honored via NO_COLOR env var)
  #[arg(long)]
  pub no_color: bool,

  /// Show full /nix/store/… paths instead of the abbreviated 12-char hash
  #[arg(long)]
  pub verbose: bool,

  /// Maximum number of hosts to query concurrently (default: unlimited)
  #[arg(long)]
  pub jobs: Option<std::num::NonZeroUsize>,

  /// SSH connection timeout in seconds
  #[arg(long, default_value = "10")]
  pub ssh_timeout: u32,
}

/// Fully-typed, validated application configuration derived from [`CliRaw`].
pub struct Config {
  pub flake: String,
  pub json: bool,
  pub log_level: LogLevel,
  pub no_color: bool,
  pub verbose: bool,
  pub jobs: Option<usize>,
  pub ssh_timeout: u32,
}

impl Config {
  pub fn from_cli(raw: CliRaw) -> Self {
    Config {
      flake: raw.flake,
      json: raw.json,
      log_level: raw.log_level.parse::<LogLevel>().unwrap_or(LogLevel::Warn),
      no_color: raw.no_color,
      verbose: raw.verbose,
      jobs: raw.jobs.map(|n| n.get()),
      ssh_timeout: raw.ssh_timeout,
    }
  }
}

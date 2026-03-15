use crate::color;
use crate::flake::HostStatus;
use serde_json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OutputError {
  #[error("JSON serialization failed: {0}")]
  Json(#[from] serde_json::Error),
}

pub fn print_json(hosts: &[HostStatus]) -> Result<(), OutputError> {
  println!("{}", serde_json::to_string_pretty(hosts)?);
  Ok(())
}

pub fn print_human(hosts: &[HostStatus]) {
  let hostname_w = hosts
    .iter()
    .map(|h| h.hostname.len())
    .max()
    .unwrap_or(8)
    .max(8);
  let system_w = hosts
    .iter()
    .map(|h| h.system.len())
    .max()
    .unwrap_or(12)
    .max(6);

  println!(
    "{:<hw$}  {:<sw$}  {:<12}  {:<12}  {}",
    "HOSTNAME",
    "SYSTEM",
    "EXPECTED",
    "CURRENT",
    "SYNC",
    hw = hostname_w,
    sw = system_w,
  );
  println!("{}", "-".repeat(hostname_w + system_w + 46));

  for host in hosts {
    // Color the text first, then pad with plain spaces so ANSI escape codes
    // don't corrupt column alignment.
    let hostname_col =
      pad(&color::rainbow(&host.hostname), host.hostname.len(), hostname_w);

    let system_col =
      pad(&color::rainbow_platform(&host.system), host.system.len(), system_w);

    let expected_col = match &host.flake_path {
      Some(path) => {
        let hash = color::store_hash_abbrev(path);
        pad(&color::rainbow(hash), hash.len(), 12)
      }
      None => pad(&color::error_bg("eval error"), 10, 12),
    };

    let current_col = match &host.current_path {
      Some(path) => {
        let hash = color::store_hash_abbrev(path);
        pad(&color::rainbow(hash), hash.len(), 12)
      }
      None => pad(&color::error_bg("unreachable"), 11, 12),
    };

    let sync = if host.in_sync {
      color::green("✓")
    } else if host.flake_path.is_none() || host.current_path.is_none() {
      color::yellow("?")
    } else {
      color::red("✗")
    };

    println!(
      "{}  {}  {}  {}  {}",
      hostname_col, system_col, expected_col, current_col, sync,
    );
  }

  let errored: Vec<&HostStatus> =
    hosts.iter().filter(|h| !h.errors.is_empty()).collect();
  if !errored.is_empty() {
    println!();
    println!("Errors:");
    for host in errored {
      let prefix =
        pad(&color::rainbow(&host.hostname), host.hostname.len(), hostname_w);
      for error in &host.errors {
        println!("  {}  {}", prefix, error);
      }
    }
  }
}

/// Pad a pre-colored string to `width` visible characters by appending plain
/// spaces.  `visible_len` must be the character count without escape codes.
fn pad(colored: &str, visible_len: usize, width: usize) -> String {
  format!("{}{}", colored, " ".repeat(width.saturating_sub(visible_len)))
}

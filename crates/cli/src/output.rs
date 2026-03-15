use crate::color::{rainbow_platform, store_hash_abbrev, Colored};
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
    "{:<hw$}  {:<sw$}  {:<12}  {:<12}  SYNC",
    "HOSTNAME",
    "SYSTEM",
    "EXPECTED",
    "CURRENT",
    hw = hostname_w,
    sw = system_w,
  );
  println!("{}", "-".repeat(hostname_w + system_w + 44));

  for host in hosts {
    // Color the text first, then pad with plain spaces so ANSI escape codes
    // don't corrupt column alignment.
    let hostname_col = colored_padded(
      Colored::rainbow(&host.hostname).to_string(),
      host.hostname.len(),
      hostname_w,
    );
    let system_col = format!(
      "{}{}",
      rainbow_platform(&host.system),
      spaces(system_w.saturating_sub(host.system.len())),
    );

    let expected_col = match &host.flake_path {
      Some(path) => {
        let hash = store_hash_abbrev(path);
        colored_padded(Colored::rainbow(hash).to_string(), hash.len(), 12)
      }
      None => colored_padded(Colored::yellow("eval error").to_string(), 10, 12),
    };

    let current_col = match &host.current_path {
      Some(path) => {
        let hash = store_hash_abbrev(path);
        colored_padded(Colored::rainbow(hash).to_string(), hash.len(), 12)
      }
      None => {
        let msg: String = "unreachable".chars().take(12).collect();
        colored_padded(Colored::yellow(&msg as &str).to_string(), msg.len(), 12)
      }
    };

    let sync = if host.in_sync {
      Colored::green("✓").to_string()
    } else if host.flake_path.is_none() || host.current_path.is_none() {
      Colored::yellow("?").to_string()
    } else {
      Colored::red("✗").to_string()
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
      for error in &host.errors {
        println!(
          "  {}  {}",
          colored_padded(
            Colored::rainbow(&host.hostname).to_string(),
            host.hostname.len(),
            hostname_w,
          ),
          error,
        );
      }
    }
  }
}

/// Return `colored_text` (with ANSI codes) right-padded to `width` visible
/// characters by appending plain spaces.  `visible_len` is the display width
/// of `colored_text` without escape codes.
fn colored_padded(
  colored_text: String,
  visible_len: usize,
  width: usize,
) -> String {
  format!("{}{}", colored_text, spaces(width.saturating_sub(visible_len)))
}

fn spaces(n: usize) -> String {
  " ".repeat(n)
}

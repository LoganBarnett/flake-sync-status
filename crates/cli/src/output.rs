use crate::color;
use crate::flake::HostStatus;
use serde_json;
use tabled::settings::style::HorizontalLine;
use tabled::settings::{Padding, Style};
use tabled::{Table, Tabled};
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

#[derive(Tabled)]
struct HostRow {
  #[tabled(rename = "HOSTNAME")]
  hostname: String,
  #[tabled(rename = "SYSTEM")]
  system: String,
  #[tabled(rename = "EXPECTED")]
  expected: String,
  #[tabled(rename = "CURRENT")]
  current: String,
  #[tabled(rename = "SYNC")]
  sync: String,
}

pub fn print_human(hosts: &[HostStatus]) {
  let rows: Vec<HostRow> = hosts
    .iter()
    .map(|h| {
      let expected = match &h.flake_path {
        Some(path) => color::rainbow(color::store_hash_abbrev(path)),
        None => color::error_bg("eval error"),
      };
      let current = match &h.current_path {
        Some(path) => color::rainbow(color::store_hash_abbrev(path)),
        None => color::error_bg("unreachable"),
      };
      let sync = if h.in_sync {
        color::green("✓")
      } else if h.flake_path.is_none() || h.current_path.is_none() {
        color::yellow("?")
      } else {
        color::red("✗")
      };
      HostRow {
        hostname: color::rainbow(&h.hostname),
        system: color::rainbow_platform(&h.system),
        expected,
        current,
        sync,
      }
    })
    .collect();

  // Padding: 0 left so the table starts at column 0, 2 right so adjacent
  // columns are separated by two spaces.  The HorizontalLine after row 1
  // (the header) replaces the hand-rolled separator with dashes sized to
  // the actual column widths.
  let style = Style::blank()
    .horizontals([(1, HorizontalLine::new('-').intersection('-'))]);
  let table = Table::new(rows)
    .with(style)
    .with(Padding::new(0, 2, 0, 0))
    .to_string();
  println!("{}", table);

  let errored: Vec<&HostStatus> =
    hosts.iter().filter(|h| !h.errors.is_empty()).collect();
  if !errored.is_empty() {
    let hostname_w = hosts
      .iter()
      .map(|h| h.hostname.len())
      .max()
      .unwrap_or(8)
      .max(8);
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

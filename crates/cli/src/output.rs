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
  #[error("Write failed: {0}")]
  Write(#[from] std::io::Error),
}

pub fn print_json<W: std::io::Write>(
  hosts: &[HostStatus],
  out: &mut W,
) -> Result<(), OutputError> {
  writeln!(out, "{}", serde_json::to_string_pretty(hosts)?)?;
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

pub fn print_human<W: std::io::Write>(hosts: &[HostStatus], out: &mut W) {
  let rows: Vec<HostRow> = hosts
    .iter()
    .map(|h| {
      let expected = match &h.flake_path {
        Some(path) => color::rainbow(color::store_hash_abbrev(path)),
        None => color::error_bg("eval error"),
      };
      let current = if h.offline {
        color::yellow("offline")
      } else {
        match &h.current_path {
          Some(path) => color::rainbow(color::store_hash_abbrev(path)),
          None => color::error_bg("unreachable"),
        }
      };
      let sync = if h.offline {
        "\u{2014}".to_string() // —
      } else if h.in_sync == Some(true) {
        color::green("\u{2713}") // ✓
      } else if h.flake_path.is_none() || h.current_path.is_none() {
        color::yellow("?")
      } else {
        color::red("\u{2717}") // ✗
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

  // Padding: 0 left so the table starts at column 0, 2 right so
  // adjacent columns are separated by two spaces.  The HorizontalLine
  // after row 1 (the header) replaces the hand-rolled separator with
  // dashes sized to the actual column widths.
  let style = Style::blank()
    .horizontals([(1, HorizontalLine::new('-').intersection('-'))]);
  let table = Table::new(rows)
    .with(style)
    .with(Padding::new(0, 2, 0, 0))
    .to_string();
  writeln!(out, "{}", table).ok();

  let errored: Vec<&HostStatus> =
    hosts.iter().filter(|h| !h.errors.is_empty()).collect();
  if !errored.is_empty() {
    let hostname_w = hosts
      .iter()
      .map(|h| h.hostname.len())
      .max()
      .unwrap_or(8)
      .max(8);
    writeln!(out).ok();
    writeln!(out, "Errors:").ok();
    for host in errored {
      let prefix =
        pad(&color::rainbow(&host.hostname), host.hostname.len(), hostname_w);
      for error in &host.errors {
        writeln!(out, "  {}  {}", prefix, error).ok();
      }
    }
  }

  // Summary line — lets users scan the bottom without reading every row.
  let in_sync_n = hosts
    .iter()
    .filter(|h| !h.offline && h.errors.is_empty() && h.in_sync == Some(true))
    .count();
  let out_of_sync_n = hosts
    .iter()
    .filter(|h| !h.offline && h.errors.is_empty() && h.in_sync == Some(false))
    .count();
  let offline_n = hosts.iter().filter(|h| h.offline).count();
  let error_n = hosts
    .iter()
    .filter(|h| !h.offline && !h.errors.is_empty())
    .count();
  writeln!(
    out,
    "\n{} in sync  \u{b7}  {} out of sync  \u{b7}  {} offline  \u{b7}  {} errors",
    in_sync_n, out_of_sync_n, offline_n, error_n
  )
  .ok();
}

/// Pad a pre-colored string to `width` visible characters by appending
/// plain spaces.  `visible_len` must be the character count without
/// escape codes.
pub fn pad(colored: &str, visible_len: usize, width: usize) -> String {
  format!("{}{}", colored, " ".repeat(width.saturating_sub(visible_len)))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::flake::HostStatus;

  fn make_host(
    hostname: &str,
    in_sync: Option<bool>,
    offline: bool,
  ) -> HostStatus {
    let current_path = if offline || in_sync.is_none() {
      None
    } else if in_sync == Some(true) {
      Some("/nix/store/aaa111-nixos-system".to_string())
    } else {
      Some("/nix/store/bbb222-nixos-system".to_string())
    };
    HostStatus {
      hostname: hostname.to_string(),
      system: "x86_64-linux".to_string(),
      offline,
      flake_path: Some("/nix/store/aaa111-nixos-system".to_string()),
      current_path,
      errors: vec![],
      in_sync,
    }
  }

  #[test]
  fn pad_adds_spaces_to_reach_width() {
    assert_eq!(pad("abc", 3, 6), "abc   ");
  }

  #[test]
  fn pad_does_not_truncate_when_already_wide() {
    assert_eq!(pad("abcdefgh", 8, 4), "abcdefgh");
  }

  #[test]
  fn print_json_round_trips_hostname() {
    let hosts = vec![make_host("silicon", Some(true), false)];
    let mut buf = Vec::new();
    print_json(&hosts, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("\"silicon\""));
    assert!(output.contains("in_sync"));
  }

  #[test]
  fn print_human_contains_hostname() {
    let hosts = vec![make_host("silicon", Some(true), false)];
    let mut buf = Vec::new();
    print_human(&hosts, &mut buf);
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("silicon"));
  }

  #[test]
  fn print_human_shows_offline_indicator() {
    let hosts = vec![make_host("argon", None, true)];
    let mut buf = Vec::new();
    print_human(&hosts, &mut buf);
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("offline"));
    assert!(output.contains('\u{2014}')); // —
  }

  #[test]
  fn print_human_summary_counts_are_correct() {
    let hosts = vec![
      make_host("silicon", Some(true), false),
      make_host("argon", Some(false), false),
      make_host("neon", None, true),
    ];
    let mut buf = Vec::new();
    print_human(&hosts, &mut buf);
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("1 in sync"));
    assert!(output.contains("1 out of sync"));
    assert!(output.contains("1 offline"));
  }
}

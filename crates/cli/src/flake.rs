use crate::host;
use serde::Serialize;
use std::process::Command;
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Error)]
pub enum FlakeError {
  #[error(
    "Failed to spawn nix for attribute {attr} at flake {flake}: {source}"
  )]
  Spawn {
    flake: String,
    attr: String,
    #[source]
    source: std::io::Error,
  },

  #[error("Failed to parse host list from flake {flake}: {source}")]
  ParseHostList {
    flake: String,
    #[source]
    source: serde_json::Error,
  },
}

#[derive(Debug, Serialize)]
pub struct HostStatus {
  pub hostname: String,
  /// Platform double such as `x86_64-linux` or `aarch64-darwin`.
  pub system: String,
  /// Store path the flake expects for this host, or null when flake
  /// evaluation failed for this host.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub flake_path: Option<String>,
  /// Store path of the currently-active generation on the host, or null when
  /// the host was unreachable.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub current_path: Option<String>,
  /// Error messages from flake evaluation or SSH connection failures.
  #[serde(skip_serializing_if = "Vec::is_empty")]
  pub errors: Vec<String>,
  pub in_sync: bool,
}

/// Query all nixosConfigurations and darwinConfigurations in the flake and
/// return a status entry for each host.  Per-host eval or SSH errors are
/// captured in HostStatus.errors rather than aborting the whole run.
///
/// All hosts are queried in parallel: each makes independent nix eval and SSH
/// calls with no shared mutable state, so serializing them buys nothing.
pub fn query_all_hosts(flake: &str) -> Result<Vec<HostStatus>, FlakeError> {
  // Collect (hostname, config_type) pairs first — list_hosts is fast and must
  // propagate FlakeError before we start spawning threads.
  let mut pairs: Vec<(String, String)> = Vec::new();
  for config_type in &["nixosConfigurations", "darwinConfigurations"] {
    for hostname in list_hosts(flake, config_type)? {
      pairs.push((hostname, config_type.to_string()));
    }
  }

  let flake = flake.to_string();
  let handles: Vec<_> = pairs
    .into_iter()
    .map(|(hostname, config_type)| {
      let flake = flake.clone();
      std::thread::spawn(move || query_host(&flake, &hostname, &config_type))
    })
    .collect();

  Ok(
    handles
      .into_iter()
      .map(|h| h.join().expect("host query thread panicked"))
      .collect(),
  )
}

fn list_hosts(
  flake: &str,
  config_type: &str,
) -> Result<Vec<String>, FlakeError> {
  let attr = format!("{}#{}", flake, config_type);
  let output = Command::new("nix")
    .args(["eval", "--json", &attr, "--apply", "builtins.attrNames"])
    .output()
    .map_err(|source| FlakeError::Spawn {
      flake: flake.to_string(),
      attr: attr.clone(),
      source,
    })?;

  // A missing attribute means no hosts of this type; treat as empty.
  if !output.status.success() {
    return Ok(vec![]);
  }

  let names: Vec<String> =
    serde_json::from_slice(&output.stdout).map_err(|source| {
      FlakeError::ParseHostList {
        flake: flake.to_string(),
        source,
      }
    })?;

  Ok(names)
}

fn query_host(flake: &str, hostname: &str, config_type: &str) -> HostStatus {
  let mut errors = Vec::new();

  let path_attr = format!(
    "{}#{}.{}.config.system.build.toplevel.outPath",
    flake, config_type, hostname
  );
  let system_attr = format!(
    "{}#{}.{}.pkgs.stdenv.hostPlatform.system",
    flake, config_type, hostname
  );

  let flake_path = match nix_eval_raw(flake, &path_attr) {
    Ok(p) => Some(p),
    Err(e) => {
      warn!(hostname, error = %e, "flake eval failed");
      errors.push(e);
      None
    }
  };

  let system =
    nix_eval_raw(flake, &system_attr).unwrap_or_else(|_| "unknown".to_string());

  // Use the FQDN from the flake config as the SSH target so that host key
  // verification matches known_hosts entries (which are keyed by FQDN).
  // Falls back to the attribute name if the option is absent (e.g. darwin).
  let fqdn_attr =
    format!("{}#{}.{}.config.networking.fqdn", flake, config_type, hostname);
  let ssh_host =
    nix_eval_raw(flake, &fqdn_attr).unwrap_or_else(|_| hostname.to_string());

  let current_path = match host::get_current_system(&ssh_host) {
    Ok(path) => Some(path),
    Err(e) => {
      warn!(hostname, error = %e, "host query failed");
      errors.push(e.to_string());
      None
    }
  };

  let in_sync = match (&flake_path, &current_path) {
    (Some(f), Some(c)) => f == c,
    _ => false,
  };

  HostStatus {
    hostname: hostname.to_string(),
    system,
    flake_path,
    current_path,
    errors,
    in_sync,
  }
}

fn nix_eval_raw(flake: &str, attr: &str) -> Result<String, String> {
  let output = Command::new("nix")
    .args(["eval", "--raw", attr])
    .output()
    .map_err(|e| {
      format!("Failed to spawn nix for attribute {attr} at flake {flake}: {e}")
    })?;

  if !output.status.success() {
    // Extract just the final error line from nix's verbose stderr rather than
    // dumping the full trace into the errors list.
    let stderr = String::from_utf8_lossy(&output.stderr);
    let summary = stderr
      .lines()
      .filter(|l| l.trim_start().starts_with("error:"))
      .last()
      .unwrap_or(stderr.trim())
      .trim()
      .to_string();
    return Err(format!("nix eval {attr}: {summary}"));
  }

  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

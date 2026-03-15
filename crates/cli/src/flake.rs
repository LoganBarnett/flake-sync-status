use crate::host;
use serde::Serialize;
use std::process::Command;
use thiserror::Error;

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

  #[error("nix eval of {attr} failed: {stderr}")]
  Eval { attr: String, stderr: String },

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
  /// Store path the flake expects for this host.
  pub flake_path: String,
  /// Store path of the currently-active generation on the host, or null when
  /// the host was unreachable.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub current_path: Option<String>,
  /// Error message when the host was unreachable.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
  pub in_sync: bool,
}

/// Query all nixosConfigurations and darwinConfigurations in the flake and
/// return a status entry for each host.
pub fn query_all_hosts(flake: &str) -> Result<Vec<HostStatus>, FlakeError> {
  let mut results = Vec::new();

  for config_type in &["nixosConfigurations", "darwinConfigurations"] {
    let hosts = list_hosts(flake, config_type)?;
    for hostname in hosts {
      let status = query_host(flake, &hostname, config_type)?;
      results.push(status);
    }
  }

  Ok(results)
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

  let names: Vec<String> = serde_json::from_slice(&output.stdout)
    .map_err(|source| FlakeError::ParseHostList {
      flake: flake.to_string(),
      source,
    })?;

  Ok(names)
}

fn query_host(
  flake: &str,
  hostname: &str,
  config_type: &str,
) -> Result<HostStatus, FlakeError> {
  let path_attr = format!(
    "{}#{}.{}.config.system.build.toplevel.outPath",
    flake, config_type, hostname
  );
  let system_attr = format!(
    "{}#{}.{}.pkgs.stdenv.hostPlatform.system",
    flake, config_type, hostname
  );

  let flake_path = nix_eval_raw(flake, &path_attr)?;
  let system =
    nix_eval_raw(flake, &system_attr).unwrap_or_else(|_| "unknown".to_string());

  let (current_path, error) = match host::get_current_system(hostname) {
    Ok(path) => (Some(path), None),
    Err(e) => (None, Some(e.to_string())),
  };

  let in_sync = current_path
    .as_ref()
    .map(|p| *p == flake_path)
    .unwrap_or(false);

  Ok(HostStatus {
    hostname: hostname.to_string(),
    system,
    flake_path,
    current_path,
    error,
    in_sync,
  })
}

fn nix_eval_raw(flake: &str, attr: &str) -> Result<String, FlakeError> {
  let output = Command::new("nix")
    .args(["eval", "--raw", attr])
    .output()
    .map_err(|source| FlakeError::Spawn {
      flake: flake.to_string(),
      attr: attr.to_string(),
      source,
    })?;

  if !output.status.success() {
    return Err(FlakeError::Eval {
      attr: attr.to_string(),
      stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    });
  }

  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}


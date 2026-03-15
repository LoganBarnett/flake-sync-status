use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HostError {
  #[error("SSH to {host} failed: {reason}")]
  Ssh { host: String, reason: String },

  #[error("Failed to read /run/current-system locally: {0}")]
  Local(String),
}

/// Collect all names this machine is known by: short hostname and any
/// dot-separated prefixes of the FQDN.
fn local_hostnames() -> Vec<String> {
  let mut names = Vec::new();
  if let Ok(output) = Command::new("hostname").output() {
    if output.status.success() {
      let fqdn = String::from_utf8_lossy(&output.stdout).trim().to_string();
      // Include the short hostname.
      if let Some(short) = fqdn.split('.').next() {
        if !short.is_empty() {
          names.push(short.to_string());
        }
      }
      // Include the full FQDN if it differs.
      if !fqdn.is_empty() && !names.contains(&fqdn) {
        names.push(fqdn);
      }
    }
  }
  names
}

/// Return true when the given hostname refers to the local machine.
fn is_localhost(hostname: &str) -> bool {
  let locals = local_hostnames();
  locals.iter().any(|local| {
    local == hostname
      || local.starts_with(&format!("{}.", hostname))
      || hostname.starts_with(&format!("{}.", local))
  })
}

/// Return the store path of the currently-active system generation on the
/// named host.  Connects via SSH for remote hosts; reads directly for local.
pub fn get_current_system(hostname: &str) -> Result<String, HostError> {
  if is_localhost(hostname) {
    get_current_system_local()
  } else {
    get_current_system_remote(hostname)
  }
}

fn get_current_system_local() -> Result<String, HostError> {
  let output = Command::new("readlink")
    .args(["-f", "/run/current-system"])
    .output()
    .map_err(|e| HostError::Local(e.to_string()))?;
  if !output.status.success() {
    return Err(HostError::Local(
      String::from_utf8_lossy(&output.stderr).trim().to_string(),
    ));
  }
  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_current_system_remote(hostname: &str) -> Result<String, HostError> {
  let output = Command::new("ssh")
    .args([
      "-o",
      "ConnectTimeout=5",
      // Fail immediately rather than prompting for passwords or host
      // key confirmations.
      "-o",
      "BatchMode=yes",
      hostname,
      "readlink -f /run/current-system",
    ])
    .output()
    .map_err(|e| HostError::Ssh {
      host: hostname.to_string(),
      reason: e.to_string(),
    })?;
  if !output.status.success() {
    return Err(HostError::Ssh {
      host: hostname.to_string(),
      reason: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    });
  }
  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

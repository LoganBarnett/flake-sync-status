use thiserror::Error;

/// Normalized result of spawning a subprocess.  Using this instead of
/// `std::process::Output` directly lets tests inject canned responses
/// without spawning real processes.
pub struct CommandOutput {
  pub stdout: Vec<u8>,
  pub stderr: Vec<u8>,
  pub success: bool,
}

/// Function-pointer type for spawning subprocesses.  Production code
/// uses `system_runner`; tests supply a deterministic mock function.
pub type Runner = fn(&str, &[&str]) -> std::io::Result<CommandOutput>;

/// Production `Runner` implementation that delegates to
/// `std::process::Command`.
pub fn system_runner(
  program: &str,
  args: &[&str],
) -> std::io::Result<CommandOutput> {
  std::process::Command::new(program)
    .args(args)
    .output()
    .map(|o| CommandOutput {
      stdout: o.stdout,
      stderr: o.stderr,
      success: o.status.success(),
    })
}

#[derive(Debug, Error)]
pub enum HostError {
  #[error("SSH to {host} failed: {reason}")]
  Ssh { host: String, reason: String },

  #[error("Failed to read /run/current-system locally: {0}")]
  Local(String),
}

/// Collect all names this machine is known by: short hostname and any
/// dot-separated prefixes of the FQDN.
fn local_hostnames(runner: Runner) -> Vec<String> {
  let mut names = Vec::new();
  if let Ok(output) = runner("hostname", &[]) {
    if output.success {
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
fn is_localhost(runner: Runner, hostname: &str) -> bool {
  let locals = local_hostnames(runner);
  locals.iter().any(|local| {
    local == hostname
      || local.starts_with(&format!("{}.", hostname))
      || hostname.starts_with(&format!("{}.", local))
  })
}

/// Return the store path of the currently-active system generation on
/// the named host.  Connects via SSH for remote hosts; reads directly
/// for local.  `ssh_timeout` controls the SSH `ConnectTimeout` in seconds.
pub fn get_current_system(
  runner: Runner,
  hostname: &str,
  ssh_timeout: u32,
) -> Result<String, HostError> {
  if is_localhost(runner, hostname) {
    get_current_system_local(runner)
  } else {
    get_current_system_remote(runner, hostname, ssh_timeout)
  }
}

fn get_current_system_local(runner: Runner) -> Result<String, HostError> {
  let output = runner("readlink", &["-f", "/run/current-system"])
    .map_err(|e| HostError::Local(e.to_string()))?;
  if !output.success {
    return Err(HostError::Local(
      String::from_utf8_lossy(&output.stderr).trim().to_string(),
    ));
  }
  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_current_system_remote(
  runner: Runner,
  hostname: &str,
  ssh_timeout: u32,
) -> Result<String, HostError> {
  let timeout_opt = format!("ConnectTimeout={ssh_timeout}");
  let output = runner(
    "ssh",
    &[
      "-o",
      &timeout_opt,
      // Fail immediately rather than prompting for passwords or host
      // key confirmations.
      "-o",
      "BatchMode=yes",
      hostname,
      "readlink -f /run/current-system",
    ],
  )
  .map_err(|e| HostError::Ssh {
    host: hostname.to_string(),
    reason: e.to_string(),
  })?;
  if !output.success {
    return Err(HostError::Ssh {
      host: hostname.to_string(),
      reason: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    });
  }
  Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn ok_output(stdout: &[u8]) -> std::io::Result<CommandOutput> {
    Ok(CommandOutput {
      stdout: stdout.to_vec(),
      stderr: vec![],
      success: true,
    })
  }

  fn err_output(stderr: &[u8]) -> std::io::Result<CommandOutput> {
    Ok(CommandOutput {
      stdout: vec![],
      stderr: stderr.to_vec(),
      success: false,
    })
  }

  fn hostname_runner(_p: &str, _a: &[&str]) -> std::io::Result<CommandOutput> {
    ok_output(b"silicon\n")
  }

  fn fqdn_runner(_p: &str, _a: &[&str]) -> std::io::Result<CommandOutput> {
    ok_output(b"silicon.proton\n")
  }

  #[test]
  fn is_localhost_matches_short_name() {
    assert!(is_localhost(hostname_runner, "silicon"));
  }

  #[test]
  fn is_localhost_rejects_other_host() {
    assert!(!is_localhost(hostname_runner, "argon"));
  }

  #[test]
  fn is_localhost_matches_both_short_and_fqdn() {
    assert!(is_localhost(fqdn_runner, "silicon"));
    assert!(is_localhost(fqdn_runner, "silicon.proton"));
  }

  #[test]
  fn get_current_system_local_returns_trimmed_path() {
    fn runner(_p: &str, _a: &[&str]) -> std::io::Result<CommandOutput> {
      ok_output(b"/nix/store/abc123-nixos-system\n")
    }
    assert_eq!(
      get_current_system_local(runner).unwrap(),
      "/nix/store/abc123-nixos-system"
    );
  }

  #[test]
  fn get_current_system_local_maps_failure_to_local_error() {
    fn runner(_p: &str, _a: &[&str]) -> std::io::Result<CommandOutput> {
      err_output(b"readlink: /run/current-system: No such file or directory")
    }
    assert!(matches!(
      get_current_system_local(runner),
      Err(HostError::Local(_))
    ));
  }

  #[test]
  fn get_current_system_remote_returns_trimmed_path() {
    fn runner(_p: &str, _a: &[&str]) -> std::io::Result<CommandOutput> {
      ok_output(b"/nix/store/xyz999-nixos-system\n")
    }
    assert_eq!(
      get_current_system_remote(runner, "argon", 10).unwrap(),
      "/nix/store/xyz999-nixos-system"
    );
  }

  #[test]
  fn get_current_system_remote_maps_failure_to_ssh_error() {
    fn runner(_p: &str, _a: &[&str]) -> std::io::Result<CommandOutput> {
      err_output(b"ssh: connect to host argon port 22: Connection refused")
    }
    let err = get_current_system_remote(runner, "argon", 10).unwrap_err();
    assert!(matches!(err, HostError::Ssh { host, .. } if host == "argon"));
  }
}

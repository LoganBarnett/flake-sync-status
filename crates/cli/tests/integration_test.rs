use std::{path::PathBuf, process::Command};

fn get_binary_path() -> PathBuf {
  let mut path =
    std::env::current_exe().expect("Failed to get current executable path");

  path.pop(); // remove test executable name
  path.pop(); // remove deps dir
  path.push("flake-sync-status");

  if !path.exists() {
    path.pop();
    path.pop();
    path.push("debug");
    path.push("flake-sync-status");
  }

  path
}

#[test]
fn test_help_flag() {
  let output = Command::new(get_binary_path())
    .arg("--help")
    .output()
    .expect("Failed to execute binary. Build first: cargo build -p flake-sync-status");

  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(
    stdout.contains("Usage:"),
    "Expected help text, got: {}",
    stdout
  );
}

#[test]
fn test_version_flag() {
  let output = Command::new(get_binary_path())
    .arg("--version")
    .output()
    .expect("Failed to execute binary. Build first: cargo build -p flake-sync-status");

  assert!(output.status.success());
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(
    stdout.contains("flake-sync-status"),
    "Expected version text, got: {}",
    stdout
  );
}

#[test]
fn test_json_flag_is_accepted() {
  // Running against a real flake would require nix in PATH and a network
  // connection.  Just verify the flag is accepted without errors unrelated
  // to the flake itself.
  let output = Command::new(get_binary_path())
    .args(["--json", "--help"])
    .output()
    .expect("Failed to execute binary. Build first: cargo build -p flake-sync-status");

  assert!(output.status.success());
}

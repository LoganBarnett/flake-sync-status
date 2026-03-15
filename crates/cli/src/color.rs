use std::fmt;

/// FNV-1a hash for consistent, deterministic color generation from strings.
fn fnv1a(s: &str) -> u64 {
  const FNV_PRIME: u64 = 1099511628211;
  const FNV_OFFSET: u64 = 14695981039346656037;
  s.bytes().fold(FNV_OFFSET, |hash, byte| {
    (hash ^ byte as u64).wrapping_mul(FNV_PRIME)
  })
}

/// Convert HSV (hue in [0, 360), saturation and value in [0, 1]) to RGB.
fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> (u8, u8, u8) {
  let h = hue / 60.0;
  let i = h.floor() as u32;
  let f = h - h.floor();
  let p = value * (1.0 - saturation);
  let q = value * (1.0 - saturation * f);
  let t = value * (1.0 - saturation * (1.0 - f));
  let (r, g, b) = match i % 6 {
    0 => (value, t, p),
    1 => (q, value, p),
    2 => (p, value, t),
    3 => (p, q, value),
    4 => (t, p, value),
    _ => (value, p, q),
  };
  ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// Derive a rainbow color from a string via FNV-1a hashing.  Consistent
/// across runs: the same input always yields the same color.
pub fn rainbow_color(s: &str) -> (u8, u8, u8) {
  let hue = (fnv1a(s) % 360) as f64;
  // S=0.85, V=1.0 - vivid but avoids near-white and muddy brown.
  hsv_to_rgb(hue, 0.85, 1.0)
}

pub struct Colored<T: fmt::Display> {
  text: T,
  r: u8,
  g: u8,
  b: u8,
}

impl<T: fmt::Display> Colored<T> {
  pub fn new(text: T, r: u8, g: u8, b: u8) -> Self {
    Self { text, r, g, b }
  }

  pub fn green(text: T) -> Self {
    Self::new(text, 60, 220, 100)
  }

  pub fn red(text: T) -> Self {
    Self::new(text, 220, 60, 60)
  }

  pub fn yellow(text: T) -> Self {
    Self::new(text, 220, 180, 60)
  }
}

impl Colored<&str> {
  pub fn rainbow(text: &str) -> Self {
    let (r, g, b) = rainbow_color(text);
    Self::new(text, r, g, b)
  }
}

impl<T: fmt::Display> fmt::Display for Colored<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "\x1b[38;2;{};{};{}m{}\x1b[0m",
      self.r, self.g, self.b, self.text
    )
  }
}

/// Colorize each segment of a platform double (e.g. `aarch64-linux`) with its
/// own rainbow-derived color.  The `-` separator is uncolored.
pub fn rainbow_platform(system: &str) -> String {
  system
    .split('-')
    .map(|seg| Colored::rainbow(seg).to_string())
    .collect::<Vec<_>>()
    .join("\x1b[0m-")
}

/// Extract the hash portion of a Nix store path and return up to 12 chars.
/// `/nix/store/abc12345xyz0-name` → `abc12345xyz0`
pub fn store_hash_abbrev(store_path: &str) -> &str {
  let after_store = store_path
    .strip_prefix("/nix/store/")
    .unwrap_or(store_path);
  let hash_end = after_store
    .find('-')
    .unwrap_or(after_store.len())
    .min(12);
  &after_store[..hash_end]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_same_string_same_color() {
    assert_eq!(rainbow_color("silicon"), rainbow_color("silicon"));
  }

  #[test]
  fn test_different_strings_likely_different_color() {
    // Not guaranteed but extremely likely with a good hash.
    assert_ne!(rainbow_color("silicon"), rainbow_color("argon"));
  }

  #[test]
  fn test_store_hash_abbrev_full_path() {
    let path = "/nix/store/abc12345xyz0abcdefgh-nixos-system-silicon";
    assert_eq!(store_hash_abbrev(path), "abc12345xyz0");
  }

  #[test]
  fn test_store_hash_abbrev_bare_hash() {
    assert_eq!(store_hash_abbrev("abc12345xyz0"), "abc12345xyz0");
  }
}

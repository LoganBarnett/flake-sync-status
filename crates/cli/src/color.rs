/// FNV-1a hash for consistent, deterministic color generation from strings.
fn fnv1a(s: &str) -> u64 {
  const FNV_PRIME: u64 = 1099511628211;
  const FNV_OFFSET: u64 = 14695981039346656037;
  s.bytes()
    .fold(FNV_OFFSET, |hash, byte| (hash ^ byte as u64).wrapping_mul(FNV_PRIME))
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

/// Wrap text in a 24-bit ANSI foreground color sequence.
fn fg(r: u8, g: u8, b: u8, text: &str) -> String {
  format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
}

/// Color text with a rainbow foreground color derived from its content via
/// FNV-1a hashing.  The same string always yields the same color across runs.
pub fn rainbow(text: &str) -> String {
  let hue = (fnv1a(text) % 360) as f64;
  let (r, g, b) = hsv_to_rgb(hue, 0.85, 1.0);
  fg(r, g, b, text)
}

pub fn green(text: &str) -> String {
  fg(60, 220, 100, text)
}

pub fn red(text: &str) -> String {
  fg(220, 60, 60, text)
}

pub fn yellow(text: &str) -> String {
  fg(220, 180, 60, text)
}

/// Colorize each segment of a platform double (e.g. `aarch64-linux`) with its
/// own rainbow-derived foreground color.  The `-` separator is uncolored.
pub fn rainbow_platform(system: &str) -> String {
  system
    .split('-')
    .map(|seg| rainbow(seg))
    .collect::<Vec<_>>()
    .join("-")
}

/// Extract the hash portion of a Nix store path and return up to 12 chars.
/// `/nix/store/abc12345xyz0-name` → `abc12345xyz0`
pub fn store_hash_abbrev(store_path: &str) -> &str {
  let after_store =
    store_path.strip_prefix("/nix/store/").unwrap_or(store_path);
  let hash_end = after_store.find('-').unwrap_or(after_store.len()).min(12);
  &after_store[..hash_end]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_rainbow_is_deterministic() {
    assert_eq!(rainbow("silicon"), rainbow("silicon"));
  }

  #[test]
  fn test_rainbow_differs_per_input() {
    // Not guaranteed but extremely likely with a good hash.
    assert_ne!(rainbow("silicon"), rainbow("argon"));
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

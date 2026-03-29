use std::sync::atomic::{AtomicBool, Ordering};

use hash_color_lib::{
  detect_color_support, ColorSupport, ColorizerOptions, Ground, HashColorizer,
};

static FORCE_NO_COLOR: AtomicBool = AtomicBool::new(false);

/// Disable all color output.  Called at startup when `--no-color` is passed.
/// The `NO_COLOR` environment variable (https://no-color.org/) is also honored.
pub fn disable_color() {
  FORCE_NO_COLOR.store(true, Ordering::Relaxed);
}

fn color_disabled() -> bool {
  FORCE_NO_COLOR.load(Ordering::Relaxed)
    || std::env::var_os("NO_COLOR").is_some()
}

/// Map an 8-bit RGB component to the nearest xterm-256 color cube level
/// (0-5).  Cube values are 0, 95, 135, 175, 215, 255.
fn to_cube_level(v: u8) -> u8 {
  // Midpoints between adjacent cube values.
  if v < 48 {
    0
  } else if v < 115 {
    1
  } else if v < 155 {
    2
  } else if v < 195 {
    3
  } else if v < 235 {
    4
  } else {
    5
  }
}

/// Convert 8-bit RGB to the nearest xterm-256 6x6x6 color cube index (16-231).
fn rgb_to_xterm256(r: u8, g: u8, b: u8) -> u8 {
  16 + 36 * to_cube_level(r) + 6 * to_cube_level(g) + to_cube_level(b)
}

/// Wrap text in an ANSI foreground color sequence.  Terminal capability is
/// detected via hash-color-lib, which checks NO_COLOR, FORCE_COLOR, COLORTERM,
/// terminfo capabilities, and TERM in priority order.  Returns plain text when
/// color is disabled or when the terminal does not support at least 256 colors.
fn fg(r: u8, g: u8, b: u8, text: &str) -> String {
  if color_disabled() {
    return text.to_string();
  }
  match detect_color_support() {
    ColorSupport::None | ColorSupport::Ansi16 => text.to_string(),
    ColorSupport::Ansi256 => {
      format!("\x1b[38;5;{}m{text}\x1b[0m", rgb_to_xterm256(r, g, b))
    }
    ColorSupport::TrueColor => {
      format!("\x1b[38;2;{r};{g};{b}m{text}\x1b[0m")
    }
  }
}

/// Color text with a rainbow foreground color derived from its content.
/// Delegates to hash-color-lib for hashing and terminal-capability detection;
/// the same string always yields the same color across runs and machines.
pub fn rainbow(text: &str) -> String {
  // Pass None to let hash-color-lib auto-detect terminal support, or force
  // None explicitly when color has been disabled via --no-color / NO_COLOR.
  let color_support = color_disabled().then_some(ColorSupport::None);
  HashColorizer::new(ColorizerOptions {
    color_support,
    ground: Ground::Foreground,
    seed: 0,
    hue_exclusions: vec![],
    saturation: 0.85,
    lightness: None,
  })
  .colorize(text)
  .to_string()
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

/// Color text with a red background and black foreground, used to flag error
/// states inline in the table.  Returns plain text when color is disabled.
pub fn error_bg(text: &str) -> String {
  if color_disabled() {
    return text.to_string();
  }
  // Standard ANSI codes (41 = red bg, 30 = black fg) work on all terminals
  // including Terminal.app's xterm profile.
  format!("\x1b[41m\x1b[30m{text}\x1b[0m")
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
  fn test_rgb_to_xterm256_primaries() {
    // Primary colors should map to the expected 6x6x6 cube corners.
    assert_eq!(rgb_to_xterm256(255, 0, 0), 196); // red
    assert_eq!(rgb_to_xterm256(0, 255, 0), 46); // green
    assert_eq!(rgb_to_xterm256(0, 0, 255), 21); // blue
    assert_eq!(rgb_to_xterm256(255, 255, 0), 226); // yellow
    assert_eq!(rgb_to_xterm256(0, 255, 255), 51); // cyan
    assert_eq!(rgb_to_xterm256(255, 0, 255), 201); // magenta
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

  #[test]
  fn test_platform_segments_distinct() {
    // Each platform segment must produce a distinct color so that
    // e.g. "aarch64-linux" and "aarch64-darwin" are visually different.
    assert_ne!(rainbow("aarch64"), rainbow("linux"));
    assert_ne!(rainbow("aarch64"), rainbow("darwin"));
    assert_ne!(rainbow("linux"), rainbow("darwin"));
  }
}

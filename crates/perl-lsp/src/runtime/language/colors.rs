//! Document color support for Perl LSP
//!
//! Detects color literals in Perl code (hex codes, ANSI escape sequences,
//! named CSS colors, Term::ANSIColor calls) and provides color presentation
//! options for editors.

use super::super::{byte_to_utf16_col, *};
use once_cell::sync::Lazy;
use perl_position_tracking::{WirePosition, WireRange};
use regex::Regex;

/// Regex for hex color codes: #RGB, #RRGGBB, #RRGGBBAA
static HEX_COLOR_RE: Lazy<Option<Regex>> =
    Lazy::new(|| Regex::new(r"#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})\b").ok());

/// Regex for ANSI escape codes: \e[31m, \e[32m, etc.
static ANSI_COLOR_RE: Lazy<Option<Regex>> = Lazy::new(|| Regex::new(r"\\e\[([0-9;]+)m").ok());

/// Regex for named CSS colors inside quoted strings
static NAMED_COLOR_RE: Lazy<Option<Regex>> = Lazy::new(|| {
    Regex::new(r"(?i)\b(aqua|black|blue|fuchsia|gray|green|lime|maroon|navy|olive|orange|purple|red|silver|teal|white|yellow)\b").ok()
});

/// Regex for Term::ANSIColor color('name') calls
static TERM_ANSICOLOR_RE: Lazy<Option<Regex>> =
    Lazy::new(|| Regex::new(r#"color(?:ed)?\s*\(\s*(?:[^,]*,\s*)?['"](\w+)['"]\s*\)"#).ok());

/// The 17 CSS basic named colors with their RGB values
const NAMED_COLORS: &[(&str, u8, u8, u8)] = &[
    ("aqua", 0, 255, 255),
    ("black", 0, 0, 0),
    ("blue", 0, 0, 255),
    ("fuchsia", 255, 0, 255),
    ("gray", 128, 128, 128),
    ("green", 0, 128, 0),
    ("lime", 0, 255, 0),
    ("maroon", 128, 0, 0),
    ("navy", 0, 0, 128),
    ("olive", 128, 128, 0),
    ("orange", 255, 165, 0),
    ("purple", 128, 0, 128),
    ("red", 255, 0, 0),
    ("silver", 192, 192, 192),
    ("teal", 0, 128, 128),
    ("white", 255, 255, 255),
    ("yellow", 255, 255, 0),
];

/// Look up a named color (case-insensitive) and return its Color
fn lookup_named_color(name: &str) -> Option<Color> {
    let lower = name.to_ascii_lowercase();
    NAMED_COLORS.iter().find(|(n, _, _, _)| *n == lower).map(|(_, r, g, b)| Color {
        red: *r as f64 / 255.0,
        green: *g as f64 / 255.0,
        blue: *b as f64 / 255.0,
        alpha: 1.0,
    })
}

/// Color information with range and RGBA values
#[derive(Debug, Clone)]
pub(super) struct ColorInformation {
    pub range: WireRange,
    pub color: Color,
}

/// RGBA color with values 0.0-1.0
#[derive(Debug, Clone)]
pub(super) struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

/// Detect colors in Perl source code
pub(crate) fn detect_colors(text: &str) -> Vec<ColorInformation> {
    let mut colors = Vec::new();

    // Detect hex color codes in comments: # color: #RRGGBB or #RRGGBBAA
    colors.extend(detect_hex_colors(text));

    // Detect ANSI escape codes: \e[31m, \e[32m, etc.
    colors.extend(detect_ansi_colors(text));

    // Detect named CSS colors inside quoted strings
    colors.extend(detect_named_colors(text));

    // Detect Term::ANSIColor calls: color('red'), colored($text, 'blue')
    colors.extend(detect_term_ansicolor(text));

    colors
}

/// Detect hex color codes in format: #RGB, #RRGGBB, #RRGGBBAA
fn detect_hex_colors(text: &str) -> Vec<ColorInformation> {
    let mut colors = Vec::new();

    let Some(re) = HEX_COLOR_RE.as_ref() else {
        return colors;
    };
    for (line_num, line) in text.lines().enumerate() {
        for cap in re.captures_iter(line) {
            if let Some(mat) = cap.get(0) {
                let hex = &cap[1];
                if let Some(color) = parse_hex_color(hex) {
                    // Convert byte offsets to UTF-16 positions (LSP requirement)
                    let start_char = byte_to_utf16_col(line, mat.start()) as u32;
                    let end_char = byte_to_utf16_col(line, mat.end()) as u32;

                    colors.push(ColorInformation {
                        range: WireRange {
                            start: WirePosition::new(line_num as u32, start_char),
                            end: WirePosition::new(line_num as u32, end_char),
                        },
                        color,
                    });
                }
            }
        }
    }

    colors
}

/// Parse hex color string to RGBA, returning None for invalid input
fn parse_hex_color(hex: &str) -> Option<Color> {
    match hex.len() {
        3 => {
            // #RGB -> #RRGGBB
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Color {
                red: r as f64 / 255.0,
                green: g as f64 / 255.0,
                blue: b as f64 / 255.0,
                alpha: 1.0,
            })
        }
        6 => {
            // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color {
                red: r as f64 / 255.0,
                green: g as f64 / 255.0,
                blue: b as f64 / 255.0,
                alpha: 1.0,
            })
        }
        8 => {
            // #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color {
                red: r as f64 / 255.0,
                green: g as f64 / 255.0,
                blue: b as f64 / 255.0,
                alpha: a as f64 / 255.0,
            })
        }
        _ => None,
    }
}

/// Detect ANSI color escape codes: \e[31m, \e[38;5;196m, \e[38;2;R;G;Bm, etc.
fn detect_ansi_colors(text: &str) -> Vec<ColorInformation> {
    let mut colors = Vec::new();

    let Some(re) = ANSI_COLOR_RE.as_ref() else {
        return colors;
    };
    for (line_num, line) in text.lines().enumerate() {
        for cap in re.captures_iter(line) {
            if let Some(mat) = cap.get(0) {
                let code = &cap[1];
                if let Some(color) = parse_ansi_color(code) {
                    // Convert byte offsets to UTF-16 positions (LSP requirement)
                    let start_char = byte_to_utf16_col(line, mat.start()) as u32;
                    let end_char = byte_to_utf16_col(line, mat.end()) as u32;

                    colors.push(ColorInformation {
                        range: WireRange {
                            start: WirePosition::new(line_num as u32, start_char),
                            end: WirePosition::new(line_num as u32, end_char),
                        },
                        color,
                    });
                }
            }
        }
    }

    colors
}

/// Parse ANSI color code to RGBA
fn parse_ansi_color(code: &str) -> Option<Color> {
    // 24-bit true color: 38;2;R;G;B
    if let Some(rest) = code.strip_prefix("38;2;") {
        let parts: Vec<&str> = rest.splitn(3, ';').collect();
        if parts.len() == 3 {
            let r: u8 = parts[0].parse().ok()?;
            let g: u8 = parts[1].parse().ok()?;
            let b: u8 = parts[2].parse().ok()?;
            return Some(Color {
                red: r as f64 / 255.0,
                green: g as f64 / 255.0,
                blue: b as f64 / 255.0,
                alpha: 1.0,
            });
        }
        return None;
    }

    // 256-color: 38;5;N
    if let Some(rest) = code.strip_prefix("38;5;") {
        let n: u8 = rest.parse().ok()?;
        return Some(color_from_256(n));
    }

    // Basic ANSI color codes (30-37 foreground)
    match code {
        "30" | "0;30" => Some(Color { red: 0.0, green: 0.0, blue: 0.0, alpha: 1.0 }), // Black
        "31" | "0;31" => Some(Color { red: 0.8, green: 0.0, blue: 0.0, alpha: 1.0 }), // Red
        "32" | "0;32" => Some(Color { red: 0.0, green: 0.8, blue: 0.0, alpha: 1.0 }), // Green
        "33" | "0;33" => Some(Color { red: 0.8, green: 0.8, blue: 0.0, alpha: 1.0 }), // Yellow
        "34" | "0;34" => Some(Color { red: 0.0, green: 0.0, blue: 0.8, alpha: 1.0 }), // Blue
        "35" | "0;35" => Some(Color { red: 0.8, green: 0.0, blue: 0.8, alpha: 1.0 }), // Magenta
        "36" | "0;36" => Some(Color { red: 0.0, green: 0.8, blue: 0.8, alpha: 1.0 }), // Cyan
        "37" | "0;37" => Some(Color { red: 0.8, green: 0.8, blue: 0.8, alpha: 1.0 }), // White
        // Bright colors (90-97)
        "90" | "1;30" => Some(Color { red: 0.5, green: 0.5, blue: 0.5, alpha: 1.0 }), // Bright Black
        "91" | "1;31" => Some(Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 }), // Bright Red
        "92" | "1;32" => Some(Color { red: 0.0, green: 1.0, blue: 0.0, alpha: 1.0 }), // Bright Green
        "93" | "1;33" => Some(Color { red: 1.0, green: 1.0, blue: 0.0, alpha: 1.0 }), // Bright Yellow
        "94" | "1;34" => Some(Color { red: 0.0, green: 0.0, blue: 1.0, alpha: 1.0 }), // Bright Blue
        "95" | "1;35" => Some(Color { red: 1.0, green: 0.0, blue: 1.0, alpha: 1.0 }), // Bright Magenta
        "96" | "1;36" => Some(Color { red: 0.0, green: 1.0, blue: 1.0, alpha: 1.0 }), // Bright Cyan
        "97" | "1;37" => Some(Color { red: 1.0, green: 1.0, blue: 1.0, alpha: 1.0 }), // Bright White
        _ => None,
    }
}

/// Convert a 256-color palette index to an RGB Color
fn color_from_256(n: u8) -> Color {
    let (r, g, b) = match n {
        // Standard colors (0-7) — same as basic ANSI 30-37
        0 => (0, 0, 0),
        1 => (204, 0, 0), // 0.8 * 255 ≈ 204
        2 => (0, 204, 0),
        3 => (204, 204, 0),
        4 => (0, 0, 204),
        5 => (204, 0, 204),
        6 => (0, 204, 204),
        7 => (204, 204, 204),
        // Bright colors (8-15) — same as ANSI 90-97
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        // 6x6x6 color cube (16-231)
        16..=231 => {
            let idx = n - 16;
            let ri = idx / 36;
            let gi = (idx % 36) / 6;
            let bi = idx % 6;
            (ri * 51, gi * 51, bi * 51)
        }
        // Grayscale (232-255)
        232..=255 => {
            let val = (n - 232) * 10 + 8;
            (val, val, val)
        }
    };
    Color { red: r as f64 / 255.0, green: g as f64 / 255.0, blue: b as f64 / 255.0, alpha: 1.0 }
}

/// Find quoted string regions (both single and double quotes) in a line.
/// Returns a vec of (start_byte, end_byte) ranges for the content inside quotes.
fn find_quoted_regions(line: &str) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let ch = bytes[i];
        if ch == b'"' || ch == b'\'' {
            let quote = ch;
            let start = i + 1;
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            if i < len {
                // Found closing quote
                regions.push((start, i));
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    regions
}

/// Detect named CSS colors inside quoted strings
fn detect_named_colors(text: &str) -> Vec<ColorInformation> {
    let mut colors = Vec::new();

    let Some(re) = NAMED_COLOR_RE.as_ref() else {
        return colors;
    };

    for (line_num, line) in text.lines().enumerate() {
        let quoted_regions = find_quoted_regions(line);
        if quoted_regions.is_empty() {
            continue;
        }

        for mat in re.find_iter(line) {
            let match_start = mat.start();
            let match_end = mat.end();

            // Only accept matches inside a quoted region
            let in_string =
                quoted_regions.iter().any(|(qs, qe)| match_start >= *qs && match_end <= *qe);
            if !in_string {
                continue;
            }

            if let Some(color) = lookup_named_color(mat.as_str()) {
                let start_char = byte_to_utf16_col(line, match_start) as u32;
                let end_char = byte_to_utf16_col(line, match_end) as u32;

                colors.push(ColorInformation {
                    range: WireRange {
                        start: WirePosition::new(line_num as u32, start_char),
                        end: WirePosition::new(line_num as u32, end_char),
                    },
                    color,
                });
            }
        }
    }

    colors
}

/// Detect Term::ANSIColor calls: color('red'), colored($text, 'blue')
fn detect_term_ansicolor(text: &str) -> Vec<ColorInformation> {
    let mut colors = Vec::new();

    let Some(re) = TERM_ANSICOLOR_RE.as_ref() else {
        return colors;
    };

    for (line_num, line) in text.lines().enumerate() {
        for cap in re.captures_iter(line) {
            if let (Some(mat), Some(name_match)) = (cap.get(0), cap.get(1)) {
                if let Some(color) = lookup_named_color(name_match.as_str()) {
                    let start_char = byte_to_utf16_col(line, mat.start()) as u32;
                    let end_char = byte_to_utf16_col(line, mat.end()) as u32;

                    colors.push(ColorInformation {
                        range: WireRange {
                            start: WirePosition::new(line_num as u32, start_char),
                            end: WirePosition::new(line_num as u32, end_char),
                        },
                        color,
                    });
                }
            }
        }
    }

    colors
}

/// Generate color presentation options for a given color
pub(crate) fn color_to_presentations(color: &Color) -> Vec<Value> {
    let mut presentations = Vec::new();

    // Convert to 0-255 range
    let r = (color.red * 255.0).round() as u8;
    let g = (color.green * 255.0).round() as u8;
    let b = (color.blue * 255.0).round() as u8;
    let a = (color.alpha * 255.0).round() as u8;

    // Hex format: #RRGGBB
    if color.alpha >= 0.99 {
        presentations.push(json!({
            "label": format!("#{:02X}{:02X}{:02X}", r, g, b)
        }));
    } else {
        // Hex format with alpha: #RRGGBBAA
        presentations.push(json!({
            "label": format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        }));
    }

    // RGB format: rgb(r, g, b)
    if color.alpha >= 0.99 {
        presentations.push(json!({
            "label": format!("rgb({}, {}, {})", r, g, b)
        }));
    } else {
        // RGBA format: rgba(r, g, b, a)
        presentations.push(json!({
            "label": format!("rgba({}, {}, {}, {:.2})", r, g, b, color.alpha)
        }));
    }

    // HSL format (basic conversion)
    let (h, s, l) = rgb_to_hsl(color.red, color.green, color.blue);
    if color.alpha >= 0.99 {
        presentations.push(json!({
            "label": format!("hsl({}, {}%, {}%)", h, s, l)
        }));
    } else {
        presentations.push(json!({
            "label": format!("hsla({}, {}%, {}%, {:.2})", h, s, l, color.alpha)
        }));
    }

    // Named color presentation if the color matches a known named color exactly
    if color.alpha >= 0.99 {
        if let Some(name) = lookup_color_name(r, g, b) {
            presentations.push(json!({
                "label": name
            }));
        }
    }

    presentations
}

/// Look up a color name by RGB values (exact match only)
fn lookup_color_name(r: u8, g: u8, b: u8) -> Option<&'static str> {
    NAMED_COLORS
        .iter()
        .find(|(_, cr, cg, cb)| *cr == r && *cg == g && *cb == b)
        .map(|(n, _, _, _)| *n)
}

/// Convert RGB to HSL color space
fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (u32, u32, u32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let l = (max + min) / 2.0;

    let s = if delta == 0.0 { 0.0 } else { delta / (1.0 - (2.0 * l - 1.0).abs()) };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    ((h.round() as u32), (s * 100.0).round() as u32, (l * 100.0).round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_detect_hex_colors() {
        let text = "# This is a red color: #FF0000\n# Blue: #0000FF";
        let colors = detect_hex_colors(text);
        assert_eq!(colors.len(), 2);

        // Check red color
        assert_eq!(colors[0].range.start.line, 0);
        assert!((colors[0].color.red - 1.0).abs() < 0.01);
        assert!((colors[0].color.green - 0.0).abs() < 0.01);
        assert!((colors[0].color.blue - 0.0).abs() < 0.01);

        // Check blue color
        assert_eq!(colors[1].range.start.line, 1);
        assert!((colors[1].color.red - 0.0).abs() < 0.01);
        assert!((colors[1].color.green - 0.0).abs() < 0.01);
        assert!((colors[1].color.blue - 1.0).abs() < 0.01);
    }

    #[test]
    fn parser_detect_short_hex_colors() {
        let text = "# Red: #F00";
        let colors = detect_hex_colors(text);
        assert_eq!(colors.len(), 1);
        assert!((colors[0].color.red - 1.0).abs() < 0.01);
    }

    #[test]
    fn parser_detect_ansi_colors() {
        let text = r"print \e[31mRed\e[0m";
        let colors = detect_ansi_colors(text);
        assert_eq!(colors.len(), 1);
        assert!((colors[0].color.red - 0.8).abs() < 0.01);
    }

    #[test]
    fn parser_color_presentations() {
        let color = Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
        let presentations = color_to_presentations(&color);
        assert!(presentations.len() >= 3);

        // Check that we have hex, rgb, and hsl formats
        let labels: Vec<String> = presentations
            .iter()
            .filter_map(|p| p["label"].as_str().map(|s| s.to_string()))
            .collect();
        assert!(labels.iter().any(|l| l.starts_with('#')));
        assert!(labels.iter().any(|l| l.starts_with("rgb(")));
        assert!(labels.iter().any(|l| l.starts_with("hsl(")));
    }

    #[test]
    fn parser_detect_hex_colors_utf16_positions() {
        // Test that color positions are in UTF-16 code units, not byte offsets
        // Emoji 🎉 = 4 bytes, 2 UTF-16 code units
        let text = "# 🎉 #FF0000";
        let colors = detect_hex_colors(text);
        assert_eq!(colors.len(), 1);

        // Position should be UTF-16 based:
        // "# " = 2 UTF-16 units
        // "🎉" = 2 UTF-16 units (surrogate pair)
        // " " = 1 UTF-16 unit
        // Total before #: 5 UTF-16 units
        assert_eq!(colors[0].range.start.character, 5);

        // "#FF0000" = 7 UTF-16 units
        // End position: 5 + 7 = 12 UTF-16 units
        assert_eq!(colors[0].range.end.character, 12);
    }

    #[test]
    fn parser_detect_ansi_colors_utf16_positions() {
        // Test that ANSI color positions are in UTF-16 code units
        // Chinese char 世 = 3 bytes, 1 UTF-16 code unit
        let text = r"世界 \e[31m";
        let colors = detect_ansi_colors(text);
        assert_eq!(colors.len(), 1);

        // "世界 " = 3 UTF-16 units (2 chars + 1 space)
        // Color starts at position 3
        assert_eq!(colors[0].range.start.character, 3);
    }

    #[test]
    fn test_parse_hex_color_returns_none_for_invalid() {
        // Invalid length returns None
        assert!(parse_hex_color("").is_none());
        assert!(parse_hex_color("1").is_none());
        assert!(parse_hex_color("12345").is_none());

        // Valid hex strings return Some
        assert!(parse_hex_color("F00").is_some());
        assert!(parse_hex_color("FF0000").is_some());
        assert!(parse_hex_color("FF0000FF").is_some());
    }

    #[test]
    fn test_detect_named_colors_in_strings() {
        let text = r#"my $color = "red";"#;
        let colors = detect_named_colors(text);
        assert_eq!(colors.len(), 1);
        assert!((colors[0].color.red - 1.0).abs() < 0.01);
        assert!((colors[0].color.green - 0.0).abs() < 0.01);
        assert!((colors[0].color.blue - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_detect_named_colors_case_insensitive() {
        let text = r#"my $a = "Red"; my $b = "RED";"#;
        let colors = detect_named_colors(text);
        assert_eq!(colors.len(), 2);
        // Both should resolve to red
        for c in &colors {
            assert!((c.color.red - 1.0).abs() < 0.01);
            assert!((c.color.green - 0.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_detect_named_colors_not_outside_strings() {
        // bare red outside quotes should not be detected
        let text = "my $red = 1;";
        let colors = detect_named_colors(text);
        assert_eq!(colors.len(), 0);
    }

    #[test]
    fn test_detect_256_color_ansi() {
        // 256-color foreground: \e[38;5;196m (196 = bright red in 6x6x6 cube)
        let text = r"\e[38;5;196m";
        let colors = detect_ansi_colors(text);
        assert_eq!(colors.len(), 1);
        // 196 - 16 = 180; r = 180/36 = 5, g = (180%36)/6 = 0, b = 180%6 = 0
        // r = 5*51 = 255, g = 0, b = 0
        assert!((colors[0].color.red - 1.0).abs() < 0.01);
        assert!((colors[0].color.green - 0.0).abs() < 0.01);
        assert!((colors[0].color.blue - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_detect_24bit_color_ansi() {
        // 24-bit true color: \e[38;2;255;0;128m
        let text = r"\e[38;2;255;0;128m";
        let colors = detect_ansi_colors(text);
        assert_eq!(colors.len(), 1);
        assert!((colors[0].color.red - 1.0).abs() < 0.01);
        assert!((colors[0].color.green - 0.0).abs() < 0.01);
        assert!((colors[0].color.blue - 128.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_detect_term_ansicolor() {
        let text = "print color('red'), 'hello';";
        let colors = detect_term_ansicolor(text);
        assert_eq!(colors.len(), 1);
        assert!((colors[0].color.red - 1.0).abs() < 0.01);
        assert!((colors[0].color.green - 0.0).abs() < 0.01);
        assert!((colors[0].color.blue - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_named_color_presentation() {
        let color = Color { red: 1.0, green: 0.0, blue: 0.0, alpha: 1.0 };
        let presentations = color_to_presentations(&color);
        let labels: Vec<String> = presentations
            .iter()
            .filter_map(|p| p["label"].as_str().map(|s| s.to_string()))
            .collect();
        // Should include a named color label "red"
        assert!(labels.iter().any(|l| l == "red"), "Expected 'red' in labels: {:?}", labels);
    }
}

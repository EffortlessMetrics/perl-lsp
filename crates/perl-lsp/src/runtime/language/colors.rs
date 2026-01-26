//! Document color support for Perl LSP
//!
//! Detects color literals in Perl code (hex codes, ANSI escape sequences)
//! and provides color presentation options for editors.

use super::super::{byte_to_utf16_col, *};
use lazy_static::lazy_static;
use perl_position_tracking::{WirePosition, WireRange};
use regex::Regex;

lazy_static! {
    /// Regex for hex color codes: #RGB, #RRGGBB, #RRGGBBAA
    static ref HEX_COLOR_RE: Regex = Regex::new(r"#([0-9A-Fa-f]{3}|[0-9A-Fa-f]{6}|[0-9A-Fa-f]{8})\b")
        .unwrap_or_else(|e| panic!("Invalid hex color regex: {e}"));

    /// Regex for ANSI escape codes: \e[31m, \e[32m, etc.
    static ref ANSI_COLOR_RE: Regex = Regex::new(r"\\e\[([0-9;]+)m").unwrap_or_else(|e| panic!("Invalid color regex: {e}"));
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

    colors
}

/// Detect hex color codes in format: #RGB, #RRGGBB, #RRGGBBAA
fn detect_hex_colors(text: &str) -> Vec<ColorInformation> {
    let mut colors = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        for cap in HEX_COLOR_RE.captures_iter(line) {
            if let Some(mat) = cap.get(0) {
                let hex = &cap[1];
                let color = parse_hex_color(hex);

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

    colors
}

/// Parse hex color string to RGBA
fn parse_hex_color(hex: &str) -> Color {
    match hex.len() {
        3 => {
            // #RGB -> #RRGGBB
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
            Color {
                red: r as f64 / 255.0,
                green: g as f64 / 255.0,
                blue: b as f64 / 255.0,
                alpha: 1.0,
            }
        }
        6 => {
            // #RRGGBB
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            Color {
                red: r as f64 / 255.0,
                green: g as f64 / 255.0,
                blue: b as f64 / 255.0,
                alpha: 1.0,
            }
        }
        8 => {
            // #RRGGBBAA
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            Color {
                red: r as f64 / 255.0,
                green: g as f64 / 255.0,
                blue: b as f64 / 255.0,
                alpha: a as f64 / 255.0,
            }
        }
        _ => Color { red: 0.0, green: 0.0, blue: 0.0, alpha: 1.0 },
    }
}

/// Detect ANSI color escape codes: \e[31m, \e[32m, etc.
fn detect_ansi_colors(text: &str) -> Vec<ColorInformation> {
    let mut colors = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        for cap in ANSI_COLOR_RE.captures_iter(line) {
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
    // Basic ANSI color codes (30-37 foreground, 40-47 background)
    // We'll focus on foreground colors for now
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

    presentations
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
}

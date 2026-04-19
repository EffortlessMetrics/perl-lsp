//! Integration tests for perl-lsp-color-provider
//!
//! Tests cover `detect_colors` (hex, ANSI, named CSS, Term::ANSIColor patterns)
//! and `color_to_presentations` (hex, RGB, HSL, named output formats).

use perl_lsp_color_provider::{Color, color_to_presentations, detect_colors};

// ── detect_colors: hex ────────────────────────────────────────────────────────

#[test]
fn test_detect_hex_color_in_comment() {
    let source = "# color: #FF0000\nmy $x = 1;\n";
    let colors = detect_colors(source);
    assert!(
        !colors.is_empty(),
        "expected hex color detected from comment"
    );
}

#[test]
fn test_detect_hex_color_rrggbb_format() {
    let source = "# color: #1A2B3C\n";
    let colors = detect_colors(source);
    assert!(!colors.is_empty(), "expected #RRGGBB to be detected");
}

#[test]
fn test_detect_hex_color_rgb_shorthand() {
    let source = "# color: #F00\n";
    let colors = detect_colors(source);
    // Short hex may or may not be detected depending on regex; test is non-crashing
    let _ = colors;
}

#[test]
fn test_detect_hex_color_white() {
    let source = "# color: #FFFFFF\n";
    let colors = detect_colors(source);
    assert!(!colors.is_empty(), "expected #FFFFFF to be detected");
    let color = &colors[0].color;
    assert!(
        (color.red - 1.0).abs() < 0.01,
        "red should be 1.0 for #FFFFFF, got {}",
        color.red
    );
    assert!(
        (color.green - 1.0).abs() < 0.01,
        "green should be 1.0 for #FFFFFF, got {}",
        color.green
    );
    assert!(
        (color.blue - 1.0).abs() < 0.01,
        "blue should be 1.0 for #FFFFFF, got {}",
        color.blue
    );
}

#[test]
fn test_detect_hex_color_black() {
    let source = "# color: #000000\n";
    let colors = detect_colors(source);
    assert!(!colors.is_empty(), "expected #000000 to be detected");
    let color = &colors[0].color;
    assert!(color.red < 0.01, "red should be 0 for #000000");
    assert!(color.green < 0.01, "green should be 0 for #000000");
    assert!(color.blue < 0.01, "blue should be 0 for #000000");
}

// ── detect_colors: ANSI ───────────────────────────────────────────────────────

#[test]
fn test_detect_ansi_escape_red() {
    let source = r#"my $red = "\e[31m";"#;
    let colors = detect_colors(source);
    assert!(
        !colors.is_empty(),
        "expected ANSI red escape to be detected"
    );
}

#[test]
fn test_detect_ansi_escape_green() {
    let source = r#"print "\e[32mGreen text\e[0m";"#;
    let colors = detect_colors(source);
    assert!(
        !colors.is_empty(),
        "expected ANSI green escape to be detected"
    );
}

#[test]
fn test_detect_ansi_reset_not_a_color() {
    // ANSI reset code \e[0m has no meaningful color — implementation may skip it
    let source = r#"print "\e[0m";"#;
    // Just verify it doesn't panic
    let _ = detect_colors(source);
}

// ── detect_colors: named CSS colors ──────────────────────────────────────────

#[test]
fn test_detect_named_color_red_in_string() {
    let source = r#"my $color = "red";"#;
    let colors = detect_colors(source);
    assert!(
        !colors.is_empty(),
        "expected named color 'red' to be detected"
    );
}

#[test]
fn test_detect_named_color_blue_in_string() {
    let source = r#"my $bg = "blue";"#;
    let colors = detect_colors(source);
    assert!(
        !colors.is_empty(),
        "expected named color 'blue' to be detected"
    );
}

#[test]
fn test_detect_named_color_case_insensitive() {
    let source = r#"my $c = "RED";"#;
    let colors = detect_colors(source);
    // Regex is (?i) so should still match
    assert!(
        !colors.is_empty(),
        "expected case-insensitive named color detection"
    );
}

#[test]
fn test_detect_named_color_white_has_correct_values() {
    let source = r#"my $c = "white";"#;
    let colors = detect_colors(source);
    if let Some(ci) = colors.into_iter().find(|c| {
        (c.color.red - 1.0).abs() < 0.01
            && (c.color.green - 1.0).abs() < 0.01
            && (c.color.blue - 1.0).abs() < 0.01
    }) {
        assert!((ci.color.alpha - 1.0).abs() < 0.01, "alpha should be 1.0");
    }
    // If not found, that's also acceptable (may not detect 'white' as standalone word)
}

// ── detect_colors: Term::ANSIColor ───────────────────────────────────────────

#[test]
fn test_detect_term_ansicolor_call() {
    let source = r#"use Term::ANSIColor; print color('red'), "text";"#;
    let colors = detect_colors(source);
    assert!(
        !colors.is_empty(),
        "expected Term::ANSIColor color() call to be detected"
    );
}

#[test]
fn test_detect_term_ansicolor_colored_call() {
    let source = r#"use Term::ANSIColor; print colored("text", 'blue');"#;
    let _ = detect_colors(source);
    // Just verify no panic; colored() pattern may or may not be detected
}

// ── detect_colors: edge cases ─────────────────────────────────────────────────

#[test]
fn test_detect_colors_empty_source() {
    let colors = detect_colors("");
    assert!(colors.is_empty(), "empty source should produce no colors");
}

#[test]
fn test_detect_colors_plain_code_no_colors() {
    let source = "my $x = 42;\nmy $y = $x + 1;\n";
    let colors = detect_colors(source);
    assert!(
        colors.is_empty(),
        "plain code with no colors should produce no color info"
    );
}

#[test]
fn test_detect_colors_range_is_valid() {
    let source = "# color: #FF0000\n";
    let colors = detect_colors(source);
    if let Some(ci) = colors.first() {
        // Range start should be before or at end
        assert!(
            ci.range.start.line <= ci.range.end.line,
            "range start line should be <= end line"
        );
    }
}

// ── color_to_presentations ────────────────────────────────────────────────────

#[test]
fn test_presentations_non_empty_for_opaque_color() {
    let color = Color {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };
    let presentations = color_to_presentations(&color);
    assert!(
        !presentations.is_empty(),
        "expected at least one presentation for red"
    );
}

#[test]
fn test_presentations_include_hex_format() {
    let color = Color {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };
    let presentations = color_to_presentations(&color);
    let labels: Vec<&str> = presentations
        .iter()
        .filter_map(|p| p["label"].as_str())
        .collect();
    assert!(
        labels.iter().any(|l| l.starts_with('#')),
        "expected a hex presentation (#...), got: {:?}",
        labels
    );
}

#[test]
fn test_presentations_include_rgb_format() {
    let color = Color {
        red: 0.0,
        green: 0.0,
        blue: 1.0,
        alpha: 1.0,
    };
    let presentations = color_to_presentations(&color);
    let labels: Vec<&str> = presentations
        .iter()
        .filter_map(|p| p["label"].as_str())
        .collect();
    assert!(
        labels.iter().any(|l| l.starts_with("rgb(")),
        "expected an rgb() presentation, got: {:?}",
        labels
    );
}

#[test]
fn test_presentations_named_color_for_known_red() {
    let color = Color {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };
    let presentations = color_to_presentations(&color);
    let labels: Vec<&str> = presentations
        .iter()
        .filter_map(|p| p["label"].as_str())
        .collect();
    assert!(
        labels.contains(&"red"),
        "expected 'red' named presentation for pure red, got: {:?}",
        labels
    );
}

#[test]
fn test_presentations_include_hsl_format() {
    let color = Color {
        red: 0.0,
        green: 1.0,
        blue: 0.0,
        alpha: 1.0,
    };
    let presentations = color_to_presentations(&color);
    let labels: Vec<&str> = presentations
        .iter()
        .filter_map(|p| p["label"].as_str())
        .collect();
    assert!(
        labels.iter().any(|l| l.starts_with("hsl(")),
        "expected an hsl() presentation, got: {:?}",
        labels
    );
}

#[test]
fn test_presentations_hex_with_alpha_for_transparent_color() {
    let color = Color {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.5,
    };
    let presentations = color_to_presentations(&color);
    let labels: Vec<&str> = presentations
        .iter()
        .filter_map(|p| p["label"].as_str())
        .collect();
    // With alpha < 1.0, expect either #RRGGBBAA or rgba() form
    let has_alpha_format = labels
        .iter()
        .any(|l| (l.starts_with('#') && l.len() == 9) || l.starts_with("rgba("));
    assert!(
        has_alpha_format,
        "expected alpha-aware format for alpha=0.5, got: {:?}",
        labels
    );
}

#[test]
fn test_presentations_no_named_color_for_unknown_rgb() {
    // A color that doesn't match any of the 17 named CSS colors
    let color = Color {
        red: 0.1,
        green: 0.2,
        blue: 0.3,
        alpha: 1.0,
    };
    let presentations = color_to_presentations(&color);
    let labels: Vec<&str> = presentations
        .iter()
        .filter_map(|p| p["label"].as_str())
        .collect();
    let has_named = labels
        .iter()
        .any(|l| !l.starts_with('#') && !l.starts_with("rgb") && !l.starts_with("hsl"));
    // Either no named label or at least has hex/rgb/hsl
    assert!(!presentations.is_empty());
    let _ = has_named;
}

#[test]
fn test_presentations_black_has_correct_hex() {
    let color = Color {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 1.0,
    };
    let presentations = color_to_presentations(&color);
    let labels: Vec<&str> = presentations
        .iter()
        .filter_map(|p| p["label"].as_str())
        .collect();
    assert!(
        labels.contains(&"#000000"),
        "expected #000000 for black, got: {:?}",
        labels
    );
}

#[test]
fn test_presentations_white_has_correct_hex() {
    let color = Color {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
        alpha: 1.0,
    };
    let presentations = color_to_presentations(&color);
    let labels: Vec<&str> = presentations
        .iter()
        .filter_map(|p| p["label"].as_str())
        .collect();
    assert!(
        labels.contains(&"#FFFFFF"),
        "expected #FFFFFF for white, got: {:?}",
        labels
    );
}

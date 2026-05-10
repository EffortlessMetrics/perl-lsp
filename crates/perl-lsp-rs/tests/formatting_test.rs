//! Integration tests for code formatting

use perl_lsp::convert::{WirePosition, WireRange};
use perl_lsp::features::formatting::{CodeFormatter, FormattingOptions};
use perl_tdd_support::must;

#[test]
fn test_basic_formatting() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };

    // Test simple unformatted code supported by the native formatter.
    let code = "my$x=1;\n";

    match formatter.format_document(code, &options) {
        Ok(edits) => {
            // Should have at least one edit
            assert!(!edits.is_empty(), "Expected formatting edits");

            let formatted = &edits[0].new_text;

            // Check that formatting improved spacing
            assert!(formatted.contains("my $x"));
        }
        Err(e) => {
            // If perltidy is not installed, skip the test
            if e.to_string().contains("not found") {
                eprintln!("Skipping test: perltidy not installed");
                return;
            }
            must(Err::<(), _>(format!("Formatting failed: {}", e)));
        }
    }
}

#[test]
fn test_range_formatting() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };

    // Multi-line code
    let code = "my $x = 1;\nsub test{return$x;}\nmy $y = 2;";

    // Format only the middle line
    let range = WireRange { start: WirePosition::new(1, 0), end: WirePosition::new(1, 20) };

    match formatter.format_range(code, &range, &options) {
        Ok(edits) => {
            if !edits.is_empty() {
                let formatted = &edits[0].new_text;
                // Should format the subroutine
                assert!(formatted.contains("sub test"));
                assert!(formatted.contains("return $x"));
            }
        }
        Err(e) => {
            // If perltidy is not installed, skip the test
            if e.to_string().contains("not found") {
                eprintln!("Skipping test: perltidy not installed");
                return;
            }
            must(Err::<(), _>(format!("Range formatting failed: {}", e)));
        }
    }
}

#[test]
fn test_formatting_preserves_trailing_comment() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let code = "my$x=1; # keep\n";

    let edits = must(formatter.format_document(code, &options));

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "my $x = 1; # keep\n");
}

#[test]
fn test_range_formatting_preserves_trailing_comment() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };
    let code = "my$x=1; # keep\nmy$y=2;\n";
    let range = WireRange { start: WirePosition::new(0, 0), end: WirePosition::new(0, 14) };

    let edits = must(formatter.format_range(code, &range, &options));

    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].new_text, "my $x = 1; # keep");
}

#[test]
fn test_empty_document() {
    let formatter = CodeFormatter::new();
    let options = FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        trim_trailing_whitespace: None,
        insert_final_newline: None,
        trim_final_newlines: None,
    };

    // Empty document should return no edits
    match formatter.format_document("", &options) {
        Ok(edits) => {
            assert!(edits.is_empty() || edits[0].new_text.is_empty());
        }
        Err(e) => {
            // If perltidy is not installed, skip the test
            if e.to_string().contains("not found") {
                eprintln!("Skipping test: perltidy not installed");
                return;
            }
            must(Err::<(), _>(format!("Formatting empty document failed: {}", e)));
        }
    }
}

//! Integration tests for code formatting

use perl_lsp::convert::{WirePosition, WireRange};
use perl_lsp::features::formatting::{CodeFormatter, FormattingOptions};

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

    // Test simple unformatted code
    let code = "sub test{my$x=1;return$x;}";

    match formatter.format_document(code, &options) {
        Ok(edits) => {
            // Should have at least one edit
            assert!(!edits.is_empty(), "Expected formatting edits");

            let formatted = &edits[0].new_text;

            // Check that formatting improved spacing
            assert!(formatted.contains("sub test"));
            assert!(formatted.contains("my $x"));
            assert!(formatted.contains("return $x"));
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

//! BDD-style UX coverage for `textDocument/linkedEditingRange`.
//!
//! This suite builds a tiny scenario harness around cursor-marked fixtures so
//! each test focuses on one user-facing behavior.

use std::error::Error;

use perl_lsp_rs_core::providers::lsp_compat::linked_editing::handle_linked_editing;
use perl_parser_core::position::offset_to_utf16_line_col;

type TestResult = Result<(), Box<dyn Error>>;

const CURSOR: &str = "<<CURSOR>>";

struct LinkedEditingScenario {
    text: String,
    line: u32,
    character: u32,
}

impl LinkedEditingScenario {
    fn from_fixture(fixture: &str) -> Result<Self, String> {
        let byte = fixture
            .find(CURSOR)
            .ok_or_else(|| "fixture must include <<CURSOR>> marker".to_string())?;
        let text = fixture.replacen(CURSOR, "", 1);
        let (line, character) = offset_to_utf16_line_col(&text, byte);

        Ok(Self { text, line, character })
    }

    fn linked_ranges(&self) -> Option<lsp_types::LinkedEditingRanges> {
        handle_linked_editing(&self.text, self.line, self.character)
    }

    fn assert_pair(self, expected_first: &str, expected_second: &str) -> TestResult {
        let ranges = self
            .linked_ranges()
            .ok_or_else(|| "expected linked range pair but got none".to_string())?;
        if ranges.ranges.len() != 2 {
            return Err(format!("expected exactly 2 ranges, got {}", ranges.ranges.len()).into());
        }

        let first = slice_for_range(&self.text, &ranges.ranges[0])?;
        let second = slice_for_range(&self.text, &ranges.ranges[1])?;

        if first != expected_first {
            return Err(format!(
                "first range mismatch: expected '{expected_first}', got '{first}'"
            )
            .into());
        }
        if second != expected_second {
            return Err(format!(
                "second range mismatch: expected '{expected_second}', got '{second}'"
            )
            .into());
        }

        Ok(())
    }
}

fn slice_for_range<'a>(text: &'a str, range: &lsp_types::Range) -> Result<&'a str, String> {
    let start = perl_parser_core::position::utf16_line_col_to_offset(
        text,
        range.start.line,
        range.start.character,
    );
    let end = perl_parser_core::position::utf16_line_col_to_offset(
        text,
        range.end.line,
        range.end.character,
    );
    text.get(start..end).ok_or_else(|| format!("invalid UTF-8 boundaries for range {start}..{end}"))
}

#[test]
fn given_cursor_on_opening_brace_when_linked_editing_requested_then_matching_braces_are_returned()
-> TestResult {
    LinkedEditingScenario::from_fixture("if (1) <<CURSOR>>{ say 'x'; }")?.assert_pair("{", "}")
}

#[test]
fn given_cursor_after_closing_brace_when_linked_editing_requested_then_matching_braces_are_still_returned()
-> TestResult {
    LinkedEditingScenario::from_fixture("if (1) { say 'x'; }<<CURSOR>>")?.assert_pair("{", "}")
}

#[test]
fn given_cursor_inside_quoted_heredoc_label_when_linked_editing_requested_then_label_and_terminator_are_returned()
-> TestResult {
    LinkedEditingScenario::from_fixture("my $doc = <<\"EO<<CURSOR>>D\";\ncontent\nEOD\n")?
        .assert_pair("EOD", "EOD")
}

#[test]
fn given_cursor_on_substitution_middle_delimiter_when_linked_editing_requested_then_replacement_pair_is_returned()
-> TestResult {
    LinkedEditingScenario::from_fixture("$x =~ s/foo<<CURSOR>>/bar/;")?.assert_pair("/", "/")
}

#[test]
fn given_cursor_on_regex_delimiter_when_linked_editing_requested_then_delimiter_pair_is_returned()
-> TestResult {
    LinkedEditingScenario::from_fixture("$x =~ m<<CURSOR>>#foo#;")?.assert_pair("#", "#")
}

#[test]
fn given_cursor_on_plain_identifier_when_linked_editing_requested_then_no_ranges_are_returned()
-> TestResult {
    let scenario = LinkedEditingScenario::from_fixture("my <<CURSOR>>$name = 'x';")?;
    if scenario.linked_ranges().is_some() {
        return Err("expected no linked ranges for plain identifier cursor".into());
    }
    Ok(())
}

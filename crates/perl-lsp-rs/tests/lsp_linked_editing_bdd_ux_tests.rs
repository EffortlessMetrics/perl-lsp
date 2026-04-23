//! BDD UX coverage for linked-editing flows.
//!
//! Focuses on user-visible cursor journeys (open delimiter, close delimiter,
//! and heredoc labels) so behavior remains stable across server refactors.

mod support;

use support::lsp_harness::{LinkedEditingSpan, LspHarness};

struct Scenario {
    name: &'static str,
}

impl Scenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, message: &str) {
        eprintln!("[{}] Given {}", self.name, message);
    }

    fn when(&self, message: &str) {
        eprintln!("[{}] When {}", self.name, message);
    }

    fn then(&self, message: &str) {
        eprintln!("[{}] Then {}", self.name, message);
    }
}

#[test]
fn bdd_linked_editing_braces_prioritizes_opening_delimiter()
-> Result<(), Box<dyn std::error::Error>> {
    let scenario = Scenario::new("Linked editing for braces prioritizes opening delimiter");
    scenario.given("a Perl hash literal with opening and closing braces");

    let mut harness = LspHarness::new();
    harness.initialize_ready("file:///workspace", None)?;
    let uri = "file:///test.pl";
    let source = "sub build_hash { my $h = { answer => 42 }; return $h; }\n";
    harness.open_document(uri, source)?;

    scenario.when("requesting linked ranges from the opening brace");
    let from_open = harness.linked_editing_ranges(uri, 0, 25)?;

    scenario.then("the opening delimiter yields the paired brace spans");
    let expected = vec![
        LinkedEditingSpan { start_line: 0, start_character: 25, end_line: 0, end_character: 26 },
        LinkedEditingSpan { start_line: 0, start_character: 40, end_line: 0, end_character: 41 },
    ];
    if from_open != expected {
        return Err(format!("unexpected brace spans from opener: {from_open:?}").into());
    }

    scenario.when("requesting linked ranges from the closing brace");
    let from_close = harness.linked_editing_ranges(uri, 0, 39)?;

    scenario.then("the closing delimiter currently returns no linked range payload");
    if !from_close.is_empty() {
        return Err(format!("expected empty spans from closing brace, got {from_close:?}").into());
    }

    Ok(())
}

#[test]
fn bdd_linked_editing_heredoc_tracks_label_on_both_ends() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = Scenario::new("Linked editing for heredoc labels tracks both endpoints");
    scenario.given("a heredoc with an identifier label and matching terminator");

    let mut harness = LspHarness::new();
    harness.initialize_ready("file:///workspace", None)?;
    let uri = "file:///heredoc.pl";
    let source = "my $body = <<EOF;\nhello\nEOF\n";
    harness.open_document(uri, source)?;

    scenario.when("requesting linked ranges from the opener label");
    let from_opener = harness.linked_editing_ranges(uri, 0, 13)?;

    scenario.then("the returned opener spans are exactly the expected heredoc label extents");
    let expected = vec![
        LinkedEditingSpan { start_line: 0, start_character: 13, end_line: 0, end_character: 16 },
        LinkedEditingSpan { start_line: 2, start_character: 0, end_line: 2, end_character: 3 },
    ];
    if from_opener != expected {
        return Err(format!("unexpected heredoc spans from opener: {from_opener:?}").into());
    }

    scenario.when("requesting linked ranges from the terminator label");
    let from_terminator = harness.linked_editing_ranges(uri, 2, 1)?;

    scenario.then("the terminator label currently returns no linked range payload");
    if !from_terminator.is_empty() {
        return Err(
            format!("expected empty spans from terminator label, got {from_terminator:?}").into()
        );
    }

    Ok(())
}

#[test]
fn bdd_linked_editing_returns_none_on_non_delimiter_text() -> Result<(), Box<dyn std::error::Error>>
{
    let scenario = Scenario::new("Linked editing ignores plain identifier text");
    scenario.given("a Perl statement without a delimiter at the cursor");

    let mut harness = LspHarness::new();
    harness.initialize_ready("file:///workspace", None)?;
    let uri = "file:///plain.pl";
    harness.open_document(uri, "my $value = 42;\n")?;

    scenario.when("requesting linked ranges over identifier text");
    let result = harness.linked_editing_range(uri, 0, 4)?;

    scenario.then("the server returns null because there is no linked editing target");
    if !result.is_null() {
        return Err(format!("expected null linked-editing response, got {result}").into());
    }

    Ok(())
}

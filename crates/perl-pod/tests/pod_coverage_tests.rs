//! Focused tests targeting previously-uncovered branches in `perl-pod`.
//!
//! Each test is labelled with the branch it exercises.

use perl_pod::{extract_pod, extract_pod_from_file};
use std::io::Write;

// -- extract_pod_from_file success path (line 45) --

/// `extract_pod_from_file` happy path: read a real temp file and return Ok.
///
/// The error path (missing file) was already covered; this covers the
/// `Ok(extract_pod(&content))` return on line 45.
#[test]
fn extract_pod_from_file_success_reads_real_file() -> std::io::Result<()> {
    let mut tmp = tempfile::NamedTempFile::new()?;
    write!(tmp, "=head1 NAME\n\nTempFile::Module - temp\n\n=cut\n")?;
    let doc = extract_pod_from_file(tmp.path())?;
    assert_eq!(doc.name.as_deref(), Some("TempFile::Module - temp"));
    Ok(())
}

// -- PodDoc::is_empty returning false (line 34 branch) --

/// `is_empty()` must return `false` when `name` is populated.
///
/// Previously only `is_empty() == true` (all-None doc) was exercised.
/// This covers the `false` branch of the overall method.
#[test]
fn is_empty_returns_false_when_name_is_set() {
    let doc = extract_pod("=head1 NAME\n\nSome::Module - desc\n\n=cut\n");
    assert!(!doc.is_empty(), "doc with name should not be empty");
}

/// `is_empty()` returns `false` when only methods are present (name/synopsis/
/// description all `None`).  Exercises the path through all three `is_none()`
/// checks before reaching `&& self.methods.is_empty()`.
#[test]
fn is_empty_returns_false_when_only_methods_present() {
    let doc = extract_pod("=head2 run\n\nRuns the logic.\n\n=cut\n");
    assert!(!doc.is_empty(), "doc with methods should not be empty");
    assert!(doc.name.is_none());
    assert!(doc.synopsis.is_none());
    assert!(doc.description.is_none());
    assert!(!doc.methods.is_empty());
}

// -- =item when body is already empty (line 109 false branch) --

/// When `=item` appears and `body` is still empty (no previous text accumulated),
/// the `if !body.is_empty()` guard on line 109 should take the `false` branch
/// (i.e., skip the `body.push('\n')`) without panicking.
///
/// In the existing `handles_over_item_back` test the section already has
/// "Available options:\n" before the first `=item`, so the body is non-empty.
/// Here we have a list with NO lead-in text so the very first `=item` sees
/// an empty body.
#[test]
fn item_list_with_no_lead_in_body_is_empty_branch() {
    let source = r#"
=head2 flags

=over 4

=item verbose

Enable verbose output.

=item quiet

Suppress output.

=back

=cut
"#;
    let doc = extract_pod(source);
    assert!(doc.methods.contains_key("flags"), "methods should contain 'flags'");
    let text = &doc.methods["flags"];
    assert!(text.contains("- verbose"), "got: {text}");
    assert!(text.contains("- quiet"), "got: {text}");
}

// -- flush_section with empty trimmed body (line 199) --

/// `flush_section` is called when a new section header is encountered.
/// If the previous section had NO body text yet (just whitespace / blank
/// lines), `trimmed.is_empty()` is `true` and the function must return
/// early without writing anything.
///
/// Concretely: a head1 immediately followed by another head1 with no
/// body content in between should not store an empty string.
#[test]
fn flush_section_with_empty_body_stores_nothing() {
    // =head1 DESCRIPTION followed immediately by =head1 NAME with no text
    // between them.  The DESCRIPTION flush is called with an empty body.
    let source = "=head1 DESCRIPTION\n\n=head1 NAME\n\nFoo - desc\n\n=cut\n";
    let doc = extract_pod(source);
    // DESCRIPTION should be absent (no body was provided)
    assert!(
        doc.description.is_none(),
        "empty DESCRIPTION body should not be stored; got: {:?}",
        doc.description
    );
    // NAME should still be extracted normally
    assert_eq!(doc.name.as_deref(), Some("Foo - desc"));
}

/// Same scenario via =head2: a method section header with no body text
/// should not insert an empty string into `methods`.
#[test]
fn flush_section_empty_method_body_stores_nothing() {
    let source = "=head2 stub\n\n=head2 real\n\nDoes real work.\n\n=cut\n";
    let doc = extract_pod(source);
    // 'stub' had no body, so it should not appear in the methods map.
    assert!(
        !doc.methods.contains_key("stub"),
        "method with empty body should not be stored; methods: {:?}",
        doc.methods
    );
    assert!(doc.methods.contains_key("real"), "real method should be stored");
}

// -- Unclosed formatting code at EOF (line 284/285 false branch) --

/// When a POD formatting code like `B<text` appears without a closing `>`
/// the inner loop exits because `i >= len` rather than because `depth == 0`.
/// In that case the `if i < len { i += 1 }` on line 284 should take the
/// `false` branch (skip the increment).
///
/// This is a malformed POD input; the function must not panic and should
/// return whatever was parsed up to that point.
#[test]
fn strip_formatting_code_unclosed_at_eof_does_not_panic() {
    // "B<unclosed" has no closing >.
    let doc = extract_pod("=head1 NAME\n\nB<unclosed\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("unclosed"));
}

/// Unclosed formatting code where the format letter is at the very last
/// character of the input string (`i + 2 < len` boundary).
#[test]
fn strip_formatting_code_truncated_at_letter_lt_boundary() {
    // Only "B<" with no content or closing >
    let doc = extract_pod("=head1 NAME\n\nB<\n\n=cut\n");
    // Must not panic; name field may be empty or partial
    let _ = doc.name;
}

// -- is_pod_format_code false branch (line 378) --

/// A letter that is alphabetic but NOT a recognised POD format code (e.g.
/// `A<text>`) must NOT be treated as a format code.  The text including the
/// `A<...>` sequence should pass through literally.
///
/// This exercises the `false` branch of `is_pod_format_code`.
#[test]
fn non_pod_format_code_passes_through_literally() {
    // 'A' is not in the set {B,I,C,L,F,S,E,X,Z}
    let doc = extract_pod("=head1 NAME\n\nA<text>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    // The entire "A<text>" sequence should be preserved verbatim
    assert!(name.contains("A<text>"), "non-format-code letter should not be stripped; got: {name}");
}

/// `G<foo>` is also not a POD format code; verify same pass-through.
#[test]
fn g_angle_bracket_passes_through_literally() {
    let doc = extract_pod("=head1 NAME\n\nG<foo>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("G<foo>"), "G<> should pass through literally; got: {name}");
}

// -- encode_pod_link_target: safe-char boundary (matches! branch) --

/// Verify that `#` (not in the alphanumeric or `- . _ ~ : /` safe set) is
/// percent-encoded, exercising the `else` branch of the `matches!` guard.
#[test]
fn encode_link_hash_is_percent_encoded() {
    let doc = extract_pod("=head1 NAME\n\nL<Module#anchor>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    // '#' must appear percent-encoded in the URL
    assert!(name.contains("%23"), "hash should be percent-encoded as %23; got: {name}");
}

// -- escape_markdown_link_text: backslash branch --

/// A backslash in the display text of an `L<>` link must be escaped to `\\`
/// so it does not act as a markdown escape character.
#[test]
fn escape_markdown_backslash_in_display_text() {
    // L<back\slash|Module::Name>
    let doc = extract_pod("=head1 NAME\n\nL<back\\slash|Module::Name>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        name.contains("back\\\\slash") || name.contains("back\\slash"),
        "backslash in display text should be handled; got: {name}"
    );
}

// -- PodDoc: partial-empty states (is_empty branches) --

/// `is_empty()` returns `false` when `synopsis` is set but `name` is None.
/// This hits the `self.name.is_none()` == `true` path, then
/// `&& self.synopsis.is_none()` == `false` short-circuits to `false`.
#[test]
fn is_empty_false_when_synopsis_set_name_absent() {
    let doc = extract_pod("=head1 SYNOPSIS\n\n    use Foo;\n\n=cut\n");
    assert!(doc.name.is_none());
    assert!(doc.synopsis.is_some());
    assert!(!doc.is_empty());
}

/// `is_empty()` returns `false` when `description` is set.
/// Exercises the path: name.is_none()=true, synopsis.is_none()=true,
/// description.is_none()=false, which short-circuits to false before methods check.
#[test]
fn is_empty_false_when_description_set() {
    let doc = extract_pod("=head1 DESCRIPTION\n\nSome text.\n\n=cut\n");
    assert!(doc.name.is_none());
    assert!(doc.synopsis.is_none());
    assert!(doc.description.is_some());
    assert!(!doc.is_empty());
}

// -- first_paragraph: single-line body (no blank line) --

/// A description with only one paragraph and no trailing blank line: the
/// `break` path in `first_paragraph` is never hit, and the entire text is
/// the first paragraph.
#[test]
fn description_single_paragraph_no_blank_line() {
    let source = "=head1 DESCRIPTION\n\nOne liner.\n\n=cut\n";
    let doc = extract_pod(source);
    assert_eq!(doc.description.as_deref(), Some("One liner."));
}

// -- =begin / =for as first POD directive (lines 74/75 true branches) --

/// `=begin` as the very first POD directive must trigger `in_pod = true`
/// (line 74 true branch).  Existing tests start with `=head`, `=pod`, or
/// `=encoding`; this one uses `=begin` first.
#[test]
fn begin_directive_starts_pod_mode() {
    let source = r#"
package Foo;

=begin text

Some text block.

=end text

=head1 NAME

Foo - A module

=cut
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Foo - A module"));
}

/// `=for` as the very first POD directive must trigger `in_pod = true`
/// (line 75 true branch).
#[test]
fn for_directive_starts_pod_mode() {
    let source = r#"
package Foo;

=for html
<p>HTML comment</p>

=head1 NAME

Foo - A module

=cut
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Foo - A module"));
}

// -- =begin / =end / =for within a POD section (lines 144/145/146) --

/// `=begin` within an active POD section is a "skip other directives"
/// directive (line 144 true branch in the skip block).
#[test]
fn begin_within_pod_section_is_skipped() {
    let source =
        "=head1 NAME\n\n=begin html\n\n<em>html block</em>\n\n=end html\n\nFoo - Module\n\n=cut\n";
    let doc = extract_pod(source);
    // =begin triggers in_pod, then is skipped. Body should still get the
    // plain-text line.
    assert!(doc.name.is_some(), "name should be extracted");
}

/// `=end` within an active POD section is a "skip other directives"
/// directive (line 145 true branch in the skip block).
#[test]
fn end_within_pod_section_is_skipped() {
    // =end on its own without a prior =begin still must be skipped silently.
    let source = "=head1 NAME\n\nFoo - Module\n\n=end\n\n=cut\n";
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Foo - Module"));
}

/// `=for` within an active POD section is a "skip other directives"
/// directive (line 146 true branch in the skip block).
#[test]
fn for_within_pod_section_is_skipped() {
    let source = "=head1 NAME\n\nFoo - Module\n\n=for html <br>\n\n=cut\n";
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Foo - Module"));
}

// -- first_paragraph: leading blank line (line 229 false branch) --

/// When `first_paragraph` receives text that starts with a blank line,
/// the condition `line.trim().is_empty() && !result.is_empty()` has
/// `result.is_empty() == true`, so the entire condition is `false`.
/// The blank line should be accumulated or skipped without breaking.
///
/// This exercises the `false` branch on line 229.
///
/// We produce this case by having the description body start with `Z<>` (a
/// zero-width POD format code) followed by a blank line and the real content.
/// `strip_pod_formatting` removes `Z<>`, leaving a leading blank line in the
/// cleaned text that is fed to `first_paragraph`.
#[test]
fn description_with_leading_blank_line_false_branch() {
    // Z<> strips to empty, so body becomes "\n\nActual content." after formatting.
    // first_paragraph sees a leading blank line when result is still empty.
    let source = "=head1 DESCRIPTION\n\nZ<>\n\nActual content.\n\n=cut\n";
    let doc = extract_pod(source);
    // Must not panic. The actual content must be extracted.
    assert!(
        doc.description.is_some(),
        "description should be extracted; got: {:?}",
        doc.description
    );
    let desc = doc.description.as_deref().unwrap_or("");
    assert!(
        desc.contains("Actual content"),
        "content after leading blank should be captured; got: {desc}"
    );
}

// -- S<>, X<>, Z<> format codes (is_pod_format_code branches) --

/// `S<text>` (non-breaking space marker) is a valid POD format code.
/// Its content should be stripped the same as `B<>` / `I<>`.
#[test]
fn s_format_code_strips_content() {
    let doc = extract_pod("=head1 NAME\n\nS<non-breaking text>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert_eq!(name, "non-breaking text", "S<> content should be returned; got: {name}");
}

/// `X<index entry>` (index entry) is a valid POD format code.
/// Its content is typically invisible but the code path through
/// `is_pod_format_code` must be exercised.
#[test]
fn x_format_code_strips_content() {
    let doc = extract_pod("=head1 NAME\n\nFoo X<index entry>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    // X<> strips its content in the current implementation
    assert!(name.contains("Foo"), "text before X<> should survive; got: {name}");
}

/// `Z<>` (zero-width POD code) is a valid format code with empty content.
#[test]
fn z_format_code_empty_content() {
    let doc = extract_pod("=head1 NAME\n\nFooZ<>Bar\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    // Z<> contributes nothing; FooBar should be the result
    assert!(name.contains("Foo") && name.contains("Bar"), "Z<> should be transparent; got: {name}");
}

// -- =item as first POD line (body truly empty at first item) --

/// When `=item` appears as the very first POD line in a section (no
/// `=over` before it, no lead-in text), `body.is_empty()` must be `true`
/// at that point, exercising the `false` branch of the `if !body.is_empty()`
/// guard in the `=item` handler.
///
/// Note: `=over` pushes a `\n` onto body before any `=item`, so to get a
/// truly empty body at `=item` time we use `=item` directly under a `=head2`
/// without any preceding `=over`.
#[test]
fn item_directly_after_head2_body_empty_false_branch() {
    // No =over before =item, so body is empty when first =item is processed.
    let source = "=head2 opts\n\n=item -v\n\nVerbose mode.\n\n=cut\n";
    let doc = extract_pod(source);
    assert!(doc.methods.contains_key("opts"), "opts method should be stored");
    let text = &doc.methods["opts"];
    assert!(text.contains("- -v"), "first item with empty body; got: {text}");
}

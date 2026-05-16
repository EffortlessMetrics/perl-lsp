use perl_pod::{extract_pod, extract_pod_from_file};
use std::io::Write as _;
use std::path::Path;

#[test]
fn empty_source_returns_empty_doc() {
    let doc = extract_pod("");
    assert!(doc.is_empty());
}

#[test]
fn pure_code_no_pod() {
    let source = r#"
package Foo::Bar;
use strict;
sub new { bless {}, shift }
1;
"#;
    let doc = extract_pod(source);
    assert!(doc.is_empty());
}

#[test]
fn code_before_pod_still_allows_extraction() {
    let source = r#"
package Inventory;

sub add_item { }

=head1 NAME

Inventory - Tracks stock

=cut
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Inventory - Tracks stock"));
}

#[test]
fn extracts_name_section() {
    let source = r#"
=head1 NAME

Foo::Bar - A sample module

=cut
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Foo::Bar - A sample module"));
}

#[test]
fn extracts_synopsis() {
    let source = r#"
=head1 SYNOPSIS

    use Foo::Bar;
    my $obj = Foo::Bar->new();

=cut
"#;
    let doc = extract_pod(source);
    assert!(doc.synopsis.is_some());
    assert!(doc.synopsis.as_ref().is_some_and(|s| s.contains("use Foo::Bar")));
}

#[test]
fn extracts_description_first_paragraph() {
    let source = r#"
=head1 DESCRIPTION

This module does amazing things.
It is very useful.

This second paragraph should not be included.

=cut
"#;
    let doc = extract_pod(source);
    assert_eq!(
        doc.description.as_deref(),
        Some("This module does amazing things.\nIt is very useful.")
    );
}

#[test]
fn extracts_methods() {
    let source = r#"
=head2 new

Creates a new instance of the object.

=head2 process

Processes the input data.

=cut
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.methods.len(), 2);
    assert!(doc.methods.contains_key("new"));
    assert!(doc.methods.contains_key("process"));
    assert!(doc.methods["new"].contains("Creates a new instance"));
    assert!(doc.methods["process"].contains("Processes the input data"));
}

#[test]
fn strips_bold_formatting() {
    let doc = extract_pod("=head1 NAME\n\nB<bold text>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("bold text"));
}

#[test]
fn strips_italic_formatting() {
    let doc = extract_pod("=head1 NAME\n\nI<italic text>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("italic text"));
}

#[test]
fn strips_code_formatting() {
    let doc = extract_pod("=head1 NAME\n\nC<my $var>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("my $var"));
}

#[test]
fn strips_link_simple() {
    // L<Module::Name> now renders as a markdown link (Option B)
    let doc = extract_pod("=head1 NAME\n\nL<Module::Name>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("[Module::Name]"), "got: {name}");
    assert!(name.contains("perl-module://Module::Name"), "got: {name}");
}

#[test]
fn strips_link_with_display_text() {
    // L<click here|Module::Name> displays the explicit text as a markdown link
    let doc = extract_pod("=head1 NAME\n\nL<click here|Module::Name>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("[click here]"), "got: {name}");
    assert!(name.contains("perl-module://Module::Name"), "got: {name}");
}

#[test]
fn strips_link_with_section() {
    // L<Module::Name/method> displays module as link, target includes section
    let doc = extract_pod("=head1 NAME\n\nL<Module::Name/method>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("[Module::Name]"), "got: {name}");
    assert!(name.contains("perl-module://Module::Name/method"), "got: {name}");
}

#[test]
fn mixed_formatting() {
    let doc = extract_pod("=head1 NAME\n\nUse B<new> to create a C<Foo> object\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("Use new to create a Foo object"));
}

#[test]
fn handles_cut_properly() {
    let source = r#"
=head1 NAME

First - Module

=cut

package First;

=head1 NAME

Second - Module

=cut
"#;
    let doc = extract_pod(source);
    // Second =head1 NAME overwrites the first
    assert_eq!(doc.name.as_deref(), Some("Second - Module"));
}

#[test]
fn handles_pod_without_cut_at_eof() {
    let source = r#"
=head1 NAME

Foo::Bar - No cut at end
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Foo::Bar - No cut at end"));
}

#[test]
fn handles_over_item_back() {
    let source = r#"
=head2 options

Available options:

=over 4

=item B<verbose>

Enable verbose output.

=item B<quiet>

Suppress output.

=back

=cut
"#;
    let doc = extract_pod(source);
    assert!(doc.methods.contains_key("options"));
    let method_doc = &doc.methods["options"];
    assert!(method_doc.contains("Available options:"));
    assert!(method_doc.contains("- verbose"));
    assert!(method_doc.contains("- quiet"));
}

#[test]
fn full_module_extraction() {
    let source = r#"
package DateTime::Format::Custom;

use strict;
use warnings;

=head1 NAME

DateTime::Format::Custom - Parse and format dates

=head1 SYNOPSIS

    use DateTime::Format::Custom;
    my $dt = DateTime::Format::Custom->parse("2024-01-01");

=head1 DESCRIPTION

This module provides custom date parsing and formatting
capabilities for the DateTime ecosystem.

It supports multiple input formats and can auto-detect
the format of input strings.

=head2 parse

    my $dt = DateTime::Format::Custom->parse($string);

Parses a date string and returns a L<DateTime> object.

=head2 format

    my $str = DateTime::Format::Custom->format($dt);

Formats a B<DateTime> object as a string.

=head1 AUTHOR

Jane Doe

=cut

sub parse { ... }
sub format { ... }

1;
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("DateTime::Format::Custom - Parse and format dates"));
    assert!(doc.synopsis.as_ref().is_some_and(|s| s.contains("use DateTime::Format::Custom")));
    assert!(doc.description.as_ref().is_some_and(|s| s.contains("custom date parsing")));
    // Description should only be first paragraph
    assert!(!doc.description.as_ref().is_none_or(|s| s.contains("auto-detect")));
    assert_eq!(doc.methods.len(), 2);
    assert!(doc.methods["parse"].contains("Parses a date string"));
    assert!(doc.methods["parse"].contains("DateTime"));
    assert!(doc.methods["format"].contains("Formats a DateTime object"));
}

#[test]
fn extract_pod_from_file_missing_file() {
    let result = extract_pod_from_file(Path::new("/nonexistent/file.pm"));
    assert!(result.is_err());
}

#[test]
fn nested_formatting_codes() {
    // Depth tracking handles nested angle brackets correctly
    let doc = extract_pod("=head1 NAME\n\nB<I<bold italic>>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("bold italic"));
}

#[test]
fn no_formatting_passthrough() {
    let doc = extract_pod("=head1 NAME\n\nplain text here\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("plain text here"));
}

#[test]
fn empty_formatting_code() {
    let doc = extract_pod("=head1 NAME\n\nB<> and C<>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some(" and "));
}

#[test]
fn head1_other_sections_ignored() {
    let source = r#"
=head1 AUTHOR

John Doe

=head1 LICENSE

Same as Perl itself.

=cut
"#;
    let doc = extract_pod(source);
    assert!(doc.name.is_none());
    assert!(doc.synopsis.is_none());
    assert!(doc.description.is_none());
    assert!(doc.methods.is_empty());
}

#[test]
fn pod_directive_starts_pod_mode() {
    let source = r#"
package Foo;

=pod

This is some POD text.

=head1 NAME

Foo - A module

=cut
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Foo - A module"));
}

#[test]
fn encoding_directive_starts_pod() {
    let source = r#"
=encoding utf-8

=head1 NAME

Encoded::Module - Uses UTF-8

=cut
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Encoded::Module - Uses UTF-8"));
}

#[test]
fn f_format_code_for_filenames() {
    let doc = extract_pod("=head1 NAME\n\nSee F<config.yml>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("See config.yml"));
}

#[test]
fn e_format_code_decodes_common_entities() {
    let doc = extract_pod("=head1 NAME\n\nE<lt> E<gt> E<amp> E<quot> E<apos>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("< > & \" '"));
}

// ── POD L<> link → markdown link tests (Option B) ───────────────────────

/// `L<Module::Name>` should produce a markdown link with target `perl-module://Module::Name`.
#[test]
fn link_simple_module_renders_markdown() {
    let doc = extract_pod("=head1 NAME\n\nL<File::Path>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        name.contains("[File::Path]"),
        "expected markdown display '[File::Path]' but got: {name}"
    );
    assert!(
        name.contains("perl-module://File::Path"),
        "expected markdown target 'perl-module://File::Path' but got: {name}"
    );
}

/// `L<text|Module::Name>` should use the display text and link to the module.
#[test]
fn link_with_display_text_renders_markdown() {
    let doc = extract_pod("=head1 NAME\n\nL<detailed guide|File::Path>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        name.contains("[detailed guide]"),
        "expected markdown display '[detailed guide]' but got: {name}"
    );
    assert!(
        name.contains("perl-module://File::Path"),
        "expected markdown target 'perl-module://File::Path' but got: {name}"
    );
}

/// `L<Module::Name/section>` should link to module and include section in URI.
#[test]
fn link_with_section_renders_markdown() {
    let doc = extract_pod("=head1 NAME\n\nL<File::Path/DESCRIPTION>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        name.contains("[File::Path]"),
        "expected markdown display '[File::Path]' but got: {name}"
    );
    assert!(
        name.contains("perl-module://File::Path/DESCRIPTION"),
        "expected markdown target 'perl-module://File::Path/DESCRIPTION' but got: {name}"
    );
}

/// `B<L<Module::Name>>` — nested: bold outer, link inner. Both should be preserved.
#[test]
fn nested_bold_around_link_preserves_markdown() {
    let doc = extract_pod("=head1 NAME\n\nB<L<File::Path>>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        name.contains("[File::Path]"),
        "expected markdown display '[File::Path]' in nested B<L<>> but got: {name}"
    );
    assert!(
        name.contains("perl-module://"),
        "expected 'perl-module://' in nested B<L<>> but got: {name}"
    );
}

/// Inline link inside a sentence: "See L<File::Path> for details."
#[test]
fn inline_link_in_description_renders_markdown() {
    let source = "=head1 DESCRIPTION\n\nSee L<File::Path> for details.\n\n=cut\n";
    let doc = extract_pod(source);
    let desc = doc.description.as_deref().unwrap_or("");
    assert!(
        desc.contains("[File::Path]"),
        "expected '[File::Path]' in description but got: {desc}"
    );
    assert!(
        desc.contains("perl-module://File::Path"),
        "expected 'perl-module://File::Path' in description but got: {desc}"
    );
    // The surrounding text should also be preserved
    assert!(
        desc.contains("See") && desc.contains("for details"),
        "surrounding text lost; got: {desc}"
    );
}

/// `L<text|Module::Name/section>` — display text with section target.
#[test]
fn link_display_text_with_section_target_renders_markdown() {
    let doc = extract_pod("=head1 NAME\n\nL<the docs|File::Path/DESCRIPTION>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("[the docs]"), "expected '[the docs]' but got: {name}");
    assert!(
        name.contains("perl-module://File::Path/DESCRIPTION"),
        "expected 'perl-module://File::Path/DESCRIPTION' but got: {name}"
    );
}

/// `L<Module::Name/Section With Spaces>` — section names with spaces must be
/// percent-encoded in the URL so the markdown link is well-formed.
/// This is common in CPAN POD: `L<perlfunc/"use Module LIST">`.
#[test]
fn link_section_with_spaces_encodes_url() {
    let doc = extract_pod("=head1 NAME\n\nL<File::Find/The wanted function>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("[File::Find]"), "expected '[File::Find]' but got: {name}");
    // Spaces must be encoded — a raw space makes the markdown URL malformed
    assert!(
        name.contains("perl-module://File::Find/The%20wanted%20function"),
        "expected percent-encoded URL but got: {name}"
    );
    assert!(!name.contains("The wanted function"), "raw space in URL — should be encoded: {name}");
}

/// `L<click here|Module/Section With Spaces>` — pipe form with spaces in section.
#[test]
fn link_pipe_with_spaced_section_encodes_url() {
    let doc = extract_pod("=head1 NAME\n\nL<click here|File::Find/The wanted function>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("[click here]"), "expected '[click here]' but got: {name}");
    assert!(
        name.contains("perl-module://File::Find/The%20wanted%20function"),
        "expected percent-encoded URL but got: {name}"
    );
}

#[test]
fn link_target_reserved_chars_are_percent_encoded() {
    let doc =
        extract_pod("=head1 NAME\n\nL<click here|File::Path) [evil](http://x.test)>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("[click here]"), "expected '[click here]' but got: {name}");
    assert!(
        name.contains("perl-module://File::Path%29%20%5Bevil%5D%28http://x.test%29"),
        "expected markdown-breaking characters in target to be percent-encoded; got: {name}"
    );
    assert!(
        !name.contains("[evil](http://x.test)"),
        "injected markdown link should not appear as standalone markdown: {name}"
    );
}

#[test]
fn link_display_text_markdown_delimiters_are_escaped() {
    let doc = extract_pod("=head1 NAME\n\nL<click ] here|File::Path>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        name.contains("[click \\] here](perl-module://File::Path)"),
        "expected closing bracket in display text to be escaped; got: {name}"
    );
}

#[test]
fn link_display_text_open_bracket_is_escaped() {
    let doc = extract_pod("=head1 NAME\n\nL<[optional]|Module::Name>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    // Both '[' and ']' in display text must be escaped so the markdown renderer
    // does not mistake them for a nested link boundary.
    assert!(
        name.contains("[\\[optional\\]](perl-module://Module::Name)"),
        "expected open and close brackets in display to be escaped; got: {name}"
    );
}

#[test]
fn link_target_with_unicode_module_name_is_percent_encoded() {
    // Non-ASCII bytes in a link target must be percent-encoded byte-by-byte (UTF-8).
    // This ensures the resulting URL is well-formed even for exotic CPAN module names.
    // 'Ü' is U+00DC, encoded in UTF-8 as the two bytes 0xC3 0x9C.
    let doc = extract_pod("=head1 NAME\n\nL<click here|\u{dc}ber::Module>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    // Both UTF-8 bytes must appear as %C3%9C in the URL.
    assert!(
        name.contains("perl-module://%C3%9Cber::Module"),
        "expected non-ASCII bytes in target to be percent-encoded; got: {name}"
    );
}

#[test]
fn multiple_pod_blocks() {
    let source = r#"
package Multi;

=head1 NAME

Multi - Multiple POD blocks

=cut

sub helper { 1 }

=head2 run

Runs the main logic.

=cut

sub run { }

1;
"#;
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("Multi - Multiple POD blocks"));
    assert!(doc.methods.contains_key("run"));
}

// ── New coverage gap tests ────────────────────────────────────────────────

/// `PodDoc::is_empty()` returns `false` when the doc has content.
/// Exercises the `False` branches of the short-circuit `&&` chain (lines 31-33).
#[test]
fn is_empty_returns_false_when_doc_has_content() {
    let doc = extract_pod("=head1 NAME\n\nFoo - something\n\n=cut\n");
    assert!(!doc.is_empty(), "doc with a name should not be empty");
}

/// `=begin` as the very first directive triggers `in_pod` (line 74 True branch).
/// Without this, POD mode never starts and the body would be skipped.
#[test]
fn begin_directive_starts_pod_mode() {
    let source = "=begin pod\n\n=head1 NAME\n\nBegin::Module - started with =begin\n\n=cut\n";
    let doc = extract_pod(source);
    assert_eq!(
        doc.name.as_deref(),
        Some("Begin::Module - started with =begin"),
        "=begin should initiate POD mode so subsequent sections are parsed"
    );
}

/// `=for` as the very first directive triggers `in_pod` (line 75 True branch).
#[test]
fn for_directive_starts_pod_mode() {
    let source =
        "=for html <b>intro</b>\n\n=head1 NAME\n\nFor::Module - started with =for\n\n=cut\n";
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some("For::Module - started with =for"));
}

/// `=item` appearing as the very first content line in a section (body is empty).
/// Covers the `!body.is_empty()` False branch at line 109 — we should NOT push
/// an extra newline when the body is still empty.
#[test]
fn item_as_first_line_in_section_no_leading_newline() {
    let source = "=head2 options\n\n=over 4\n\n=item alpha\n\nFirst item.\n\n=back\n\n=cut\n";
    let doc = extract_pod(source);
    let method_doc = doc.methods.get("options").map(String::as_str).unwrap_or("");
    // The item should appear but must not start with a blank line
    assert!(method_doc.contains("- alpha"), "item text should be present; got: {method_doc}");
    assert!(
        !method_doc.starts_with('\n'),
        "method doc must not start with a leading newline; got: {method_doc:?}"
    );
}

/// `=begin` inside an active POD block hits the skip-directive branch (line 144).
/// The directive line itself is skipped; subsequent content lines are still accumulated.
#[test]
fn begin_directive_line_inside_pod_is_skipped() {
    // Only the "=begin html" line itself is skipped — the content of the block
    // continues to accumulate. The directive line ("=begin html") must not
    // appear verbatim in the output.
    let source = "=head1 NAME\n\n=begin html\nMy::Module - real name\n=end html\n\n=cut\n";
    let doc = extract_pod(source);
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        !name.contains("=begin html"),
        "the =begin directive line itself should not appear in name; got: {name}"
    );
    assert!(
        name.contains("My::Module - real name"),
        "content after the =begin line should still be captured; got: {name}"
    );
}

/// `=end` inside an active POD block hits the skip-directive branch (line 145).
#[test]
fn end_directive_inside_pod_is_skipped() {
    let source = "=head1 NAME\n\nMy::Module - real\n\n=end\n\n=cut\n";
    let doc = extract_pod(source);
    let name = doc.name.as_deref().unwrap_or("");
    assert!(name.contains("My::Module - real"), "name should be captured before =end; got: {name}");
}

/// `=for` inside an active POD block hits the skip-directive branch (line 146).
#[test]
fn for_directive_inside_pod_is_skipped() {
    let source =
        "=head1 NAME\n\n=for comment this is a private note\n\nMy::ForModule - the name\n\n=cut\n";
    let doc = extract_pod(source);
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        name.contains("My::ForModule - the name"),
        "text after =for directive should be captured; got: {name}"
    );
    assert!(!name.contains("private note"), "=for content should not appear in name; got: {name}");
}

/// `flush_section` called on a section whose body is empty (line 198-199 True branch).
/// This happens when two section headers appear back-to-back with no content between them.
#[test]
fn flush_section_with_empty_body_is_silently_ignored() {
    // =head1 NAME immediately followed by =head1 SYNOPSIS — the NAME section has no body
    let source = "=head1 NAME\n\n=head1 SYNOPSIS\n\nuse Empty::Name;\n\n=cut\n";
    let doc = extract_pod(source);
    // NAME should be absent (empty body → flush is a no-op)
    assert!(doc.name.is_none(), "empty NAME body should produce no name; got: {:?}", doc.name);
    // SYNOPSIS should still be captured
    assert!(
        doc.synopsis.as_deref().is_some_and(|s| s.contains("use Empty::Name")),
        "synopsis should be captured; got: {:?}",
        doc.synopsis
    );
}

/// `extract_pod_from_file` success path (line 45) — read a real temp file.
#[test]
fn extract_pod_from_file_success() -> Result<(), Box<dyn std::error::Error>> {
    let mut tmp = tempfile::NamedTempFile::new()?;
    write!(tmp, "=head1 NAME\n\nTempFile::Module - loaded from disk\n\n=cut\n")?;
    let doc = extract_pod_from_file(tmp.path())?;
    assert_eq!(doc.name.as_deref(), Some("TempFile::Module - loaded from disk"));
    Ok(())
}

/// Unknown `E<>` entity passes through as the entity name (line 373).
/// e.g. `E<nbsp>` is not in the known set and should return "nbsp".
#[test]
fn unknown_e_entity_passes_through_as_text() {
    let doc = extract_pod("=head1 NAME\n\nA E<nbsp> B\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert_eq!(name, "A nbsp B", "unknown E<> entity should pass through as text; got: {name}");
}

/// `first_paragraph` with a leading blank line before actual content.
/// Exercises the `!result.is_empty()` False branch (line 229) — when we encounter
/// a blank line but result is still empty, we must NOT break out of the loop.
#[test]
fn first_paragraph_skips_leading_blank_lines() {
    // strip_pod_formatting is applied before first_paragraph, so feed the
    // description via extract_pod with a blank line at the start of DESCRIPTION.
    let source = "=head1 DESCRIPTION\n\n\nActual first line.\nSecond line.\n\nNot in first paragraph.\n\n=cut\n";
    let doc = extract_pod(source);
    let desc = doc.description.as_deref().unwrap_or("");
    assert!(
        desc.contains("Actual first line."),
        "first paragraph text should be present; got: {desc}"
    );
    assert!(
        !desc.contains("Not in first paragraph."),
        "second paragraph should be excluded; got: {desc}"
    );
}

/// `=over`/`=back` nesting: multiple overlapping list blocks within a single section.
/// Verifies that `in_over` toggles correctly across nested/sequential lists.
#[test]
fn multiple_sequential_over_back_blocks() {
    let source = r#"
=head2 lists

First list:

=over 4

=item one

=item two

=back

Second list:

=over 4

=item three

=back

=cut
"#;
    let doc = extract_pod(source);
    let method_doc = doc.methods.get("lists").map(String::as_str).unwrap_or("");
    assert!(method_doc.contains("- one"), "first list item; got: {method_doc}");
    assert!(method_doc.contains("- two"), "second list item; got: {method_doc}");
    assert!(method_doc.contains("- three"), "third list item in second list; got: {method_doc}");
}

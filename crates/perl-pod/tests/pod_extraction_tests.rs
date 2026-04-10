use perl_pod::{extract_pod, extract_pod_from_file, render_pod_to_markdown};
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
    let doc = extract_pod("=head1 NAME\n\nL<Module::Name>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("Module::Name"));
}

#[test]
fn strips_link_with_display_text() {
    let doc = extract_pod("=head1 NAME\n\nL<click here|Module::Name>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("click here"));
}

#[test]
fn strips_link_with_section() {
    let doc = extract_pod("=head1 NAME\n\nL<Module::Name/method>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("Module::Name"));
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
fn e_format_code_passthrough() {
    // E<lt> E<gt> etc. — we just strip the code, leaving the entity name
    let doc = extract_pod("=head1 NAME\n\nE<lt>\n\n=cut\n");
    assert_eq!(doc.name.as_deref(), Some("lt"));
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

// ===================== render_pod_to_markdown tests =====================

#[test]
fn markdown_bold_b_code() {
    let result = render_pod_to_markdown("B<bold text>");
    assert_eq!(result, "**bold text**");
}

#[test]
fn markdown_italic_i_code() {
    let result = render_pod_to_markdown("I<italic text>");
    assert_eq!(result, "_italic text_");
}

#[test]
fn markdown_code_c_code() {
    let result = render_pod_to_markdown("C<my $var>");
    assert_eq!(result, "`my $var`");
}

#[test]
fn markdown_link_simple() {
    let result = render_pod_to_markdown("L<Module::Name>");
    assert_eq!(result, "[Module::Name](Module::Name)");
}

#[test]
fn markdown_link_with_display_text() {
    let result = render_pod_to_markdown("L<click here|Module::Name>");
    assert_eq!(result, "[click here](Module::Name)");
}

#[test]
fn markdown_link_with_section() {
    let result = render_pod_to_markdown("L<Module::Name/method>");
    assert_eq!(result, "[Module::Name](Module::Name/method)");
}

#[test]
fn markdown_filename_f_code() {
    let result = render_pod_to_markdown("F<config.yml>");
    assert_eq!(result, "`config.yml`");
}

#[test]
fn markdown_entity_e_code_lt() {
    let result = render_pod_to_markdown("E<lt>");
    assert_eq!(result, "<");
}

#[test]
fn markdown_entity_e_code_gt() {
    let result = render_pod_to_markdown("E<gt>");
    assert_eq!(result, ">");
}

#[test]
fn markdown_entity_e_code_amp() {
    let result = render_pod_to_markdown("E<amp>");
    assert_eq!(result, "&");
}

#[test]
fn markdown_entity_e_code_verbar() {
    let result = render_pod_to_markdown("E<verbar>");
    assert_eq!(result, "|");
}

#[test]
fn markdown_entity_e_code_sol() {
    let result = render_pod_to_markdown("E<sol>");
    assert_eq!(result, "/");
}

#[test]
fn markdown_entity_e_code_unknown_passthrough() {
    // Unknown entity names are left as-is
    let result = render_pod_to_markdown("E<unknown>");
    assert_eq!(result, "E<unknown>");
}

#[test]
fn markdown_nested_b_i() {
    // B<I<text>> → **_text_**
    let result = render_pod_to_markdown("B<I<text>>");
    assert_eq!(result, "**_text_**");
}

#[test]
fn markdown_mixed_inline() {
    let result = render_pod_to_markdown("Use B<new> to create a C<Foo> object");
    assert_eq!(result, "Use **new** to create a `Foo` object");
}

#[test]
fn markdown_plain_text_passthrough() {
    let result = render_pod_to_markdown("plain text here");
    assert_eq!(result, "plain text here");
}

#[test]
fn markdown_head1_to_h2() {
    let source = "=head1 NAME\n\nFoo - example\n\n=cut\n";
    let result = render_pod_to_markdown(source);
    assert!(result.contains("## NAME"), "head1 should become ## heading");
    assert!(result.contains("Foo - example"));
}

#[test]
fn markdown_head2_to_h3() {
    let source = "=head2 new\n\nCreates a new instance.\n\n=cut\n";
    let result = render_pod_to_markdown(source);
    assert!(result.contains("### new"), "head2 should become ### heading");
    assert!(result.contains("Creates a new instance."));
}

#[test]
fn markdown_over_item_back_to_list() {
    let source =
        "=over 4\n\n=item B<verbose>\n\nEnable verbose.\n\n=item B<quiet>\n\nSuppress.\n\n=back\n";
    let result = render_pod_to_markdown(source);
    assert!(
        result.contains("- **verbose**"),
        "items should become markdown bullets with formatting"
    );
    assert!(result.contains("- **quiet**"));
}

#[test]
fn markdown_verbatim_code_block() {
    // Lines indented with whitespace are verbatim (code) blocks
    let source = "=head1 SYNOPSIS\n\n    use Foo;\n    my $obj = Foo->new();\n\n=cut\n";
    let result = render_pod_to_markdown(source);
    assert!(result.contains("```"), "verbatim blocks should become fenced code blocks");
    assert!(result.contains("use Foo;"));
}

#[test]
fn markdown_full_pod_document() {
    let source = r#"=head1 NAME

DateTime::Format::Custom - Parse and format dates

=head1 SYNOPSIS

    use DateTime::Format::Custom;
    my $dt = DateTime::Format::Custom->parse("2024-01-01");

=head1 DESCRIPTION

This module provides custom date parsing.

=head2 parse

Parses a date string and returns a L<DateTime> object.

=head2 format

Formats a B<DateTime> object as a string.

=cut
"#;
    let result = render_pod_to_markdown(source);
    assert!(result.contains("## NAME"));
    assert!(result.contains("DateTime::Format::Custom - Parse and format dates"));
    assert!(result.contains("## SYNOPSIS"));
    assert!(result.contains("```"));
    assert!(result.contains("## DESCRIPTION"));
    assert!(result.contains("### parse"));
    assert!(result.contains("[DateTime](DateTime)"));
    assert!(result.contains("### format"));
    assert!(result.contains("**DateTime**"));
}

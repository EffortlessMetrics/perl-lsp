use perl_pod::{extract_pod, extract_pod_from_file};
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
    assert!(
        doc.synopsis
            .as_ref()
            .is_some_and(|s| s.contains("use Foo::Bar"))
    );
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
    assert!(
        name.contains("perl-module://Module::Name/method"),
        "got: {name}"
    );
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
    assert_eq!(
        doc.name.as_deref(),
        Some("DateTime::Format::Custom - Parse and format dates")
    );
    assert!(
        doc.synopsis
            .as_ref()
            .is_some_and(|s| s.contains("use DateTime::Format::Custom"))
    );
    assert!(
        doc.description
            .as_ref()
            .is_some_and(|s| s.contains("custom date parsing"))
    );
    // Description should only be first paragraph
    assert!(
        !doc.description
            .as_ref()
            .is_none_or(|s| s.contains("auto-detect"))
    );
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
    assert!(
        name.contains("[the docs]"),
        "expected '[the docs]' but got: {name}"
    );
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
    assert!(
        name.contains("[File::Find]"),
        "expected '[File::Find]' but got: {name}"
    );
    // Spaces must be encoded — a raw space makes the markdown URL malformed
    assert!(
        name.contains("perl-module://File::Find/The%20wanted%20function"),
        "expected percent-encoded URL but got: {name}"
    );
    assert!(
        !name.contains("The wanted function"),
        "raw space in URL — should be encoded: {name}"
    );
}

/// `L<click here|Module/Section With Spaces>` — pipe form with spaces in section.
#[test]
fn link_pipe_with_spaced_section_encodes_url() {
    let doc = extract_pod("=head1 NAME\n\nL<click here|File::Find/The wanted function>\n\n=cut\n");
    let name = doc.name.as_deref().unwrap_or("");
    assert!(
        name.contains("[click here]"),
        "expected '[click here]' but got: {name}"
    );
    assert!(
        name.contains("perl-module://File::Find/The%20wanted%20function"),
        "expected percent-encoded URL but got: {name}"
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

//! BDD-style behavior specifications for `perl-pod`.
//!
//! These tests describe crate behavior from a consumer point of view:
//! extracting module-level sections, method docs, lists, and inline formatting.

use perl_pod::{extract_pod, extract_pod_from_file};
use std::fs;
use std::io;

#[test]
fn when_source_has_no_pod_then_extraction_returns_empty_doc() {
    let source = r#"
package Foo::Bar;
use strict;
sub run { 1 }
1;
"#;

    let doc = extract_pod(source);

    assert!(doc.is_empty());
}

#[test]
fn when_name_synopsis_and_description_are_present_then_they_are_extracted() {
    let source = r#"
=head1 NAME

Sample::Module - Demonstrates extraction

=head1 SYNOPSIS

    use Sample::Module;
    my $obj = Sample::Module->new();

=head1 DESCRIPTION

This module provides a tiny POD fixture.

Additional detail that should not appear in description.

=cut
"#;

    let doc = extract_pod(source);

    assert_eq!(doc.name.as_deref(), Some("Sample::Module - Demonstrates extraction"));
    assert!(doc.synopsis.as_ref().is_some_and(|text| text.contains("use Sample::Module")));
    assert_eq!(doc.description.as_deref(), Some("This module provides a tiny POD fixture."));
}

#[test]
fn when_head2_sections_are_present_then_each_method_doc_is_indexed_by_heading() {
    let source = r#"
=head2 new

Constructs the object.

=head2 process

Processes user input.

=cut
"#;

    let doc = extract_pod(source);

    assert_eq!(doc.methods.len(), 2);
    assert!(doc.methods.contains_key("new"));
    assert!(doc.methods.contains_key("process"));
    assert!(doc.methods["new"].contains("Constructs the object."));
    assert!(doc.methods["process"].contains("Processes user input."));
}

#[test]
fn when_lists_are_used_in_method_docs_then_items_are_normalized_to_bullets() {
    let source = r#"
=head2 configure

Supported settings:

=over 4

=item B<verbose>

Emit extra logs.

=item I<quiet>

Suppress logs.

=back

=cut
"#;

    let doc = extract_pod(source);
    let method_doc = &doc.methods["configure"];

    assert!(method_doc.contains("Supported settings:"));
    assert!(method_doc.contains("- verbose"));
    assert!(method_doc.contains("- quiet"));
}

#[test]
fn when_inline_formatting_is_present_then_visible_text_is_preserved() {
    let source = r#"
=head1 NAME

Use B<new> with C<Sample::Module> and L<the docs|Sample::Module>.

=cut
"#;

    let doc = extract_pod(source);

    assert_eq!(doc.name.as_deref(), Some("Use new with Sample::Module and the docs."));
}

#[test]
fn when_multiple_pod_blocks_exist_then_later_sections_can_extend_the_same_document() {
    let source = r#"
package Multi::Block;

=head1 NAME

Multi::Block - Demonstrates multi-block POD

=cut

sub helper { 1 }

=head2 run

Runs the main entry point.

=cut
"#;

    let doc = extract_pod(source);

    assert_eq!(doc.name.as_deref(), Some("Multi::Block - Demonstrates multi-block POD"));
    assert!(doc.methods.contains_key("run"));
}

#[test]
fn when_pod_reaches_eof_without_cut_then_last_section_is_still_flushed() {
    let source = "=head1 NAME\n\nNo::Cut - POD continues to EOF\n";

    let doc = extract_pod(source);

    assert_eq!(doc.name.as_deref(), Some("No::Cut - POD continues to EOF"));
}

#[test]
fn when_loading_pod_from_file_then_file_contents_are_parsed() -> io::Result<()> {
    let unique = format!(
        "perl_pod_behavior_spec_{}_{}.pm",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    );
    let path = std::env::temp_dir().join(unique);

    let source = "=head1 NAME\n\nFrom::File - Parsed from disk\n\n=cut\n";
    fs::write(&path, source)?;

    let parsed = extract_pod_from_file(&path);
    let _ = fs::remove_file(&path);

    let doc = parsed?;
    assert_eq!(doc.name.as_deref(), Some("From::File - Parsed from disk"));

    Ok(())
}

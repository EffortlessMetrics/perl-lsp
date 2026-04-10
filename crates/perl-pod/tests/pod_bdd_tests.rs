use perl_pod::{extract_pod, extract_pod_from_file};
use std::error::Error;
use std::fs;

#[test]
fn scenario_extracts_core_sections_from_documented_module() {
    // Given: a Perl module with NAME, SYNOPSIS, and DESCRIPTION POD sections.
    let source = r#"
package Billing::Invoice;

=head1 NAME

Billing::Invoice - Invoice model

=head1 SYNOPSIS

    use Billing::Invoice;

=head1 DESCRIPTION

Represents invoice state and lifecycle transitions.

Includes helper behavior for tax handling.

=cut
"#;

    // When: POD extraction runs.
    let doc = extract_pod(source);

    // Then: known top-level sections are mapped into PodDoc.
    assert_eq!(doc.name.as_deref(), Some("Billing::Invoice - Invoice model"));
    assert!(doc.synopsis.as_deref().is_some_and(|text| text.contains("use Billing::Invoice;")));
    assert_eq!(
        doc.description.as_deref(),
        Some("Represents invoice state and lifecycle transitions.")
    );
}

#[test]
fn scenario_extracts_method_docs_and_list_items_for_hover_content() {
    // Given: method docs that include POD list directives.
    let source = r#"
=head2 validate

Ensures payload fields are complete.

=over 4

=item B<amount>

Must be positive.

=item B<currency>

Must be an ISO code.

=back

=cut
"#;

    // When: POD extraction runs.
    let doc = extract_pod(source);

    // Then: method docs are captured and list items are normalized as bullets.
    assert!(doc.methods.contains_key("validate"));
    let method_doc = &doc.methods["validate"];
    assert!(method_doc.contains("Ensures payload fields are complete."));
    assert!(method_doc.contains("- amount"));
    assert!(method_doc.contains("- currency"));
}

#[test]
fn scenario_strips_inline_formatting_codes_across_sections() {
    // Given: inline POD formatting used in section content.
    let source = r#"
=head1 NAME

B<Billing::Invoice> - C<normalize> and I<present>

=head2 normalize

Converts L<Invoice::DTO> into L<DTO docs|Invoice::DTO/overview>.

=cut
"#;

    // When: POD extraction runs.
    let doc = extract_pod(source);

    // Then: formatting markers are removed but human-readable text remains.
    assert_eq!(doc.name.as_deref(), Some("Billing::Invoice - normalize and present"));
    assert_eq!(
        doc.methods.get("normalize").map(String::as_str),
        Some("Converts Invoice::DTO into DTO docs.")
    );
}

#[test]
fn scenario_ignores_non_target_head1_sections_without_noise() {
    // Given: documentation that only contains unsupported head1 headings.
    let source = r#"
=head1 AUTHOR

Team Billing

=head1 LICENSE

Same terms as Perl itself.

=cut
"#;

    // When: POD extraction runs.
    let doc = extract_pod(source);

    // Then: no top-level docs or method docs are produced.
    assert!(doc.is_empty());
}

#[test]
fn scenario_merges_multiple_pod_blocks_in_single_file() {
    // Given: two POD blocks in one source file.
    let source = r#"
=head1 NAME

Billing::Invoice - multi block module

=cut

sub helper { 1 }

=head2 summarize

Creates one-line invoice summaries.

=cut
"#;

    // When: POD extraction runs.
    let doc = extract_pod(source);

    // Then: earlier section data and later method docs are both preserved.
    assert_eq!(doc.name.as_deref(), Some("Billing::Invoice - multi block module"));
    assert_eq!(
        doc.methods.get("summarize").map(String::as_str),
        Some("Creates one-line invoice summaries.")
    );
}

#[test]
fn scenario_reads_and_extracts_pod_from_file_path() -> Result<(), Box<dyn Error>> {
    // Given: a temporary Perl module file on disk.
    let unique = format!(
        "perl_pod_bdd_{}_{}.pm",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos()
    );
    let path = std::env::temp_dir().join(unique);

    let source = r#"
=head1 NAME

Billing::Invoice::Disk - from file

=cut
"#;

    fs::write(&path, source)?;

    // When: extraction runs against a file path.
    let doc = extract_pod_from_file(&path)?;

    // Then: the extracted model contains NAME content from disk.
    assert_eq!(doc.name.as_deref(), Some("Billing::Invoice::Disk - from file"));

    fs::remove_file(path)?;

    Ok(())
}

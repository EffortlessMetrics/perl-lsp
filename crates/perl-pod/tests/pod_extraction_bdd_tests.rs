use perl_pod::extract_pod;

fn given_pod_source_when_extracting_then_name_is(source: &str, expected_name: &str) {
    let doc = extract_pod(source);
    assert_eq!(doc.name.as_deref(), Some(expected_name));
}

#[test]
fn scenario_extracts_core_sections_from_realistic_module() {
    // Given
    let source = r#"
package Payment::Gateway;

=head1 NAME

Payment::Gateway - Process customer payments

=head1 SYNOPSIS

    my $gateway = Payment::Gateway->new();

=head1 DESCRIPTION

Gateway wrapper for multiple processors.

Supports retries and request tracing.

=head2 charge

Charges a customer card.

=cut

1;
"#;

    // When
    let doc = extract_pod(source);

    // Then
    assert_eq!(doc.name.as_deref(), Some("Payment::Gateway - Process customer payments"));
    assert!(doc.synopsis.as_ref().is_some_and(|s| s.contains("Payment::Gateway->new")));
    assert_eq!(doc.description.as_deref(), Some("Gateway wrapper for multiple processors."));
    assert!(doc.methods.get("charge").is_some_and(|m| m.contains("Charges a customer card")));
}

#[test]
fn scenario_ignores_perl_code_until_pod_starts() {
    // Given
    let source = r#"
package Inventory;

sub add_item { }

=head1 NAME

Inventory - Tracks stock

=cut
"#;

    // When / Then
    given_pod_source_when_extracting_then_name_is(source, "Inventory - Tracks stock");
}

#[test]
fn scenario_formats_nested_inline_markup_inside_method_docs() {
    // Given
    let source = r#"
=head2 connect

Use B<I<secure>> mode with C<TLSv1.3>.

=cut
"#;

    // When
    let doc = extract_pod(source);

    // Then
    assert!(
        doc.methods.get("connect").is_some_and(|m| m.contains("Use secure mode with TLSv1.3."))
    );
}

#[test]
fn scenario_list_items_render_as_bullets_in_section_body() {
    // Given
    let source = r#"
=head2 flags

=over 4

=item B<strict>

Abort on first error.

=item B<verbose>

Include trace output.

=back

=cut
"#;

    // When
    let doc = extract_pod(source);

    // Then
    assert!(doc.methods.get("flags").is_some_and(|flags| flags.contains("- strict")));
    assert!(doc.methods.get("flags").is_some_and(|flags| flags.contains("- verbose")));
    assert!(doc.methods.get("flags").is_some_and(|flags| flags.contains("Abort on first error")));
    assert!(doc.methods.get("flags").is_some_and(|flags| flags.contains("Include trace output")));
}

#[test]
fn scenario_latest_name_wins_across_multiple_pod_blocks() {
    // Given
    let source = r#"
=head1 NAME

First::Name - Initial

=cut

package First::Name;

=head1 NAME

First::Name - Final

=cut
"#;

    // When / Then
    given_pod_source_when_extracting_then_name_is(source, "First::Name - Final");
}

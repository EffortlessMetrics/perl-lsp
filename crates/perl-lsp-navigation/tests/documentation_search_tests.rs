//! Tests for documentation search functionality (textDocument/documentationSearch)
//!
//! These tests define the expected behavior for workspace documentation search
//! across POD (Plain Old Documentation) files in Perl modules.
//!
//! The `DocumentationSearchProvider` should:
//! - Index POD documentation extracted from Perl source files
//! - Search across name, synopsis, description, and method fields
//! - Support scope filtering (search specific POD sections)
//! - Return results with module name, matched section, and excerpt

use perl_lsp_navigation::{DocumentationSearchProvider, DocumentationSearchScope};

// =============================================================================
// TESTS
// =============================================================================

#[test]
fn test_index_document_extracts_pod_name() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package My::Module;

=head1 NAME

My::Module - A wonderful module

=cut

sub new { }
"#;

    provider.index_document("file:///lib/My/Module.pm", source);

    // Search for the module name
    let results = provider.search("My::Module", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Expected to find documentation for 'My::Module' in NAME section");
    assert_eq!(results[0].module, "My::Module - A wonderful module");
}

#[test]
fn test_index_document_extracts_pod_synopsis() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package My::Module;

=head1 NAME

My::Module - A wonderful module

=head1 SYNOPSIS

    use My::Module;
    my $obj = My::Module->new();

=cut

sub new { }
"#;

    provider.index_document("file:///lib/My/Module.pm", source);

    // Search in synopsis
    let results = provider.search("My::Module->new", DocumentationSearchScope::Synopsis);
    assert!(!results.is_empty(), "Expected to find 'My::Module->new' in SYNOPSIS section");
}

#[test]
fn test_index_document_extracts_pod_description() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package My::Module;

=head1 NAME

My::Module - A wonderful module

=head1 DESCRIPTION

This module provides wonderful functionality for processing data.
It supports various operations including parsing, transformation, and export.

=cut

sub process { }
"#;

    provider.index_document("file:///lib/My/Module.pm", source);

    // Search in description
    let results = provider.search("processing data", DocumentationSearchScope::Description);
    assert!(!results.is_empty(), "Expected to find 'processing data' in DESCRIPTION section");
}

#[test]
fn test_index_document_extracts_pod_method_documentation() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package My::Module;

=head1 NAME

My::Module - A wonderful module

=head1 DESCRIPTION

This module does wonderful things.

=head2 process

    my $result = $obj->process($input);

The process method transforms the input and returns the result.

=head2 validate

    $obj->validate($data) or die "Invalid data";

Validates the input data structure.

=cut

sub process { }
sub validate { }
"#;

    provider.index_document("file:///lib/My/Module.pm", source);

    // Search for method documentation
    let results = provider.search("transforms the input", DocumentationSearchScope::Methods);
    assert!(!results.is_empty(), "Expected to find method documentation for 'process'");
    assert_eq!(results[0].section, Some("process".to_string()));
}

#[test]
fn test_search_across_all_fields() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Search::Test;

=head1 NAME

Search::Test - Testing search functionality

=head1 SYNOPSIS

    use Search::Test;
    my $obj = Search::Test->new();

=head1 DESCRIPTION

This module provides search capabilities.

=cut
"#;

    provider.index_document("file:///lib/Search/Test.pm", source);

    // Query that might match across different fields
    let results = provider.search("search", DocumentationSearchScope::All);
    assert!(!results.is_empty(), "Expected to find 'search' in at least one field");
}

#[test]
fn test_search_returns_uri_and_module() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Exact::Match;

=head1 NAME

Exact::Match - Exact match test module

=cut
"#;

    let uri = "file:///lib/Exact/Match.pm";
    provider.index_document(uri, source);

    let results = provider.search("Exact::Match", DocumentationSearchScope::All);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].uri, uri);
    assert!(results[0].module.contains("Exact::Match"));
}

#[test]
fn test_search_no_results_returns_empty() {
    let provider = DocumentationSearchProvider::new();

    let results = provider.search("nonexistent term xyz123", DocumentationSearchScope::All);
    assert!(results.is_empty(), "Expected no results for nonexistent search term");
}

#[test]
fn test_remove_document() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Remove::Me;

=head1 NAME

Remove::Me - This will be removed

=cut
"#;

    let uri = "file:///lib/Remove/Me.pm";
    provider.index_document(uri, source);
    assert_eq!(provider.document_count(), 1);

    provider.remove_document(uri);
    assert_eq!(provider.document_count(), 0);

    // Search should return nothing after removal
    let results = provider.search("Remove::Me", DocumentationSearchScope::All);
    assert!(results.is_empty(), "Expected no results after document removal");
}

#[test]
fn test_multiple_documents_indexed() {
    let mut provider = DocumentationSearchProvider::new();

    let source1 = r#"
package Module::One;

=head1 NAME

Module::One - First module

=cut
"#;

    let source2 = r#"
package Module::Two;

=head1 NAME

Module::Two - Second module

=cut
"#;

    provider.index_document("file:///lib/Module/One.pm", source1);
    provider.index_document("file:///lib/Module/Two.pm", source2);

    assert_eq!(provider.document_count(), 2);

    let results = provider.search("module", DocumentationSearchScope::All);
    // Should find matches in both modules
    assert_eq!(results.len(), 2);
}

#[test]
fn test_search_scope_name_only() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Scope::Test;

=head1 NAME

Scope::Test - Testing scope filter

=head1 SYNOPSIS

    This text should not be found when searching NAME only.

=cut
"#;

    provider.index_document("file:///lib/Scope/Test.pm", source);

    // Search for something in SYNOPSIS but with NAME scope only
    let results = provider.search("This text", DocumentationSearchScope::Name);
    assert!(results.is_empty(), "Should not find 'This text' when searching NAME scope only");

    // Search for module name with NAME scope
    let results = provider.search("Scope::Test", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should find 'Scope::Test' when searching NAME scope");
}

#[test]
fn test_search_is_case_insensitive() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Case::Test;

=head1 NAME

Case::Test - Testing case insensitivity

=cut
"#;

    provider.index_document("file:///lib/Case/Test.pm", source);

    let results_upper = provider.search("CASE::TEST", DocumentationSearchScope::Name);
    let results_lower = provider.search("case::test", DocumentationSearchScope::Name);
    let results_mixed = provider.search("CaSe::TeSt", DocumentationSearchScope::Name);

    assert!(!results_upper.is_empty(), "Uppercase query should match");
    assert!(!results_lower.is_empty(), "Lowercase query should match");
    assert!(!results_mixed.is_empty(), "Mixed case query should match");
}

#[test]
fn test_empty_pod_document() {
    let mut provider = DocumentationSearchProvider::new();
    // Document with no POD
    let source = r#"
package No::Pod;

sub new { }

sub method {
    return 42;
}
"#;

    provider.index_document("file:///lib/No/Pod.pm", source);

    // Should not crash, should return empty results
    let results = provider.search("anything", DocumentationSearchScope::All);
    assert!(results.is_empty(), "Document with no POD should return no results");
    assert_eq!(provider.document_count(), 1); // Document is still indexed
}

#[test]
fn test_pod_with_multiple_methods() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Multi::Method;

=head1 NAME

Multi::Method - Multiple methods test

=head2 first_method

Documentation for first_method.

=head2 second_method

Documentation for second_method.

=head2 third_method

Documentation for third_method.

=cut

sub first_method { }
sub second_method { }
sub third_method { }
"#;

    provider.index_document("file:///lib/Multi/Method.pm", source);

    // Search for specific method
    let results = provider.search("second_method", DocumentationSearchScope::Methods);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].section, Some("second_method".to_string()));

    // Search for common text in multiple methods
    let results = provider.search("Documentation for", DocumentationSearchScope::Methods);
    assert_eq!(results.len(), 3, "Should find matches in all three methods");
}

#[test]
fn test_update_existing_document() {
    let mut provider = DocumentationSearchProvider::new();
    let source_v1 = r#"
package Update::Test;

=head1 NAME

Update::Test - Original version

=cut
"#;

    let source_v2 = r#"
package Update::Test;

=head1 NAME

Update::Test - Updated version

=cut
"#;

    provider.index_document("file:///lib/Update/Test.pm", source_v1);
    assert_eq!(provider.document_count(), 1);

    // Update the document
    provider.index_document("file:///lib/Update/Test.pm", source_v2);
    assert_eq!(provider.document_count(), 1, "Should still be 1 document after update");

    // Old content should not be found
    let results = provider.search("Original", DocumentationSearchScope::Name);
    assert!(results.is_empty(), "Old content should not be found after update");

    // New content should be found
    let results = provider.search("Updated", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "New content should be found after update");
}

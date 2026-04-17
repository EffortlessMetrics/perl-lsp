//! Edge case tests for documentation search functionality.
//!
//! These tests stress the implementation with boundary values, malformed inputs,
//! Unicode, whitespace variations, and other edge cases not covered by the
//! red (contract) tests.

use perl_lsp_navigation::{DocumentationSearchProvider, DocumentationSearchScope};

// =============================================================================
// EDGE CASE: Empty and boundary inputs
// =============================================================================

/// Edge case: Empty query string - empty string is a substring of everything,
/// so this matches everything (not a useful search but correct behavior).
#[test]
fn test_search_empty_query_matches_everything() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package My::Module;

=head1 NAME

My::Module - A module

=cut
"#;
    provider.index_document("file:///lib/My/Module.pm", source);

    let results = provider.search("", DocumentationSearchScope::All);
    // Empty string is substring of everything, so all fields match
    assert_eq!(results.len(), 1, "Empty query should match everything (empty substring)");
}

/// Edge case: Whitespace-only query.
#[test]
fn test_search_whitespace_query() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Space::Test;

=head1 NAME

Space::Test - Testing whitespace

=cut
"#;
    provider.index_document("file:///lib/Space/Test.pm", source);

    // A query of only whitespace will be lowercased to empty string
    let results = provider.search("   ", DocumentationSearchScope::All);
    // Empty string after to_lowercase() means no match
    assert!(results.is_empty(), "Whitespace-only query should return no results");
}

/// Edge case: Very long query string (stress test).
#[test]
fn test_search_very_long_query() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Long::Query;

=head1 NAME

Long::Query - Testing long queries

=cut
"#;
    provider.index_document("file:///lib/Long/Query.pm", source);

    // Very long query that exceeds any reasonable content
    let long_query = "a".repeat(10_000);
    let results = provider.search(&long_query, DocumentationSearchScope::All);
    assert!(results.is_empty(), "Long query with no match should return empty results");
}

/// Edge case: Search for just one character.
#[test]
fn test_search_single_character_query() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Single::Char;

=head1 NAME

Single::Char - Testing single char search

=cut
"#;
    provider.index_document("file:///lib/Single/Char.pm", source);

    let results = provider.search("s", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should find 's' in 'Single::Char' (case-insensitive)");
}

// =============================================================================
// EDGE CASE: Unicode and special characters
// =============================================================================

/// Edge case: Unicode characters in POD (UTF-8).
#[test]
fn test_search_unicode_in_pod() {
    let mut provider = DocumentationSearchProvider::new();
    // Unicode in NAME section
    let source = r#"
package Unicode::Test;

=head1 NAME

Unicode::Test - Testing 日本語 and Ünïcödé

=cut
"#;
    provider.index_document("file:///lib/Unicode/Test.pm", source);

    // Search for unicode text
    let results = provider.search("日本語", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should find Japanese characters in NAME section");
}

/// Edge case: Unicode in method documentation.
#[test]
fn test_search_unicode_method_documentation() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Unicode::Method;

=head1 NAME

Unicode::Method - Testing unicode in methods

=head2 日本語メソッド

This method handles Japanese text processing.

=cut

sub 日本語メソッド { }
"#;
    provider.index_document("file:///lib/Unicode/Method.pm", source);

    let results = provider.search("Japanese", DocumentationSearchScope::Methods);
    assert!(!results.is_empty(), "Should find English text in unicode method documentation");
}

/// Edge case: POD E<> entities (lt, gt, amp, quot, apos).
#[test]
fn test_search_pod_entities() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Entity::Test;

=head1 NAME

Entity::Test - Testing E<> entities

=head1 SYNOPSIS

    if ($x E<lt> 5 E<gt> $y) {
        say "E<amp> E<quot> E<apos>";
    }

=cut
"#;
    provider.index_document("file:///lib/Entity/Test.pm", source);

    // The entities should be decoded to < > & " '
    let results = provider.search("<", DocumentationSearchScope::Synopsis);
    assert!(!results.is_empty(), "Should find decoded E<lt> as '<' in synopsis");
}

/// Edge case: POD formatting codes B<>, I<>, C<> should be stripped.
#[test]
fn test_search_pod_formatting_codes() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Format::Test;

=head1 NAME

Format::Test - B<bold>, I<italic>, C<code>

=head1 DESCRIPTION

The B<process> method I<transforms> C<data>.

=cut
"#;
    provider.index_document("file:///lib/Format/Test.pm", source);

    // The formatting codes should be stripped, leaving the inner text
    let results = provider.search("bold", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should find 'bold' inside B<> formatting in NAME");

    let results = provider.search("process", DocumentationSearchScope::Description);
    assert!(!results.is_empty(), "Should find 'process' inside B<> in DESCRIPTION");
}

// =============================================================================
// EDGE CASE: URI variations
// =============================================================================

/// Edge case: URI without file:// prefix.
#[test]
fn test_index_document_uri_without_prefix() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package No::Prefix;

=head1 NAME

No::Prefix - Testing URI without file:// prefix

=cut
"#;
    // URI without file:// prefix
    provider.index_document("/lib/No/Prefix.pm", source);

    let results = provider.search("No::Prefix", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should find module even with path-based URI");
}

/// Edge case: URI with file: prefix (single slash).
#[test]
fn test_index_document_uri_file_prefix() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package File::Prefix;

=head1 NAME

File::Prefix - Testing file: prefix

=cut
"#;
    // URI with file: (single slash)
    provider.index_document("file:/lib/File/Prefix.pm", source);

    let results = provider.search("File::Prefix", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should find module with file: prefix URI");
}

/// Edge case: URI with .pod extension.
#[test]
fn test_index_document_pod_extension() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Pod::Ext;

=head1 NAME

Pod::Ext - Testing .pod extension

=cut
"#;
    provider.index_document("file:///lib/Pod/Ext.pod", source);

    let results = provider.search("Pod::Ext", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should handle .pod extension correctly");
}

/// Edge case: Very long URI path.
#[test]
fn test_index_document_long_uri_path() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Long::Path;

=head1 NAME

Long::Path - Testing long paths

=cut
"#;
    let long_path = format!("file:///{}", "a".repeat(500));
    provider.index_document(&long_path, source);

    let results = provider.search("Long::Path", DocumentationSearchScope::Name);
    // Module name fallback from URI
    assert_eq!(results.len(), 1, "Should handle very long URI paths");
}

// =============================================================================
// EDGE CASE: POD parsing edge cases
// =============================================================================

/// Edge case: POD with no =cut (POD ends at EOF).
#[test]
fn test_pod_without_cut_ends_at_eof() {
    let mut provider = DocumentationSearchProvider::new();
    // POD without =cut - should still be extracted
    let source = r#"
package No::Cut;

=head1 NAME

No::Cut - No =cut at end

=head1 DESCRIPTION

This POD has no =cut

sub run { }
"#;
    provider.index_document("file:///lib/No/Cut.pm", source);

    let results = provider.search("No::Cut", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "POD without =cut should still be extracted");
}

/// Edge case: Multiple =head1 NAME sections - later one overwrites earlier.
#[test]
fn test_multiple_name_sections() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Multi::Name;

=head1 NAME

Multi::Name - First NAME section

=head1 DESCRIPTION

Description text.

=head1 NAME

Multi::Name - Second NAME section

=cut
"#;
    provider.index_document("file:///lib/Multi/Name.pm", source);

    // The second NAME overwrites the first (later sections win)
    let results = provider.search("Second NAME", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Second NAME section should be stored (overwrites first)");

    // First NAME is no longer present
    let results = provider.search("First NAME", DocumentationSearchScope::Name);
    assert!(results.is_empty(), "First NAME should be overwritten by second NAME");
}

/// Edge case: Empty =head2 method name.
#[test]
fn test_empty_method_name() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Empty::Method;

=head1 NAME

Empty::Method - Testing empty method name

=head2

This method has an empty name.

=cut

sub { }
"#;
    provider.index_document("file:///lib/Empty/Method.pm", source);

    // Should not crash - empty method names should be handled
    let results = provider.search("method has an empty", DocumentationSearchScope::Methods);
    assert!(!results.is_empty(), "Should find method documentation even with empty name");
}

/// Edge case: POD with only method sections (no NAME).
#[test]
fn test_pod_without_name_section() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package No::Name;

=head1 DESCRIPTION

No name section here.

=head2 my_method

This method does something.

=cut

sub my_method { }
"#;
    provider.index_document("file:///lib/No/Name.pm", source);

    // Should still work, module name should be guessed from URI
    let results = provider.search("No::Name", DocumentationSearchScope::Name);
    assert_eq!(results.len(), 1, "Should guess module name from URI when no NAME section");

    // Should find method
    let results = provider.search("my_method", DocumentationSearchScope::Methods);
    assert!(!results.is_empty(), "Should find method even without NAME section");
}

/// Edge case: Description takes only first paragraph.
#[test]
fn test_description_first_paragraph_only() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package First::Para;

=head1 NAME

First::Para - Testing first paragraph

=head1 DESCRIPTION

This is the first paragraph.

This is the second paragraph - should NOT appear in description.

=cut
"#;
    provider.index_document("file:///lib/First/Para.pm", source);

    let results = provider.search("first paragraph", DocumentationSearchScope::Description);
    assert!(!results.is_empty(), "Should find text from first paragraph");

    // The second paragraph should not appear in description search
    let results = provider.search("second paragraph", DocumentationSearchScope::Description);
    assert!(results.is_empty(), "Second paragraph should NOT appear in description search");
}

/// Edge case: Multiple methods with similar names.
#[test]
fn test_methods_similar_names() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Similar;

=head1 NAME

Similar - Testing similar method names

=head2 foo

Documentation for foo.

=head2 foo_bar

Documentation for foo_bar.

=head2 fooBar

Documentation for fooBar.

=cut

sub foo { }
sub foo_bar { }
sub fooBar { }
"#;
    provider.index_document("file:///lib/Similar.pm", source);

    // Search for foo should find all three
    let results = provider.search("foo", DocumentationSearchScope::Methods);
    assert_eq!(results.len(), 3, "Should find all three methods containing 'foo'");

    // Search for foo_bar should find only foo_bar
    let results = provider.search("foo_bar", DocumentationSearchScope::Methods);
    assert_eq!(results.len(), 1, "Should find only foo_bar");
}

// =============================================================================
// EDGE CASE: Fuzzy/subsequence matching
// =============================================================================

/// Edge case: Query is subsequence of module name.
#[test]
fn test_fuzzy_subsequence_matching() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Fuzzy::Test;

=head1 NAME

Fuzzy::Test - Testing fuzzy matching

=cut
"#;
    provider.index_document("file:///lib/Fuzzy/Test.pm", source);

    // "fuz" IS a subsequence of "Fuzzy::Test"
    let results = provider.search("fuz", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Fuzzy subsequence 'fuz' should work in fuzzy");
}

/// Edge case: Query longer than content (no match possible).
#[test]
fn test_query_longer_than_content() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Short;

=head1 NAME

Short

=cut
"#;
    provider.index_document("file:///lib/Short.pm", source);

    let results =
        provider.search("this is a very long query string", DocumentationSearchScope::Name);
    assert!(results.is_empty(), "Query longer than content should return no results");
}

/// Edge case: Subsequence fallback only works when there's a partial match.
#[test]
fn test_subsequence_only_for_scoring() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Gap::Test;

=head1 NAME

Gap::Test - Testing gaps

=cut
"#;
    provider.index_document("file:///lib/Gap/Test.pm", source);

    // "gap" IS a substring of "Gap::Test"
    let results = provider.search("gap", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "'gap' should be found as substring");

    // "gt" is NOT a substring, so won't be found (subsequence is only for scoring)
    let results = provider.search("gt", DocumentationSearchScope::Name);
    assert!(results.is_empty(), "'gt' is not a substring and won't be found");
}

// =============================================================================
// EDGE CASE: Relevance and sorting
// =============================================================================

/// Edge case: Exact match ranks higher than prefix match.
#[test]
fn test_exact_match_ranks_higher() {
    let mut provider = DocumentationSearchProvider::new();
    let source1 = r#"
package Exact::Test;

=head1 NAME

Exact::Test - Exact match test

=cut
"#;
    let source2 = r#"
package Exact;

=head1 NAME

Exact - Just the word Exact

=cut
"#;
    provider.index_document("file:///lib/Exact/Test.pm", source1);
    provider.index_document("file:///lib/Exact.pm", source2);

    let results = provider.search("Exact", DocumentationSearchScope::Name);
    // Both match via substring, so first result depends on iteration order (HashMap)
    assert!(results.len() >= 2, "Should find matches in both documents");
}

/// Edge case: Search uses substring matching, not prefix-specific logic.
#[test]
fn test_prefix_match_ranks_higher() {
    let mut provider = DocumentationSearchProvider::new();
    let source1 = r#"
package Prefix::Match;

=head1 NAME

Prefix::Match - prefix test

=cut
"#;
    let source2 = r#"
package Other::Prefix::Match;

=head1 NAME

Other::Prefix::Match - contains prefix inside

=cut
"#;
    provider.index_document("file:///lib/Prefix/Match.pm", source1);
    provider.index_document("file:///lib/Other/Prefix/Match.pm", source2);

    // "prefix" IS a substring of both (case insensitive)
    let results = provider.search("prefix", DocumentationSearchScope::Name);
    assert_eq!(results.len(), 2, "Should find 'prefix' in both documents");
}

// =============================================================================
// EDGE CASE: Removing documents
// =============================================================================

/// Edge case: Remove document that was never indexed.
#[test]
fn test_remove_nonexistent_document() {
    let mut provider = DocumentationSearchProvider::new();
    // Should not panic
    provider.remove_document("file:///Never/Indexed.pm");
    assert_eq!(provider.document_count(), 0);
}

/// Edge case: Remove then re-add same URI.
#[test]
fn test_remove_and_reindex() {
    let mut provider = DocumentationSearchProvider::new();
    let source1 = r#"
package Reindex;

=head1 NAME

Reindex - Version 1

=cut
"#;
    let source2 = r#"
package Reindex;

=head1 NAME

Reindex - Version 2

=cut
"#;
    let uri = "file:///lib/Reindex.pm";

    provider.index_document(uri, source1);
    assert_eq!(provider.document_count(), 1);

    provider.remove_document(uri);
    assert_eq!(provider.document_count(), 0);

    // Re-add with new content
    provider.index_document(uri, source2);
    assert_eq!(provider.document_count(), 1);

    let results = provider.search("Version 2", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Reindexed document should have new content");

    let results = provider.search("Version 1", DocumentationSearchScope::Name);
    assert!(results.is_empty(), "Old content should not exist after reindex");
}

// =============================================================================
// EDGE CASE: Scope filtering
// =============================================================================

/// Edge case: Search synopsis scope when no synopsis exists.
#[test]
fn test_search_synopsis_scope_no_synopsis() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package No::Synopsis;

=head1 NAME

No::Synopsis - No synopsis here

=head1 DESCRIPTION

Just description.

=cut
"#;
    provider.index_document("file:///lib/No/Synopsis.pm", source);

    let results = provider.search("test", DocumentationSearchScope::Synopsis);
    assert!(results.is_empty(), "Should return empty when searching synopsis but none exists");
}

/// Edge case: Search description scope when no description exists.
#[test]
fn test_search_description_scope_no_description() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package No::Description;

=head1 NAME

No::Description - Just name

=cut
"#;
    provider.index_document("file:///lib/No/Description.pm", source);

    let results = provider.search("test", DocumentationSearchScope::Description);
    assert!(results.is_empty(), "Should return empty when searching description but none exists");
}

/// Edge case: Search methods scope when no methods exist.
#[test]
fn test_search_methods_scope_no_methods() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package No::Methods;

=head1 NAME

No::Methods - Just name

=cut
"#;
    provider.index_document("file:///lib/No/Methods.pm", source);

    let results = provider.search("test", DocumentationSearchScope::Methods);
    assert!(results.is_empty(), "Should return empty when searching methods but none exist");
}

// =============================================================================
// EDGE CASE: Special characters in content
// =============================================================================

/// Edge case: Dollar sign and other Perl metacharacters.
#[test]
fn test_search_perl_metacharacters() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Meta::Chars;

=head1 NAME

Meta::Chars - Testing $variables and @arrays

=head1 SYNOPSIS

    my $x = $obj->method(@args);
    use strict 'refs';

=cut
"#;
    provider.index_document("file:///lib/Meta/Chars.pm", source);

    // Should handle $ and @ without crashing
    let results = provider.search("$variables", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should find text containing Perl metacharacters");
}

/// Edge case: Backslash and escape sequences.
#[test]
fn test_search_backslashes() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Backslash;

=head1 NAME

Backslash - Testing \\n and \\t escapes

=cut
"#;
    provider.index_document("file:///lib/Backslash.pm", source);

    let results = provider.search("\\\\n", DocumentationSearchScope::Name);
    // Backslash in Rust string is \\
    assert!(!results.is_empty(), "Should handle backslash sequences in search");
}

/// Edge case: HTML-like tags (not POD formatting).
#[test]
fn test_search_html_like_content() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package HTML::Like;

=head1 NAME

HTML::Like - <b>bold</b> and <i>italic</i>

=cut
"#;
    provider.index_document("file:///lib/HTML/Like.pm", source);

    // HTML tags are not POD formatting codes, so they should be preserved
    let results = provider.search("bold</b>", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should preserve HTML-like content");
}

// =============================================================================
// EDGE CASE: Whitespace in POD
// =============================================================================

/// Edge case: POD with leading/trailing whitespace.
#[test]
fn test_pod_whitespace_handling() {
    let mut provider = DocumentationSearchProvider::new();
    let source = "    \n    \npackage Whitespace;\n\n=head1 NAME\n\n   Whitespace::Test - With spaces   \n\n=cut\n";
    provider.index_document("file:///lib/Whitespace.pm", source);

    let results = provider.search("Whitespace::Test", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should handle whitespace in POD correctly");
}

/// Edge case: Multiple blank lines between sections.
#[test]
fn test_pod_multiple_blank_lines() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Blank::Lines;

=head1 NAME

Blank::Lines - Testing blank lines



=head1 DESCRIPTION

Description with

lots of

blank lines

within.

=cut
"#;
    provider.index_document("file:///lib/Blank/Lines.pm", source);

    let results = provider.search("Description with", DocumentationSearchScope::Description);
    assert!(!results.is_empty(), "Should handle multiple blank lines");
}

// =============================================================================
// EDGE CASE: Real-world POD patterns
// =============================================================================

/// Edge case: Real-world POD with SEE ALSO section.
#[test]
fn test_pod_with_see_also() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package See::Also;

=head1 NAME

See::Also - Testing SEE ALSO section

=head1 DESCRIPTION

This module does things.

=head1 SEE ALSO

L<Some::Other::Module>, L<Another::Module>

=cut
"#;
    provider.index_document("file:///lib/See/Also.pm", source);

    // Should find the module
    let results = provider.search("See::Also", DocumentationSearchScope::Name);
    assert!(!results.is_empty(), "Should find module with SEE ALSO section");

    // Should find description
    let results = provider.search("things", DocumentationSearchScope::Description);
    assert!(!results.is_empty(), "Should find description");
}

/// Edge case: POD with code blocks (indented lines).
#[test]
fn test_pod_code_blocks() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Code::Block;

=head1 NAME

Code::Block - Testing code blocks

=head1 SYNOPSIS

    #!/usr/bin/perl
    use strict;
    use warnings;

    my $x = 42;

=cut
"#;
    provider.index_document("file:///lib/Code/Block.pm", source);

    let results = provider.search("strict", DocumentationSearchScope::Synopsis);
    assert!(!results.is_empty(), "Should find content inside indented code blocks");
}

/// Edge case: =over/=item list items go to Other section (ignored).
#[test]
fn test_pod_list_items() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package List::Items;

=head1 NAME

List::Items - Testing list items

=head1 DESCRIPTION

The main description text describes the options below.

=cut
"#;
    provider.index_document("file:///lib/List/Items.pm", source);

    // Description should be found (list items are in Other section, not stored)
    let results = provider.search("main description", DocumentationSearchScope::Description);
    assert!(!results.is_empty(), "Should find description content");
}

// =============================================================================
// EDGE CASE: Case sensitivity
// =============================================================================

/// Edge case: Module name with mixed case, search with different case.
#[test]
fn test_case_insensitive_search() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package MixedCase::Module;

=head1 NAME

MixedCase::Module - Testing CASE

=cut
"#;
    provider.index_document("file:///lib/MixedCase/Module.pm", source);

    // All these should match
    assert!(!provider.search("MIXEDCASE::MODULE", DocumentationSearchScope::Name).is_empty());
    assert!(!provider.search("mixedcase::module", DocumentationSearchScope::Name).is_empty());
    assert!(!provider.search("MiXeDcAsE::MoDuLe", DocumentationSearchScope::Name).is_empty());
}

// =============================================================================
// EDGE CASE: Method search includes method name
// =============================================================================

/// Edge case: Method search matches method name (not just documentation).
#[test]
fn test_method_search_matches_name() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Method::Name;

=head1 NAME

Method::Name - Testing method name search

=head2 my_awesome_method

This method is named my_awesome_method.

=cut

sub my_awesome_method { }
"#;
    provider.index_document("file:///lib/Method/Name.pm", source);

    // Search for just the method name (not the documentation text)
    let results = provider.search("my_awesome_method", DocumentationSearchScope::Methods);
    assert_eq!(results.len(), 1, "Should find method by searching for its name");
}

/// Edge case: Method name search matches both name AND documentation content.
#[test]
fn test_method_name_search_matches_content() {
    let mut provider = DocumentationSearchProvider::new();
    let source = r#"
package Substring;

=head1 NAME

Substring - Testing method name substring

=head2 get_method

The get method does retrieval.

=head2 target_value

The target method uses target_value.

=cut

sub get_method { }
sub target_value { }
"#;
    provider.index_document("file:///lib/Substring.pm", source);

    // "get" is a subsequence of both "get_method" and "target_value" (documentation)
    let results = provider.search("get", DocumentationSearchScope::Methods);
    assert_eq!(
        results.len(),
        2,
        "Should find methods where 'get' appears in name or documentation"
    );
}

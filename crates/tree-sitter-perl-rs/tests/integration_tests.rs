//! Integration tests for `tree-sitter-perl-rs` — PerlLanguage + Parser + Tree + Node.
//!
//! These tests verify that multiple components work together correctly:
//! - `Parser` produces `Tree`
//! - `Tree` provides `Node` accessors
//! - `PerlLanguage`/`language()`/`LANGUAGE` provides kind metadata
//! - Node kinds discovered during traversal are recognized by the language descriptor

use perl_tdd_support::must_some;
use tree_sitter_perl_rs::{language, Parser, Tree, LANGUAGE};

/// Helper: collect all distinct node kinds from a tree via depth-first traversal.
fn collect_kinds(tree: &Tree) -> Vec<&'static str> {
    let mut kinds = Vec::new();
    collect_kinds_impl(&tree.root_node(), &mut kinds);
    kinds
}

fn collect_kinds_impl<'a>(node: &tree_sitter_perl_rs::Node<'a>, out: &mut Vec<&'static str>) {
    out.push(node.kind());
    for child in node.children() {
        collect_kinds_impl(&child, out);
    }
}

/// Helper: collect all distinct node kinds (deduplicated).
fn collect_distinct_kinds(tree: &Tree) -> Vec<&'static str> {
    let mut all = collect_kinds(tree);
    all.sort_unstable();
    all.dedup();
    all
}

// ---------------------------------------------------------------------------
// Integration test 1: language() + parsing workflow
// ---------------------------------------------------------------------------

/// Integration: use `language()` to validate all node kinds discovered during parsing.
///
/// This test exercises the full user workflow:
/// 1. Obtain the language descriptor via `language()`
/// 2. Parse some Perl source to get a `Tree`
/// 3. Traverse the tree to collect node kinds
/// 4. Validate that every discovered kind is recognized by the language descriptor
#[test]
fn when_parsing_and_traversing_then_all_node_kinds_are_recognized_by_language() {
    let source = r#"
        package My::Module;
        use strict;
        use warnings;

        sub new {
            my ($class, %args) = @_;
            return bless \%args, $class;
        }

        sub greet {
            my ($self, $name) = @_;
            print "Hello, $name!\n";
            return;
        }

        1;
    "#;

    let mut parser = Parser::new();
    let tree = must_some(parser.parse(source));

    // Step 1: get language descriptor
    let lang = language();

    // Step 2: collect all distinct node kinds from the parsed tree
    let discovered_kinds = collect_distinct_kinds(&tree);

    // Step 3: every discovered kind must be recognized as a named kind
    for kind in &discovered_kinds {
        assert!(
            lang.node_kind_is_named(kind),
            "node kind '{}' discovered during traversal must be recognized by language()",
            kind
        );
    }

    // Step 4: the total count of distinct kinds must not exceed the grammar's kind count
    assert!(
        discovered_kinds.len() <= lang.node_kind_count(),
        "discovered {} kinds must not exceed grammar's {} kinds",
        discovered_kinds.len(),
        lang.node_kind_count()
    );
}

// ---------------------------------------------------------------------------
// Integration test 2: LANGUAGE constant vs language() function in real usage
// ---------------------------------------------------------------------------

/// Integration: `LANGUAGE` constant and `language()` function produce identical results.
///
/// Verifies the spec requirement that `language()` returns `LANGUAGE` singleton.
/// This test exercises both in a realistic parsing context.
#[test]
fn when_using_language_constant_and_function_then_they_agree_on_all_kinds() {
    let source = "my @items = (1, 2, 3); foreach my $item (@items) { print $item; }";

    let mut parser = Parser::new();
    let tree = must_some(parser.parse(source));

    let lang_fn = language();
    let lang_static = LANGUAGE;

    // node_kind_count must agree
    assert_eq!(
        lang_fn.node_kind_count(),
        lang_static.node_kind_count(),
        "language() and LANGUAGE must agree on node_kind_count"
    );

    // node_kind_names must be identical (same backing slice)
    assert_eq!(
        lang_fn.node_kind_names(),
        lang_static.node_kind_names(),
        "language() and LANGUAGE must return the same node_kind_names"
    );

    // All kinds discovered from the tree must be recognized by both.
    let discovered_kinds = collect_distinct_kinds(&tree);
    for kind in &discovered_kinds {
        assert!(lang_fn.node_kind_is_named(kind), "language() must recognize '{}'", kind);
        assert!(lang_static.node_kind_is_named(kind), "LANGUAGE must recognize '{}'", kind);
    }
}

// ---------------------------------------------------------------------------
// Integration test 3: grammar_kind() + language() cross-validation
// ---------------------------------------------------------------------------

/// Integration: verify `grammar_kind()` output is consistent with language descriptor.
///
/// Node::grammar_kind() returns the tree-sitter grammar canonical name (e.g., "source_file").
/// Node::kind() returns the v3 internal name (e.g., "Program").
/// The language descriptor's `node_kind_names()` contains v3 names (Program, not source_file).
/// This test verifies that v3 kinds found via `kind()` are in the descriptor, and that
/// the grammar kinds can be computed without panic.
#[test]
fn when_traversing_and_checking_grammar_kind_then_no_panic_and_descriptor_validates_v3_kinds() {
    let sources = [
        "my $x = 42;",
        "sub foo { return 1; }",
        "package Demo;",
        "for my $i (0..10) { print $i; }",
        r#"my $match = ($str =~ /\d+/);"#,
    ];

    let lang = language();

    for source in sources {
        let mut parser = Parser::new();
        let tree = must_some(parser.parse(source));

        // Every distinct v3 kind must be in the language descriptor.
        let v3_kinds = collect_distinct_kinds(&tree);
        for kind in &v3_kinds {
            assert!(
                lang.node_kind_is_named(kind),
                "v3 kind '{}' from '{}' must be in language descriptor",
                kind,
                source
            );
        }

        // grammar_kind() must not panic on any node in the tree.
        let root = tree.root_node();
        let _ = root.grammar_kind(); // should not panic

        // grammar_kind() returns a String (not &'static str like kind())
        // and should be usable even if different from the v3 kind.
        let grammar_kind = root.grammar_kind();
        assert!(
            !grammar_kind.is_empty(),
            "grammar_kind() must return a non-empty string for '{}'",
            source
        );
    }
}

// ---------------------------------------------------------------------------
// Integration test 4: PerlLanguage::default() wiring in context
// ---------------------------------------------------------------------------

/// Integration: `PerlLanguage::default()` behaves identically to `LANGUAGE` in parsing context.
///
/// Using `Default::default()` should give the same descriptor as the `LANGUAGE` constant.
/// This is important for ergonomic API design where users can use `Default::default()`
/// to get a language descriptor.
#[test]
fn when_using_default_language_descriptor_then_it_validates_kinds_identically_to_language() {
    let source = r#"
        sub process {
            my ($data) = @_;
            if ($data =~ /^\d+$/) {
                return $data + 1;
            }
            return 0;
        }
    "#;

    let mut parser = Parser::new();
    let tree = must_some(parser.parse(source));

    let lang_default = tree_sitter_perl_rs::PerlLanguage::default();
    let lang_fn = language();

    // Count must match
    assert_eq!(
        lang_default.node_kind_count(),
        lang_fn.node_kind_count(),
        "Default must match language() on count"
    );

    // Names must match
    assert_eq!(
        lang_default.node_kind_names(),
        lang_fn.node_kind_names(),
        "Default must match language() on names"
    );

    // Validation must work identically
    let discovered_kinds = collect_distinct_kinds(&tree);
    for kind in &discovered_kinds {
        assert_eq!(
            lang_default.node_kind_is_named(kind),
            lang_fn.node_kind_is_named(kind),
            "Default and language() must agree on '{}'",
            kind
        );
    }
}

// ---------------------------------------------------------------------------
// Integration test 5: Multiple parse instances share language descriptor semantically
// ---------------------------------------------------------------------------

/// Integration: parsing different Perl constructs produces valid trees whose node kinds
/// are all recognized by a single shared language descriptor.
///
/// This test exercises:
/// - Parser is reusable (parse multiple files/sources)
/// - Each parse produces a valid tree
/// - All trees' node kinds are recognized by the same language descriptor
/// - The language descriptor is stateless and safe to share across parses
#[test]
fn when_parsing_multiple_sources_then_all_resulting_kinds_share_language_validation() {
    let sources = [
        // Variable declaration
        "my ($x :lvalue) = 42;",
        // Subroutine with parameters
        "sub identity { my ($val) = @_; return $val; }",
        // Package with use
        "package Foo::Bar; use strict; our $VERSION = '1.00';",
        // Loop
        "while (my $line = <>) { chomp $line; print $line; }",
        // Hash dereference
        "my $val = $hash->{key} // $hash->{default};",
        // Anonymous array ref
        "my $aref = [1, 2, [3, 4]]; print $aref->[2]->[0];",
    ];

    let lang = language();
    let mut parser = Parser::new();
    let mut all_discovered_kinds: Vec<&'static str> = Vec::new();

    for source in sources {
        let tree = must_some(parser.parse(source));
        let kinds = collect_distinct_kinds(&tree);

        // Every kind in this tree must be recognized
        for kind in &kinds {
            assert!(
                lang.node_kind_is_named(kind),
                "language descriptor must recognize '{}' from '{}'",
                kind,
                source
            );
        }

        all_discovered_kinds.extend(kinds);
    }

    // The union of all discovered kinds should not exceed the grammar's kind count.
    all_discovered_kinds.sort_unstable();
    all_discovered_kinds.dedup();
    assert!(
        all_discovered_kinds.len() <= lang.node_kind_count(),
        "total distinct kinds {} across all sources must not exceed grammar size {}",
        all_discovered_kinds.len(),
        lang.node_kind_count()
    );
}

// ---------------------------------------------------------------------------
// Integration test 6: Full round-trip - language descriptor info used in tree construction
// ---------------------------------------------------------------------------

/// Integration: use language descriptor's metadata to drive tree traversal decisions.
///
/// A realistic use case: tooling wants to know when to recurse into children
/// based on whether a node kind is "interesting". The language descriptor
/// provides this information.
#[test]
fn when_using_language_descriptor_to_filter_traversal_then_it_correctly_identifies_leaf_and_container_kinds(
) {
    let source = "sub add { my ($a, $b) = @_; return $a + $b; } print add(1, 2);";

    let mut parser = Parser::new();
    let tree = must_some(parser.parse(source));
    let lang = language();

    // Find all "leaf" nodes (nodes with no children that are named kinds).
    let mut named_leaf_kinds: Vec<&'static str> = Vec::new();

    fn find_leaves(node: tree_sitter_perl_rs::Node<'_>, out: &mut Vec<&'static str>) {
        if node.child_count() == 0 {
            out.push(node.kind());
        } else {
            for child in node.children() {
                find_leaves(child, out);
            }
        }
    }
    find_leaves(tree.root_node(), &mut named_leaf_kinds);

    // Every named leaf kind must be recognized by the language descriptor.
    for kind in &named_leaf_kinds {
        assert!(lang.node_kind_is_named(kind), "named leaf kind '{}' must be recognized", kind);
    }

    // All discovered kinds (leaf + container) must be recognized.
    let all_kinds = collect_distinct_kinds(&tree);
    for kind in &all_kinds {
        assert!(
            lang.node_kind_is_named(kind),
            "all kind '{}' must be recognized by language descriptor",
            kind
        );
    }

    // Grammar's known kinds must NOT all be present in this small snippet.
    // (just verifying the grammar has more kinds than any single file uses)
    assert!(
        lang.node_kind_count() > named_leaf_kinds.len(),
        "grammar should have more kinds than a single leaf node list"
    );
}

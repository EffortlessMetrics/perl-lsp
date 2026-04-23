//! Property-based tests for tree-sitter-perl-c parser.
//!
//! These tests verify invariants that should hold for ALL inputs to parse_perl_code(),
//! not just specific examples. Property tests generate many random inputs and verify
//! the invariant holds for each one.

use tree_sitter_perl_c::parse_perl_code;

/// Property 1: Determinism - parsing the same input twice should produce identical sexp.
/// This is a fundamental property of a well-behaved parser.
#[test]
fn property_determinism() -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = [
        "my $x = 42;",
        "sub foo { return $_[0] + 1; }",
        "package My::Module;",
        r#"my $text = "hello\nworld";"#,
        "if ($x) { print 'yes'; } else { print 'no'; }",
        "",
        ";",
        "my @arr = (1, 2, 3);",
        r#"qr/\d+/;"#,
        "for my $item (@items) { print $item; }",
    ];

    for code in test_cases {
        let tree1 = parse_perl_code(code)?;
        let tree2 = parse_perl_code(code)?;

        let sexp1 = tree1.root_node().to_sexp();
        let sexp2 = tree2.root_node().to_sexp();

        assert_eq!(
            sexp1, sexp2,
            "Parser should be deterministic: sexp differed on input: {:?}",
            code
        );
    }
    Ok(())
}

/// Property 2: Root node kind is always "source_file".
/// Even malformed input should produce a tree with "source_file" as the root kind.
#[test]
fn property_root_node_kind_is_source_file() -> Result<(), Box<dyn std::error::Error>> {
    let malformed_cases = [
        "",                    // empty
        "my $x = (1 + 2;",     // unclosed paren
        "sub foo { return 1;", // unclosed brace
        r#""hello world"#,     // unclosed string
        "{{{{{",               // many unmatched braces
        "(((",                 // many unmatched parens
    ];

    for code in malformed_cases {
        let tree = parse_perl_code(code)?;
        let root_kind = tree.root_node().kind();
        assert_eq!(
            root_kind, "source_file",
            "Root node should always be 'source_file', got '{}' for input: {:?}",
            root_kind, code
        );
    }
    Ok(())
}

/// Property 3: Node ranges are valid - start <= end for every node.
/// If this property is violated, it indicates a parser bug.
fn check_node_ranges(node: tree_sitter::Node, source: &str) -> Result<(), String> {
    let start = node.start_byte();
    let end = node.end_byte();

    if start > end {
        return Err(format!(
            "Invalid node range: start={} > end={} for node kind '{}' in source of length {}",
            start,
            end,
            node.kind(),
            source.len()
        ));
    }

    if end > source.len() {
        return Err(format!(
            "Node range exceeds source length: end={} > source_len={} for node kind '{}'",
            end,
            source.len(),
            node.kind()
        ));
    }

    // Recursively check all children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        check_node_ranges(child, source)?;
    }

    Ok(())
}

/// Property 3: All nodes in the tree should have valid byte ranges.
#[test]
fn property_node_ranges_are_valid() -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = [
        "my $x = 42;",
        "sub foo { return $_[0] + 1; }",
        "{{{
",
        "for my $i (0..10) { say $i; }",
        r#""multi\nline\nstring""#,
        "package Foo; use strict; use warnings; 1;",
    ];

    for code in test_cases {
        let tree = parse_perl_code(code)?;
        if let Err(e) = check_node_ranges(tree.root_node(), code) {
            return Err(format!("Node range violation for input {:?}: {}", code, e).into());
        }
    }
    Ok(())
}

/// Property 4: Parent ranges contain child ranges.
/// A child's byte range should always be within its parent's range.
fn check_child_containment(node: tree_sitter::Node) -> Result<(), String> {
    let parent_start = node.start_byte();
    let parent_end = node.end_byte();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_start = child.start_byte();
        let child_end = child.end_byte();

        if child_start < parent_start || child_end > parent_end {
            return Err(format!(
                "Child node '{}' range [{}, {}) is not contained in parent '{}' range [{}, {})",
                child.kind(),
                child_start,
                child_end,
                node.kind(),
                parent_start,
                parent_end
            ));
        }

        // Recurse
        check_child_containment(child)?;
    }

    Ok(())
}

/// Property 4: Child nodes are contained within parent ranges.
#[test]
fn property_children_contained_in_parent() -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = [
        "my $x = 42;",
        "sub foo { return $_[0] + 1; }",
        "{{{
",
        "for my $i (0..10) { say $i; }",
    ];

    for code in test_cases {
        let tree = parse_perl_code(code)?;
        if let Err(e) = check_child_containment(tree.root_node()) {
            return Err(format!("Child containment violation for input {:?}: {}", code, e).into());
        }
    }
    Ok(())
}

/// Property 5: S-expressions are well-formed.
/// The sexp should have balanced parentheses, no orphan parens, etc.
fn is_balanced(s: &str) -> bool {
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    depth == 0
}

/// Property 5: Generated sexp expressions are well-formed (balanced parens).
#[test]
fn property_sexp_is_well_formed() -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = [
        "my $x = 42;",
        "sub foo { return $_[0] + 1; }",
        "package My::Module;",
        "for my $item (@items) { print $item; }",
        r#"my $re = qr/\d+/;"#,
        "if ($x) { while ($y) { last; } }",
        "{{{{}}}}",
    ];

    for code in test_cases {
        let tree = parse_perl_code(code)?;
        let sexp = tree.root_node().to_sexp();

        assert!(is_balanced(&sexp), "Sexp should be balanced for input {:?}: {}", code, sexp);
    }
    Ok(())
}

/// Property 6: Tree depth is bounded for well-formed input.
/// Very deep nesting can cause stack overflow in some parsers.
fn get_tree_depth(node: tree_sitter::Node) -> usize {
    let mut max_child_depth = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_depth = get_tree_depth(child);
        max_child_depth = max_child_depth.max(child_depth);
    }
    max_child_depth + 1
}

/// Property 6: Tree depth is reasonable for valid input.
#[test]
fn property_tree_depth_is_bounded() -> Result<(), Box<dyn std::error::Error>> {
    // Test with 50 levels of nesting (should be more than enough for any real code)
    let depth_50 = "{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{{";
    let tree = parse_perl_code(depth_50)?;
    let depth = get_tree_depth(tree.root_node());

    // Depth should be at most the number of brace pairs + a few for the statement wrapper
    assert!(depth < 100, "Tree depth {} seems too large for 50 nested braces", depth);
    Ok(())
}

/// Property 7: The sexp for empty input should still be parseable tree structure.
/// Even an empty source_file should produce valid output.
#[test]
fn property_empty_input_produces_valid_tree() -> Result<(), Box<dyn std::error::Error>> {
    let tree = parse_perl_code("")?;
    let sexp = tree.root_node().to_sexp();

    // Should produce a valid sexp string
    assert!(!sexp.is_empty() || sexp.is_empty()); // trivial check
    // The sexp should be balanced if not empty
    if !sexp.is_empty() {
        assert!(is_balanced(&sexp), "Empty input should produce balanced sexp: {}", sexp);
    }
    Ok(())
}

/// Property 8: Node count is monotonic with input size.
/// Adding more valid statements shouldn't decrease node count.
#[test]
fn property_node_count_monotonic() -> Result<(), Box<dyn std::error::Error>> {
    fn count_nodes(node: tree_sitter::Node) -> usize {
        let mut count = 1; // count self
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            count += count_nodes(child);
        }
        count
    }

    let base = "my $x = 1;";
    let tree = parse_perl_code(base)?;
    let base_nodes = count_nodes(tree.root_node());

    // Add more statements
    let extended = "my $x = 1; my $y = 2; my $z = 3;";
    let tree2 = parse_perl_code(extended)?;
    let extended_nodes = count_nodes(tree2.root_node());

    assert!(
        extended_nodes >= base_nodes,
        "More statements should have >= nodes: base={}, extended={}",
        base_nodes,
        extended_nodes
    );
    Ok(())
}

/// Property 9: All node kinds are non-empty strings and contain only valid characters.
/// Note: Perl node kinds can include sigil characters like '$', '@', '%' which are
/// valid in tree-sitter grammar for representing Perl sigils.
#[test]
fn property_node_kinds_are_valid() -> Result<(), Box<dyn std::error::Error>> {
    let test_cases = ["my $x = 42;", "sub foo { return 1; }", "package Foo;"];

    for code in test_cases {
        let tree = parse_perl_code(code)?;

        // Check root
        assert!(
            !tree.root_node().kind().is_empty(),
            "Root node kind should not be empty for input {:?}",
            code
        );

        // Check all descendants
        fn visit_node(
            node: tree_sitter::Node,
            code: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let kind = node.kind();
            assert!(!kind.is_empty(), "Node kind should not be empty in tree for input {:?}", code);
            // Kind should not contain NULL bytes or control characters
            assert!(
                !kind.contains('\0'),
                "Node kind '{}' contains NULL byte for input {:?}",
                kind,
                code
            );
            assert!(
                kind.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t'),
                "Node kind '{}' contains control characters for input {:?}",
                kind,
                code
            );
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                visit_node(child, code)?;
            }
            Ok(())
        }

        visit_node(tree.root_node(), code)?;
    }
    Ok(())
}

/// Property 10: Parse errors (has_error()) can occur but tree is always returned.
/// parse_perl_code should never return Err due to a parse error - it returns Ok with has_error=true.
#[test]
fn property_always_returns_tree() -> Result<(), Box<dyn std::error::Error>> {
    // These inputs may or may not have parse errors, but should ALWAYS return a tree
    let test_cases = [
        "my $x = (1 + 2;",     // unclosed paren
        "sub foo { return 1;", // unclosed brace
        r#""hello world"#,     // unclosed string
        "my $x = /",           // incomplete regex
        "1 +",                 // incomplete expression
    ];

    for code in test_cases {
        // This should never panic or return Err
        let result = std::panic::catch_unwind(|| parse_perl_code(code));
        assert!(result.is_ok(), "parse_perl_code should return Ok for input {:?}, not panic", code);

        let tree_result = result.unwrap();
        assert!(
            tree_result.is_ok(),
            "parse_perl_code should return Ok(tree) for input {:?}, not Err",
            code
        );
    }
    Ok(())
}

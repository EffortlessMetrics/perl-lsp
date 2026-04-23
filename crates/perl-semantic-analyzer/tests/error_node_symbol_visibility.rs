//! Tests for symbol visibility in and around Error nodes.
//!
//! ## Background
//!
//! `NodeKind::Error` has a `partial: Option<Box<Node>>` field that stores a
//! partially-parsed sub-tree when the parser managed to build some structure
//! before failing.  In the current parser, `partial: Some(...)` is produced by
//! the postfix arrow-expression recovery path (when `$expr->` is encountered
//! but the token after `->` is not a valid continuation such as a method name,
//! opening paren, bracket, or brace).
//!
//! Before the fix, `SymbolExtractor::visit_node()` treated `NodeKind::Error {
//! .. }` as an opaque no-op leaf and never descended into `partial`, while
//! every other traversal in the codebase (semantic tokens, class model, scope
//! analyzer via children()) already visited `partial`.
//!
//! ## What these tests cover
//!
//! 1. **Arrow truncation** — `$r` declared via `my $r = $obj->;` is still
//!    extracted even though the initializer is an Error node.
//! 2. **Regression: variables on both sides of an arrow-truncation Error** —
//!    declarations before and after the error are both visible.
//! 3. **Regression: unclosed sub** — the parser currently returns a `Subroutine`
//!    node directly (not `Error { partial: Some(Subroutine) }`), so `sub foo`
//!    is visible without the partial-descent fix.  This test guards that
//!    behaviour against future parser changes.
//! 4. **Regression: missing RHS** — `my $x = ;` produces a
//!    `VariableDeclaration` with a `MissingExpression` initializer, not an
//!    Error node.  Symbols before and after are both visible.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind, SymbolTable};
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_and_extract(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|syms| syms.iter().any(|s| s.kind == kind))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// The declared variable (`$r`) is visible even when its initializer is an
/// Error node produced by a truncated arrow expression.
///
/// Parser produces:
/// ```text
/// VariableDeclaration($r,
///   Error { partial: Some(Variable($before)) })
/// ```
///
/// `$r` must be extracted from the `VariableDeclaration` wrapper.  The partial
/// node contains `Variable($before)` which is a read reference — not a new
/// declaration — so partial descent does not add new symbols here, but it must
/// not panic or skip siblings.
#[test]
fn decl_visible_when_initializer_is_error_node() -> Result<(), Box<dyn std::error::Error>> {
    // Terminating semicolon after `->` triggers the arrow-truncation recovery.
    let source = "my $before = 1;\nmy $r = $before->;\nmy $after = 2;\n";
    let table = parse_and_extract(source);

    assert!(
        has_symbol(&table, "before", SymbolKind::scalar()),
        "$before should be visible; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    assert!(
        has_symbol(&table, "r", SymbolKind::scalar()),
        "$r should be visible even though its initializer is Error{{partial}}; \
         symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    assert!(
        has_symbol(&table, "after", SymbolKind::scalar()),
        "$after (declared after the Error node) should be visible; \
         symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    Ok(())
}

/// Arrow truncation at EOF: the declared variable is still indexed.
///
/// Parser produces:
/// ```text
/// VariableDeclaration($r, Error { partial: Some(Variable($before)) })
/// ```
/// No subsequent statements, so this is a pure "is the decl visible" check.
#[test]
fn decl_visible_when_rhs_is_truncated_arrow_at_eof() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $obj = {};\nmy $r = $obj->";
    let table = parse_and_extract(source);

    assert!(
        has_symbol(&table, "obj", SymbolKind::scalar()),
        "$obj should be visible; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    assert!(
        has_symbol(&table, "r", SymbolKind::scalar()),
        "$r should be visible even when its initializer arrow is truncated at EOF; \
         symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    Ok(())
}

/// Subroutine whose block is unclosed at EOF is still indexed.
///
/// The parser currently produces a `Subroutine` node directly (not wrapped in
/// `Error { partial: Some(Subroutine) }`).  This test guards that existing
/// recovery path against future parser changes.
#[test]
fn unclosed_sub_at_eof_is_visible() -> Result<(), Box<dyn std::error::Error>> {
    // Missing closing `}` — parser recovers and returns a Subroutine node.
    let source = "sub foo { my $x = 1; ";
    let table = parse_and_extract(source);

    assert!(
        has_symbol(&table, "foo", SymbolKind::Subroutine),
        "sub foo should be visible even when its block is unclosed at EOF; \
         symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    Ok(())
}

/// When two subroutines are present and the second is unclosed, both are
/// indexed.  This exercises the statement-level recovery path where parsing
/// continues after an error statement.
#[test]
fn second_unclosed_sub_is_visible() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo { } sub bar { my $x = 1;";
    let table = parse_and_extract(source);

    assert!(
        has_symbol(&table, "foo", SymbolKind::Subroutine),
        "sub foo (closed) should be visible; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    assert!(
        has_symbol(&table, "bar", SymbolKind::Subroutine),
        "sub bar (unclosed at EOF) should be visible; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    Ok(())
}

/// Combined regression for issue #3499:
/// - parser recovers from an unclosed block when a new `sub` starts
/// - symbol extraction continues through a partial Error node (`$obj->;`)
///
/// This guards the interaction between PR #4079 (unclosed-block recovery) and
/// PR #4071 (descend into `Error.partial`).
#[test]
fn recovers_unclosed_block_and_keeps_symbols_after_partial_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub foo {\n  my $obj = {};\n  my $broken = $obj->;\nsub bar { }\n";
    let table = parse_and_extract(source);

    assert!(
        has_symbol(&table, "foo", SymbolKind::Subroutine),
        "sub foo should be visible despite unclosed block; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    assert!(
        has_symbol(&table, "broken", SymbolKind::scalar()),
        "$broken should be visible with Error{{partial}} initializer; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    assert!(
        has_symbol(&table, "bar", SymbolKind::Subroutine),
        "sub bar should be visible after recovery from unclosed block; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    Ok(())
}

/// Variables before and after a missing-RHS error are both visible.
///
/// `my $broken = ;` triggers Phase 2 recovery and produces a
/// `VariableDeclaration` with a `MissingExpression` initializer — not an
/// `Error { partial }` node.  This tests an independent recovery path.
#[test]
fn variables_around_missing_rhs_are_visible() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $before = 1;\nmy $broken = ;\nmy $after = 2;\n";
    let table = parse_and_extract(source);

    assert!(
        has_symbol(&table, "before", SymbolKind::scalar()),
        "$before should be visible; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    assert!(
        has_symbol(&table, "after", SymbolKind::scalar()),
        "$after declared after error should be visible; symbols: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    Ok(())
}

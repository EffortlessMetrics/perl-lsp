/// Tests for the `<<>>` double-diamond operator (Perl 5.22+).
///
/// `<<>>` is the "safer" diamond: it reads from @ARGV but refuses magic/pipe
/// filenames.  Semantically it is an input operator like `<>`, so we reuse
/// `NodeKind::Diamond` rather than adding a new variant.
mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

// ── helpers ──────────────────────────────────────────────────────────────────

fn parse_first_expr(source: &str) -> NodeKind {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());
    match ast.kind {
        NodeKind::Program { ref statements } => {
            let stmt = statements.first().expect("no statements");
            match &stmt.kind {
                NodeKind::ExpressionStatement { expression } => expression.kind.clone(),
                other => panic!("expected ExpressionStatement, got {other:?}"),
            }
        }
        other => panic!("expected Program, got {other:?}"),
    }
}

// ── primary test: <<>> parses as Diamond ─────────────────────────────────────

/// `<<>>` must parse without any ERROR node and yield `NodeKind::Diamond`.
#[test]
fn test_double_diamond_standalone_parses_as_diamond() {
    assert_clean_parse("<<>>;");
    let kind = parse_first_expr("<<>>;");
    assert!(
        matches!(kind, NodeKind::Diamond),
        "expected NodeKind::Diamond for `<<>>`, got {kind:?}"
    );
}

/// `my $line = <<>>;`
#[test]
fn test_double_diamond_assigned_parses_cleanly() {
    assert_clean_parse("my $line = <<>>;");
}

/// `while (<<>>) { ... }`
#[test]
fn test_double_diamond_while_condition_parses_cleanly() {
    assert_clean_parse("while (<<>>) { print; }");
}

// ── regression guards ─────────────────────────────────────────────────────────

/// `<>` must still parse as Diamond.
#[test]
fn test_single_diamond_unchanged() {
    assert_clean_parse("<>;");
    let kind = parse_first_expr("<>;");
    assert!(matches!(kind, NodeKind::Diamond), "expected NodeKind::Diamond for `<>`, got {kind:?}");
}

/// `<STDIN>` must still parse as Readline.
#[test]
fn test_stdin_readline_unchanged() {
    assert_clean_parse("<STDIN>;");
    let kind = parse_first_expr("<STDIN>;");
    assert!(
        matches!(kind, NodeKind::Readline { .. }),
        "expected NodeKind::Readline for `<STDIN>`, got {kind:?}"
    );
}

/// `$x << 2` must still parse as arithmetic left-shift (BinaryOp), not Diamond.
#[test]
fn test_left_shift_arithmetic_unchanged() {
    assert_clean_parse("my $y = $x << 2;");
    // Verify no Diamond node in the arithmetic shift expression
    let mut parser = Parser::new("my $y = $x << 2;");
    let ast = must(parser.parse());
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("diamond"), "`$x << 2` must not produce a Diamond node; sexp:\n{sexp}");
}

/// A heredoc `<<EOF ... EOF` must still parse as Heredoc, not Diamond.
#[test]
fn test_heredoc_unchanged() {
    assert_clean_parse("my $s = <<EOF;\nhello\nEOF\n");
}

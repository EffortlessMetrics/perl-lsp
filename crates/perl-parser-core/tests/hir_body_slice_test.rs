//! HIR body infrastructure — first vertical slice test.
//!
//! Specimen: `my $x = $a + $b;`
//!
//! Asserts the exact body structure and source spans produced when lowering the
//! specimen, per ADR #2564 and the first-slice work item.
//!
//! Expected arena layout (verified against actual parser output):
//!
//! ```text
//! source:  m  y     $  x     =     $  a     +     $  b  ;
//! offset:  0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15
//!
//! Expr arena:
//!   [0] Variable($x, Write)   → 3..5
//!   [1] Variable($a, Read)    → 8..10
//!   [2] Variable($b, Read)    → 13..15
//!   [3] Binary(Add, [1], [2]) → 8..15
//!   [4] Assign([0], [3])      → 3..15
//!
//! Stmt arena:
//!   [0] Let { name: "x", init: Some(ExprId(4)) } → 0..15
//!
//! Block arena:
//!   [0] root { stmts: [StmtId(0)] }              → 0..16  (full Program node)
//! ```

use perl_parser_core::hir::{
    AccessMode, AssignMode, BinaryOp, BodyOwnerKind, DeclStorageClass, HirExpr, HirStmt, Sigil,
    VariableKind, lower_body,
};
use perl_parser_core::{Parser, SourceLocation};

fn parse_and_lower(source: &str) -> perl_parser_core::hir::HirBody {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    lower_body(&output.ast)
}

#[test]
fn hir_body_slice_specimen_structure() {
    let source = "my $x = $a + $b;";
    let body = parse_and_lower(source);

    // ── 1. Body owner must be ProgramRoot ──────────────────────────────────
    assert_eq!(body.owner, BodyOwnerKind::ProgramRoot, "owner must be ProgramRoot");

    // ── 2. Root block has exactly one statement ────────────────────────────
    let root = body.block(body.root_block).expect("root block must exist");
    assert_eq!(root.stmts.len(), 1, "root block must have exactly one statement");

    let stmt_id = root.stmts[0];
    let stmt = body.stmt(stmt_id).expect("stmt must exist");

    // ── 3. Statement is Let { name: "x", sigil: $, storage: my } ──────────
    let (let_name, let_sigil, let_storage, init_id) = match stmt {
        HirStmt::Let { name, sigil, storage, init } => (name.as_str(), sigil, storage, *init),
        other => panic!("expected HirStmt::Let, got {:?}", other),
    };

    assert_eq!(let_name, "x", "declared variable name must be 'x'");
    assert!(matches!(let_sigil, Sigil::Scalar), "sigil must be Scalar ($)");
    assert!(matches!(let_storage, DeclStorageClass::My), "storage class must be My");

    // ── 4. Statement source span: `my $x = $a + $b` (0..15, semicolon excluded
    //        by the VariableDeclaration AST node boundary) ───────────────────
    let stmt_range =
        body.source_map.stmt_range(stmt_id).expect("stmt source range must be present");
    assert_eq!(
        stmt_range,
        SourceLocation { start: 0, end: 15 },
        "stmt range must span bytes 0..15 (VariableDeclaration node, semicolon at 15 excluded)"
    );

    // ── 5. Initializer is an Assign expression ────────────────────────────
    let init_expr_id = init_id.expect("initializer expression must be present");
    let init_expr = body.expr(init_expr_id).expect("init expr must exist");

    let (assign_lhs, assign_rhs) = match init_expr {
        HirExpr::Assign { lhs, rhs, mode } => {
            assert!(matches!(mode, AssignMode::Simple), "assign mode must be Simple");
            (*lhs, *rhs)
        }
        other => panic!("expected HirExpr::Assign, got {:?}", other),
    };

    // Assign node spans from `$x` to end of `$b`: 3..15
    let assign_range =
        body.source_map.expr_range(init_expr_id).expect("assign expr range must be present");
    assert_eq!(
        assign_range,
        SourceLocation { start: 3, end: 15 },
        "Assign expr must span bytes 3..15"
    );

    // ── 6. Assign LHS is a Variable($x, Write) at 3..5 ───────────────────
    let lhs_expr = body.expr(assign_lhs).expect("lhs expr must exist");
    match lhs_expr {
        HirExpr::Variable(var) => {
            assert_eq!(var.name, "x", "LHS variable name must be 'x'");
            assert!(matches!(var.sigil, Sigil::Scalar), "LHS sigil must be Scalar");
            assert!(matches!(var.kind, VariableKind::Lexical), "LHS must be Lexical");
            assert!(matches!(var.access, AccessMode::Write), "LHS must have Write access (place)");
        }
        other => panic!("expected HirExpr::Variable for LHS, got {:?}", other),
    }

    let lhs_range = body.source_map.expr_range(assign_lhs).expect("LHS expr range must be present");
    assert_eq!(lhs_range, SourceLocation { start: 3, end: 5 }, "LHS $x must span bytes 3..5");

    // ── 7. Assign RHS is Binary(Add) with explicit child IDs ──────────────
    let rhs_expr = body.expr(assign_rhs).expect("rhs expr must exist");
    let (bin_lhs, bin_op, bin_rhs) = match rhs_expr {
        HirExpr::Binary { lhs, op, rhs } => (*lhs, op, *rhs),
        other => panic!("expected HirExpr::Binary for RHS, got {:?}", other),
    };

    assert!(matches!(bin_op, BinaryOp::Add), "binary operator must be Add (+), got {:?}", bin_op);

    // Binary node spans `$a + $b`: 8..15
    let bin_range =
        body.source_map.expr_range(assign_rhs).expect("binary expr range must be present");
    assert_eq!(
        bin_range,
        SourceLocation { start: 8, end: 15 },
        "Binary expr must span bytes 8..15 ($a + $b)"
    );

    // ── 8. Binary LHS is Variable($a, Read) at 8..10 ─────────────────────
    let bin_lhs_expr = body.expr(bin_lhs).expect("binary lhs expr must exist");
    match bin_lhs_expr {
        HirExpr::Variable(var) => {
            assert_eq!(var.name, "a", "binary LHS variable name must be 'a'");
            assert!(matches!(var.sigil, Sigil::Scalar), "binary LHS sigil must be Scalar");
            assert!(matches!(var.access, AccessMode::Read), "binary LHS must have Read access");
        }
        other => panic!("expected HirExpr::Variable for binary LHS ($a), got {:?}", other),
    }

    let bin_lhs_range =
        body.source_map.expr_range(bin_lhs).expect("binary LHS range must be present");
    assert_eq!(bin_lhs_range, SourceLocation { start: 8, end: 10 }, "$a must span bytes 8..10");

    // ── 9. Binary RHS is Variable($b, Read) at 13..15 ────────────────────
    let bin_rhs_expr = body.expr(bin_rhs).expect("binary rhs expr must exist");
    match bin_rhs_expr {
        HirExpr::Variable(var) => {
            assert_eq!(var.name, "b", "binary RHS variable name must be 'b'");
            assert!(matches!(var.sigil, Sigil::Scalar), "binary RHS sigil must be Scalar");
            assert!(matches!(var.access, AccessMode::Read), "binary RHS must have Read access");
        }
        other => panic!("expected HirExpr::Variable for binary RHS ($b), got {:?}", other),
    }

    let bin_rhs_range =
        body.source_map.expr_range(bin_rhs).expect("binary RHS range must be present");
    assert_eq!(bin_rhs_range, SourceLocation { start: 13, end: 15 }, "$b must span bytes 13..15");

    // ── 10. Source map is parallel to the arenas (consistency check) ───────
    assert_eq!(
        body.source_map.stmt_ranges.len(),
        body.stmts.len(),
        "stmt source map must be parallel to stmt arena"
    );
    assert_eq!(
        body.source_map.expr_ranges.len(),
        body.exprs.len(),
        "expr source map must be parallel to expr arena"
    );
    assert_eq!(
        body.source_map.block_ranges.len(),
        body.blocks.len(),
        "block source map must be parallel to block arena"
    );
}

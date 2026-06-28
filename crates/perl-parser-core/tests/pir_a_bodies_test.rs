//! PIR-A bodies — integration tests for PR 2 (#2578).
//!
//! Verifies that `lower_hir_bodies()` correctly lowers canonical HIR body arenas
//! (from `HirFile::bodies`) into PIR-A Read/Write/Modify operations with correct
//! source anchors, scope/package context, and verifier fail-closed behaviour.
//!
//! Test coverage:
//!   1. `my $x = $a + $b;` — canonical receipt: exactly 1 Write $x (double-emit guard), Read $a, Read $b, Binary
//!   2. `$x = $y;` — plain assign → Write $x, Read $y
//!   3. `$x += 1;` — compound assign → Modify $x (place evaluated once)
//!   4. `our $x; $x = $y;` — package place → StashWrite / StashRead, NO LexicalWrite for $x
//!   5. `$Foo::x = $y;` — package slot → StashWrite / StashRead
//!   6. Recovery `my $x = ;` — NO exact operation emitted from recovered syntax
//!   7. Unresolved/Dynamic place — no exact Read/Write op emitted
//!   8. Source anchors present on all non-opaque nodes
//!   9. `lower_hir_bodies` schema version matches PIR_RECEIPT_VERSION
//!  10. Deterministic output for same source
//!  11. `$x++` — Modify (ReadModifyWrite unary)
//!  12. Modify evaluates place once (no duplicate target evaluation in receipt)
//!  13. `sub foo { my $x = 1; }` — sub body produces Write in sub body operations
//!  14. body_model_version guard — version 0 → no body ops emitted / version 1 → ok
//!  15. `our $x = $y;` — exactly 1 StashWrite, 0 LexicalWrite for $x (storage-aware Let guard)
//!  16. `our $x += 1;` — StashModify from lower_variable_modify (package compound assign)
//!  17. Version-mismatch empty graph — body_model_version != HIR_BODY_MODEL_VERSION → 0 nodes + ambient_input
//!  18. `lower_hir_bodies_with_identity` threads source_identity to receipt
//!  19. Unary Read (`-$x`) — lowers operand as a Read, no Modify
//!  20. `foo($x)` in body — Call node is unsupported, $x produces a Read (not Write)
//!  21. Array sigil `@arr` — sigil_str emits `@`, not `$`
//!  22. Hash sigil `%h` — sigil_str emits `%`
//!  23. Opaque function call counted in unsupported receipt
//!  24. `local $x;` → StashWrite, 0 LexicalWrite (regression guard for #2612)

use perl_parser_core::Parser;
use perl_parser_core::SourceLocation;
use perl_parser_core::hir::{
    AccessMode, Arena, BodyOwnerKind, BodySourceMap, HirBlock, HirBlockId, HirBody, HirExpr,
    HirExprId, HirStmt, HirStmtId, HirVariable, Sigil, VariableKind, lower_ast,
};
use perl_parser_core::pir::{
    PIR_RECEIPT_VERSION, PirGraph, PirOperation, lower_hir_bodies, lower_hir_bodies_with_identity,
};

fn parse_and_lower(source: &str) -> PirGraph {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);
    lower_hir_bodies(&hir)
}

// ── 1. Canonical receipt: `my $x = $a + $b;` ────────────────────────────────
// Expected: Write (LexicalWrite) for $x, Read (LexicalRead) for $a, Read for $b,
// plus a Binary node (or Opaque for the + operator — acceptable), plus Assign.

#[test]
fn pir_a_canonical_receipt_my_x_equals_a_plus_b() {
    let graph = parse_and_lower("my $x = $a + $b;");

    // Must have exactly one LexicalWrite for $x — double-emission from the Let arm was a bug
    let write_count = graph
        .nodes
        .iter()
        .filter(|n| matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "x"))
        .count();
    assert_eq!(
        write_count, 1,
        "must have exactly 1 LexicalWrite for $x, not {write_count} (double-emit guard)"
    );

    // Must have LexicalRead nodes (for $a and $b — both undeclared, package vars OR
    // reads from the rhs. Note: undeclared vars may resolve as StashRead, not LexicalRead.
    // What we require: at least two Read-type operations (Lexical or Stash) on the RHS.
    let read_count = graph
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                &n.operation,
                PirOperation::LexicalRead { .. } | PirOperation::StashRead { .. }
            )
        })
        .count();
    assert!(read_count >= 2, "must have at least 2 read ops for $a and $b; got {read_count}");

    // All source-derived nodes must carry a source anchor
    for node in &graph.nodes {
        assert!(
            node.source_anchor.is_anchored(),
            "every PIR-A node must have a source anchor; got unanchored node: {:?}",
            node.operation.name()
        );
    }
}

// ── 2. Plain assign `$x = $y;` → Write $x, Read $y ─────────────────────────

#[test]
fn pir_a_plain_assign_write_and_read() {
    let graph = parse_and_lower("$x = $y;");

    // LHS: Write (package — $x undeclared)
    let write = graph.nodes.iter().find(|n| {
        matches!(&n.operation,
            PirOperation::LexicalWrite { name } if name.name == "x"
        ) || matches!(&n.operation,
            PirOperation::StashWrite { symbol } if symbol.name == "x"
        )
    });
    assert!(write.is_some(), "must have a Write op for $x");

    // RHS: Read (package — $y undeclared)
    let read = graph.nodes.iter().find(|n| {
        matches!(&n.operation,
            PirOperation::LexicalRead { name } if name.name == "y"
        ) || matches!(&n.operation,
            PirOperation::StashRead { symbol } if symbol.name == "y"
        )
    });
    assert!(read.is_some(), "must have a Read op for $y");
}

// ── 3. Compound assign `$x += 1;` → Modify $x ───────────────────────────────

#[test]
fn pir_a_compound_assign_produces_modify() {
    // Use a declared variable so we get a deterministic Lexical→Modify (not StashModify).
    let graph = parse_and_lower("my $x = 0; $x += 1;");

    let modify = graph.nodes.iter().find(|n| {
        matches!(&n.operation, PirOperation::Modify { .. } | PirOperation::StashModify { .. })
    });
    assert!(
        modify.is_some(),
        "compound assign `$x += 1` must produce a Modify or StashModify operation; got: {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );

    // The Modify op must name $x
    let names_x = graph.nodes.iter().any(|n| match &n.operation {
        PirOperation::Modify { name, .. } => name.name == "x",
        PirOperation::StashModify { symbol, .. } => symbol.name == "x",
        _ => false,
    });
    assert!(names_x, "Modify op must target $x");
}

// ── 4. `our $x; $x = $y;` — package place → StashWrite / StashRead ──────────

#[test]
fn pir_a_our_var_produces_stash_ops() {
    let graph = parse_and_lower("our $x; $x = $y;");

    // `our $x` is a StashWrite (declaration)
    let stash_write = graph.nodes.iter().find(
        |n| matches!(&n.operation, PirOperation::StashWrite { symbol } if symbol.name == "x"),
    );
    assert!(
        stash_write.is_some(),
        "`our $x` must produce StashWrite; got {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );

    // `our $x` must NOT produce a LexicalWrite — the old Let arm bug emitted LexicalWrite
    // unconditionally, ignoring the storage class.
    let lex_write = graph
        .nodes
        .iter()
        .find(|n| matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "x"));
    assert!(
        lex_write.is_none(),
        "`our $x` declaration must NOT produce LexicalWrite; got: {:?}",
        lex_write.map(|n| n.operation.name())
    );
}

// ── 5. `$Foo::x = $y;` — qualified name → StashWrite ────────────────────────

#[test]
fn pir_a_qualified_var_produces_stash_write() {
    let graph = parse_and_lower("$Foo::x = $y;");

    let stash_write = graph.nodes.iter().find(|n| {
        matches!(&n.operation, PirOperation::StashWrite { symbol } if symbol.name.contains("x"))
    });
    assert!(
        stash_write.is_some(),
        "qualified `$Foo::x` must produce StashWrite; got {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );
}

// ── 6. Recovery `my $x = ;` — NO exact operation from recovered syntax ───────

#[test]
fn pir_a_recovery_no_exact_operation() {
    let graph = parse_and_lower("my $x = ;");

    // The recovered initializer must not produce a clean Read operation.
    // A Write for the declaration target ($x) is acceptable — the declaration IS known.
    // What must not happen: a Read op appearing to claim the RHS was a real variable.
    // (In practice: recovery → Opaque RHS → no Read emitted for the phantom RHS.)
    let clean_reads: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                &n.operation,
                PirOperation::LexicalRead { .. } | PirOperation::StashRead { .. }
            )
        })
        .collect();

    // There should be no reads from a recovered RHS. The declaration Write is ok.
    // This test is conservative: if a Read appears it must have a real variable name
    // (not be an artifact of recovery). We assert no reads exist for this case.
    assert!(
        clean_reads.is_empty(),
        "recovery `my $x = ;` must not emit any Read ops; got {:?}",
        clean_reads.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );
}

// ── 7. Unresolved/Dynamic place — no exact op emitted ────────────────────────

#[test]
fn pir_a_opaque_expression_no_wrong_fact() {
    // A call `foo($x)` — the call is Opaque in the body. The Modify/Write op
    // for $x should NOT be emitted through the call's Opaque boundary.
    // What we assert: no StashWrite or LexicalWrite claiming $x in the call target.
    let graph = parse_and_lower("foo($x);");

    // The call itself may or may not be modeled. The argument $x appearing as
    // a Write target inside a call-arg would be wrong — $x is Read here.
    // Assert: no Write op targeting $x appears.
    let wrong_write = graph.nodes.iter().find(|n| match &n.operation {
        PirOperation::LexicalWrite { name } => name.name == "x",
        PirOperation::StashWrite { symbol } => symbol.name == "x",
        _ => false,
    });
    assert!(
        wrong_write.is_none(),
        "call argument $x must not produce a Write op; got: {:?}",
        wrong_write.map(|n| n.operation.name())
    );
}

// ── 8. Source anchors present on all non-opaque nodes ────────────────────────

#[test]
fn pir_a_all_nodes_anchored() {
    let graph = parse_and_lower("my $x = 1; $y = $z; $a += 2;");
    for node in &graph.nodes {
        assert!(
            node.source_anchor.is_anchored(),
            "all PIR-A nodes must carry a source anchor; unanchored op: {}",
            node.operation.name()
        );
    }
    assert_eq!(
        graph.receipt.source_anchor_coverage.unanchored, 0,
        "receipt must report 0 unanchored nodes"
    );
}

// ── 9. Schema version matches PIR_RECEIPT_VERSION ────────────────────────────

#[test]
fn pir_a_schema_version_matches() {
    let graph = parse_and_lower("my $x = 1;");
    assert_eq!(
        graph.receipt.schema_version, PIR_RECEIPT_VERSION,
        "PIR-A receipt schema_version must match PIR_RECEIPT_VERSION"
    );
}

// ── 10. Deterministic output ─────────────────────────────────────────────────

#[test]
fn pir_a_deterministic() {
    let source = "my $x = 1; our $y; $x = $y; $x += 1;";
    let graph1 = parse_and_lower(source);
    let graph2 = parse_and_lower(source);
    assert_eq!(graph1.nodes.len(), graph2.nodes.len(), "node count must be deterministic");
    assert_eq!(graph1.edges.len(), graph2.edges.len(), "edge count must be deterministic");
    for (n1, n2) in graph1.nodes.iter().zip(graph2.nodes.iter()) {
        assert_eq!(n1.operation, n2.operation, "operations must be deterministic");
        assert_eq!(n1.source_anchor, n2.source_anchor, "anchors must be deterministic");
    }
}

// ── 11. `$x++` → Modify (ReadModifyWrite unary) ──────────────────────────────

#[test]
fn pir_a_postfix_increment_is_modify() {
    // Use a declared variable so the kind is deterministic (Lexical→Modify).
    let graph = parse_and_lower("my $x = 0; $x++;");

    let modify = graph.nodes.iter().find(|n| {
        matches!(&n.operation, PirOperation::Modify { .. } | PirOperation::StashModify { .. })
    });
    assert!(
        modify.is_some(),
        "`$x++` must produce a Modify or StashModify operation; got ops: {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );
}

// ── 12. Modify evaluates place once (no duplicate target in receipt) ──────────

#[test]
fn pir_a_modify_place_evaluated_once() {
    // Use a declared variable to get Lexical→Modify path.
    let graph = parse_and_lower("my $x = 0; $x += 1;");

    // There must be exactly ONE Modify (or StashModify) op for $x,
    // not two (which would mean the place was evaluated twice — a bug).
    let modify_count = graph
        .nodes
        .iter()
        .filter(|n| match &n.operation {
            PirOperation::Modify { name, .. } => name.name == "x",
            PirOperation::StashModify { symbol, .. } => symbol.name == "x",
            _ => false,
        })
        .count();
    assert_eq!(
        modify_count, 1,
        "Modify must evaluate its place exactly once; got {modify_count} Modify ops for $x"
    );
}

// ── 13. Sub body operations ───────────────────────────────────────────────────

#[test]
fn pir_a_sub_body_produces_ops() {
    let graph = parse_and_lower("sub foo { my $x = 1; }");

    // The sub body `my $x = 1` must produce a LexicalWrite for $x.
    let write = graph
        .nodes
        .iter()
        .find(|n| matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "x"));
    assert!(
        write.is_some(),
        "sub body `my $x = 1` must produce LexicalWrite for $x; got: {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );
}

// ── 14. body_model_version guard ─────────────────────────────────────────────
// If body_model_version != HIR_BODY_MODEL_VERSION, lower_hir_bodies must
// return an empty or schema-mismatch graph (no operations from unversioned bodies).

#[test]
fn pir_a_body_model_version_check() {
    use perl_parser_core::hir::HIR_BODY_MODEL_VERSION;
    // The canonical path (lower_ast) always sets body_model_version correctly.
    // Verify that a normally lowered file passes the version check.
    let mut parser = Parser::new("my $x = 1;");
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);
    assert_eq!(
        hir.body_model_version, HIR_BODY_MODEL_VERSION,
        "lower_ast must set body_model_version = HIR_BODY_MODEL_VERSION"
    );
    let graph = lower_hir_bodies(&hir);
    // With correct version, must produce at least one op
    assert!(
        !graph.nodes.is_empty(),
        "lower_hir_bodies with correct body_model_version must produce ops"
    );
}

// ── 15. `our $x = $y;` — storage-aware Let: 1 StashWrite, 0 LexicalWrite ──────
// Regression guard for the Let arm bug: storage class was ignored → LexicalWrite
// was always emitted. Now `our` must produce exactly 1 StashWrite for $x and
// zero LexicalWrite ops for $x.

#[test]
fn pir_a_our_with_init_produces_stash_write_not_lexical() {
    let graph = parse_and_lower("our $x = $y;");

    // Exactly one StashWrite for $x
    let stash_write_count = graph
        .nodes
        .iter()
        .filter(
            |n| matches!(&n.operation, PirOperation::StashWrite { symbol } if symbol.name == "x"),
        )
        .count();
    assert_eq!(
        stash_write_count, 1,
        "`our $x = $y` must produce exactly 1 StashWrite for $x; got {stash_write_count}"
    );

    // Zero LexicalWrite for $x
    let lex_write_count = graph
        .nodes
        .iter()
        .filter(|n| matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "x"))
        .count();
    assert_eq!(
        lex_write_count, 0,
        "`our $x = $y` must produce 0 LexicalWrite for $x; got {lex_write_count}"
    );
}

// ── 16. `our $x += 1;` — StashModify from lower_variable_modify ──────────────
// `our $x` is a Package variable; compound assign on a Package place must
// produce a StashModify (not Modify).  This exercises the Package arm in
// lower_variable_modify, which was previously uncovered.

#[test]
fn pir_a_our_compound_assign_produces_stash_modify() {
    // Declare our $x so it is known as a package variable, then compound-assign.
    let graph = parse_and_lower("our $x; $x += 1;");

    let stash_modify = graph.nodes.iter().find(
        |n| matches!(&n.operation, PirOperation::StashModify { symbol, .. } if symbol.name == "x"),
    );
    assert!(
        stash_modify.is_some(),
        "`our $x += 1` must produce StashModify for $x; got ops: {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );

    // Must not produce a Lexical Modify for $x
    let lex_modify = graph
        .nodes
        .iter()
        .find(|n| matches!(&n.operation, PirOperation::Modify { name, .. } if name.name == "x"));
    assert!(
        lex_modify.is_none(),
        "`our $x += 1` must NOT produce Modify (lexical) for $x; got: {:?}",
        lex_modify.map(|n| n.operation.name())
    );
}

// ── 17. Version-mismatch empty graph ─────────────────────────────────────────
// When body_model_version != HIR_BODY_MODEL_VERSION, lower_hir_bodies must
// return an empty graph and record the mismatch in ambient_inputs.

#[test]
fn pir_a_version_mismatch_yields_empty_graph_with_ambient_input() {
    use perl_parser_core::hir::HIR_BODY_MODEL_VERSION;

    let mut parser = Parser::new("my $x = 1;");
    let output = parser.parse_with_recovery();
    let mut hir = lower_ast(&output.ast);

    // Corrupt the version to trigger the fail-closed path.
    // body_model_version 0 means "second pass not yet run" — it is never a valid
    // post-lower value, so it reliably triggers the mismatch guard.
    hir.body_model_version = 0;
    assert_ne!(hir.body_model_version, HIR_BODY_MODEL_VERSION);

    let graph = lower_hir_bodies(&hir);

    assert_eq!(graph.nodes.len(), 0, "version-mismatch must yield empty node list");
    assert_eq!(graph.edges.len(), 0, "version-mismatch must yield empty edge list");
    assert!(
        !graph.receipt.ambient_inputs.is_empty(),
        "version-mismatch must record mismatch in ambient_inputs"
    );
    assert!(
        graph.receipt.ambient_inputs[0].contains("body_model_version"),
        "ambient_input must mention body_model_version; got: {:?}",
        graph.receipt.ambient_inputs[0]
    );
}

// ── 17b. body_model_version threshold boundary (below / equal / above) ────────
// ripr seam proof for the schema-version predicate in
// `lower_hir_bodies_with_identity` (`file.body_model_version != HIR_BODY_MODEL_VERSION`).
// Pins the equality boundary from BOTH sides with *literal* version values so a
// mutation that removes or flips the guard is caught: only the exact match version
// lowers the body; below and above fail-closed to an empty graph with the mismatch
// recorded in ambient_inputs.

#[test]
fn pir_a_body_model_version_threshold_below_equal_above() {
    use perl_parser_core::hir::HIR_BODY_MODEL_VERSION;

    // A real non-empty HIR (`my $x = 1;` lowers to a LexicalWrite) so the body
    // WOULD emit a node if the version guard were bypassed.
    let build = || {
        let mut parser = Parser::new("my $x = 1;");
        let output = parser.parse_with_recovery();
        lower_ast(&output.ast)
    };

    // EQUAL: body_model_version == HIR_BODY_MODEL_VERSION → lowering proceeds.
    let mut equal = build();
    equal.body_model_version = HIR_BODY_MODEL_VERSION;
    let g = lower_hir_bodies(&equal);
    assert!(
        !g.nodes.is_empty(),
        "version == HIR_BODY_MODEL_VERSION must lower the body (got empty graph)"
    );
    assert!(
        g.receipt.ambient_inputs.iter().all(|s| !s.contains("body_model_version mismatch")),
        "matching version must NOT record a version mismatch; got: {:?}",
        g.receipt.ambient_inputs
    );

    // BELOW: version < HIR_BODY_MODEL_VERSION → fail-closed empty graph.
    let mut below = build();
    below.body_model_version = HIR_BODY_MODEL_VERSION - 1;
    let g = lower_hir_bodies(&below);
    assert_eq!(g.nodes.len(), 0, "below-threshold version must yield an empty graph");
    assert!(
        g.receipt.ambient_inputs.iter().any(|s| s.contains("body_model_version mismatch")),
        "below-threshold version must record the mismatch; got: {:?}",
        g.receipt.ambient_inputs
    );

    // ABOVE: version > HIR_BODY_MODEL_VERSION → fail-closed empty graph.
    let mut above = build();
    above.body_model_version = HIR_BODY_MODEL_VERSION + 1;
    let g = lower_hir_bodies(&above);
    assert_eq!(g.nodes.len(), 0, "above-threshold version must yield an empty graph");
    assert!(
        g.receipt.ambient_inputs.iter().any(|s| s.contains("body_model_version mismatch")),
        "above-threshold version must record the mismatch; got: {:?}",
        g.receipt.ambient_inputs
    );
}

// ── 18. `lower_hir_bodies_with_identity` threads source identity ──────────────
// Exercises the `source_identity: Some(...)` path in the bodies lowerer,
// which was previously only tested via the None path (lower_hir_bodies).

#[test]
fn pir_a_bodies_source_identity_threaded_to_receipt() {
    let mut parser = Parser::new("my $x = 1;");
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);

    let graph = lower_hir_bodies_with_identity(&hir, Some("fixture://test.pl".to_string()));
    assert_eq!(
        graph.receipt.source_identity.as_deref(),
        Some("fixture://test.pl"),
        "lower_hir_bodies_with_identity must thread source_identity to receipt"
    );
    // Must still produce operations (version is correct)
    assert!(!graph.nodes.is_empty(), "must produce ops with valid version and source_identity");
}

// ── 19. Unary Read (`-$x`) — lowers operand as Read, no Modify ───────────────
// `UnaryMode::Read` in `lower_expr` (negation, logical-not, etc.) must lower
// its operand as a read expression, producing a Read op — not a Modify.

#[test]
fn pir_a_unary_read_lowers_operand() {
    // `-$x` is a unary read; the operand $x should appear as a Read in the receipt.
    // Use a declared variable so the kind is deterministic.
    let graph = parse_and_lower("my $x = 1; my $y = -$x;");

    // $x must appear as a Read (not a Modify) when used as a unary-read operand.
    let read_x_lexical = graph
        .nodes
        .iter()
        .find(|n| matches!(&n.operation, PirOperation::LexicalRead { name } if name.name == "x"));
    let read_x_stash = graph
        .nodes
        .iter()
        .find(|n| matches!(&n.operation, PirOperation::StashRead { symbol } if symbol.name == "x"));
    assert!(
        read_x_lexical.is_some() || read_x_stash.is_some(),
        "unary-read operand $x must produce a Read op; got ops: {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );

    // Must NOT produce a Modify for $x from this unary-read context
    let modify_x = graph.nodes.iter().find(|n| match &n.operation {
        PirOperation::Modify { name, .. } => name.name == "x",
        PirOperation::StashModify { symbol, .. } => symbol.name == "x",
        _ => false,
    });
    assert!(
        modify_x.is_none(),
        "unary-read operand $x must NOT produce a Modify; got: {:?}",
        modify_x.map(|n| n.operation.name())
    );
}

// ── 20. `foo($x)` in body — Call unsupported, $x produces Read ───────────────
// Exercises the `HirExpr::Call` arm in `lower_expr` (body arenas), which records
// calls as unsupported in the receipt. The argument $x should still appear as a Read.
// Note: the body lowerer may or may not reach the call's args depending on whether
// the parser represents the call in the body arena. What we assert: the receipt
// does not produce a Write for $x (the argument) and the graph is valid.

#[test]
fn pir_a_call_in_body_does_not_produce_write_for_arg() {
    // `foo($x)` — $x is an argument (read position), not a write target.
    let graph = parse_and_lower("foo($x);");

    // $x must not appear as a Write target
    let write_x = graph.nodes.iter().find(|n| match &n.operation {
        PirOperation::LexicalWrite { name } => name.name == "x",
        PirOperation::StashWrite { symbol } => symbol.name == "x",
        _ => false,
    });
    assert!(
        write_x.is_none(),
        "`foo($x)` must not produce a Write for the argument $x; got: {:?}",
        write_x.map(|n| n.operation.name())
    );
    // The graph must be valid (receipt schema version correct, nodes consistent)
    assert_eq!(graph.receipt.schema_version, PIR_RECEIPT_VERSION);
}

// ── 21. Array sigil `@arr` — sigil_str emits `@`, not `$` ───────────────────
// Exercises `sigil_str` for the Array variant. Without a test, the `@`, `%`,
// `&`, `*` arms in `sigil_str` are untouched by the patch and fail coverage.

#[test]
fn pir_a_array_var_sigil_is_at_sign() {
    // `my @arr = ();` — declaration of an array variable. The LexicalWrite for
    // @arr must carry the `@` sigil, not `$`.
    let graph = parse_and_lower("my @arr = ();");

    let write = graph.nodes.iter().find(
        |n| matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "arr"),
    );
    assert!(
        write.is_some(),
        "`my @arr = ()` must produce a LexicalWrite for @arr; ops: {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );

    // Sigil must be `@`.
    if let Some(node) = write {
        if let PirOperation::LexicalWrite { name } = &node.operation {
            assert_eq!(
                name.sigil, "@",
                "LexicalWrite for @arr must have sigil `@`, got `{}`",
                name.sigil
            );
        }
    }
}

// ── 22. Hash sigil `%h` — sigil_str emits `%` ────────────────────────────────

#[test]
fn pir_a_hash_var_sigil_is_percent() {
    let graph = parse_and_lower("my %h;");

    let write = graph
        .nodes
        .iter()
        .find(|n| matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "h"));
    assert!(write.is_some(), "`my %h` must produce a LexicalWrite for %h");

    if let Some(node) = write {
        if let PirOperation::LexicalWrite { name } = &node.operation {
            assert_eq!(
                name.sigil, "%",
                "LexicalWrite for %h must have sigil `%`, got `{}`",
                name.sigil
            );
        }
    }
}

// ── 24. `local $x;` — lowers to StashWrite, 0 LexicalWrite (#2612) ───────────
// Regression guard for the PIR-A body path: `local` dynamically scopes a
// package/global slot, so `local $x` must lower to StashWrite (not LexicalWrite).
// The flat-items path (`lower_hir`) handled this correctly; the body-arena path
// (`lower_hir_bodies`) had `DeclStorageClass::Local` falling into the `_` arm
// (→ LexicalWrite) before this fix.

#[test]
fn pir_a_local_declaration_is_stash_write() {
    let graph = parse_and_lower("local $x;");

    // Must produce at least one StashWrite for $x.
    let stash_write = graph.nodes.iter().find(
        |n| matches!(&n.operation, PirOperation::StashWrite { symbol } if symbol.name == "x"),
    );
    assert!(
        stash_write.is_some(),
        "`local $x` must lower to StashWrite in the PIR-A body path; got ops: {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );

    // Must produce 0 LexicalWrite nodes for $x (regression guard).
    let lex_write_count = graph
        .nodes
        .iter()
        .filter(|n| matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "x"))
        .count();
    assert_eq!(
        lex_write_count, 0,
        "`local $x` must NOT produce any LexicalWrite for $x; got {lex_write_count}"
    );

    // Sigil must be `$`.
    if let Some(node) = stash_write {
        if let PirOperation::StashWrite { symbol } = &node.operation {
            assert_eq!(
                symbol.sigil, "$",
                "StashWrite for local $x must carry sigil `$`, got `{}`",
                symbol.sigil
            );
        }
    }
}

// ── 23. Opaque function call counted in unsupported receipt ───────────────────
// `foo($x)` in a body arena lowers to `HirExpr::Opaque { ast_kind: "FunctionCall" }`,
// which hits `ast_kind_to_static("FunctionCall") → "OpaqueCall"`. The receipt must
// record this as unsupported rather than silently dropping it or crashing.

#[test]
fn pir_a_opaque_function_call_counted_as_unsupported() {
    let graph = parse_and_lower("foo($x);");

    // The call must be recorded in unsupported_construct_counts, not dropped.
    // It will appear as "OpaqueCall" or "Call" depending on whether HIR body
    // lowering emits an Opaque or a Call node.
    let has_unsupported_call =
        graph.receipt.unsupported_construct_counts.contains_key("OpaqueCall")
            || graph.receipt.unsupported_construct_counts.contains_key("Call");

    assert!(
        has_unsupported_call,
        "`foo($x)` in body must record an unsupported call op in the receipt; got: {:?}",
        graph.receipt.unsupported_construct_counts
    );
}

// ── 24. Opaque method call counted in unsupported receipt ────────────────────
// `$obj->method()` lowers to `HirExpr::Opaque { ast_kind: "MethodCall" }` in
// body arenas, hitting `ast_kind_to_static("MethodCall") → "OpaqueMethodCall"`.

#[test]
fn pir_a_opaque_method_call_counted_as_unsupported() {
    let graph = parse_and_lower("$obj->method();");

    let has_unsupported_method =
        graph.receipt.unsupported_construct_counts.contains_key("OpaqueMethodCall")
            || graph.receipt.unsupported_construct_counts.contains_key("Call")
            || graph.receipt.unsupported_construct_counts.contains_key("OpaqueExpr");

    assert!(
        has_unsupported_method,
        "`$obj->method()` in body must record an unsupported op in the receipt; got: {:?}",
        graph.receipt.unsupported_construct_counts
    );

    // Must not produce a spurious Write for $obj
    let write_obj = graph.nodes.iter().find(|n| match &n.operation {
        PirOperation::LexicalWrite { name } => name.name == "obj",
        PirOperation::StashWrite { symbol } => symbol.name == "obj",
        _ => false,
    });
    assert!(
        write_obj.is_none(),
        "`$obj->method()` must not produce a Write for $obj; got: {:?}",
        write_obj.map(|n| n.operation.name())
    );
}

// ── 25. Cross-body no spurious fallthrough edges ─────────────────────────────
// When a file has both a subroutine body and the program-root body, the last
// PIR node of the sub body must NOT be connected by a Fallthrough edge to the
// first PIR node of the program-root body. Bodies are independent control-flow
// regions — `last_in_scope` must be cleared at the start of each body.

#[test]
fn pir_a_no_cross_body_fallthrough_edges() {
    use perl_parser_core::pir::PirEdgeKind;

    // Source with two bodies: the subroutine body and the program-root body.
    // Each body has exactly one PIR node. If `last_in_scope` is not cleared
    // between bodies, body 0's last node is connected to body 1's first node.
    let graph = parse_and_lower("sub foo { my $x = 1; } my $y = 2;");

    // Collect all Fallthrough edges.
    let fallthroughs: Vec<_> =
        graph.edges.iter().filter(|e| e.kind == PirEdgeKind::Fallthrough).collect();

    // In a two-body graph, there must be AT MOST one Fallthrough edge per body
    // (connecting consecutive nodes WITHIN the same body). There must be ZERO
    // cross-body Fallthrough edges.
    //
    // Specifically: if both bodies produce at least one node, a cross-body edge
    // would connect the last node of body 0 to the first node of body 1.
    // We detect this conservatively: if there are N nodes total in two separate
    // bodies, any fallthrough from the last node of body 0 to the first of body 1
    // would be the only edge that crosses the body boundary. With 2+ nodes and
    // 1 edge, the edge is internal. With 3+ nodes and edges where the from-node
    // belongs to one body and to-node to another, that is the bug.
    //
    // Simplest check: with only ONE node in each body (no internal edges needed),
    // there must be ZERO Fallthrough edges total.
    let sub_body_nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| {
            // Nodes in the sub body ($x) vs program root ($y)
            matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "x")
        })
        .collect();
    let root_body_nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| matches!(&n.operation, PirOperation::LexicalWrite { name } if name.name == "y"))
        .collect();

    if !sub_body_nodes.is_empty() && !root_body_nodes.is_empty() {
        let sub_last = sub_body_nodes.last().expect("sub body node");
        let root_first = root_body_nodes.first().expect("root body node");

        // Check no Fallthrough edge goes from sub-body-last to root-body-first.
        let cross_body_edge =
            fallthroughs.iter().find(|e| e.from == sub_last.id && e.to == Some(root_first.id));
        assert!(
            cross_body_edge.is_none(),
            "must not have a cross-body Fallthrough edge from $x (sub body) to $y (root body); \
             found: {:?}",
            cross_body_edge
        );
    }
}

// ── 26. Leading-`::` qualified var — no empty-string package ─────────────────
// `$::x` is a main-package global written as a leading-`::` name. The
// `package_from_name` helper must not emit `package: Some("")` for this; the
// empty prefix from `rsplit_once("::")` must be filtered to `None`.

#[test]
fn pir_a_leading_colon_var_no_empty_package() {
    let graph = parse_and_lower("$::x = 1;");

    // Find any StashWrite or StashRead for a variable with name "x" or "::x".
    for node in &graph.nodes {
        if let PirOperation::StashWrite { symbol } | PirOperation::StashRead { symbol } =
            &node.operation
        {
            if symbol.name.contains('x') {
                assert_ne!(
                    symbol.package.as_deref(),
                    Some(""),
                    "package must not be Some(\"\") for leading-`::` name `$::x`; \
                     got symbol={:?}",
                    symbol
                );
            }
        }
    }

    // The graph itself must be valid.
    assert_eq!(graph.receipt.schema_version, PIR_RECEIPT_VERSION);
}

// ── 27. Opaque literal/generic expression counted in unsupported ──────────────
// Exercises the `_ => "OpaqueExpr"` fallthrough in `ast_kind_to_static`.
// A numeric literal (e.g. `1`) in expression-statement position produces an
// Opaque node with an ast_kind that doesn't match the specific match arms.

#[test]
fn pir_a_opaque_literal_counted_as_unsupported_expr() {
    // `1;` — a bare numeric literal in statement position.
    // In the body arena this becomes an Opaque node (the literal is not a
    // modeled HIR expression in this slice). The receipt must record it.
    let graph = parse_and_lower("1;");

    // The graph may be empty (if the literal collapses to nothing) or have an
    // opaque entry. What we assert: the receipt schema is valid and no panic
    // occurred. Additionally, if the literal DID produce a receipt entry, it
    // must be in unsupported_construct_counts (not misclassified as a variable op).
    assert_eq!(
        graph.receipt.schema_version, PIR_RECEIPT_VERSION,
        "bare literal `1;` must produce a valid receipt"
    );
    // No LexicalWrite or StashWrite from a bare literal
    let write_nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                &n.operation,
                PirOperation::LexicalWrite { .. } | PirOperation::StashWrite { .. }
            )
        })
        .collect();
    assert!(
        write_nodes.is_empty(),
        "bare literal `1;` must not produce any Write ops; got: {:?}",
        write_nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );
}

// ── 28. RMW variable reaching lower_variable_expr → fail-closed, no wrong op ──
// Exercises the early-return guard in `lower_variable_expr` for the
// `AccessMode::ReadModifyWrite` case. In the current HIR/PIR design this path
// is structurally unreachable through the normal parser path (RMW variables are
// always handled by `lower_variable_modify` before `lower_expr` is called). This
// test constructs a synthetic HIR body where a `Variable { RMW }` node appears
// as a standalone statement expression — the only way to reach the guard.
//
// Expected: no node emitted for the RMW variable (fail-closed); the receipt
// records it in `unsupported_construct_counts` under "RmwVariableFallthrough".

#[test]
fn pir_a_rmw_variable_in_expr_position_is_fail_closed() {
    use perl_parser_core::hir::{HIR_BODY_MODEL_VERSION, HirFile};

    // Build a synthetic HIR body containing exactly one statement:
    // `HirStmt::Expr(rmw_var_id)` where the expression is
    // `HirExpr::Variable { access: ReadModifyWrite, kind: Lexical }`.
    let loc = SourceLocation { start: 0, end: 3 };

    let mut exprs: Arena<HirExpr> = Arena::default();
    let rmw_var = HirExpr::Variable(HirVariable {
        sigil: Sigil::Scalar,
        name: "x".to_string(),
        kind: VariableKind::Lexical,
        access: AccessMode::ReadModifyWrite,
    });
    let expr_idx = exprs.alloc(rmw_var);
    let expr_id = HirExprId(expr_idx);

    let mut stmts: Arena<HirStmt> = Arena::default();
    let stmt_idx = stmts.alloc(HirStmt::Expr(expr_id));
    let stmt_id = HirStmtId(stmt_idx);

    let mut blocks: Arena<HirBlock> = Arena::default();
    let mut root_block = HirBlock::default();
    root_block.stmts.push(stmt_id);
    let block_idx = blocks.alloc(root_block);
    let root_block_id = HirBlockId(block_idx);

    let source_map =
        BodySourceMap { expr_ranges: vec![loc], stmt_ranges: vec![loc], block_ranges: vec![loc] };

    let body = HirBody {
        exprs,
        stmts,
        blocks,
        source_map,
        root_block: root_block_id,
        owner: BodyOwnerKind::ProgramRoot,
    };

    let mut file = HirFile::default();
    file.body_model_version = HIR_BODY_MODEL_VERSION;
    file.bodies.push(body);

    let graph = lower_hir_bodies(&file);

    // The RMW variable must produce NO PIR node (fail-closed, no wrong fact).
    assert_eq!(
        graph.nodes.len(),
        0,
        "RMW variable in standalone expr position must not produce any PIR node; got: {:?}",
        graph.nodes.iter().map(|n| n.operation.name()).collect::<Vec<_>>()
    );

    // The receipt must record the gap EXACTLY ONCE under "RmwVariableFallthrough".
    // Asserting the exact count (not just key presence) is the ripr seam proof: it
    // kills the `+= 1` → `+= 0` mutant on the counter increment in
    // `lower_variable_expr` — `.or_insert(0)` would still insert the key with value
    // 0, so a `contains_key` assertion would survive that mutation.
    assert_eq!(
        graph.receipt.unsupported_construct_counts.get("RmwVariableFallthrough").copied(),
        Some(1),
        "RMW variable fallthrough must be recorded exactly once; got: {:?}",
        graph.receipt.unsupported_construct_counts
    );
}

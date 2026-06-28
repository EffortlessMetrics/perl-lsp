/// Test suite for lexical reference extraction (#2577 PR1).
///
/// These tests verify that the extractor correctly:
/// - Isolates scope by body
/// - Captures all reads and writes of `my`/`state` variables
/// - Skips Modify operations
/// - Handles empty bodies without panic
/// - Maintains receipt invariants
use perl_parser_core::{Parser, hir::lower_ast, pir::extract_lexical_facts};

/// Helper to extract lexical facts from a Perl source string.
fn extract(source: &str) -> perl_parser_core::pir::LexicalExtractorReceipt {
    let mut parser = Parser::new(source);
    let output = parser.parse_with_recovery();
    let hir = lower_ast(&output.ast);
    extract_lexical_facts(&hir)
}

/// Fixture 1: Scope isolation with shadowed names.
///
/// Verifies that an outer `$x` in the program root is isolated from an inner `$x`
/// in a subroutine, even though they have the same name.
///
/// Expected:
/// - ProgramRoot (body_idx=0): 1 Write + 3 Reads of outer $x
/// - Subroutine foo (body_idx=1): 1 Write + 1 Read of inner $x
/// - All facts must be source-anchored
/// - Outer and inner must not merge across bodies
#[test]
fn fixture_1_scope_isolation_shadowed_names() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
my $x = 5;
print $x;
$x = 10;
say $x;
sub foo { my $x = 20; return $x; }
say $x;
"#;
    let receipt = extract(source);

    // Schema version must be correct
    assert_eq!(receipt.schema_version, 1, "schema version must be 1");

    // Provider behavior must not change
    assert!(!receipt.provider_behavior_changed, "PR1 must not change provider behavior");

    // Must have exactly 2 bodies: ProgramRoot + foo
    assert_eq!(receipt.bodies.len(), 2, "must have exactly 2 bodies (ProgramRoot + foo)");

    // ProgramRoot (body_idx=0)
    let prog_body = &receipt.bodies[0];
    assert_eq!(prog_body.body_idx, 0, "program root must have body_idx=0");

    // Filter for $x facts in program root
    let prog_x_facts: Vec<_> =
        prog_body.facts.iter().filter(|f| f.name.name == "x" && f.name.sigil == "$").collect();

    let prog_reads = prog_x_facts
        .iter()
        .filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Read))
        .count();
    let prog_writes = prog_x_facts
        .iter()
        .filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Write))
        .count();

    assert_eq!(prog_reads, 3, "outer $x should have 3 reads (print, say, say)");
    // Correction: the source has both `my $x = 5` (declaration write) and `$x = 10`
    // (assignment write), so there are 2 LexicalWrite facts for outer $x.
    // The original red-TDD expected 1 write but forgot the plain assignment `$x = 10`.
    assert_eq!(prog_writes, 2, "outer $x should have 2 writes (declaration + assignment)");

    // All program root facts must be anchored
    for fact in &prog_x_facts {
        assert!(fact.source_anchor.is_anchored(), "all program root facts must be source-anchored");
    }

    // foo body (body_idx=1)
    let foo_body = &receipt.bodies[1];
    assert_eq!(foo_body.body_idx, 1, "foo body must have body_idx=1");

    // Filter for $x facts in foo
    let foo_x_facts: Vec<_> =
        foo_body.facts.iter().filter(|f| f.name.name == "x" && f.name.sigil == "$").collect();

    let foo_reads = foo_x_facts
        .iter()
        .filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Read))
        .count();
    let foo_writes = foo_x_facts
        .iter()
        .filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Write))
        .count();

    assert_eq!(foo_reads, 1, "foo's $x should have 1 read (return)");
    assert_eq!(foo_writes, 1, "foo's $x should have 1 write (declaration)");

    // All foo facts must be anchored
    for fact in &foo_x_facts {
        assert!(fact.source_anchor.is_anchored(), "all foo facts must be source-anchored");
    }

    // Verify bodies are in separate entries (not merged)
    assert_ne!(
        receipt.bodies[0].body_idx, receipt.bodies[1].body_idx,
        "scope isolation: outer and foo must be separate body entries"
    );

    Ok(())
}

/// Fixture 2: State variables.
///
/// Verifies that `state` variables are treated as lexical reads/writes just like `my`.
///
/// Expected:
/// - ProgramRoot (body_idx=0): no lexical facts
/// - Subroutine counter (body_idx=1): 1 Write + 1 Read of state $n
#[test]
fn fixture_2_state_variable() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
sub counter { state $n = 0; return $n; }
"#;
    let receipt = extract(source);

    // Schema version
    assert_eq!(receipt.schema_version, 1, "schema version must be 1");

    // Provider behavior
    assert!(!receipt.provider_behavior_changed, "PR1 must not change provider behavior");

    // ProgramRoot (body_idx=0): no facts expected
    let prog_body = &receipt.bodies[0];
    assert!(
        prog_body.facts.is_empty(),
        "program root should have no lexical facts (declaration only)"
    );

    // counter body (body_idx=1)
    let counter_body = &receipt.bodies[1];

    let counter_n_facts: Vec<_> =
        counter_body.facts.iter().filter(|f| f.name.name == "n" && f.name.sigil == "$").collect();

    let reads = counter_n_facts
        .iter()
        .filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Read))
        .count();
    let writes = counter_n_facts
        .iter()
        .filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Write))
        .count();

    assert_eq!(reads, 1, "state $n should have 1 read (return)");
    assert_eq!(writes, 1, "state $n should have 1 write (declaration)");

    Ok(())
}

/// Fixture 3: Modify nodes are skipped.
///
/// Verifies that compound operators like `$x++` are recorded in skipped_node_count
/// and do NOT count as Read or Write in the facts.
///
/// Expected:
/// - ProgramRoot: 1 Write (from `my $x = 0`), 0 Reads
/// - skipped_node_count >= 1 (the Modify operation)
#[test]
fn fixture_3_modify_nodes_skipped() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 0; $x++;";
    let receipt = extract(source);

    // Schema version
    assert_eq!(receipt.schema_version, 1, "schema version must be 1");

    // Provider behavior
    assert!(!receipt.provider_behavior_changed, "PR1 must not change provider behavior");

    // ProgramRoot
    let prog_body = &receipt.bodies[0];
    let x_facts: Vec<_> =
        prog_body.facts.iter().filter(|f| f.name.name == "x" && f.name.sigil == "$").collect();

    let reads = x_facts
        .iter()
        .filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Read))
        .count();
    let writes = x_facts
        .iter()
        .filter(|f| matches!(f.role, perl_parser_core::pir::LexicalRole::Write))
        .count();

    assert_eq!(reads, 0, "$x++ must NOT count as a Read (Modify is skipped)");
    assert_eq!(writes, 1, "should have 1 write from declaration");

    // Skipped count must track the Modify operation
    assert!(receipt.skipped_node_count >= 1, "Modify should be counted in skipped_node_count");

    Ok(())
}

/// Fixture 4: Empty subroutine body.
///
/// Verifies that an empty subroutine body does not panic and yields
/// empty facts with 0 nodes.
///
/// Expected:
/// - ProgramRoot (body_idx=0): no facts
/// - empty (body_idx=1): facts=[], total_node_count=0
#[test]
fn fixture_4_empty_sub_body() -> Result<(), Box<dyn std::error::Error>> {
    let source = "sub empty {}";
    let receipt = extract(source);

    // Schema version
    assert_eq!(receipt.schema_version, 1, "schema version must be 1");

    // Provider behavior
    assert!(!receipt.provider_behavior_changed, "PR1 must not change provider behavior");

    // Should have 2 bodies
    assert_eq!(receipt.bodies.len(), 2, "must have 2 bodies (ProgramRoot + empty)");

    // empty sub (body_idx=1)
    let empty_body = &receipt.bodies[1];
    assert_eq!(empty_body.body_idx, 1, "empty sub must have body_idx=1");
    assert!(empty_body.facts.is_empty(), "empty body should have no facts");
    assert_eq!(empty_body.total_node_count, 0, "empty body should have 0 total nodes");

    Ok(())
}

/// Fixture 5: Receipt invariants.
///
/// Verifies that the receipt structure maintains all required invariants:
/// - schema_version == LEXICAL_EXTRACTOR_RECEIPT_VERSION
/// - provider_behavior_changed == false
/// - total_read_count + total_write_count == sum(bodies[].facts.len())
#[test]
fn fixture_5_receipt_invariants() -> Result<(), Box<dyn std::error::Error>> {
    // Source produces facts across 2 bodies with both reads and writes:
    //   body 0 (ProgramRoot): `my $x = 1` → Write($x), `print $x` → Read($x)
    //   body 1 (foo): `my $y = 2` → Write($y), `return $y` → Read($y)
    // No Modify operations → skipped_node_count == 0
    let source = r#"
my $x = 1;
print $x;
sub foo { my $y = 2; return $y; }
"#;
    let receipt = extract(source);

    // schema_version invariant
    assert_eq!(receipt.schema_version, 1, "schema_version must be 1");
    assert_eq!(
        receipt.schema_version,
        perl_parser_core::pir::LEXICAL_EXTRACTOR_RECEIPT_VERSION,
        "schema_version must match LEXICAL_EXTRACTOR_RECEIPT_VERSION constant"
    );

    // provider_behavior_changed invariant: must always be false for PR1
    assert!(
        !receipt.provider_behavior_changed,
        "provider_behavior_changed must be false (PR1 is non-invasive)"
    );

    // Non-trivial count assertions: both bodies must yield facts.
    // If the extractor mistakenly flattens bodies or drops them, these fail.
    assert_eq!(receipt.bodies.len(), 2, "must have 2 bodies (ProgramRoot + foo)");
    assert_eq!(
        receipt.total_read_count, 2,
        "total_read_count: 1 from print $x (body 0) + 1 from return $y (body 1)"
    );
    assert_eq!(
        receipt.total_write_count, 2,
        "total_write_count: 1 from my $x=1 (body 0) + 1 from my $y=2 (body 1)"
    );
    // No Modify ops in this source → skipped count must be 0.
    assert_eq!(
        receipt.skipped_node_count, 0,
        "no Modify operations in source, so skipped_node_count must be 0"
    );

    // Count invariant: total_read_count + total_write_count == sum of all facts
    let sum_facts: usize = receipt.bodies.iter().map(|b| b.facts.len()).sum();
    assert_eq!(sum_facts, 4, "total facts must be 4 (2 reads + 2 writes across 2 bodies)");
    assert_eq!(
        receipt.total_read_count + receipt.total_write_count,
        sum_facts,
        "count invariant: total_read + total_write must equal sum of all facts"
    );

    Ok(())
}

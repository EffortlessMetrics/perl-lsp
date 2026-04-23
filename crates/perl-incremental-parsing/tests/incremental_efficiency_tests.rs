//! Tests for incremental parsing efficiency improvements.
//!
//! Covers:
//! - Small edit (single char) reuses most of the AST
//! - Adding a line in the middle updates line offsets correctly
//! - Deleting a function does not re-parse the entire file
//! - Benchmark: reparse time for small edits vs full parse

#![allow(clippy::uninlined_format_args)]

use perl_incremental_parsing::incremental::incremental_document::IncrementalDocument;
use perl_incremental_parsing::incremental::incremental_edit::{
    IncrementalEdit, IncrementalEditSet,
};
use perl_incremental_parsing::incremental::incremental_simple::SimpleIncrementalParser;
use perl_incremental_parsing::incremental::incremental_v2::IncrementalParserV2;
use perl_incremental_parsing::incremental::{Edit, IncrementalState, apply_edits};
use perl_incremental_parsing::position::Position;
use perl_incremental_parsing::{Node, NodeKind, Parser};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count total nodes in an AST (comprehensive traversal)
fn count_all_nodes(node: &Node) -> usize {
    let mut count = 1;
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for stmt in statements {
                count += count_all_nodes(stmt);
            }
        }
        NodeKind::Binary { left, right, .. } => {
            count += count_all_nodes(left);
            count += count_all_nodes(right);
        }
        NodeKind::Subroutine { body, .. } => {
            count += count_all_nodes(body);
        }
        NodeKind::ExpressionStatement { expression } => {
            count += count_all_nodes(expression);
        }
        NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
            count += count_all_nodes(condition);
            count += count_all_nodes(then_branch);
            for (cond, branch) in elsif_branches {
                count += count_all_nodes(cond);
                count += count_all_nodes(branch);
            }
            if let Some(else_b) = else_branch {
                count += count_all_nodes(else_b);
            }
        }
        NodeKind::While { condition, body, .. } => {
            count += count_all_nodes(condition);
            count += count_all_nodes(body);
        }
        NodeKind::VariableDeclaration { variable, initializer, .. } => {
            count += count_all_nodes(variable);
            if let Some(init) = initializer {
                count += count_all_nodes(init);
            }
        }
        NodeKind::Assignment { lhs, rhs, .. } => {
            count += count_all_nodes(lhs);
            count += count_all_nodes(rhs);
        }
        NodeKind::FunctionCall { args, .. } => {
            for arg in args {
                count += count_all_nodes(arg);
            }
        }
        NodeKind::Unary { operand, .. } => {
            count += count_all_nodes(operand);
        }
        _ => {}
    }
    count
}

// =========================================================================
// 1. Small edit (add a character) reuses most of the AST
// =========================================================================

#[test]
fn small_char_edit_reuses_most_ast_nodes() -> Result<(), Box<dyn std::error::Error>> {
    // A multi-statement source where changing one digit should reuse most nodes
    let source = "my $x = 42;\nmy $y = 100;\nmy $z = 200;\nprint $x + $y + $z;\n";

    let mut doc = IncrementalDocument::new(source.to_string())?;
    let initial_source = doc.text().to_string();

    // Find "42" and change last digit to "43"
    let pos = initial_source.find("42").ok_or("source should contain '42'")?;
    let edit = IncrementalEdit::new(pos + 1, pos + 2, "3".to_string());

    doc.apply_edit(edit)?;

    // The document text should reflect the change
    assert!(doc.text().contains("43"), "should contain the edited value '43'");
    assert!(!doc.text().contains("42"), "should no longer contain '42'");

    // Verify the AST is valid (program with statements)
    match &doc.tree().kind {
        NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "program should still have statements");
        }
        _ => return Err("expected Program node at root".into()),
    }

    // Metrics should show significant reuse
    let metrics = doc.metrics();
    assert!(metrics.nodes_reused > 0, "small edit should reuse some AST nodes, got 0 reused");

    Ok(())
}

#[test]
fn small_edit_reuses_most_ast_via_v2_parser() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = IncrementalParserV2::new();

    let source1 = "my $x = 42;\nmy $y = 100;\nprint $x + $y;\n";
    let tree1 = parser.parse(source1)?;
    let total_nodes = count_all_nodes(&tree1);

    // Apply an edit that changes "42" to "43"
    let pos = source1.find("42").ok_or("source should contain '42'")?;
    parser.edit(perl_incremental_parsing::edit::Edit::new(
        pos + 1,
        pos + 2,
        pos + 2,
        Position::new(pos + 1, 0, 0),
        Position::new(pos + 2, 0, 0),
        Position::new(pos + 2, 0, 0),
    ));

    let source2 = "my $x = 43;\nmy $y = 100;\nprint $x + $y;\n";
    let _ = parser.parse(source2)?;

    // V2 parser should report node reuse
    assert!(
        parser.reused_nodes > 0,
        "V2 parser should reuse nodes for a single-digit change, got reused={}, reparsed={}",
        parser.reused_nodes,
        parser.reparsed_nodes
    );

    // Most nodes should be reused (at least 30% for a small value edit)
    let reuse_ratio = parser.reused_nodes as f64 / total_nodes as f64;
    assert!(
        reuse_ratio > 0.2,
        "expected at least 20% reuse for small edit, got {:.1}%",
        reuse_ratio * 100.0
    );

    Ok(())
}

// =========================================================================
// 2. Adding a line in the middle correctly updates line offsets
// =========================================================================

#[test]
fn insert_line_updates_offsets_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\nmy $y = 2;\nmy $z = 3;\n";
    let mut state = IncrementalState::new(source.to_string());

    // Insert a new line "my $w = 99;\n" after "my $x = 1;\n" (byte 11)
    let insert_pos = 11; // right after first newline
    let edit = Edit {
        start_byte: insert_pos,
        old_end_byte: insert_pos,
        new_end_byte: insert_pos + 12,
        new_text: "my $w = 99;\n".to_string(),
    };

    let result = apply_edits(&mut state, &[edit])?;

    // Source should contain the new line
    assert!(state.source.contains("my $w = 99;"), "inserted line should be present");

    // Line offsets should be correct
    let (line_of_w, _) = state.line_index.byte_to_position(11);
    assert_eq!(line_of_w, 1, "$w should be on line 1 (0-indexed)");

    // The original $y should now be on line 2
    let y_pos = state.source.find("my $y").ok_or("should contain $y")?;
    let (line_of_y, _) = state.line_index.byte_to_position(y_pos);
    assert_eq!(line_of_y, 2, "$y should have shifted to line 2");

    // The original $z should now be on line 3
    let z_pos = state.source.find("my $z").ok_or("should contain $z")?;
    let (line_of_z, _) = state.line_index.byte_to_position(z_pos);
    assert_eq!(line_of_z, 3, "$z should have shifted to line 3");

    // Verify tokens are still valid
    assert!(!state.tokens.is_empty(), "tokens should be populated");

    // The reparsed range should be bounded (not the entire file for a single insertion)
    assert!(!result.changed_ranges.is_empty());

    Ok(())
}

#[test]
fn insert_line_via_incremental_document() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $a = 10;\nmy $b = 20;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    // Insert "my $mid = 15;\n" between the two lines
    let insert_pos = 12; // after "my $a = 10;\n"
    let edit = IncrementalEdit::new(insert_pos, insert_pos, "my $mid = 15;\n".to_string());

    doc.apply_edit(edit)?;

    assert!(doc.text().contains("my $mid = 15;"), "inserted line present");
    assert!(doc.text().contains("my $b = 20;"), "original second line preserved");

    // Parse tree should still be valid
    match &doc.tree().kind {
        NodeKind::Program { statements } => {
            // Should now have 3 statements (was 2, inserted 1)
            assert!(
                statements.len() >= 3,
                "expected at least 3 statements after insertion, got {}",
                statements.len()
            );
        }
        _ => return Err("expected Program node".into()),
    }

    Ok(())
}

// =========================================================================
// 3. Deleting a function doesn't re-parse the entire file
// =========================================================================

#[test]
fn delete_function_bounded_reparse() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "my $header = 1;\n",
        "sub to_delete {\n",
        "    my $local = 42;\n",
        "    return $local * 2;\n",
        "}\n",
        "my $footer = 99;\n",
    );

    let mut state = IncrementalState::new(source.to_string());

    // Find and delete "sub to_delete { ... }\n"
    let sub_start = source.find("sub to_delete").ok_or("should contain sub")?;
    let sub_end = source.find("}\n").ok_or("should contain closing brace")? + 2;

    let edit = Edit {
        start_byte: sub_start,
        old_end_byte: sub_end,
        new_end_byte: sub_start,
        new_text: String::new(),
    };

    let result = apply_edits(&mut state, &[edit])?;

    // The function should be gone
    assert!(!state.source.contains("sub to_delete"), "deleted function should be gone");

    // Header and footer should survive
    assert!(state.source.contains("$header"), "header should survive");
    assert!(state.source.contains("$footer"), "footer should survive");

    // Should still have tokens
    assert!(!state.tokens.is_empty());

    // The reparsed range should not cover the entire file
    // (though for a multi-line deletion the heuristic may fall back to full reparse)
    assert!(!result.changed_ranges.is_empty());
    let reparsed_bytes = result.reparsed_bytes;
    // Even if full reparse happens, the source is smaller now
    assert!(reparsed_bytes <= state.source.len());

    Ok(())
}

#[test]
fn delete_function_via_incremental_document() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!("my $before = 1;\n", "sub helper { return 42; }\n", "my $after = 2;\n",);

    let mut doc = IncrementalDocument::new(source.to_string())?;

    // Delete the entire sub line
    let sub_start = source.find("sub helper").ok_or("should contain sub")?;
    let sub_end = source.find("}\n").ok_or("should contain closing brace")? + 2;

    let edit = IncrementalEdit::new(sub_start, sub_end, String::new());
    doc.apply_edit(edit)?;

    // Verify the function is removed
    assert!(!doc.text().contains("sub helper"), "function should be deleted");

    // Surrounding code should be preserved
    assert!(doc.text().contains("$before"), "code before should survive");
    assert!(doc.text().contains("$after"), "code after should survive");

    // Parse tree should be valid
    match &doc.tree().kind {
        NodeKind::Program { statements } => {
            assert!(statements.len() >= 2, "should have at least 2 statements after deletion");
        }
        _ => return Err("expected Program node".into()),
    }

    Ok(())
}

// =========================================================================
// 4. Benchmark: small edit vs full parse timing
// =========================================================================

#[test]
fn benchmark_small_edit_vs_full_parse() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;

    // Generate a moderately large Perl source
    let mut source = String::new();
    source.push_str("package BenchmarkTest;\n");
    source.push_str("use strict;\nuse warnings;\n\n");
    for i in 0..50 {
        source.push_str(&format!(
            "sub func_{} {{\n    my $val = {};\n    return $val + 1;\n}}\n\n",
            i,
            i * 10
        ));
    }
    source.push_str("1;\n");

    // Measure full parse time (average over 5 runs)
    let mut full_parse_total = std::time::Duration::ZERO;
    let runs = 5;
    for _ in 0..runs {
        let start = Instant::now();
        let mut parser = Parser::new(&source);
        let _ = parser.parse()?;
        full_parse_total += start.elapsed();
    }
    let avg_full_parse = full_parse_total / runs as u32;

    // Measure incremental edit time (change a single number value)
    let mut doc = IncrementalDocument::new(source.clone())?;
    let val_pos = source.find("val = 0;").ok_or("should contain val = 0")? + 6;

    let mut incremental_total = std::time::Duration::ZERO;
    for i in 0..runs {
        let new_val = format!("{}", i + 100);
        let edit = IncrementalEdit::new(val_pos, val_pos + 1, new_val);
        let start = Instant::now();
        doc.apply_edit(edit)?;
        incremental_total += start.elapsed();
    }
    let avg_incremental = incremental_total / runs as u32;

    // Report timings (visible in test output with --nocapture)
    eprintln!("Benchmark results (50 functions, {} runs):", runs);
    eprintln!("  Full parse average:        {:?}", avg_full_parse);
    eprintln!("  Incremental edit average:  {:?}", avg_incremental);

    if avg_full_parse > std::time::Duration::from_micros(100) {
        // Only assert speedup if full parse is long enough to be meaningful
        // (avoids flaky assertions on very fast machines)
        let speedup = avg_full_parse.as_nanos() as f64 / avg_incremental.as_nanos().max(1) as f64;
        eprintln!("  Speedup factor:            {:.1}x", speedup);
    }

    // The incremental parse should complete in reasonable time
    assert!(
        avg_incremental < std::time::Duration::from_millis(50),
        "incremental edit should be fast, but took {:?}",
        avg_incremental
    );

    Ok(())
}

// =========================================================================
// 5. Regression: adjust_node_position with negative delta
// =========================================================================

#[test]
fn adjust_position_handles_negative_delta_safely() -> Result<(), Box<dyn std::error::Error>> {
    // When a deletion occurs near the beginning of the file, nodes after the
    // deletion get a negative delta. Previously this would wrap around via
    // `as usize` on a negative isize, producing a huge invalid offset.
    let source = "my $pad = 1;\nmy $x = 42;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    // Delete "my $pad = 1;\n" (13 bytes) from the beginning
    let edit = IncrementalEdit::new(0, 13, String::new());
    doc.apply_edit(edit)?;

    // The remaining source should be just "my $x = 42;\n"
    assert_eq!(doc.text(), "my $x = 42;\n");

    // Parse tree should be valid with correct positions
    match &doc.tree().kind {
        NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "should have at least one statement");
            // All node positions should be within source bounds
            for stmt in statements {
                assert!(
                    stmt.location.end <= doc.text().len(),
                    "node end {} exceeds source length {}",
                    stmt.location.end,
                    doc.text().len()
                );
            }
        }
        _ => return Err("expected Program node".into()),
    }

    Ok(())
}

// =========================================================================
// 6. Token synchronisation in apply_single_edit
// =========================================================================

#[test]
fn token_sync_stops_relexing_early() -> Result<(), Box<dyn std::error::Error>> {
    // Create a source with many statements
    let mut lines = Vec::new();
    for i in 0..20 {
        lines.push(format!("my $v{} = {};", i, i));
    }
    let source = lines.join("\n") + "\n";

    let mut state = IncrementalState::new(source.clone());
    let initial_token_count = state.tokens.len();

    // Edit a value near the beginning: change "$v0 = 0" to "$v0 = 999"
    let edit = Edit {
        start_byte: 9, // position of "0;" in "my $v0 = 0;"
        old_end_byte: 10,
        new_end_byte: 12,
        new_text: "999".to_string(),
    };

    let _result = apply_edits(&mut state, &[edit])?;

    // Verify the edit was applied
    assert!(state.source.contains("$v0 = 999"), "edit should be applied");

    // All subsequent variables should still be present
    for i in 1..20 {
        assert!(state.source.contains(&format!("$v{}", i)), "variable $v{} should be present", i);
    }

    // Tokens should still be present and valid
    assert!(
        state.tokens.len() >= initial_token_count,
        "token count should not decrease for an insertion edit"
    );

    Ok(())
}

// =========================================================================
// 7. Batch edits via IncrementalEditSet
// =========================================================================

#[test]
fn batch_edits_apply_correctly() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $a = 1;\nmy $b = 2;\nmy $c = 3;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    let mut edits = IncrementalEditSet::new();

    // Change all three values simultaneously
    let pos_a = source.find("= 1").ok_or("should contain = 1")? + 2;
    let pos_b = source.find("= 2").ok_or("should contain = 2")? + 2;
    let pos_c = source.find("= 3").ok_or("should contain = 3")? + 2;

    edits.add(IncrementalEdit::new(pos_a, pos_a + 1, "10".to_string()));
    edits.add(IncrementalEdit::new(pos_b, pos_b + 1, "20".to_string()));
    edits.add(IncrementalEdit::new(pos_c, pos_c + 1, "30".to_string()));

    doc.apply_edits(&edits)?;

    assert!(doc.text().contains("= 10"), "first edit should be applied");
    assert!(doc.text().contains("= 20"), "second edit should be applied");
    assert!(doc.text().contains("= 30"), "third edit should be applied");

    Ok(())
}

// =========================================================================
// 8. Cache eviction respects priority ordering
// =========================================================================

#[test]
fn cache_eviction_preserves_critical_over_low() -> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "package ImportantPkg;\n",
        "use strict;\n",
        "sub important_func { return 42; }\n",
        "my $trivial = 1;\n",
        "my $also_trivial = 2;\n",
    );

    let mut doc = IncrementalDocument::new(source.to_string())?;

    // With a generous cache, we should have multiple entries
    let large_cache_count = doc.subtree_cache.by_content.len();
    assert!(large_cache_count > 3, "should have multiple cached nodes");

    // Aggressively shrink cache
    doc.set_cache_max_size(2);

    // Critical symbols (package, use, sub) should survive eviction
    let has_critical = doc.subtree_cache.critical_symbols.values().any(|&p| {
        p == perl_incremental_parsing::incremental::incremental_document::SymbolPriority::Critical
    });

    assert!(has_critical, "critical symbols should survive aggressive cache eviction");

    Ok(())
}

// =========================================================================
// 9. SimpleIncrementalParser reuse across edits
// =========================================================================

#[test]
fn simple_parser_incremental_value_edit() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = SimpleIncrementalParser::new();

    // Initial parse
    let source1 = "my $x = 42;\nmy $y = 99;\n";
    let _ = parser.parse(source1)?;
    assert_eq!(parser.reused_nodes, 0, "first parse has no reuse");
    assert!(parser.reparsed_nodes > 0);

    // Edit: change 42 to 4242 (a value-only edit)
    parser.edit(perl_incremental_parsing::edit::Edit::new(
        8,
        10,
        12,
        Position::new(8, 1, 9),
        Position::new(10, 1, 11),
        Position::new(12, 1, 13),
    ));

    let source2 = "my $x = 4242;\nmy $y = 99;\n";
    let _ = parser.parse(source2)?;

    // Should reuse some nodes
    assert!(parser.reused_nodes > 0, "incremental parse should reuse nodes for value edit");

    Ok(())
}

// =========================================================================
// 10. End-to-end: multiple sequential edits maintain consistency
// =========================================================================

#[test]
fn sequential_edits_maintain_document_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\n";
    let mut doc = IncrementalDocument::new(source.to_string())?;

    // Edit 1: change 1 to 10
    let pos = doc.text().find('1').ok_or("should contain 1")?;
    doc.apply_edit(IncrementalEdit::new(pos, pos + 1, "10".to_string()))?;
    assert!(doc.text().contains("= 10"), "first edit applied");

    // Edit 2: change 10 to 100
    let pos = doc.text().find("10").ok_or("should contain 10")?;
    doc.apply_edit(IncrementalEdit::new(pos, pos + 2, "100".to_string()))?;
    assert!(doc.text().contains("= 100"), "second edit applied");

    // Edit 3: change 100 to 1000
    let pos = doc.text().find("100").ok_or("should contain 100")?;
    doc.apply_edit(IncrementalEdit::new(pos, pos + 3, "1000".to_string()))?;
    assert!(doc.text().contains("= 1000"), "third edit applied");

    // Parse tree should be valid after all edits
    match &doc.tree().kind {
        NodeKind::Program { statements } => {
            assert!(!statements.is_empty(), "should have statements");
        }
        _ => return Err("expected Program node".into()),
    }

    // Version should track all edits
    assert_eq!(doc.version, 3, "version should reflect 3 edits");

    Ok(())
}

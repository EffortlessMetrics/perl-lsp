//! Comprehensive integration tests for the perl-dead-code crate.
//!
//! These tests exercise the public API: DeadCodeDetector, DeadCodeAnalysis,
//! DeadCodeStats, DeadCodeType, and generate_report.

#![allow(clippy::panic)]

use perl_dead_code::{
    DeadCode, DeadCodeAnalysis, DeadCodeDetector, DeadCodeStats, DeadCodeType, generate_report,
};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a WorkspaceIndex containing a single Perl file.
fn index_with_file(uri: &str, code: &str) -> Result<WorkspaceIndex, String> {
    let index = WorkspaceIndex::new();
    index.index_file_str(uri, code)?;
    Ok(index)
}

/// Build a WorkspaceIndex containing multiple Perl files.
fn index_with_files(files: &[(&str, &str)]) -> Result<WorkspaceIndex, String> {
    let index = WorkspaceIndex::new();
    for (uri, code) in files {
        index.index_file_str(uri, code)?;
    }
    Ok(index)
}

// ---------------------------------------------------------------------------
// DeadCodeType — enum variant identity
// ---------------------------------------------------------------------------

#[test]
fn dead_code_type_variants_are_distinct() {
    let variants = [
        DeadCodeType::UnusedSubroutine,
        DeadCodeType::UnusedVariable,
        DeadCodeType::UnusedConstant,
        DeadCodeType::UnusedPackage,
        DeadCodeType::UnreachableCode,
        DeadCodeType::DeadBranch,
        DeadCodeType::UnusedImport,
        DeadCodeType::UnusedExport,
    ];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn dead_code_type_clone_and_copy() {
    let t = DeadCodeType::UnusedSubroutine;
    let cloned = t;
    assert_eq!(t, cloned);
}

#[test]
fn dead_code_type_debug_format() {
    let dbg = format!("{:?}", DeadCodeType::UnreachableCode);
    assert!(dbg.contains("UnreachableCode"));
}

// ---------------------------------------------------------------------------
// DeadCodeStats — default and field access
// ---------------------------------------------------------------------------

#[test]
fn dead_code_stats_default_is_all_zero() {
    let stats = DeadCodeStats::default();
    assert_eq!(stats.unused_subroutines, 0);
    assert_eq!(stats.unused_variables, 0);
    assert_eq!(stats.unused_constants, 0);
    assert_eq!(stats.unused_packages, 0);
    assert_eq!(stats.unreachable_statements, 0);
    assert_eq!(stats.dead_branches, 0);
    assert_eq!(stats.total_dead_lines, 0);
}

// ---------------------------------------------------------------------------
// DeadCode struct — construction and field access
// ---------------------------------------------------------------------------

#[test]
fn dead_code_struct_fields() {
    let dc = DeadCode {
        code_type: DeadCodeType::UnusedSubroutine,
        name: Some("unused_sub".to_string()),
        file_path: PathBuf::from("/tmp/test.pl"),
        start_line: 5,
        end_line: 10,
        reason: "Never called".to_string(),
        confidence: 0.95,
        suggestion: Some("Remove this subroutine".to_string()),
    };
    assert_eq!(dc.code_type, DeadCodeType::UnusedSubroutine);
    assert_eq!(dc.name.as_deref(), Some("unused_sub"));
    assert_eq!(dc.file_path, PathBuf::from("/tmp/test.pl"));
    assert_eq!(dc.start_line, 5);
    assert_eq!(dc.end_line, 10);
    assert!((dc.confidence - 0.95).abs() < f32::EPSILON);
    assert!(dc.suggestion.is_some());
}

#[test]
fn dead_code_with_none_optional_fields() {
    let dc = DeadCode {
        code_type: DeadCodeType::UnreachableCode,
        name: None,
        file_path: PathBuf::from("/tmp/test.pl"),
        start_line: 1,
        end_line: 1,
        reason: "After return".to_string(),
        confidence: 0.5,
        suggestion: None,
    };
    assert!(dc.name.is_none());
    assert!(dc.suggestion.is_none());
}

// ---------------------------------------------------------------------------
// DeadCodeDetector — construction
// ---------------------------------------------------------------------------

#[test]
fn detector_new_with_empty_index() {
    let index = WorkspaceIndex::new();
    let _detector = DeadCodeDetector::new(index);
}

#[test]
fn detector_add_entry_point() {
    let index = WorkspaceIndex::new();
    let mut detector = DeadCodeDetector::new(index);
    detector.add_entry_point(PathBuf::from("/tmp/main.pl"));
    // entry_points is private, but this should not panic
}

// ---------------------------------------------------------------------------
// analyze_file — unreachable code after return/die/exit
// ---------------------------------------------------------------------------

#[test]
fn analyze_file_detects_unreachable_after_return() -> Result<(), String> {
    let code = "sub foo {\n    return 1;\n    my $x = 2;\n}\n";
    let index = index_with_file("file:///test_return.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_return.pl"))?;
    assert!(!results.is_empty(), "should detect unreachable code after return");

    let item = &results[0];
    assert_eq!(item.code_type, DeadCodeType::UnreachableCode);
    assert_eq!(item.start_line, 3);
    assert!(item.reason.contains("return"));
    Ok(())
}

#[test]
fn analyze_file_detects_unreachable_after_die() -> Result<(), String> {
    let code = "die \"fatal\";\nprint \"never\";\n";
    let index = index_with_file("file:///test_die.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_die.pl"))?;
    assert!(!results.is_empty(), "should detect unreachable code after die");
    assert_eq!(results[0].code_type, DeadCodeType::UnreachableCode);
    assert!(results[0].reason.contains("die"));
    Ok(())
}

#[test]
fn analyze_file_detects_unreachable_after_exit() -> Result<(), String> {
    let code = "exit 0;\nprint \"never\";\n";
    let index = index_with_file("file:///test_exit.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_exit.pl"))?;
    assert!(!results.is_empty(), "should detect unreachable code after exit");
    assert_eq!(results[0].code_type, DeadCodeType::UnreachableCode);
    assert!(results[0].reason.contains("exit"));
    Ok(())
}

#[test]
fn analyze_file_no_false_positive_when_no_unreachable() -> Result<(), String> {
    let code = "my $x = 1;\nprint $x;\n";
    let index = index_with_file("file:///test_clean.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_clean.pl"))?;
    assert!(results.is_empty(), "clean code should have no unreachable items");
    Ok(())
}

#[test]
fn analyze_file_blank_line_separating_return_and_code() -> Result<(), String> {
    // Blank lines are skipped; the next non-empty line after a terminator is flagged
    let code = "return 1;\n\n\nprint 1;\n";
    let index = index_with_file("file:///test_blank.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_blank.pl"))?;
    assert!(
        !results.is_empty(),
        "non-empty line after blank lines following return is unreachable"
    );
    assert_eq!(results[0].start_line, 4);
    Ok(())
}

#[test]
fn analyze_file_error_for_unindexed_file() {
    let index = WorkspaceIndex::new();
    let detector = DeadCodeDetector::new(index);
    let result = detector.analyze_file(&PathBuf::from("/nonexistent.pl"));
    assert!(result.is_err());
}

#[test]
fn analyze_file_only_flags_first_unreachable_line() -> Result<(), String> {
    // The implementation breaks after the first unreachable line
    let code = "return;\nprint 1;\nprint 2;\n";
    let index = index_with_file("file:///test_multi.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_multi.pl"))?;
    assert_eq!(results.len(), 1, "detector should report only first unreachable line");
    Ok(())
}

// ---------------------------------------------------------------------------
// analyze_file — return inside sub only (not top-level unreachable)
// ---------------------------------------------------------------------------

#[test]
fn analyze_file_return_at_end_of_sub_flags_closing_brace() -> Result<(), String> {
    // The stub implementation is line-by-line and flags the closing brace
    // since it is the next non-empty line after `return`
    let code = "sub foo {\n    return 42;\n}\n";
    let index = index_with_file("file:///test_end_return.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_end_return.pl"))?;
    assert_eq!(results.len(), 1, "closing brace after return is flagged by stub");
    assert_eq!(results[0].start_line, 3);
    Ok(())
}

// ---------------------------------------------------------------------------
// analyze_workspace — empty workspace
// ---------------------------------------------------------------------------

#[test]
fn analyze_workspace_empty_index() {
    let index = WorkspaceIndex::new();
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    assert_eq!(analysis.files_analyzed, 0);
    assert_eq!(analysis.total_lines, 0);
    assert!(analysis.dead_code.is_empty());
}

// ---------------------------------------------------------------------------
// analyze_workspace — unreachable code counted in stats
// ---------------------------------------------------------------------------

#[test]
fn analyze_workspace_counts_unreachable_statements() -> Result<(), String> {
    let code = "die \"bye\";\nprint \"unreachable\";\n";
    let index = index_with_file("file:///test_ws.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    assert_eq!(analysis.files_analyzed, 1);
    assert!(analysis.stats.unreachable_statements >= 1);
    assert!(analysis.stats.total_dead_lines >= 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// analyze_workspace — unused symbols detection
// ---------------------------------------------------------------------------

#[test]
fn analyze_workspace_detects_unused_subroutine() -> Result<(), String> {
    // A subroutine defined but never called
    let code = "sub unused_helper { return 1; }\n";
    let index = index_with_file("file:///test_unused_sub.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    let unused_subs: Vec<_> = analysis
        .dead_code
        .iter()
        .filter(|d| d.code_type == DeadCodeType::UnusedSubroutine)
        .collect();
    assert!(
        !unused_subs.is_empty(),
        "should detect unused subroutine; found: {:?}",
        analysis.dead_code.iter().map(|d| &d.code_type).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn analyze_workspace_used_subroutine_not_flagged() -> Result<(), String> {
    // A subroutine defined and called should NOT be flagged
    let code = "sub helper { return 1; }\nhelper();\n";
    let index = index_with_file("file:///test_used_sub.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    let unused_subs: Vec<_> = analysis
        .dead_code
        .iter()
        .filter(|d| {
            d.code_type == DeadCodeType::UnusedSubroutine && d.name.as_deref() == Some("helper")
        })
        .collect();
    assert!(unused_subs.is_empty(), "called subroutine should not be flagged as unused");
    Ok(())
}

// ---------------------------------------------------------------------------
// analyze_workspace — multi-file cross-referencing
// ---------------------------------------------------------------------------

#[test]
fn analyze_workspace_cross_file_reference_not_flagged() -> Result<(), String> {
    // Sub defined in one file, called in another
    let files = &[
        ("file:///lib.pm", "package Lib;\nsub shared { return 1; }\n1;\n"),
        ("file:///main.pl", "use Lib;\nLib::shared();\n"),
    ];
    let index = index_with_files(files)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    let flagged: Vec<_> =
        analysis.dead_code.iter().filter(|d| d.name.as_deref() == Some("shared")).collect();
    assert!(flagged.is_empty(), "cross-file referenced sub should not be flagged: {:?}", flagged);
    Ok(())
}

// ---------------------------------------------------------------------------
// analyze_workspace — stats aggregation
// ---------------------------------------------------------------------------

#[test]
fn analyze_workspace_stats_match_dead_code_items() -> Result<(), String> {
    let code = "exit 1;\nprint \"unreachable\";\n";
    let index = index_with_file("file:///test_stats.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    // Verify stats are consistent with the dead_code vector
    let unreachable_count =
        analysis.dead_code.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).count();
    assert_eq!(analysis.stats.unreachable_statements, unreachable_count);
    Ok(())
}

#[test]
fn analyze_workspace_total_lines_is_positive_for_nonempty() -> Result<(), String> {
    let code = "my $x = 42;\nprint $x;\n";
    let index = index_with_file("file:///test_lines.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    assert!(analysis.total_lines > 0);
    assert_eq!(analysis.files_analyzed, 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// generate_report — output format
// ---------------------------------------------------------------------------

#[test]
fn generate_report_contains_header() {
    let analysis = DeadCodeAnalysis {
        dead_code: vec![],
        stats: DeadCodeStats::default(),
        files_analyzed: 0,
        total_lines: 0,
    };
    let report = generate_report(&analysis);
    assert!(report.contains("Dead Code Analysis Report"));
}

#[test]
fn generate_report_shows_file_count() {
    let analysis = DeadCodeAnalysis {
        dead_code: vec![],
        stats: DeadCodeStats::default(),
        files_analyzed: 7,
        total_lines: 350,
    };
    let report = generate_report(&analysis);
    assert!(report.contains("Files analyzed: 7"));
    assert!(report.contains("Total lines: 350"));
}

#[test]
fn generate_report_shows_stats() {
    let stats = DeadCodeStats {
        unused_subroutines: 3,
        unused_variables: 2,
        unused_constants: 1,
        unused_packages: 0,
        unreachable_statements: 4,
        dead_branches: 0,
        total_dead_lines: 10,
    };
    let analysis =
        DeadCodeAnalysis { dead_code: vec![], stats, files_analyzed: 5, total_lines: 200 };
    let report = generate_report(&analysis);
    assert!(report.contains("Unused subroutines: 3"));
    assert!(report.contains("Unused variables: 2"));
    assert!(report.contains("Unused constants: 1"));
    assert!(report.contains("Unreachable statements: 4"));
    assert!(report.contains("Total dead lines: 10"));
}

#[test]
fn generate_report_shows_dead_code_item_count() {
    let item = DeadCode {
        code_type: DeadCodeType::UnreachableCode,
        name: None,
        file_path: PathBuf::from("/test.pl"),
        start_line: 5,
        end_line: 5,
        reason: "after return".to_string(),
        confidence: 0.5,
        suggestion: None,
    };
    let analysis = DeadCodeAnalysis {
        dead_code: vec![item],
        stats: DeadCodeStats {
            unreachable_statements: 1,
            total_dead_lines: 1,
            ..Default::default()
        },
        files_analyzed: 1,
        total_lines: 10,
    };
    let report = generate_report(&analysis);
    assert!(report.contains("Dead code items: 1"));
}

// ---------------------------------------------------------------------------
// Serde serialization round-trip
// ---------------------------------------------------------------------------

#[test]
fn dead_code_type_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let original = DeadCodeType::UnusedSubroutine;
    let json = serde_json::to_string(&original)?;
    let restored: DeadCodeType = serde_json::from_str(&json)?;
    assert_eq!(original, restored);
    Ok(())
}

#[test]
fn dead_code_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let original = DeadCode {
        code_type: DeadCodeType::UnusedVariable,
        name: Some("$unused".to_string()),
        file_path: PathBuf::from("/lib/Foo.pm"),
        start_line: 10,
        end_line: 10,
        reason: "Never read".to_string(),
        confidence: 0.85,
        suggestion: Some("Remove declaration".to_string()),
    };
    let json = serde_json::to_string(&original)?;
    let restored: DeadCode = serde_json::from_str(&json)?;
    assert_eq!(restored.code_type, original.code_type);
    assert_eq!(restored.name, original.name);
    assert_eq!(restored.start_line, original.start_line);
    Ok(())
}

#[test]
fn dead_code_stats_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let original = DeadCodeStats {
        unused_subroutines: 5,
        unused_variables: 3,
        unused_constants: 1,
        unused_packages: 2,
        unreachable_statements: 4,
        dead_branches: 0,
        total_dead_lines: 15,
    };
    let json = serde_json::to_string(&original)?;
    let restored: DeadCodeStats = serde_json::from_str(&json)?;
    assert_eq!(restored.unused_subroutines, 5);
    assert_eq!(restored.total_dead_lines, 15);
    Ok(())
}

#[test]
fn dead_code_analysis_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let analysis = DeadCodeAnalysis {
        dead_code: vec![DeadCode {
            code_type: DeadCodeType::DeadBranch,
            name: None,
            file_path: PathBuf::from("/test.pl"),
            start_line: 1,
            end_line: 3,
            reason: "always false".to_string(),
            confidence: 0.7,
            suggestion: None,
        }],
        stats: DeadCodeStats { dead_branches: 1, total_dead_lines: 3, ..Default::default() },
        files_analyzed: 1,
        total_lines: 20,
    };
    let json = serde_json::to_string(&analysis)?;
    let restored: DeadCodeAnalysis = serde_json::from_str(&json)?;
    assert_eq!(restored.dead_code.len(), 1);
    assert_eq!(restored.files_analyzed, 1);
    Ok(())
}

// ---------------------------------------------------------------------------
// DeadCode confidence bounds
// ---------------------------------------------------------------------------

#[test]
fn analyze_file_confidence_within_bounds() -> Result<(), String> {
    let code = "return;\nmy $x = 1;\n";
    let index = index_with_file("file:///test_conf.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_conf.pl"))?;
    for item in &results {
        assert!(
            (0.0..=1.0).contains(&item.confidence),
            "confidence {} should be in [0.0, 1.0]",
            item.confidence
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Multiple entry points
// ---------------------------------------------------------------------------

#[test]
fn detector_multiple_entry_points() {
    let index = WorkspaceIndex::new();
    let mut detector = DeadCodeDetector::new(index);
    detector.add_entry_point(PathBuf::from("/main.pl"));
    detector.add_entry_point(PathBuf::from("/app.pl"));
    // Should not panic, workspace analysis still works on empty index
    let analysis = detector.analyze_workspace();
    assert_eq!(analysis.files_analyzed, 0);
}

// ---------------------------------------------------------------------------
// Multiple files in workspace analysis
// ---------------------------------------------------------------------------

#[test]
fn analyze_workspace_multiple_files() -> Result<(), String> {
    let files = &[
        ("file:///a.pl", "my $x = 1;\nprint $x;\n"),
        ("file:///b.pl", "die \"error\";\nmy $y = 2;\n"),
    ];
    let index = index_with_files(files)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    assert_eq!(analysis.files_analyzed, 2);
    assert!(analysis.total_lines > 0);
    // b.pl has unreachable code after die
    assert!(analysis.stats.unreachable_statements >= 1, "should find unreachable code in b.pl");
    Ok(())
}

// ---------------------------------------------------------------------------
// Empty file
// ---------------------------------------------------------------------------

#[test]
fn analyze_file_empty_file() -> Result<(), String> {
    let index = index_with_file("file:///empty.pl", "")?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/empty.pl"))?;
    assert!(results.is_empty(), "empty file should have no dead code");
    Ok(())
}

// ---------------------------------------------------------------------------
// File with only comments
// ---------------------------------------------------------------------------

#[test]
fn analyze_file_comments_only() -> Result<(), String> {
    let code = "# This is a comment\n# Another comment\n";
    let index = index_with_file("file:///comments.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/comments.pl"))?;
    assert!(results.is_empty(), "comment-only file should have no dead code");
    Ok(())
}

// ---------------------------------------------------------------------------
// DeadCode suggestion field populated for unreachable code
// ---------------------------------------------------------------------------

#[test]
fn unreachable_code_has_suggestion() -> Result<(), String> {
    let code = "return 0;\nmy $x = 1;\n";
    let index = index_with_file("file:///test_suggest.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_suggest.pl"))?;
    assert!(!results.is_empty());
    assert!(results[0].suggestion.is_some(), "unreachable code should have a suggestion");
    Ok(())
}

// ---------------------------------------------------------------------------
// DeadCode file_path matches input
// ---------------------------------------------------------------------------

#[test]
fn dead_code_file_path_matches_input() -> Result<(), String> {
    let code = "exit;\nprint 1;\n";
    let index = index_with_file("file:///my/script.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/my/script.pl"))?;
    assert!(!results.is_empty());
    assert_eq!(results[0].file_path, PathBuf::from("/my/script.pl"));
    Ok(())
}

// ---------------------------------------------------------------------------
// DeadBranch detection
// ---------------------------------------------------------------------------

#[test]
fn dead_branch_if_zero_emits_dead_branch() -> Result<(), String> {
    let code = "if (0) {\n    print \"never\";\n}\n";
    let index = index_with_file("file:///test_dead_if0.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_dead_if0.pl"))?;
    let dead_branches: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert!(!dead_branches.is_empty(), "if (0) should produce a DeadBranch entry");
    assert!(dead_branches[0].reason.contains("always false"), "reason should mention always false");
    assert_eq!(dead_branches[0].confidence, 0.9);
    Ok(())
}

#[test]
fn dead_branch_while_zero_emits_dead_branch() -> Result<(), String> {
    let code = "while (0) {\n    do_something();\n}\n";
    let index = index_with_file("file:///test_dead_while0.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_dead_while0.pl"))?;
    let dead_branches: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert!(!dead_branches.is_empty(), "while (0) should produce a DeadBranch entry");
    assert!(dead_branches[0].reason.contains("`while`"));
    Ok(())
}

#[test]
fn dead_branch_unless_one_emits_dead_branch() -> Result<(), String> {
    let code = "unless (1) {\n    print \"never\";\n}\n";
    let index = index_with_file("file:///test_dead_unless1.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_dead_unless1.pl"))?;
    let dead_branches: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert!(!dead_branches.is_empty(), "unless (1) should produce a DeadBranch entry");
    assert!(dead_branches[0].reason.contains("`unless`"));
    assert!(dead_branches[0].reason.contains("always true"), "reason should mention always true");
    Ok(())
}

#[test]
fn dead_branch_until_one_emits_dead_branch() -> Result<(), String> {
    let code = "until (1) {\n    print \"never\";\n}\n";
    let index = index_with_file("file:///test_dead_until1.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_dead_until1.pl"))?;
    let dead_branches: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert!(!dead_branches.is_empty(), "until (1) should produce a DeadBranch entry");
    assert!(dead_branches[0].reason.contains("`until`"));
    Ok(())
}

#[test]
fn dead_branch_if_undef_emits_dead_branch() -> Result<(), String> {
    let code = "if (undef) {\n    print \"never\";\n}\n";
    let index = index_with_file("file:///test_dead_if_undef.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_dead_if_undef.pl"))?;
    let dead_branches: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert!(!dead_branches.is_empty(), "if (undef) should produce a DeadBranch entry");
    Ok(())
}

#[test]
fn dead_branch_if_empty_string_emits_dead_branch() -> Result<(), String> {
    let code = "if (\"\") {\n    print \"never\";\n}\n";
    let index = index_with_file("file:///test_dead_if_emptystr.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_dead_if_emptystr.pl"))?;
    let dead_branches: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert!(!dead_branches.is_empty(), "if (\"\") should produce a DeadBranch entry");
    Ok(())
}

#[test]
fn dead_branch_normal_if_does_not_emit() -> Result<(), String> {
    let code = "if ($x > 0) {\n    print \"yes\";\n}\n";
    let index = index_with_file("file:///test_normal_if.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_normal_if.pl"))?;
    let dead_branches: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect();
    assert!(dead_branches.is_empty(), "non-constant condition should not produce DeadBranch");
    Ok(())
}

#[test]
fn dead_branch_has_suggestion() -> Result<(), String> {
    let code = "if (0) {\n    print \"never\";\n}\n";
    let index = index_with_file("file:///test_dead_suggestion.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let results = detector.analyze_file(&PathBuf::from("/test_dead_suggestion.pl"))?;
    let branch = results.iter().find(|d| d.code_type == DeadCodeType::DeadBranch);
    let branch = branch.ok_or("expected DeadBranch entry")?;
    assert!(branch.suggestion.is_some(), "DeadBranch should have a suggestion");
    Ok(())
}

#[test]
fn dead_branch_stats_incremented_in_workspace_analysis() -> Result<(), String> {
    let code = "if (0) {\n    print \"never\";\n}\nif (1) {\n    print \"yes\";\n}\n";
    let index = index_with_file("file:///test_dead_stats.pl", code)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    assert!(
        analysis.stats.dead_branches >= 1,
        "workspace analysis should count dead branches in stats"
    );
    Ok(())
}

//! Behavior-driven tests for `perl-dead-code`.
//!
//! These tests focus on user-visible outcomes with a Given/When/Then structure.

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::{Path, PathBuf};

fn test_uri_to_index_uri(uri: &str) -> Result<String, String> {
    match uri.strip_prefix("file://") {
        Some(path) => perl_uri::fs_path_to_uri(PathBuf::from(path)),
        None => Ok(uri.to_string()),
    }
}

fn detector_with_single_file(uri: &str, source: &str) -> Result<DeadCodeDetector, String> {
    let index = WorkspaceIndex::new();
    let index_uri = test_uri_to_index_uri(uri)?;
    index.index_file_str(&index_uri, source)?;
    Ok(DeadCodeDetector::new(index))
}

fn detect_for_path(
    detector: &DeadCodeDetector,
    path: &str,
) -> Result<Vec<perl_dead_code::DeadCode>, String> {
    detector.analyze_file(Path::new(path))
}

fn assert_no_dead_branch(source_name: &str, source: &str) -> Result<(), String> {
    let uri = format!("file:///{source_name}");
    let path = format!("/{source_name}");
    let detector = detector_with_single_file(&uri, source)?;
    let dead_code = detect_for_path(&detector, &path)?;

    assert!(
        dead_code.iter().all(|item| item.code_type != DeadCodeType::DeadBranch),
        "{source_name} must not produce a dead branch; got {dead_code:?}"
    );
    Ok(())
}

fn assert_has_dead_branch(source_name: &str, source: &str) -> Result<(), String> {
    let uri = format!("file:///{source_name}");
    let path = format!("/{source_name}");
    let detector = detector_with_single_file(&uri, source)?;
    let dead_code = detect_for_path(&detector, &path)?;

    assert!(
        dead_code.iter().any(|item| item.code_type == DeadCodeType::DeadBranch),
        "{source_name} must produce a dead branch; got {dead_code:?}"
    );
    Ok(())
}

#[test]
fn scenario_unreachable_statement_after_return_is_reported() -> Result<(), String> {
    // Given a subroutine with a statement after an unconditional return
    let detector = detector_with_single_file(
        "file:///scenario_return.pl",
        "sub run {\n    return 1;\n    print 'never';\n}\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_return.pl")?;

    // Then the post-return statement is flagged as unreachable
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::UnreachableCode
            && item.start_line == 3
            && item.reason.contains("return")
    }));
    Ok(())
}

#[test]
fn scenario_if_zero_branch_is_marked_dead_branch() -> Result<(), String> {
    // Given an if block whose condition is always false
    let detector = detector_with_single_file(
        "file:///scenario_if_zero.pl",
        "if (0) {\n    print 'dead';\n}\nprint 'live';\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_if_zero.pl")?;

    // Then the block is reported as a dead branch
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::DeadBranch
            && item.start_line == 1
            && item.end_line == 3
            && item.reason.contains("always false")
    }));
    Ok(())
}

#[test]
fn scenario_unless_one_branch_is_marked_dead_branch() -> Result<(), String> {
    // Given an unless block whose condition is always true
    let detector = detector_with_single_file(
        "file:///scenario_unless_one.pl",
        "unless (1) {\n    print 'dead';\n}\nprint 'live';\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_unless_one.pl")?;

    // Then the unless body is reported as dead
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::DeadBranch
            && item.reason.contains("always true")
            && item.reason.contains("never executed")
    }));
    Ok(())
}

#[test]
fn scenario_nested_parenthesized_false_condition_is_detected() -> Result<(), String> {
    // Given a branch using a nested always-false expression
    let detector = detector_with_single_file(
        "file:///scenario_nested_false.pl",
        "while (((0))) {\n    print 'dead loop';\n}\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_nested_false.pl")?;

    // Then the loop body is reported as dead
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::DeadBranch
            && item.reason.contains("always false")
            && item.start_line == 1
    }));
    Ok(())
}

#[test]
fn scenario_for_falsey_list_elements_are_not_dead_branches() -> Result<(), String> {
    // Given for/foreach loops over falsey values in list context
    let cases = [
        ("scenario_for_zero.pl", "for (0) {\n    print 'runs once';\n}\n"),
        ("scenario_foreach_zero.pl", "foreach (0) {\n    print 'runs once';\n}\n"),
        ("scenario_for_empty_double_string.pl", "for (\"\") {\n    print 'runs once';\n}\n"),
        ("scenario_for_empty_single_string.pl", "for ('') {\n    print 'runs once';\n}\n"),
        ("scenario_for_undef.pl", "for (undef) {\n    print 'runs once';\n}\n"),
    ];

    // Then none are reported as dead branches
    for (source_name, source) in cases {
        assert_no_dead_branch(source_name, source)?;
    }
    Ok(())
}

#[test]
fn scenario_mixed_file_for_zero_and_if_zero_only_if_is_dead() -> Result<(), String> {
    // Given a file with a list iterator and a boolean false branch
    let detector = detector_with_single_file(
        "file:///scenario_mixed.pl",
        "for (0) {\n    print 'runs';\n}\nif (0) {\n    print 'dead';\n}\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_mixed.pl")?;

    // Then only the if(0) block is reported as dead
    let dead_branches: Vec<_> =
        dead_code.iter().filter(|item| item.code_type == DeadCodeType::DeadBranch).collect();
    assert_eq!(dead_branches.len(), 1, "exactly one dead branch expected; got {dead_branches:?}");
    assert_eq!(dead_branches[0].start_line, 4);
    Ok(())
}

#[test]
fn scenario_boolean_context_branches_remain_dead_after_for_fix() -> Result<(), String> {
    // Given boolean-context branches that should still be classified as dead
    let cases = [
        ("scenario_while_zero.pl", "while (0) {\n    print 'dead';\n}\n"),
        ("scenario_if_zero.pl", "if (0) {\n    print 'dead';\n}\n"),
        ("scenario_unless_one.pl", "unless (1) {\n    print 'dead';\n}\n"),
    ];

    // Then the existing boolean dead-branch detection still fires
    for (source_name, source) in cases {
        assert_has_dead_branch(source_name, source)?;
    }
    Ok(())
}

#[test]
fn scenario_workspace_analysis_aggregates_unreachable_and_dead_branch() -> Result<(), String> {
    // Given a workspace with both unreachable code and a dead branch
    let index = WorkspaceIndex::new();
    let scenario_a_uri = test_uri_to_index_uri("file:///scenario_a.pl")?;
    let scenario_b_uri = test_uri_to_index_uri("file:///scenario_b.pl")?;
    index.index_file_str(&scenario_a_uri, "exit 1;\nprint 'never';\n")?;
    index.index_file_str(&scenario_b_uri, "if (0) {\n    print 'dead';\n}\n")?;
    let detector = DeadCodeDetector::new(index);

    // When workspace analysis runs
    let analysis = detector.analyze_workspace();

    // Then both behavior classes are represented in the result summary
    assert!(analysis.stats.unreachable_statements >= 1);
    assert!(analysis.stats.dead_branches >= 1);
    assert!(analysis.dead_code.iter().any(|item| item.code_type == DeadCodeType::UnreachableCode));
    assert!(analysis.dead_code.iter().any(|item| item.code_type == DeadCodeType::DeadBranch));
    Ok(())
}

#[test]
fn scenario_falsey_numeric_and_string_literals_are_dead_branches() -> Result<(), String> {
    // Given false literals that are easy to miss when normalizing Perl truthiness
    let cases = [
        ("scenario_if_decimal_zero.pl", "if (0.0) {\n    print 'dead';\n}\n"),
        ("scenario_if_double_quoted_zero.pl", "if (\"0\") {\n    print 'dead';\n}\n"),
        ("scenario_if_single_quoted_zero.pl", "if ('0') {\n    print 'dead';\n}\n"),
    ];

    // Then each literal is classified as an always-false branch condition
    for (source_name, source) in cases {
        assert_has_dead_branch(source_name, source)?;
    }
    Ok(())
}

#[test]
fn scenario_unless_falsey_literals_are_not_dead_branches() -> Result<(), String> {
    // Given unless blocks whose falsey conditions make the body reachable
    let cases = [
        ("scenario_unless_decimal_zero.pl", "unless (0.0) {\n    print 'live';\n}\n"),
        ("scenario_unless_double_quoted_zero.pl", "unless (\"0\") {\n    print 'live';\n}\n"),
        ("scenario_unless_single_quoted_zero.pl", "unless ('0') {\n    print 'live';\n}\n"),
    ];

    // Then none of those reachable unless bodies are reported as dead branches
    for (source_name, source) in cases {
        assert_no_dead_branch(source_name, source)?;
    }
    Ok(())
}

#[test]
fn scenario_dead_branch_accepts_open_brace_on_next_line() -> Result<(), String> {
    // Given a style where the opening brace starts the following line
    let detector = detector_with_single_file(
        "file:///scenario_next_line_brace.pl",
        "if (0)\n{\n    print 'dead';\n}\nprint 'live';\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_next_line_brace.pl")?;

    // Then the whole block is still reported as a dead branch
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::DeadBranch && item.start_line == 1 && item.end_line == 4
    }));
    Ok(())
}

#[test]
fn scenario_comments_after_terminator_do_not_hide_next_unreachable_statement() -> Result<(), String>
{
    // Given an unconditional terminator followed by comments and then code
    let detector = detector_with_single_file(
        "file:///scenario_comment_gap.pl",
        "return 1; # done\n# explanatory comment\n    # indented comment\nprint 'never';\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_comment_gap.pl")?;

    // Then comments are skipped and the next real statement is reported
    assert!(dead_code.iter().any(|item| {
        item.code_type == DeadCodeType::UnreachableCode
            && item.start_line == 4
            && item.reason.contains("return")
    }));
    Ok(())
}

#[test]
fn scenario_inner_block_terminator_does_not_leak_to_outer_scope() -> Result<(), String> {
    // Given a terminator inside an inner block followed by live outer-scope code
    let detector = detector_with_single_file(
        "file:///scenario_inner_block.pl",
        "if ($ok) {\n    return 1;\n}\nprint 'still reachable outside block';\n",
    )?;

    // When dead-code analysis runs on that file
    let dead_code = detect_for_path(&detector, "/scenario_inner_block.pl")?;

    // Then the statement after the closed block is not marked unreachable
    assert!(
        dead_code.iter().all(|item| item.code_type != DeadCodeType::UnreachableCode),
        "inner-block terminator must not mark outer code unreachable; got {dead_code:?}"
    );
    Ok(())
}

#[test]
fn scenario_postfix_loop_terminators_are_not_unconditional() -> Result<(), String> {
    // Given terminator-looking statements guarded by postfix loop/list conditions
    let cases = [
        ("scenario_return_for.pl", "return $item for @items;\nsay 'after';\n"),
        ("scenario_die_foreach.pl", "die $error foreach @errors;\nsay 'after';\n"),
        ("scenario_exit_until.pl", "exit until $done;\nsay 'after';\n"),
        ("scenario_return_when.pl", "return $value when /match/;\nsay 'after';\n"),
    ];

    // Then none should produce unreachable-code diagnostics
    for (source_name, source) in cases {
        let uri = format!("file:///{source_name}");
        let path = format!("/{source_name}");
        let detector = detector_with_single_file(&uri, source)?;
        let dead_code = detect_for_path(&detector, &path)?;
        assert!(
            dead_code.iter().all(|item| item.code_type != DeadCodeType::UnreachableCode),
            "{source_name} must not produce unreachable code; got {dead_code:?}"
        );
    }
    Ok(())
}

#[test]
fn scenario_constant_branch_literals_distinguish_boolean_contexts() -> Result<(), String> {
    // Given falsey literals in positive boolean contexts and truthy literals in inverse contexts
    let cases = [
        (
            "scenario_elsif_empty.pl",
            "if ($x) {\n    say 'x';\n}\nelsif ('') {\n    say 'dead';\n}\n",
        ),
        ("scenario_while_undef.pl", "while (undef) {\n    say 'dead';\n}\n"),
        ("scenario_until_nonzero.pl", "until (2) {\n    say 'dead';\n}\n"),
    ];

    // Then each branch shape is still recognized as dead
    for (source_name, source) in cases {
        assert_has_dead_branch(source_name, source)?;
    }
    Ok(())
}

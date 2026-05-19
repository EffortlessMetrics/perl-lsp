//! Branch-coverage tests for `perl-dead-code`.
//!
//! These tests systematically exercise the branches in `dead_branches.rs`
//! and `lib.rs` that were not reached by the prior test suites.

use perl_dead_code::{DeadCodeDetector, DeadCodeType};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn detector_with_file(uri: &str, source: &str) -> Result<DeadCodeDetector, String> {
    let index = WorkspaceIndex::new();
    let index_uri = test_uri_to_index_uri(uri)?;
    index.index_file_str(&index_uri, source)?;
    Ok(DeadCodeDetector::new(index))
}

fn test_uri_to_index_uri(uri: &str) -> Result<String, String> {
    match uri.strip_prefix("file://") {
        Some(path) => perl_uri::fs_path_to_uri(PathBuf::from(path)),
        None => Ok(uri.to_string()),
    }
}

fn analyze(uri: &str, path: &str, source: &str) -> Result<Vec<perl_dead_code::DeadCode>, String> {
    let detector = detector_with_file(uri, source)?;
    detector.analyze_file(Path::new(path))
}

fn dead_branches_in(
    uri: &str,
    path: &str,
    source: &str,
) -> Result<Vec<perl_dead_code::DeadCode>, String> {
    let all = analyze(uri, path, source)?;
    Ok(all.into_iter().filter(|d| d.code_type == DeadCodeType::DeadBranch).collect())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — is_always_true: float path
// (dead_branches.rs line 94: c.parse::<f64>().is_ok_and)
// ---------------------------------------------------------------------------

#[test]
fn unless_with_float_true_condition_emits_dead_branch() -> Result<(), String> {
    // `unless (1.5)` — condition is always true (non-zero float); body is dead
    let branches = dead_branches_in(
        "file:///bc_unless_float.pl",
        "/bc_unless_float.pl",
        "unless (1.5) {\n    print 'dead';\n}\n",
    )?;
    assert!(!branches.is_empty(), "unless (1.5) should be detected as a dead branch; got nothing");
    assert!(
        branches[0].reason.contains("always true"),
        "reason should mention always true; got: {}",
        branches[0].reason
    );
    Ok(())
}

#[test]
fn until_with_float_true_condition_emits_dead_branch() -> Result<(), String> {
    // `until (2.5)` — non-zero float is always true; body never runs
    let branches = dead_branches_in(
        "file:///bc_until_float.pl",
        "/bc_until_float.pl",
        "until (2.5) {\n    do_thing();\n}\n",
    )?;
    assert!(!branches.is_empty(), "until (2.5) should be detected as a dead branch");
    assert!(branches[0].reason.contains("`until`"));
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — is_always_true: non-zero string path
// (dead_branches.rs lines 97-101: string literal check)
// ---------------------------------------------------------------------------

#[test]
fn unless_with_nonempty_double_quoted_string_emits_dead_branch() -> Result<(), String> {
    // `unless ("hello")` — non-empty, non-"0" double-quoted string is always true
    let branches = dead_branches_in(
        "file:///bc_unless_str_dq.pl",
        "/bc_unless_str_dq.pl",
        "unless (\"hello\") {\n    print 'dead';\n}\n",
    )?;
    assert!(!branches.is_empty(), "unless (\"hello\") should be a dead branch; got nothing");
    assert!(branches[0].reason.contains("always true"));
    Ok(())
}

#[test]
fn unless_with_nonempty_single_quoted_string_emits_dead_branch() -> Result<(), String> {
    // `unless ('yes')` — non-empty, non-"0" single-quoted string is always true
    let branches = dead_branches_in(
        "file:///bc_unless_str_sq.pl",
        "/bc_unless_str_sq.pl",
        "unless ('yes') {\n    print 'dead';\n}\n",
    )?;
    assert!(!branches.is_empty(), "unless ('yes') should be a dead branch");
    assert!(branches[0].reason.contains("always true"));
    Ok(())
}

#[test]
fn unless_with_string_zero_is_not_always_true() -> Result<(), String> {
    // `unless ("0")` — the string "0" is falsy in Perl; so `unless ("0")` runs the body
    let branches = dead_branches_in(
        "file:///bc_unless_str_zero.pl",
        "/bc_unless_str_zero.pl",
        "unless (\"0\") {\n    print 'runs';\n}\n",
    )?;
    assert!(
        branches.is_empty(),
        "unless (\"0\") should NOT be a dead branch because \"0\" is falsy; got {branches:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — is_always_true: nested paren path
// (dead_branches.rs line 103: recursive nested parens)
// ---------------------------------------------------------------------------

#[test]
fn unless_with_nested_paren_true_condition_emits_dead_branch() -> Result<(), String> {
    // `unless ((1))` — nested parens around always-true integer
    let branches = dead_branches_in(
        "file:///bc_unless_nested_true.pl",
        "/bc_unless_nested_true.pl",
        "unless ((1)) {\n    print 'dead';\n}\n",
    )?;
    assert!(!branches.is_empty(), "unless ((1)) should be a dead branch via nested-paren path");
    assert!(branches[0].reason.contains("always true"));
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — is_always_true: else-None path
// (dead_branches.rs line 45: unless/until with NON-always-true condition is NOT dead)
// ---------------------------------------------------------------------------

#[test]
fn unless_with_variable_condition_is_not_dead() -> Result<(), String> {
    // `unless ($x)` — $x is not a constant; no dead branch
    let branches = dead_branches_in(
        "file:///bc_unless_var.pl",
        "/bc_unless_var.pl",
        "unless ($x) {\n    print 'maybe runs';\n}\n",
    )?;
    assert!(
        branches.is_empty(),
        "unless with non-constant condition should not be a dead branch; got {branches:?}"
    );
    Ok(())
}

#[test]
fn until_with_variable_condition_is_not_dead() -> Result<(), String> {
    // `until ($done)` — $done is not constant; no dead branch
    let branches = dead_branches_in(
        "file:///bc_until_var.pl",
        "/bc_until_var.pl",
        "until ($done) {\n    step();\n}\n",
    )?;
    assert!(
        branches.is_empty(),
        "until with non-constant condition should not be a dead branch; got {branches:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — after_cond guard: extra content between condition and block
// (dead_branches.rs line 34: !after_cond.starts_with('{') && !after_cond.is_empty())
// ---------------------------------------------------------------------------

#[test]
fn if_with_trailing_content_before_brace_is_not_detected() -> Result<(), String> {
    // Parser skips lines where condition text is followed by unexpected tokens,
    // e.g. `if (0) do_something_without_brace;` — no block brace detected
    let branches = dead_branches_in(
        "file:///bc_if_no_block.pl",
        "/bc_if_no_block.pl",
        "if (0) do_something();\n",
    )?;
    // The implementation skips conditions where after_cond is non-empty and not `{`
    assert!(
        branches.is_empty(),
        "if (0) without block brace should not be flagged; got {branches:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — keyword with empty-rest path
// (dead_branches.rs line 18: r.is_empty() True branch — keyword immediately followed by '(')
// ---------------------------------------------------------------------------

#[test]
fn if_immediately_followed_by_paren_no_space_is_detected() -> Result<(), String> {
    // `if(0)` — no space between `if` and `(`; r.is_empty() is True
    let branches = dead_branches_in(
        "file:///bc_if_nospace.pl",
        "/bc_if_nospace.pl",
        "if(0) {\n    print 'dead';\n}\n",
    )?;
    assert!(!branches.is_empty(), "if(0) (no space) should be detected as a dead branch");
    assert!(branches[0].reason.contains("always false"));
    Ok(())
}

#[test]
fn while_immediately_followed_by_paren_no_space_is_detected() -> Result<(), String> {
    // `while(0)` — no space
    let branches = dead_branches_in(
        "file:///bc_while_nospace.pl",
        "/bc_while_nospace.pl",
        "while(0) {\n    loop_body();\n}\n",
    )?;
    assert!(!branches.is_empty(), "while(0) (no space) should be detected as a dead branch");
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — keyword that does NOT match (early continue path)
// (dead_branches.rs line 23: _ => continue)
// These are covered implicitly whenever a non-matching keyword line is processed,
// but we add an explicit test that has all 5 keywords on the same line.
// ---------------------------------------------------------------------------

#[test]
fn line_starting_with_non_matching_keyword_is_skipped() -> Result<(), String> {
    // `my $if_flag = 1;` starts with "my", not one of the matching keywords
    let branches = dead_branches_in(
        "file:///bc_non_kw.pl",
        "/bc_non_kw.pl",
        "my $if_flag = 1;\nmy $while_val = 0;\nprint 'ok';\n",
    )?;
    assert!(
        branches.is_empty(),
        "non-keyword lines should not produce dead branches; got {branches:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — find_block_end: nested braces (False branch of depth==0)
// (dead_branches.rs line 134: nested { } pairs before closing brace)
// ---------------------------------------------------------------------------

#[test]
fn dead_branch_with_nested_block_inside_body_finds_correct_end() -> Result<(), String> {
    // if (0) with nested block — find_block_end must traverse the inner brace pair
    let source = "if (0) {\n    if (1) {\n        print 'inner';\n    }\n}\n";
    let branches = dead_branches_in("file:///bc_nested_block.pl", "/bc_nested_block.pl", source)?;
    assert!(!branches.is_empty(), "if (0) with nested body should still be a dead branch");
    // end_line should be 5 (the outer closing brace), not 4 (the inner one)
    assert_eq!(
        branches[0].end_line, 5,
        "end_line should point to the outer closing brace; got {}",
        branches[0].end_line
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — find_block_end: no closing brace (lines.len() fallback)
// (dead_branches.rs line 142: return lines.len() when no closing brace found)
// ---------------------------------------------------------------------------

#[test]
fn dead_branch_without_closing_brace_uses_line_count_as_end() -> Result<(), String> {
    // Malformed Perl: if(0) { ... with no closing brace
    // find_block_end should return lines.len() (4 lines = line 4)
    let source = "if (0) {\n    print 'dead';\n    my $x = 1;\n";
    let branches = dead_branches_in("file:///bc_no_close.pl", "/bc_no_close.pl", source)?;
    assert!(!branches.is_empty(), "unclosed if(0) should still be detected as a dead branch");
    // end_line == lines.len() == 3 (3 lines total)
    assert_eq!(
        branches[0].end_line, 3,
        "end_line should equal total line count for unclosed block; got {}",
        branches[0].end_line
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — blank line after terminator does NOT fire
// (lib.rs line 133: !trimmed.is_empty() False branch — blank line after return)
// Already partially covered but we confirm the blank line is skipped.
// ---------------------------------------------------------------------------

#[test]
fn blank_line_immediately_after_return_not_flagged_as_unreachable() -> Result<(), String> {
    // `return;\n\nprint 1;` — blank line is skipped; only print 1 is flagged
    let results = analyze(
        "file:///bc_blank_after_return.pl",
        "/bc_blank_after_return.pl",
        "return;\n\nprint 1;\n",
    )?;
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    // The blank line (line 2) must NOT appear in results
    assert!(
        unreachable.iter().all(|d| d.start_line != 2),
        "blank line should not be flagged as unreachable; got {unreachable:?}"
    );
    // The real statement at line 3 should be flagged
    assert!(
        unreachable.iter().any(|d| d.start_line == 3),
        "print at line 3 should be flagged as unreachable; got {unreachable:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — comment line after terminator does NOT fire
// (lib.rs line 134: !trimmed.starts_with('#') False branch)
// ---------------------------------------------------------------------------

#[test]
fn comment_line_after_return_not_flagged_as_unreachable() -> Result<(), String> {
    // `return;\n# just a comment\nprint 1;` — comment is skipped
    let results = analyze(
        "file:///bc_comment_after_return.pl",
        "/bc_comment_after_return.pl",
        "return;\n# just a comment\nprint 1;\n",
    )?;
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    // Comment at line 2 must NOT be flagged
    assert!(
        unreachable.iter().all(|d| d.start_line != 2),
        "comment should not be flagged as unreachable; got {unreachable:?}"
    );
    // print at line 3 should be flagged
    assert!(
        unreachable.iter().any(|d| d.start_line == 3),
        "statement at line 3 should be flagged as unreachable; got {unreachable:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — terminator in nested block: depth resets when we leave the block
// (lib.rs line 130: current_depth < *term_depth — the True branch)
// ---------------------------------------------------------------------------

#[test]
fn return_inside_nested_block_does_not_flag_code_at_outer_scope() -> Result<(), String> {
    // `return` is inside an `if` block (depth 1). The statement at depth 0 after
    // the if-block should NOT be flagged as unreachable.
    let source = "if ($x) {\n    return 1;\n}\nprint 'reachable';\n";
    let results = analyze("file:///bc_nested_return.pl", "/bc_nested_return.pl", source)?;
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        unreachable.is_empty(),
        "code after a nested return should not be flagged as unreachable; got {unreachable:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — detect_unconditional_terminator: comment on same line as terminator
// (lib.rs line 244-245: split_once('#') Some branch)
// ---------------------------------------------------------------------------

#[test]
fn return_with_inline_comment_is_still_a_terminator() -> Result<(), String> {
    // `return; # end of function` — inline comment should not prevent detection
    let results = analyze(
        "file:///bc_return_comment.pl",
        "/bc_return_comment.pl",
        "return; # end of function\nprint 'unreachable';\n",
    )?;
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "return with inline comment should still terminate; code after it is unreachable"
    );
    assert_eq!(unreachable[0].start_line, 2);
    Ok(())
}

#[test]
fn die_with_inline_comment_is_still_a_terminator() -> Result<(), String> {
    // `die "oops"; # fatal` — inline comment present
    let results = analyze(
        "file:///bc_die_comment.pl",
        "/bc_die_comment.pl",
        "die \"oops\"; # fatal error\nprint 'unreachable';\n",
    )?;
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "die with inline comment should still be a terminator; got nothing"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — is_keyword_boundary: `before` True but `after` False
// (lib.rs line 265: before boundary OK but after boundary NOT OK)
// ---------------------------------------------------------------------------

#[test]
fn return_if_compound_is_not_unconditional_terminator() -> Result<(), String> {
    // `return if $cond;` — postfix `if` after return means conditional
    // The "if" keyword appears in `return if $cond` with before=space (boundary OK)
    // and after=space (boundary OK) — this one IS a keyword boundary match.
    // We want to test a case where the keyword match FAILS boundary: `iffy`
    let results = analyze(
        "file:///bc_iffy_return.pl",
        "/bc_iffy_return.pl",
        "return iffy_value();\nprint 'maybe reachable';\n",
    )?;
    // "iffy" contains "if" at position 0 with after='f' — not a keyword boundary
    // So `contains_postfix_condition` should NOT match "iffy" as "if"
    // Thus return IS an unconditional terminator here
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "`return iffy_value()` should be an unconditional terminator (iffy != if); got nothing"
    );
    Ok(())
}

#[test]
fn keyword_embedded_in_identifier_is_not_postfix_condition() -> Result<(), String> {
    // `die "while_loop_failed";` — "while" is embedded inside a string arg, not a real keyword
    // But the function signature test: `return whileTrue();` — "while" at start but followed by 'T'
    let results = analyze(
        "file:///bc_while_ident.pl",
        "/bc_while_ident.pl",
        "return whileTrue();\nprint 'unreachable';\n",
    )?;
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    // "while" in "whileTrue" has after='T' which is alphanumeric — not a keyword boundary
    // So `whileTrue` does NOT count as postfix `while`; return IS unconditional
    assert!(
        !unreachable.is_empty(),
        "`return whileTrue()` should be unconditional (whileTrue is not postfix while); got nothing"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — CORE::exit terminator
// (lib.rs line 233: "CORE::exit" in TERMINATORS)
// ---------------------------------------------------------------------------

#[test]
fn core_exit_is_an_unconditional_terminator() -> Result<(), String> {
    let results = analyze(
        "file:///bc_core_exit.pl",
        "/bc_core_exit.pl",
        "CORE::exit 0;\nprint 'unreachable';\n",
    )?;
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    assert!(
        !unreachable.is_empty(),
        "CORE::exit should be an unconditional terminator; got nothing"
    );
    assert!(
        unreachable[0].reason.contains("CORE::exit"),
        "reason should mention CORE::exit; got: {}",
        unreachable[0].reason
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — elsif keyword dead branch: line-start detection
// (dead_branches.rs line 15: "elsif" in keyword list)
//
// The detector trims each line and checks if it starts with a keyword.
// These tests exercise the `elsif` keyword path without locking in behavior
// for ordinary inline `} elsif (...) {` formatting.
// ---------------------------------------------------------------------------

#[test]
fn elsif_at_line_start_with_false_condition_emits_dead_branch() -> Result<(), String> {
    // When `elsif` is formatted at the start of its own line (unusual but valid),
    // the detector can match it.
    let source = "if ($x) {\n    print 'maybe';\n}\nelsif (0) {\n    print 'dead';\n}\n";
    let branches =
        dead_branches_in("file:///bc_elsif_linestart.pl", "/bc_elsif_linestart.pl", source)?;
    assert!(!branches.is_empty(), "elsif (0) at line start should emit a dead branch; got nothing");
    assert!(
        branches[0].reason.contains("`elsif`"),
        "reason should mention `elsif`; got: {}",
        branches[0].reason
    );
    assert!(branches[0].reason.contains("always false"));
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — stats: DeadBranch, UnusedConstant, and UnusedPackage counters
// (lib.rs lines 216-219: stats match arms)
// ---------------------------------------------------------------------------

#[test]
fn analyze_workspace_counts_dead_branch_stats() -> Result<(), String> {
    // Two dead branches in one file: stats.dead_branches should be 2
    let source = "if (0) {\n    print 'dead1';\n}\nwhile (0) {\n    print 'dead2';\n}\n";
    let index = WorkspaceIndex::new();
    let uri = perl_uri::fs_path_to_uri(PathBuf::from("/two_dead.pl")).map_err(|e| e.to_string())?;
    index.index_file_str(&uri, source)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    assert_eq!(
        analysis.stats.dead_branches, 2,
        "should count two dead branches in stats; got {}",
        analysis.stats.dead_branches
    );
    Ok(())
}

#[test]
fn analyze_workspace_counts_unused_constant_and_package_stats() -> Result<(), String> {
    let source = "package Dead::Pkg;\nuse constant DEAD_CONST => 1;\n1;\n";
    let index = WorkspaceIndex::new();
    let uri = perl_uri::fs_path_to_uri(PathBuf::from("/dead_pkg.pl")).map_err(|e| e.to_string())?;
    index.index_file_str(&uri, source)?;
    let detector = DeadCodeDetector::new(index);

    let analysis = detector.analyze_workspace();
    let unused_constant_count =
        analysis.dead_code.iter().filter(|d| d.code_type == DeadCodeType::UnusedConstant).count();
    let unused_package_count =
        analysis.dead_code.iter().filter(|d| d.code_type == DeadCodeType::UnusedPackage).count();

    assert!(
        unused_constant_count > 0,
        "workspace should report at least one unused constant; got {:?}",
        analysis.dead_code
    );
    assert!(
        unused_package_count > 0,
        "workspace should report at least one unused package; got {:?}",
        analysis.dead_code
    );
    assert_eq!(analysis.stats.unused_constants, unused_constant_count);
    assert_eq!(analysis.stats.unused_packages, unused_package_count);
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — is_always_false: nested paren True branch
// (dead_branches.rs line 86: nested paren recursion — already partially covered,
// but add explicit multi-level nesting test)
// ---------------------------------------------------------------------------

#[test]
fn if_double_nested_paren_false_condition_emits_dead_branch() -> Result<(), String> {
    // `if (((0)))` — three levels of parens around 0; still always false
    let branches = dead_branches_in(
        "file:///bc_triple_nested.pl",
        "/bc_triple_nested.pl",
        "if (((0))) {\n    print 'dead';\n}\n",
    )?;
    assert!(!branches.is_empty(), "if (((0))) should be a dead branch; got nothing");
    assert!(branches[0].reason.contains("always false"));
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — elsif with always-true condition (unless/until check for elsif)
// elsif is not "unless"/"until" so it goes through is_always_false path
// ---------------------------------------------------------------------------

#[test]
fn elsif_with_variable_condition_is_not_dead() -> Result<(), String> {
    // `elsif ($y > 0)` — not a constant; no dead branch
    let source = "if ($x) {\n    print 'a';\n}\nelsif ($y > 0) {\n    print 'b';\n}\n";
    let branches = dead_branches_in("file:///bc_elsif_var.pl", "/bc_elsif_var.pl", source)?;
    assert!(
        branches.is_empty(),
        "elsif with non-constant condition should not be a dead branch; got {branches:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lib.rs — is_structural_line: line of only semicolons is structural
// (lib.rs line 229: all chars are '}' or ';')
// ---------------------------------------------------------------------------

#[test]
fn structural_semicolon_line_after_terminator_not_flagged() -> Result<(), String> {
    // A line of just `;` after return should be considered structural (not flagged)
    let results = analyze(
        "file:///bc_semicolon_line.pl",
        "/bc_semicolon_line.pl",
        "return 1;\n;\nprint 'maybe';\n",
    )?;
    let unreachable: Vec<_> =
        results.iter().filter(|d| d.code_type == DeadCodeType::UnreachableCode).collect();
    // The `;` line (line 2) should not be flagged as unreachable (it's structural)
    assert!(
        unreachable.iter().all(|d| d.start_line != 2),
        "lone semicolon should be structural and not flagged; got {unreachable:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// dead_branches.rs — condition with single quote (is_always_false branch)
// (dead_branches.rs line 85: "''" match arm)
// ---------------------------------------------------------------------------

#[test]
fn if_single_quoted_empty_string_emits_dead_branch() -> Result<(), String> {
    // `if ('')` — empty single-quoted string is always false
    let branches = dead_branches_in(
        "file:///bc_if_sq_empty.pl",
        "/bc_if_sq_empty.pl",
        "if ('') {\n    print 'dead';\n}\n",
    )?;
    assert!(!branches.is_empty(), "if ('') should be a dead branch; got nothing");
    assert!(branches[0].reason.contains("always false"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Multiple dead branches on consecutive lines — block-advancement test
// Tests that i = end_line skips to after the dead block correctly
// ---------------------------------------------------------------------------

#[test]
fn two_consecutive_dead_branches_both_detected() -> Result<(), String> {
    // Both `if (0)` blocks should be detected even though they're adjacent
    let source =
        "if (0) {\n    print 'dead1';\n}\nif (0) {\n    print 'dead2';\n}\nprint 'live';\n";
    let branches =
        dead_branches_in("file:///bc_two_consecutive.pl", "/bc_two_consecutive.pl", source)?;
    assert_eq!(
        branches.len(),
        2,
        "both consecutive if(0) blocks should be detected; got {branches:?}"
    );
    assert_eq!(branches[0].start_line, 1);
    assert_eq!(branches[1].start_line, 4);
    Ok(())
}

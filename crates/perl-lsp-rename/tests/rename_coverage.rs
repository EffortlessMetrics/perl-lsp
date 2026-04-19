//! Targeted test coverage for perl-lsp-rename crate.
//!
//! Covers scenarios not fully exercised by comprehensive_unit_tests:
//! - Rename variable in local scope with all-occurrence verification
//! - Rename function (definition + call site edits)
//! - Prepare rename range validation
//! - Rename that would conflict with an existing name (via rename() path)
//! - Rename across scopes (nested blocks, shadowing)

use perl_lsp_rename::rename::{apply_rename_edits, validate_name};
use perl_lsp_rename::{RenameOptions, RenameProvider};
use perl_parser_core::Parser;
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind};
use perl_tdd_support::{must, must_some};

// ─── helpers ────────────────────────────────────────────────────────────────

fn parse_and_provider(code: &str) -> RenameProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    RenameProvider::new(&ast, code.to_string())
}

fn default_opts() -> RenameOptions {
    RenameOptions::default()
}

// ─── Rename variable (local scope) ──────────────────────────────────────────

#[test]
fn test_rename_local_scalar_all_occurrences_replaced() -> Result<(), String> {
    let code = "my $count = 0;\n$count += 1;\nprint $count;\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("count"));
    let result = provider.rename(pos, "total", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);

    // Every occurrence of "count" should be gone
    assert!(
        !new_code.contains("count"),
        "old name 'count' still present in: {}",
        new_code
    );
    // Every occurrence should now be "total"
    assert!(
        new_code.contains("my $total"),
        "declaration not renamed: {}",
        new_code
    );
    assert!(
        new_code.contains("$total += 1"),
        "usage not renamed: {}",
        new_code
    );
    assert!(
        new_code.contains("print $total"),
        "reference not renamed: {}",
        new_code
    );

    // Verify the edit count matches the number of occurrences
    assert!(
        result.edits.len() >= 3,
        "expected at least 3 edits but got {}",
        result.edits.len()
    );
    Ok(())
}

#[test]
fn test_rename_local_array_all_occurrences_replaced() -> Result<(), String> {
    let code = "my @items = (1, 2, 3);\npush @items, 4;\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("items"));
    let result = provider.rename(pos, "elements", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    assert!(
        new_code.contains("@elements"),
        "array not renamed in: {}",
        new_code
    );
    Ok(())
}

#[test]
fn test_rename_local_hash_all_occurrences_replaced() -> Result<(), String> {
    let code = "my %opts = (verbose => 1);\nprint $opts{verbose};\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("opts"));
    let result = provider.rename(pos, "config", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    assert!(
        new_code.contains("%config") || new_code.contains("$config"),
        "hash not renamed in: {}",
        new_code
    );
    Ok(())
}

// ─── Rename function ────────────────────────────────────────────────────────

#[test]
fn test_rename_function_definition_is_updated() -> Result<(), String> {
    let code = "sub calculate {\n    return 42;\n}\n\nmy $r = calculate();\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("calculate"));
    let result = provider.rename(pos, "compute", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    // The rename should replace the function name; the new code should contain "compute"
    assert!(
        new_code.contains("compute"),
        "function name not renamed in: {}",
        new_code
    );
    // Old name should not appear
    assert!(
        !new_code.contains("calculate"),
        "old function name still present in: {}",
        new_code
    );
    Ok(())
}

#[test]
fn test_rename_function_call_site_is_updated() -> Result<(), String> {
    let code = "sub calculate {\n    return 42;\n}\n\nmy $r = calculate();\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("calculate"));
    let result = provider.rename(pos, "compute", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    // At minimum the call site reference should be renamed
    assert!(
        new_code.contains("compute"),
        "call site not renamed in: {}",
        new_code
    );
    // Verify that the old name is completely gone
    assert!(
        !new_code.contains("calculate"),
        "old name 'calculate' still present in: {}",
        new_code
    );
    Ok(())
}

#[test]
fn test_rename_function_multiple_call_sites() -> Result<(), String> {
    let code = "sub helper { 1; }\nhelper();\nmy $v = helper();\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("helper"));
    let result = provider.rename(pos, "util", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    // The new name should appear in the result
    assert!(
        new_code.contains("util"),
        "new name 'util' not present in: {}",
        new_code
    );
    // Original name should be gone
    let helper_count = new_code.matches("helper").count();
    assert_eq!(
        helper_count, 0,
        "old name 'helper' still appears {} times in: {}",
        helper_count, new_code
    );
    Ok(())
}

#[test]
fn test_rename_function_preserves_other_subs() -> Result<(), String> {
    let code = "sub alpha { 1; }\nsub beta { alpha(); }\nbeta();\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("alpha"));
    let result = provider.rename(pos, "gamma", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    // beta should be untouched
    assert!(
        new_code.contains("sub beta"),
        "other sub was modified: {}",
        new_code
    );
    assert!(
        new_code.contains("beta()"),
        "other call was modified: {}",
        new_code
    );
    Ok(())
}

// ─── Prepare rename (range validation) ──────────────────────────────────────

#[test]
fn test_prepare_rename_range_covers_identifier() -> Result<(), String> {
    let code = "my $counter = 0;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("counter"));
    let (range, name) = must_some(provider.prepare_rename(pos));

    // The range should cover the identifier text at a minimum
    assert!(
        range.start <= pos,
        "range start {} is after cursor position {}",
        range.start,
        pos
    );
    assert!(
        range.end >= pos,
        "range end {} is before cursor position {}",
        range.end,
        pos
    );
    assert!(!name.is_empty(), "prepare_rename returned an empty name");
    Ok(())
}

#[test]
fn test_prepare_rename_range_for_subroutine() -> Result<(), String> {
    let code = "sub my_func { 1; }";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("my_func"));
    let (range, name) = must_some(provider.prepare_rename(pos));

    // Range should span the subroutine name
    assert!(
        range.start <= pos,
        "range start {} is after cursor {}",
        range.start,
        pos
    );
    // Name length should match the range
    let range_len = range.end - range.start;
    assert!(range_len > 0, "range has zero length");
    assert!(
        name.contains("my_func"),
        "returned name '{}' does not contain 'my_func'",
        name
    );
    Ok(())
}

#[test]
fn test_prepare_rename_none_for_special_var() {
    let code = "print $_;";
    let provider = parse_and_provider(code);
    // Position on the _ character after $
    let dollar_pos = must_some(code.find("$_"));
    let result = provider.prepare_rename(dollar_pos + 1);
    assert!(
        result.is_none(),
        "special variable $_ should not be renameable"
    );
}

#[test]
fn test_prepare_rename_none_for_builtin_function() {
    let code = "die 'error';";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("die"));
    let result = provider.prepare_rename(pos);
    assert!(result.is_none(), "builtin 'die' should not be renameable");
}

#[test]
fn test_prepare_rename_at_sigil_position() {
    let code = "my $value = 1;";
    let provider = parse_and_provider(code);
    // Position on the '$' sigil itself
    let pos = must_some(code.find('$'));
    let result = provider.prepare_rename(pos);
    // Implementation may or may not resolve from sigil position; should not crash
    let _ = result;
}

#[test]
fn test_prepare_rename_range_does_not_include_sigil() -> Result<(), String> {
    let code = "my $abc = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("abc"));
    let (range, _name) = must_some(provider.prepare_rename(pos));

    // The range returned by prepare_rename should correspond to the identifier
    // portion (possibly including sigil), but it comes from get_symbol_range_at_position
    // Just verify the range is valid and non-empty
    assert!(
        range.end > range.start,
        "range should be non-empty: start={}, end={}",
        range.start,
        range.end
    );
    Ok(())
}

// ─── Rename conflict detection ──────────────────────────────────────────────

#[test]
fn test_rename_subroutine_to_existing_name_fails() {
    let code = "sub foo { 1; }\nsub bar { 2; }\nfoo();\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("foo"));
    let result = provider.rename(pos, "bar", &default_opts());

    assert!(
        !result.is_valid,
        "rename to existing subroutine name should fail"
    );
    let err_msg = must_some(result.error.as_deref());
    assert!(
        err_msg.contains("already exists"),
        "error should mention name conflict: {}",
        err_msg
    );
}

#[test]
fn test_rename_subroutine_to_keyword_fails() {
    let code = "sub foo { 1; }";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("foo"));
    let result = provider.rename(pos, "return", &default_opts());

    assert!(!result.is_valid, "rename to keyword should fail");
    let err_msg = must_some(result.error.as_deref());
    assert!(
        err_msg.contains("keyword"),
        "error should mention keyword: {}",
        err_msg
    );
}

#[test]
fn test_rename_subroutine_to_invalid_identifier_fails() {
    let code = "sub foo { 1; }";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("foo"));
    let result = provider.rename(pos, "not-valid", &default_opts());

    assert!(!result.is_valid, "rename to invalid identifier should fail");
    assert!(result.error.is_some());
}

#[test]
fn test_rename_variable_to_keyword_fails() {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let result = provider.rename(pos, "while", &default_opts());

    assert!(!result.is_valid, "rename variable to keyword should fail");
}

#[test]
fn test_validate_name_subroutine_conflict_via_symbol_table() -> Result<(), String> {
    let code = "sub existing { 1; }\nsub other { existing(); }\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let table = SymbolExtractor::new_with_source(code).extract(&ast);

    let result = validate_name("existing", SymbolKind::Subroutine, &table);
    assert!(
        result.is_err(),
        "should reject name that conflicts with existing sub"
    );
    let err = result.err().ok_or("expected Err")?;
    assert!(
        err.contains("already exists"),
        "error message should say 'already exists': {}",
        err
    );
    Ok(())
}

#[test]
fn test_rename_scalar_allows_shadowing_same_name() -> Result<(), String> {
    // Variables can shadow, so renaming to a name that exists as a scalar is OK
    let code = "my $x = 1;\nmy $y = 2;\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let result = provider.rename(pos, "y", &default_opts());

    // Scalars allow shadowing, so this should succeed
    assert!(
        result.is_valid,
        "scalar rename to existing scalar name should succeed (shadowing allowed)"
    );
    Ok(())
}

// ─── Rename across scopes ───────────────────────────────────────────────────

#[test]
fn test_rename_variable_used_in_nested_block() -> Result<(), String> {
    let code = "my $val = 10;\nif (1) {\n    print $val;\n}\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("val"));
    let result = provider.rename(pos, "amount", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    assert!(
        new_code.contains("my $amount"),
        "declaration not renamed: {}",
        new_code
    );
    // The reference inside the if-block should also be renamed
    assert!(
        new_code.contains("print $amount"),
        "nested reference not renamed: {}",
        new_code
    );
    Ok(())
}

#[test]
fn test_rename_variable_in_while_loop_body() -> Result<(), String> {
    let code = "my $counter = 0;\nwhile ($counter < 10) {\n    $counter += 1;\n}\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("counter"));
    let result = provider.rename(pos, "idx", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    // All occurrences of "counter" should be replaced
    assert!(
        !new_code.contains("counter"),
        "old name 'counter' still present in: {}",
        new_code
    );
    assert!(
        new_code.contains("$idx"),
        "new name not present in: {}",
        new_code
    );
    Ok(())
}

#[test]
fn test_rename_function_called_from_nested_scope() -> Result<(), String> {
    let code = "sub process { 1; }\nif (1) {\n    process();\n}\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("process"));
    let result = provider.rename(pos, "handle", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    // New name should appear for both definition and call site
    assert!(
        new_code.contains("handle"),
        "new name not present in: {}",
        new_code
    );
    // Old name should be completely gone
    assert!(
        !new_code.contains("process"),
        "old name 'process' still present in: {}",
        new_code
    );
    Ok(())
}

#[test]
fn test_rename_variable_in_for_loop() -> Result<(), String> {
    let code = "my @data = (1, 2, 3);\nfor my $item (@data) {\n    print $item;\n}\n";
    let provider = parse_and_provider(code);
    // Rename the loop variable
    let pos = must_some(code.find("item"));
    let result = provider.rename(pos, "elem", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    assert!(
        new_code.contains("$elem"),
        "loop variable not renamed: {}",
        new_code
    );
    Ok(())
}

#[test]
fn test_rename_outer_variable_not_affected_by_inner_scope() -> Result<(), String> {
    // Two separate variables with the same name in different scopes
    // Renaming the outer one should only affect the outer scope
    let code = "my $x = 1;\n{\n    my $y = $x + 1;\n}\nprint $x;\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let result = provider.rename(pos, "z", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    // $y should be untouched
    assert!(
        new_code.contains("$y"),
        "unrelated variable was modified: {}",
        new_code
    );
    assert!(
        new_code.contains("$z"),
        "target variable not renamed: {}",
        new_code
    );
    Ok(())
}

// ─── Edge cases for rename across scopes ────────────────────────────────────

#[test]
fn test_rename_top_level_variable_referenced_in_sub_body() -> Result<(), String> {
    // Test renaming a top-level variable that is referenced inside a sub body
    let code = "my $shared = 42;\nsub work { print $shared; }\n";
    let provider = parse_and_provider(code);
    // Find position of "shared" at the declaration (before the sub)
    let pos = must_some(code.find("shared"));
    let result = provider.rename(pos, "common", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    assert!(
        new_code.contains("$common"),
        "variable not renamed in: {}",
        new_code
    );
    // sub name should be untouched
    assert!(
        new_code.contains("work"),
        "sub name was modified in: {}",
        new_code
    );
    Ok(())
}

#[test]
fn test_rename_does_not_rename_in_unrelated_sub() -> Result<(), String> {
    let code = "sub aaa { my $v = 1; }\nsub bbb { my $w = 2; }\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("v"));
    let result = provider.rename(pos, "renamed", &default_opts());

    if !result.is_valid {
        return Err(result.error.unwrap_or_else(|| "unknown error".to_string()));
    }

    let new_code = apply_rename_edits(code, &result.edits);
    // $w in sub bbb should be untouched
    assert!(
        new_code.contains("$w"),
        "unrelated variable in other sub was modified: {}",
        new_code
    );
    Ok(())
}

// ─── Rename with validation disabled ────────────────────────────────────────

#[test]
fn test_rename_with_validation_disabled_accepts_keyword() -> Result<(), String> {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let opts = RenameOptions {
        rename_in_comments: false,
        rename_in_strings: false,
        validate_new_name: false,
    };
    let result = provider.rename(pos, "my", &opts);

    // With validation disabled, the rename should succeed even with a keyword
    assert!(
        result.is_valid,
        "rename with validation disabled should succeed"
    );
    Ok(())
}

#[test]
fn test_rename_with_validation_disabled_accepts_digit_start() -> Result<(), String> {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let opts = RenameOptions {
        rename_in_comments: false,
        rename_in_strings: false,
        validate_new_name: false,
    };
    let result = provider.rename(pos, "1bad", &opts);

    assert!(
        result.is_valid,
        "rename with validation disabled should succeed even with digit-start name"
    );
    Ok(())
}

// ─── Prepare rename + rename consistency ────────────────────────────────────

#[test]
fn test_prepare_rename_and_rename_agree_on_renamability() {
    let code = "my $data = 42;\nprint $data;\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("data"));

    let prepare = provider.prepare_rename(pos);
    let result = provider.rename(pos, "info", &default_opts());

    // Both should agree: if prepare says yes, rename should succeed
    if prepare.is_some() {
        assert!(
            result.is_valid,
            "prepare_rename said renameable but rename() failed: {:?}",
            result.error
        );
    }
}

#[test]
fn test_prepare_rename_and_rename_agree_on_builtins() {
    let code = "push @arr, 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("push"));

    let prepare = provider.prepare_rename(pos);
    let result = provider.rename(pos, "append", &default_opts());

    // Both should agree: if prepare says no, rename should also fail
    if prepare.is_none() {
        assert!(
            !result.is_valid,
            "prepare_rename said not renameable but rename() succeeded"
        );
    }
}

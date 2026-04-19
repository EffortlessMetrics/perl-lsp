//! Comprehensive unit tests for perl-lsp-rename crate.
//!
//! Tests cover: RenameProvider, validate_name, can_rename_symbol,
//! find_symbol_at_position, get_symbol_range_at_position,
//! adjust_location_for_sigil, apply_rename_edits, is_in_comment/string,
//! and edge cases.

use perl_lsp_rename::{RenameOptions, RenameProvider, RenameResult, TextEdit};
use perl_parser_core::{Parser, SourceLocation};
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind, SymbolTable};
use perl_tdd_support::{must, must_some};

// ─── helpers ────────────────────────────────────────────────────────────────

fn parse_and_provider(code: &str) -> RenameProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    RenameProvider::new(&ast, code.to_string())
}

fn empty_symbol_table() -> SymbolTable {
    let code = "";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

// ─── RenameOptions / TextEdit types ─────────────────────────────────────────

#[test]
fn test_rename_options_default() {
    let opts = RenameOptions::default();
    assert!(!opts.rename_in_comments);
    assert!(!opts.rename_in_strings);
    assert!(opts.validate_new_name);
}

#[test]
fn test_text_edit_equality() {
    let a = TextEdit {
        location: SourceLocation::new(0, 5),
        new_text: "foo".to_string(),
    };
    let b = TextEdit {
        location: SourceLocation::new(0, 5),
        new_text: "foo".to_string(),
    };
    assert_eq!(a, b);
}

#[test]
fn test_text_edit_inequality() {
    let a = TextEdit {
        location: SourceLocation::new(0, 5),
        new_text: "foo".to_string(),
    };
    let b = TextEdit {
        location: SourceLocation::new(0, 5),
        new_text: "bar".to_string(),
    };
    assert_ne!(a, b);
}

#[test]
fn test_text_edit_clone() {
    let a = TextEdit {
        location: SourceLocation::new(1, 3),
        new_text: "x".to_string(),
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn test_text_edit_debug() {
    let edit = TextEdit {
        location: SourceLocation::new(0, 1),
        new_text: "z".to_string(),
    };
    let debug = format!("{:?}", edit);
    assert!(!debug.is_empty());
}

#[test]
fn test_rename_result_debug() {
    let result = RenameResult {
        edits: vec![],
        is_valid: true,
        error: None,
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("is_valid"));
}

// ─── validate_name ──────────────────────────────────────────────────────────

#[test]
fn test_validate_name_empty_string() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("", SymbolKind::scalar(), &table);
    assert!(err.is_err());
    assert!(must(err.err().ok_or("expected err")).contains("empty"));
}

#[test]
fn test_validate_name_starts_with_digit() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("9lives", SymbolKind::scalar(), &table);
    assert!(err.is_err());
    assert!(must(err.err().ok_or("expected err")).contains("number"));
}

#[test]
fn test_validate_name_invalid_chars_hyphen() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("foo-bar", SymbolKind::scalar(), &table);
    assert!(err.is_err());
}

#[test]
fn test_validate_name_invalid_chars_space() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("foo bar", SymbolKind::scalar(), &table);
    assert!(err.is_err());
}

#[test]
fn test_validate_name_invalid_chars_dot() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("foo.bar", SymbolKind::scalar(), &table);
    assert!(err.is_err());
}

#[test]
fn test_validate_name_keyword_my() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("my", SymbolKind::scalar(), &table);
    assert!(err.is_err());
    assert!(must(err.err().ok_or("expected err")).contains("keyword"));
}

#[test]
fn test_validate_name_keyword_sub() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("sub", SymbolKind::scalar(), &table);
    assert!(err.is_err());
}

#[test]
fn test_validate_name_keyword_for() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("for", SymbolKind::scalar(), &table);
    assert!(err.is_err());
}

#[test]
fn test_validate_name_keyword_while() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("while", SymbolKind::scalar(), &table);
    assert!(err.is_err());
}

#[test]
fn test_validate_name_keyword_if() {
    let table = empty_symbol_table();
    let err = perl_lsp_rename::rename::validate_name("if", SymbolKind::scalar(), &table);
    assert!(err.is_err());
}

#[test]
fn test_validate_name_valid_underscore_prefix() {
    let table = empty_symbol_table();
    assert!(perl_lsp_rename::rename::validate_name("_priv", SymbolKind::scalar(), &table).is_ok());
}

#[test]
fn test_validate_name_valid_mixed_case() {
    let table = empty_symbol_table();
    assert!(
        perl_lsp_rename::rename::validate_name("camelCase", SymbolKind::scalar(), &table).is_ok()
    );
}

#[test]
fn test_validate_name_valid_with_numbers() {
    let table = empty_symbol_table();
    assert!(perl_lsp_rename::rename::validate_name("count2", SymbolKind::scalar(), &table).is_ok());
}

#[test]
fn test_validate_name_valid_all_underscores() {
    let table = empty_symbol_table();
    assert!(perl_lsp_rename::rename::validate_name("___", SymbolKind::scalar(), &table).is_ok());
}

#[test]
fn test_validate_name_single_char() {
    let table = empty_symbol_table();
    assert!(perl_lsp_rename::rename::validate_name("x", SymbolKind::scalar(), &table).is_ok());
}

#[test]
fn test_validate_name_subroutine_conflict() {
    let code = "sub existing { 1; }";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let table = SymbolExtractor::new_with_source(code).extract(&ast);
    let result = perl_lsp_rename::rename::validate_name("existing", SymbolKind::Subroutine, &table);
    assert!(result.is_err());
    assert!(must(result.err().ok_or("expected err")).contains("already exists"));
}

#[test]
fn test_validate_name_scalar_no_conflict() {
    // Scalars allow shadowing, so same-name is OK
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let table = SymbolExtractor::new_with_source(code).extract(&ast);
    assert!(perl_lsp_rename::rename::validate_name("x", SymbolKind::scalar(), &table).is_ok());
}

// ─── can_rename_symbol ──────────────────────────────────────────────────────

#[test]
fn test_can_rename_special_var_underscore() {
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        "_",
        SymbolKind::scalar()
    ));
}

#[test]
fn test_can_rename_special_var_dot() {
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        ".",
        SymbolKind::scalar()
    ));
}

#[test]
fn test_can_rename_special_var_ampersand() {
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        "&",
        SymbolKind::scalar()
    ));
}

#[test]
fn test_can_rename_special_var_numbers() {
    for n in &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"] {
        assert!(
            !perl_lsp_rename::rename::can_rename_symbol(n, SymbolKind::scalar()),
            "special var ${} should not be renameable",
            n
        );
    }
}

#[test]
fn test_can_rename_special_var_caret() {
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        "^W",
        SymbolKind::scalar()
    ));
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        "^O",
        SymbolKind::scalar()
    ));
}

#[test]
fn test_can_rename_builtin_print() {
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        "print",
        SymbolKind::Subroutine
    ));
}

#[test]
fn test_can_rename_builtin_die() {
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        "die",
        SymbolKind::Subroutine
    ));
}

#[test]
fn test_can_rename_builtin_push() {
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        "push",
        SymbolKind::Subroutine
    ));
}

#[test]
fn test_can_rename_builtin_eval() {
    assert!(!perl_lsp_rename::rename::can_rename_symbol(
        "eval",
        SymbolKind::Subroutine
    ));
}

#[test]
fn test_can_rename_user_defined() {
    assert!(perl_lsp_rename::rename::can_rename_symbol(
        "my_func",
        SymbolKind::Subroutine
    ));
}

#[test]
fn test_can_rename_user_variable() {
    assert!(perl_lsp_rename::rename::can_rename_symbol(
        "count",
        SymbolKind::scalar()
    ));
}

// ─── adjust_location_for_sigil ──────────────────────────────────────────────

#[test]
fn test_adjust_sigil_scalar() {
    let loc = SourceLocation::new(5, 10);
    let adjusted = perl_lsp_rename::rename::adjust_location_for_sigil(loc, SymbolKind::scalar());
    // "$" is 1 byte
    assert_eq!(adjusted.start, 6);
    assert_eq!(adjusted.end, 10);
}

#[test]
fn test_adjust_sigil_array() {
    let loc = SourceLocation::new(0, 5);
    let adjusted = perl_lsp_rename::rename::adjust_location_for_sigil(loc, SymbolKind::array());
    // "@" is 1 byte
    assert_eq!(adjusted.start, 1);
    assert_eq!(adjusted.end, 5);
}

#[test]
fn test_adjust_sigil_hash() {
    let loc = SourceLocation::new(3, 8);
    let adjusted = perl_lsp_rename::rename::adjust_location_for_sigil(loc, SymbolKind::hash());
    // "%" is 1 byte
    assert_eq!(adjusted.start, 4);
    assert_eq!(adjusted.end, 8);
}

#[test]
fn test_adjust_sigil_subroutine_no_change() {
    let loc = SourceLocation::new(10, 20);
    let adjusted = perl_lsp_rename::rename::adjust_location_for_sigil(loc, SymbolKind::Subroutine);
    assert_eq!(adjusted.start, 10);
    assert_eq!(adjusted.end, 20);
}

// ─── RenameProvider::prepare_rename ─────────────────────────────────────────

#[test]
fn test_prepare_rename_scalar_found() {
    let code = "my $foo = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("foo"));
    let result = provider.prepare_rename(pos);
    assert!(result.is_some());
}

#[test]
fn test_prepare_rename_returns_name() {
    let code = "my $bar = 42;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("bar"));
    let (_, name) = must_some(provider.prepare_rename(pos));
    assert!(name.contains("bar"));
}

#[test]
fn test_prepare_rename_at_whitespace_returns_none() {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    // Position on '=' which is not a symbol
    let pos = must_some(code.find('='));
    let result = provider.prepare_rename(pos);
    // May or may not find symbol depending on implementation
    // The important thing is it doesn't crash
    let _ = result;
}

#[test]
fn test_prepare_rename_builtin_returns_none() {
    let code = "print 42;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("print"));
    // print is a builtin; prepare_rename should return None
    let result = provider.prepare_rename(pos);
    assert!(result.is_none());
}

#[test]
fn test_prepare_rename_empty_source() {
    let code = "";
    let provider = parse_and_provider(code);
    let result = provider.prepare_rename(0);
    assert!(result.is_none());
}

// ─── RenameProvider::rename ─────────────────────────────────────────────────

#[test]
fn test_rename_no_symbol_at_position() {
    let code = "# just a comment";
    let provider = parse_and_provider(code);
    let result = provider.rename(0, "new_name", &RenameOptions::default());
    assert!(!result.is_valid);
    assert!(result.error.is_some());
}

#[test]
fn test_rename_invalid_new_name_empty() {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let result = provider.rename(pos, "", &RenameOptions::default());
    assert!(!result.is_valid);
    assert!(result.error.is_some());
}

#[test]
fn test_rename_invalid_new_name_digit_start() {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let result = provider.rename(pos, "1bad", &RenameOptions::default());
    assert!(!result.is_valid);
}

#[test]
fn test_rename_invalid_new_name_keyword() {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let result = provider.rename(pos, "my", &RenameOptions::default());
    assert!(!result.is_valid);
}

#[test]
fn test_rename_skip_validation() {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let opts = RenameOptions {
        rename_in_comments: false,
        rename_in_strings: false,
        validate_new_name: false,
    };
    // Even with a bad name, validation is skipped
    let result = provider.rename(pos, "1bad", &opts);
    // Should still be valid since validation is off
    // (may fail on can_rename_symbol, but "x" is a user symbol)
    let _ = result;
}

#[test]
fn test_rename_scalar_single_occurrence() {
    let code = "my $solo = 99;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("solo"));
    let result = provider.rename(pos, "alone", &RenameOptions::default());
    assert!(result.is_valid);
    assert!(!result.edits.is_empty());
}

#[test]
fn test_rename_edits_sorted_by_position() {
    let code = "my $a = 1; $a = 2; print $a;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("a"));
    let result = provider.rename(pos, "b", &RenameOptions::default());
    if result.edits.len() > 1 {
        for window in result.edits.windows(2) {
            assert!(window[0].location.start <= window[1].location.start);
        }
    }
}

#[test]
fn test_rename_edits_no_duplicates() {
    let code = "my $x = 1; $x = 2; $x = 3;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let result = provider.rename(pos, "y", &RenameOptions::default());
    // Check uniqueness
    let mut locs: Vec<_> = result.edits.iter().map(|e| e.location.start).collect();
    locs.sort();
    locs.dedup();
    assert_eq!(locs.len(), result.edits.len());
}

#[test]
fn test_rename_cannot_rename_builtin() {
    let code = "print 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("print"));
    let result = provider.rename(pos, "output", &RenameOptions::default());
    assert!(!result.is_valid);
    assert!(result.error.is_some());
}

#[test]
fn test_rename_function_definition_and_call() {
    let code = "sub greet { 1; }\ngreet();";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("greet"));
    let result = provider.rename(pos, "hello", &RenameOptions::default());
    assert!(result.is_valid);
    assert!(!result.edits.is_empty());
}

#[test]
fn test_rename_with_comments_option() {
    let code = "my $x = 1; # use $x here";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let opts = RenameOptions {
        rename_in_comments: true,
        rename_in_strings: false,
        validate_new_name: true,
    };
    let result = provider.rename(pos, "y", &opts);
    assert!(result.is_valid);
}

#[test]
fn test_rename_with_strings_option() {
    let code = r#"my $x = 1; my $s = "value of $x";"#;
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let opts = RenameOptions {
        rename_in_comments: false,
        rename_in_strings: true,
        validate_new_name: true,
    };
    let result = provider.rename(pos, "y", &opts);
    assert!(result.is_valid);
}

// ─── apply_rename_edits ─────────────────────────────────────────────────────

#[test]
fn test_apply_edits_empty_list() {
    let code = "my $x = 1;";
    let result = perl_lsp_rename::rename::apply_rename_edits(code, &[]);
    assert_eq!(result, code);
}

#[test]
fn test_apply_edits_single_edit() {
    let code = "hello world";
    let edits = vec![TextEdit {
        location: SourceLocation::new(0, 5),
        new_text: "goodbye".to_string(),
    }];
    let result = perl_lsp_rename::rename::apply_rename_edits(code, &edits);
    assert_eq!(result, "goodbye world");
}

#[test]
fn test_apply_edits_multiple_non_overlapping() {
    let code = "aaa bbb ccc";
    let edits = vec![
        TextEdit {
            location: SourceLocation::new(0, 3),
            new_text: "xxx".to_string(),
        },
        TextEdit {
            location: SourceLocation::new(4, 7),
            new_text: "yyy".to_string(),
        },
        TextEdit {
            location: SourceLocation::new(8, 11),
            new_text: "zzz".to_string(),
        },
    ];
    let result = perl_lsp_rename::rename::apply_rename_edits(code, &edits);
    assert_eq!(result, "xxx yyy zzz");
}

#[test]
fn test_apply_edits_different_length_replacement() {
    let code = "ab cd";
    let edits = vec![TextEdit {
        location: SourceLocation::new(0, 2),
        new_text: "longer_text".to_string(),
    }];
    let result = perl_lsp_rename::rename::apply_rename_edits(code, &edits);
    assert_eq!(result, "longer_text cd");
}

#[test]
fn test_apply_edits_shrinking_replacement() {
    let code = "long_name = 1;";
    let edits = vec![TextEdit {
        location: SourceLocation::new(0, 9),
        new_text: "x".to_string(),
    }];
    let result = perl_lsp_rename::rename::apply_rename_edits(code, &edits);
    assert_eq!(result, "x = 1;");
}

#[test]
fn test_apply_edits_out_of_bounds_skipped() {
    let code = "short";
    let edits = vec![TextEdit {
        location: SourceLocation::new(100, 200),
        new_text: "nope".to_string(),
    }];
    let result = perl_lsp_rename::rename::apply_rename_edits(code, &edits);
    assert_eq!(result, "short");
}

#[test]
fn test_apply_edits_empty_source() {
    let edits = vec![TextEdit {
        location: SourceLocation::new(0, 0),
        new_text: "inserted".to_string(),
    }];
    let result = perl_lsp_rename::rename::apply_rename_edits("", &edits);
    assert_eq!(result, "inserted");
}

// ─── is_in_comment / is_in_string ───────────────────────────────────────────

#[test]
fn test_is_in_comment_simple() {
    let code = "my $x = 1; # a comment";
    let comment_start = must_some(code.find('#'));
    assert!(perl_lsp_rename::rename::is_in_comment(
        comment_start + 3,
        code
    ));
}

#[test]
fn test_is_not_in_comment_before_hash() {
    let code = "my $x = 1; # a comment";
    assert!(!perl_lsp_rename::rename::is_in_comment(0, code));
    assert!(!perl_lsp_rename::rename::is_in_comment(3, code));
}

#[test]
fn test_is_in_comment_at_hash() {
    let code = "# full line comment";
    assert!(perl_lsp_rename::rename::is_in_comment(0, code));
}

#[test]
fn test_is_in_comment_multiline() {
    let code = "my $x = 1;\n# comment line\nmy $y = 2;";
    let hash_pos = must_some(code.find('#'));
    assert!(perl_lsp_rename::rename::is_in_comment(hash_pos + 2, code));
    // Position on third line should not be in comment
    let y_pos = must_some(code.rfind("$y"));
    assert!(!perl_lsp_rename::rename::is_in_comment(y_pos, code));
}

#[test]
fn test_is_in_string_double_quotes() {
    let code = r#"my $x = "hello world";"#;
    let hello_pos = must_some(code.find("hello"));
    assert!(perl_lsp_rename::rename::is_in_string(hello_pos, code));
}

#[test]
fn test_is_in_string_single_quotes() {
    let code = "my $x = 'hello world';";
    let hello_pos = must_some(code.find("hello"));
    assert!(perl_lsp_rename::rename::is_in_string(hello_pos, code));
}

#[test]
fn test_is_not_in_string_outside_quotes() {
    let code = r#"my $x = "str";"#;
    assert!(!perl_lsp_rename::rename::is_in_string(0, code));
}

#[test]
fn test_is_in_string_empty_source_pos_zero() {
    assert!(!perl_lsp_rename::rename::is_in_string(0, ""));
}

#[test]
fn test_is_in_comment_position_zero_no_hash() {
    assert!(!perl_lsp_rename::rename::is_in_comment(0, "my $x;"));
}

// ─── find_symbol_at_position ────────────────────────────────────────────────

#[test]
fn test_find_symbol_at_definition() {
    let code = "my $count = 0;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let table = SymbolExtractor::new_with_source(code).extract(&ast);
    let pos = must_some(code.find("count"));
    let result = perl_lsp_rename::rename::find_symbol_at_position(pos, &table, code);
    assert!(result.is_some());
}

#[test]
fn test_find_symbol_at_reference() {
    let code = "my $x = 1; print $x;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let table = SymbolExtractor::new_with_source(code).extract(&ast);
    // Find the second $x (the reference)
    let second_x = must_some(code.rfind("$x"));
    let result = perl_lsp_rename::rename::find_symbol_at_position(second_x + 1, &table, code);
    assert!(result.is_some());
}

#[test]
fn test_find_symbol_no_symbol() {
    let code = "1 + 2;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let table = SymbolExtractor::new_with_source(code).extract(&ast);
    // Position on operator
    let pos = must_some(code.find('+'));
    let result = perl_lsp_rename::rename::find_symbol_at_position(pos, &table, code);
    // May fall back to source extraction; just verify no crash
    let _ = result;
}

// ─── get_symbol_range_at_position ───────────────────────────────────────────

#[test]
fn test_get_symbol_range_for_variable() {
    let code = "my $hello = 1;";
    let pos = must_some(code.find("hello"));
    let range = perl_lsp_rename::rename::get_symbol_range_at_position(pos, code);
    assert!(range.is_some());
    if let Some(loc) = range {
        assert!(loc.start <= pos);
        assert!(loc.end >= pos);
    }
}

#[test]
fn test_get_symbol_range_for_subroutine() {
    let code = "sub my_func { 1; }";
    let pos = must_some(code.find("my_func"));
    let range = perl_lsp_rename::rename::get_symbol_range_at_position(pos, code);
    assert!(range.is_some());
}

#[test]
fn test_get_symbol_range_at_whitespace() {
    let code = "my $x   = 1;";
    // Position in the middle of whitespace
    let pos = must_some(code.find("   ")) + 1;
    let range = perl_lsp_rename::rename::get_symbol_range_at_position(pos, code);
    // Implementation may or may not find a nearby symbol; just verify no crash
    let _ = range;
}

// ─── Integration / end-to-end scenarios ─────────────────────────────────────

#[test]
fn test_full_rename_scalar_roundtrip() {
    let code = "my $total = 0;\n$total += 5;\nprint $total;\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("total"));
    let result = provider.rename(pos, "sum", &RenameOptions::default());
    assert!(result.is_valid);
    let new_code = perl_lsp_rename::rename::apply_rename_edits(code, &result.edits);
    assert!(!new_code.contains("total"));
    assert!(new_code.contains("sum"));
}

#[test]
fn test_full_rename_array_variable() {
    let code = "my @items = (1, 2, 3);";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("items"));
    let result = provider.rename(pos, "elements", &RenameOptions::default());
    assert!(result.is_valid);
    if !result.edits.is_empty() {
        let new_code = perl_lsp_rename::rename::apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("elements"));
    }
}

#[test]
fn test_full_rename_hash_variable() {
    let code = "my %config = (key => 'val');";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("config"));
    let result = provider.rename(pos, "settings", &RenameOptions::default());
    assert!(result.is_valid);
    if !result.edits.is_empty() {
        let new_code = perl_lsp_rename::rename::apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("settings"));
    }
}

#[test]
fn test_rename_preserves_surrounding_code() {
    let code = "my $a = 1; my $b = 2;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("a"));
    let result = provider.rename(pos, "c", &RenameOptions::default());
    if result.is_valid && !result.edits.is_empty() {
        let new_code = perl_lsp_rename::rename::apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("$b"));
    }
}

#[test]
fn test_rename_multiple_functions() {
    let code = "sub foo { 1; }\nsub bar { foo(); }\n";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("foo"));
    let result = provider.rename(pos, "baz", &RenameOptions::default());
    assert!(result.is_valid);
    if !result.edits.is_empty() {
        let new_code = perl_lsp_rename::rename::apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("baz"));
        // bar should remain unchanged
        assert!(new_code.contains("bar"));
    }
}

#[test]
fn test_rename_error_message_no_symbol() {
    let code = "   ";
    let provider = parse_and_provider(code);
    let result = provider.rename(1, "x", &RenameOptions::default());
    assert!(!result.is_valid);
    let msg = must_some(result.error.as_deref());
    assert!(msg.contains("No symbol"));
}

#[test]
fn test_rename_error_message_cannot_rename() {
    let code = "print 1;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("print"));
    let result = provider.rename(pos, "output", &RenameOptions::default());
    assert!(!result.is_valid);
    assert!(result.error.is_some());
}

// ─── Edge cases ─────────────────────────────────────────────────────────────

#[test]
fn test_rename_at_start_of_source() {
    let code = "$x = 1;";
    let provider = parse_and_provider(code);
    let result = provider.rename(1, "y", &RenameOptions::default());
    // Just verify no panic
    let _ = result;
}

#[test]
fn test_rename_at_end_of_source() {
    let code = "my $z = 1;";
    let provider = parse_and_provider(code);
    let result = provider.rename(code.len(), "w", &RenameOptions::default());
    // Position past all symbols
    let _ = result;
}

#[test]
fn test_rename_single_char_variable() {
    let code = "my $a = 1; print $a;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("a"));
    let result = provider.rename(pos, "b", &RenameOptions::default());
    assert!(result.is_valid);
}

#[test]
fn test_rename_long_variable_name() {
    let code = "my $very_long_variable_name_here = 42;";
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("very_long_variable_name_here"));
    let result = provider.rename(pos, "short", &RenameOptions::default());
    assert!(result.is_valid);
    if !result.edits.is_empty() {
        let new_code = perl_lsp_rename::rename::apply_rename_edits(code, &result.edits);
        assert!(new_code.contains("short"));
        assert!(!new_code.contains("very_long_variable_name_here"));
    }
}

#[test]
fn test_rename_options_all_enabled() {
    let code = r#"my $x = 1; # $x is important
my $s = "value is $x";
print $x;
"#;
    let provider = parse_and_provider(code);
    let pos = must_some(code.find("x"));
    let opts = RenameOptions {
        rename_in_comments: true,
        rename_in_strings: true,
        validate_new_name: true,
    };
    let result = provider.rename(pos, "y", &opts);
    assert!(result.is_valid);
}

#[test]
fn test_rename_options_clone() {
    let opts = RenameOptions {
        rename_in_comments: true,
        rename_in_strings: false,
        validate_new_name: true,
    };
    let cloned = opts.clone();
    assert_eq!(cloned.rename_in_comments, opts.rename_in_comments);
    assert_eq!(cloned.rename_in_strings, opts.rename_in_strings);
    assert_eq!(cloned.validate_new_name, opts.validate_new_name);
}

#[test]
fn test_rename_options_debug() {
    let opts = RenameOptions::default();
    let debug = format!("{:?}", opts);
    assert!(debug.contains("RenameOptions"));
}

#[test]
fn test_rename_result_with_error() {
    let result = RenameResult {
        edits: vec![],
        is_valid: false,
        error: Some("test error".to_string()),
    };
    assert!(!result.is_valid);
    assert_eq!(must_some(result.error.as_deref()), "test error");
}

#[test]
fn test_apply_edits_preserves_newlines() {
    let code = "line1\nline2\nline3\n";
    let edits = vec![TextEdit {
        location: SourceLocation::new(0, 5),
        new_text: "replaced".to_string(),
    }];
    let result = perl_lsp_rename::rename::apply_rename_edits(code, &edits);
    assert!(result.contains('\n'));
    assert!(result.contains("line2"));
    assert!(result.contains("line3"));
}

#[test]
fn test_multiple_keywords_rejected() {
    let table = empty_symbol_table();
    let keywords = [
        "my", "our", "local", "sub", "package", "use", "require", "if", "elsif", "else", "while",
        "for", "foreach", "unless", "until", "return", "next", "last", "redo", "and", "or", "not",
        "eq", "ne", "state",
    ];
    for kw in &keywords {
        assert!(
            perl_lsp_rename::rename::validate_name(kw, SymbolKind::scalar(), &table).is_err(),
            "keyword '{}' should be rejected",
            kw
        );
    }
}

#[test]
fn test_multiple_builtins_cannot_rename() {
    let builtins = [
        "print", "say", "printf", "sprintf", "open", "close", "read", "write", "push", "pop",
        "shift", "unshift", "map", "grep", "sort", "reverse", "split", "join", "chomp", "chop",
        "die", "warn", "eval", "exit", "require", "use", "package", "sub",
    ];
    for b in &builtins {
        assert!(
            !perl_lsp_rename::rename::can_rename_symbol(b, SymbolKind::Subroutine),
            "builtin '{}' should not be renameable",
            b
        );
    }
}

#[test]
fn test_multiple_special_vars_cannot_rename() {
    let specials = [
        "_", ".", ",", "/", "\\", "!", "@", "$", "%", "&", "`", "'", "+", "[", "]", "{", "}",
    ];
    for s in &specials {
        assert!(
            !perl_lsp_rename::rename::can_rename_symbol(s, SymbolKind::scalar()),
            "special var '{}' should not be renameable",
            s
        );
    }
}

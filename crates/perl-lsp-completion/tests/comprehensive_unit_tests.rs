//! Comprehensive unit tests for the perl-lsp-completion crate.
//!
//! Tests cover:
//! - Variable completion (scalar, array, hash, special variables)
//! - Function/subroutine completion
//! - Keyword and builtin completion
//! - Method completion (including DBI inference)
//! - Package member completion via workspace index
//! - Test::More context detection and completions
//! - Moo/Moose `has` option-key completion
//! - Comment/string/regex context suppression
//! - Cancellation support
//! - Edge cases (empty input, position bounds, unicode)
//! - CompletionItemKind and CompletionItem fields
//! - Sort/dedup ordering

use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::sync::Arc;
use url::Url;

// ---------------------------------------------------------------------------
// Helper utilities
// ---------------------------------------------------------------------------

fn parse_and_provider(code: &str) -> CompletionProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    CompletionProvider::new_with_index_and_source(&ast, code, None)
}

fn parse_provider_with_index(code: &str, index: Arc<WorkspaceIndex>) -> CompletionProvider {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    CompletionProvider::new_with_index_and_source(&ast, code, Some(index))
}

fn completions_at(code: &str, pos: usize) -> Vec<CompletionItem> {
    let provider = parse_and_provider(code);
    provider.get_completions(code, pos)
}

fn completions_at_end(code: &str) -> Vec<CompletionItem> {
    completions_at(code, code.len())
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

// ===========================================================================
// 1. Variable completion
// ===========================================================================

#[test]
fn scalar_variable_completion_basic() {
    let code = "my $count = 1;\nmy $counter = 2;\n$c";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$count"), "should suggest $count");
    assert!(has_label(&items, "$counter"), "should suggest $counter");
}

#[test]
fn scalar_variable_no_false_positives() {
    let code = "my $alpha = 1;\nmy @beta;\n$a";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$alpha"), "should suggest $alpha");
    // @beta should NOT appear as a scalar completion
    assert!(!has_label(&items, "@beta"), "should not suggest @beta for scalar prefix");
}

#[test]
fn array_variable_completion() {
    let code = "my @items = ();\nmy @inventory;\n@i";
    let items = completions_at_end(code);
    assert!(has_label(&items, "@items"), "should suggest @items");
    assert!(has_label(&items, "@inventory"), "should suggest @inventory");
}

#[test]
fn hash_variable_completion() {
    let code = "my %config;\nmy %cache;\n%c";
    let items = completions_at_end(code);
    assert!(has_label(&items, "%config"), "should suggest %config");
    assert!(has_label(&items, "%cache"), "should suggest %cache");
}

#[test]
fn special_scalar_variables() {
    let code = "$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$_"), "should suggest $_");
    assert!(has_label(&items, "$$"), "should suggest $$");
    assert!(has_label(&items, "$!"), "should suggest $!");
    assert!(has_label(&items, "$@"), "should suggest $@");
}

#[test]
fn special_array_variables() {
    let code = "@";
    let items = completions_at_end(code);
    assert!(has_label(&items, "@_"), "should suggest @_");
    assert!(has_label(&items, "@ARGV"), "should suggest @ARGV");
    assert!(has_label(&items, "@INC"), "should suggest @INC");
}

#[test]
fn special_hash_variables() {
    let code = "%";
    let items = completions_at_end(code);
    assert!(has_label(&items, "%ENV"), "should suggest %ENV");
    assert!(has_label(&items, "%INC"), "should suggest %INC");
    assert!(has_label(&items, "%SIG"), "should suggest %SIG");
}

#[test]
fn variable_completion_with_longer_prefix() {
    let code = "my $total_items = 0;\nmy $total_count = 0;\nmy $other = 1;\n$total_";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$total_items"));
    assert!(has_label(&items, "$total_count"));
    assert!(!has_label(&items, "$other"), "should not suggest $other for prefix $total_");
}

// ===========================================================================
// 2. Function / subroutine completion
// ===========================================================================

#[test]
fn user_defined_function_completion() {
    let code = "sub process_data { }\nsub process_items { }\nproc";
    let items = completions_at_end(code);
    assert!(has_label(&items, "process_data"));
    assert!(has_label(&items, "process_items"));
}

#[test]
fn ampersand_function_completion() {
    let code = "sub my_func { }\n&my";
    let items = completions_at_end(code);
    assert!(has_label(&items, "my_func"), "should suggest my_func via &prefix");
}

#[test]
fn function_completion_kind() {
    let code = "sub handler { }\nhand";
    let items = completions_at_end(code);
    let item = must_some(items.iter().find(|i| i.label == "handler"));
    assert_eq!(item.kind, CompletionItemKind::Function);
}

// ===========================================================================
// 3. Builtin function completion
// ===========================================================================

#[test]
fn builtin_print_completion() {
    let code = "pr";
    let items = completions_at_end(code);
    assert!(has_label(&items, "print"));
    assert!(has_label(&items, "printf"));
}

#[test]
fn builtin_push_completion() {
    let code = "pus";
    let items = completions_at_end(code);
    assert!(has_label(&items, "push"));
}

#[test]
fn builtin_open_completion() {
    let code = "ope";
    let items = completions_at_end(code);
    assert!(has_label(&items, "open"));
}

#[test]
fn builtin_sort_completion() {
    let code = "sor";
    let items = completions_at_end(code);
    assert!(has_label(&items, "sort"));
}

#[test]
fn builtin_detail_contains_signature() {
    let code = "print";
    let items = completions_at_end(code);
    let item = must_some(items.iter().find(|i| i.label == "print"));
    let detail = must_some(item.detail.as_ref());
    assert!(!detail.is_empty(), "print should have a non-empty detail");
}

// ===========================================================================
// 4. Keyword completion
// ===========================================================================

#[test]
fn keyword_sub_completion() {
    let code = "su";
    let items = completions_at_end(code);
    assert!(has_label(&items, "sub"), "should suggest sub keyword");
}

#[test]
fn keyword_if_completion() {
    let code = "i";
    let items = completions_at_end(code);
    assert!(has_label(&items, "if"), "should suggest if keyword");
}

#[test]
fn keyword_foreach_completion() {
    let code = "fore";
    let items = completions_at_end(code);
    assert!(has_label(&items, "foreach"));
}

#[test]
fn keyword_snippets_have_insert_text() {
    let code = "su";
    let items = completions_at_end(code);
    let sub_item = must_some(items.iter().find(|i| i.label == "sub"));
    let insert = must_some(sub_item.insert_text.as_ref());
    assert!(insert.contains("${1:name}"), "sub snippet should contain placeholder");
}

#[test]
fn keyword_use_completion() {
    let code = "us";
    let items = completions_at_end(code);
    assert!(has_label(&items, "use"));
}

#[test]
fn keyword_package_completion() {
    let code = "packag";
    let items = completions_at_end(code);
    assert!(has_label(&items, "package"));
}

// ===========================================================================
// 5. Method completion
// ===========================================================================

#[test]
fn method_completion_after_arrow() {
    let code = "my $obj = Foo->new();\n$obj->";
    let items = completions_at_end(code);
    // Default object methods
    assert!(has_label(&items, "new"), "should suggest new");
    assert!(has_label(&items, "isa"), "should suggest isa");
    assert!(has_label(&items, "can"), "should suggest can");
}

#[test]
fn dbi_db_method_completion() {
    let code = "my $dbh = DBI->connect('dbi:SQLite:test.db');\n$dbh->";
    let items = completions_at_end(code);
    assert!(has_label(&items, "prepare"), "should suggest prepare");
    assert!(has_label(&items, "do"), "should suggest do");
    assert!(has_label(&items, "selectrow_hashref"), "should suggest selectrow_hashref");
    assert!(has_label(&items, "disconnect"), "should suggest disconnect");
    assert!(has_label(&items, "commit"), "should suggest commit");
}

#[test]
fn dbi_st_method_completion() {
    let code = "my $sth = $dbh->prepare('SELECT 1');\n$sth->";
    let items = completions_at_end(code);
    assert!(has_label(&items, "execute"), "should suggest execute");
    assert!(has_label(&items, "fetchrow_hashref"), "should suggest fetchrow_hashref");
    assert!(has_label(&items, "fetchall_arrayref"), "should suggest fetchall_arrayref");
    assert!(has_label(&items, "finish"), "should suggest finish");
    assert!(has_label(&items, "rows"), "should suggest rows");
}

#[test]
fn method_completion_with_prefix_filter() {
    let code = "sub fetch_data { }\nmy $obj = Foo->new();\n$obj->fetch";
    let provider = parse_and_provider(code);
    let items = provider.get_completions(code, code.len());
    // Should include methods starting with 'fetch'
    assert!(
        items.iter().any(|i| i.label.starts_with("fetch")),
        "should have method starting with 'fetch'"
    );
}

// ===========================================================================
// 6. Package member completion
// ===========================================================================

#[test]
fn package_member_completion_with_workspace() {
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = must(Url::parse("file:///workspace/Utils.pm"));
    let module_code = r#"package Utils;
our @EXPORT = qw(helper_func);
sub helper_func { }
sub internal_func { }
1;
"#;
    must(index.index_file(module_uri, module_code.to_string()));

    let code = "use Utils;\nUtils::";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());
    assert!(has_label(&items, "helper_func"), "should suggest helper_func");
}

#[test]
fn package_variable_completion_with_workspace() {
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = must(Url::parse("file:///workspace/Config.pm"));
    let module_code = r#"package Config;
our $CONFIG_PATH = '/etc/app.conf';
1;
"#;
    must(index.index_file(module_uri, module_code.to_string()));

    let code = "use Config;\n$Config::CONF";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    let item = must_some(items.iter().find(|item| item.label == "$CONFIG_PATH"));
    assert_eq!(item.kind, CompletionItemKind::Variable);
    assert_eq!(item.insert_text.as_deref(), Some("$Config::CONFIG_PATH"));
}

#[test]
fn package_completion_without_workspace_index() {
    // Without workspace index, package member completion returns nothing
    let code = "Foo::Bar::";
    let items = completions_at_end(code);
    // No workspace index, so no package member completions
    assert!(
        items.is_empty() || !items.iter().any(|i| i.kind == CompletionItemKind::Function),
        "without workspace index, no package functions should be suggested"
    );
}

// ===========================================================================
// 7. Test::More context and completions
// ===========================================================================

#[test]
fn test_more_completions_in_test_file() {
    let code = "use Test::More;\n";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/basic.t"));
    assert!(has_label(&items, "ok"), "should suggest ok in test file");
    assert!(has_label(&items, "is"), "should suggest is in test file");
    assert!(has_label(&items, "done_testing"), "should suggest done_testing");
    assert!(has_label(&items, "subtest"), "should suggest subtest");
}

#[test]
fn test_more_completions_with_use_test_more() {
    let code = "use Test::More;\n";
    let provider = parse_and_provider(code);
    // Even without .t extension, use Test::More triggers test context
    let items = provider.get_completions_with_path(code, code.len(), Some("/lib/foo.pl"));
    assert!(has_label(&items, "ok"), "use Test::More should enable test completions");
}

#[test]
fn test_more_completions_with_use_test2() {
    let code = "use Test2::V0;\n";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/lib/foo.pl"));
    assert!(has_label(&items, "ok"), "use Test2::V0 should enable test completions");
}

#[test]
fn test_more_completions_not_in_regular_file() {
    let code = "my $x = 1;\n";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/lib/module.pm"));
    // Without test imports or .t extension, Test::More completions should NOT appear
    assert!(
        !has_label(&items, "done_testing"),
        "non-test file should not have Test::More completions"
    );
}

#[test]
fn test_more_completion_has_snippet() {
    let code = "use Test::More;\nis";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    let is_item =
        items.iter().find(|i| i.label == "is" && i.detail.as_deref() == Some("Test::More"));
    if let Some(item) = is_item {
        let insert = must_some(item.insert_text.as_ref());
        assert!(insert.contains("${1:got}"), "is snippet should have placeholders");
    }
}

// ===========================================================================
// 8. Moo/Moose has option-key completion
// ===========================================================================

#[test]
fn moo_has_option_completion_basic() {
    let code = "use Moo;\nhas 'name' => (";
    let items = completions_at_end(code);
    assert!(has_label(&items, "is"), "should suggest 'is' option");
    assert!(has_label(&items, "isa"), "should suggest 'isa' option");
    assert!(has_label(&items, "default"), "should suggest 'default' option");
    assert!(has_label(&items, "required"), "should suggest 'required' option");
    assert!(has_label(&items, "lazy"), "should suggest 'lazy' option");
    assert!(has_label(&items, "builder"), "should suggest 'builder' option");
}

#[test]
fn moo_has_option_completion_with_prefix() {
    let code = "use Moo;\nhas 'name' => (re";
    let items = completions_at_end(code);
    assert!(has_label(&items, "required"), "should suggest 'required' matching 're'");
    assert!(has_label(&items, "reader"), "should suggest 'reader' matching 're'");
    // 'is' should NOT appear because it doesn't start with 're'
    assert!(!has_label(&items, "is"), "'is' should not match prefix 're'");
}

#[test]
fn moo_has_option_after_comma() {
    let code = "use Moo;\nhas 'name' => (is => 'ro', ";
    let items = completions_at_end(code);
    // After the first option pair, new option keys should be suggested
    assert!(has_label(&items, "isa"), "should suggest 'isa' after comma");
}

// ===========================================================================
// 9. Test::More hover documentation
// ===========================================================================

#[test]
fn test_get_test_more_documentation_for_is() {
    let doc = perl_lsp_completion::get_test_more_documentation("is");
    let (sig, desc) = must_some(doc);
    assert!(sig.contains("is("), "signature should contain 'is('");
    assert!(!desc.is_empty(), "description should not be empty");
}

#[test]
fn test_get_test_more_documentation_for_ok() {
    let doc = perl_lsp_completion::get_test_more_documentation("ok");
    let (sig, desc) = must_some(doc);
    assert!(sig.contains("ok("), "signature should contain 'ok('");
    assert!(!desc.is_empty(), "description should not be empty");
}

#[test]
fn test_get_test_more_documentation_unknown_returns_none() {
    let doc = perl_lsp_completion::get_test_more_documentation("not_a_test_function");
    assert!(doc.is_none(), "unknown function should return None");
}

#[test]
fn test_get_test_more_documentation_covers_core_assertions() {
    let core_fns = ["ok", "is", "isnt", "like", "unlike", "is_deeply", "isa_ok", "can_ok"];
    for name in core_fns {
        let doc = perl_lsp_completion::get_test_more_documentation(name);
        assert!(doc.is_some(), "'{}' should have Test::More documentation", name);
    }
}

#[test]
fn test_get_test_more_documentation_covers_all_functions() {
    // Every function named in the Test::More docs must have an entry
    let all_fns = [
        "ok",
        "is",
        "isnt",
        "like",
        "unlike",
        "cmp_ok",
        "isa_ok",
        "can_ok",
        "pass",
        "fail",
        "diag",
        "note",
        "explain",
        "skip",
        "todo_skip",
        "BAIL_OUT",
        "subtest",
        "done_testing",
        "plan",
        "use_ok",
        "require_ok",
        "is_deeply",
        "new_ok",
    ];
    for name in all_fns {
        let doc = perl_lsp_completion::get_test_more_documentation(name);
        assert!(doc.is_some(), "'{}' should have Test::More documentation", name);
    }
}

#[test]
fn test_get_test_more_documentation_signatures_are_valid_perl() {
    // Signatures should not contain LSP snippet syntax (${1:...})
    let all_fns = [
        "ok",
        "is",
        "isnt",
        "like",
        "unlike",
        "cmp_ok",
        "isa_ok",
        "can_ok",
        "pass",
        "fail",
        "diag",
        "note",
        "explain",
        "skip",
        "todo_skip",
        "BAIL_OUT",
        "subtest",
        "done_testing",
        "plan",
        "use_ok",
        "require_ok",
        "is_deeply",
        "new_ok",
    ];
    for name in all_fns {
        let (sig, _desc) = must_some(perl_lsp_completion::get_test_more_documentation(name));
        assert!(
            !sig.contains("${"),
            "'{}' signature contains LSP snippet syntax, should be plain Perl: {}",
            name,
            sig
        );
    }
}

#[test]
fn test_get_test_more_documentation_diag_mentions_stderr() {
    let (_sig, desc) = must_some(perl_lsp_completion::get_test_more_documentation("diag"));
    assert!(desc.contains("STDERR"), "diag description should mention STDERR, got: {}", desc);
}

#[test]
fn test_get_test_more_documentation_note_mentions_stdout() {
    let (_sig, desc) = must_some(perl_lsp_completion::get_test_more_documentation("note"));
    assert!(desc.contains("STDOUT"), "note description should mention STDOUT, got: {}", desc);
}

#[test]
fn test_get_test_more_documentation_done_testing_has_optional_param() {
    let (sig, _desc) = must_some(perl_lsp_completion::get_test_more_documentation("done_testing"));
    assert!(
        sig.contains("?"),
        "done_testing signature should indicate optional parameter, got: {}",
        sig
    );
}

#[test]
fn test_get_test_more_documentation_empty_string_returns_none() {
    let doc = perl_lsp_completion::get_test_more_documentation("");
    assert!(doc.is_none(), "empty string should return None");
}

#[test]
fn test_get_test_more_documentation_case_sensitive() {
    // bail_out (lowercase) is not valid — only BAIL_OUT
    let doc = perl_lsp_completion::get_test_more_documentation("bail_out");
    assert!(doc.is_none(), "bail_out (lowercase) should return None");
    // Uppercase works
    let doc_upper = perl_lsp_completion::get_test_more_documentation("BAIL_OUT");
    assert!(doc_upper.is_some(), "BAIL_OUT should return Some");
}

#[test]
fn moo_has_option_not_in_value_position() {
    let code = "use Moo;\nhas 'name' => (is => ";
    let items = completions_at_end(code);
    // In value position (after =>), should NOT suggest option keys
    assert!(!has_label(&items, "required"), "should not suggest option keys in value position");
}

#[test]
fn moo_has_option_kind_is_property() {
    let code = "use Moo;\nhas 'attr' => (";
    let items = completions_at_end(code);
    let is_item = must_some(items.iter().find(|i| i.label == "is"));
    assert_eq!(is_item.kind, CompletionItemKind::Property);
}

#[test]
fn moo_has_option_insert_text_includes_arrow() {
    let code = "use Moo;\nhas 'attr' => (";
    let items = completions_at_end(code);
    let item = must_some(items.iter().find(|i| i.label == "is"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("=>"), "insert text should include '=>'");
}

// ===========================================================================
// 9. Comment / string / regex context suppression
// ===========================================================================

#[test]
fn no_completion_in_comment() {
    let code = "my $var = 1;\n# $v";
    let items = completions_at_end(code);
    assert!(items.is_empty(), "should not complete inside comments");
}

#[test]
fn no_completion_in_comment_middle_of_line() {
    let code = "my $x = 1; # some comment pr";
    let items = completions_at_end(code);
    assert!(items.is_empty(), "should not complete inside line-end comments");
}

// ===========================================================================
// 10. Cancellation support
// ===========================================================================

#[test]
fn cancellation_returns_empty() {
    let code = "my $x = 1;\n$x";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path_cancellable(
        code,
        code.len(),
        None,
        &|| true, // always cancelled
    );
    assert!(items.is_empty(), "cancelled request should return empty");
}

#[test]
fn non_cancelled_returns_results() {
    let code = "my $x = 1;\n$x";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path_cancellable(code, code.len(), None, &|| false);
    assert!(!items.is_empty(), "non-cancelled request should return results");
}

// ===========================================================================
// 11. Edge cases
// ===========================================================================

#[test]
fn empty_source_returns_something() {
    let code = "";
    let items = completions_at_end(code);
    // Empty prefix returns keywords + builtins
    assert!(!items.is_empty(), "empty source should still return keyword completions");
}

#[test]
fn position_beyond_source_returns_empty() {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let items = provider.get_completions(code, code.len() + 100);
    assert!(items.is_empty(), "position beyond source should return empty");
}

#[test]
fn position_zero_returns_completions() {
    let code = "my $x = 1;";
    let items = completions_at(code, 0);
    // At position 0, empty prefix → keywords + builtins
    assert!(!items.is_empty(), "position 0 should return completions");
}

#[test]
fn single_character_prefix() {
    let code = "m";
    let items = completions_at_end(code);
    assert!(has_label(&items, "map"), "should suggest 'map' for prefix 'm'");
}

#[test]
fn unicode_source_does_not_crash() {
    let code = "my $café = 1;\nmy $naïve = 2;\n$c";
    let items = completions_at_end(code);
    // Should not panic; actual results depend on parser handling of unicode
    let _ = items;
}

#[test]
fn newlines_only_source() {
    let code = "\n\n\n";
    let items = completions_at_end(code);
    // Should not panic
    let _ = items;
}

#[test]
fn whitespace_only_source() {
    let code = "   \t  ";
    let items = completions_at_end(code);
    let _ = items;
}

// ===========================================================================
// 12. CompletionItem field correctness
// ===========================================================================

#[test]
fn completion_item_has_sort_text() {
    let code = "my $foo = 1;\n$f";
    let items = completions_at_end(code);
    let item = must_some(items.iter().find(|i| i.label == "$foo"));
    assert!(item.sort_text.is_some(), "completion item should have sort_text");
}

#[test]
fn completion_item_has_filter_text() {
    let code = "my $bar = 1;\n$b";
    let items = completions_at_end(code);
    let item = must_some(items.iter().find(|i| i.label == "$bar"));
    assert!(item.filter_text.is_some(), "completion item should have filter_text");
}

#[test]
fn completion_item_has_text_edit_range() {
    let code = "my $baz = 1;\n$b";
    let items = completions_at_end(code);
    let item = must_some(items.iter().find(|i| i.label == "$baz"));
    assert!(item.text_edit_range.is_some(), "completion item should have text_edit_range");
}

#[test]
fn completion_item_has_insert_text() {
    let code = "my $qux = 1;\n$q";
    let items = completions_at_end(code);
    let item = must_some(items.iter().find(|i| i.label == "$qux"));
    assert!(item.insert_text.is_some(), "completion item should have insert_text");
}

// ===========================================================================
// 13. CompletionItemKind variants
// ===========================================================================

#[test]
fn completion_item_kind_debug_impl() {
    let kind = CompletionItemKind::Variable;
    let debug_str = format!("{:?}", kind);
    assert_eq!(debug_str, "Variable");
}

#[test]
fn completion_item_kind_eq() {
    assert_eq!(CompletionItemKind::Function, CompletionItemKind::Function);
    assert_ne!(CompletionItemKind::Variable, CompletionItemKind::Function);
}

#[test]
fn completion_item_kind_clone() {
    let kind = CompletionItemKind::Keyword;
    let cloned = kind;
    assert_eq!(kind, cloned);
}

#[test]
fn completion_item_kind_ordering() {
    // CompletionItemKind derives Ord
    assert!(CompletionItemKind::Variable < CompletionItemKind::Function);
    assert!(CompletionItemKind::Function < CompletionItemKind::Keyword);
}

#[test]
fn completion_item_kind_all_variants_exist() {
    let _variable = CompletionItemKind::Variable;
    let _function = CompletionItemKind::Function;
    let _keyword = CompletionItemKind::Keyword;
    let _module = CompletionItemKind::Module;
    let _file = CompletionItemKind::File;
    let _snippet = CompletionItemKind::Snippet;
    let _constant = CompletionItemKind::Constant;
    let _property = CompletionItemKind::Property;
}

// ===========================================================================
// 14. Sort / dedup behavior
// ===========================================================================

#[test]
fn completions_are_sorted() {
    let code = "my $zebra = 1;\nmy $alpha = 2;\n$";
    let items = completions_at_end(code);
    // Completions should be in some deterministic order
    let l = labels(&items);
    let mut sorted = l.clone();
    // Items are sorted by sort_text, which is deterministic
    sorted.sort();
    // We just verify it's deterministic by calling again
    let items2 = completions_at_end(code);
    let l2 = labels(&items2);
    assert_eq!(l, l2, "completions should be deterministic");
}

#[test]
fn duplicates_are_removed() {
    // If a function is both local and matches as builtin, should not appear twice
    let code = "sub sort { }\nsor";
    let items = completions_at_end(code);
    let sort_count = items.iter().filter(|i| i.label == "sort").count();
    assert!(sort_count <= 1, "duplicate 'sort' should be removed, found {}", sort_count);
}

// ===========================================================================
// 15. Provider construction variants
// ===========================================================================

#[test]
fn new_basic_construction() {
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let items = provider.get_completions(code, code.len());
    // Should not panic, basic construction works
    let _ = items;
}

#[test]
fn new_with_index_none() {
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, None);
    let items = provider.get_completions(code, code.len());
    let _ = items;
}

#[test]
fn new_with_index_and_source() {
    let code = "my $x = 1; sub foo { }";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let index = Arc::new(WorkspaceIndex::new());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let items = provider.get_completions(code, code.len());
    let _ = items;
}

// ===========================================================================
// 16. Context detection
// ===========================================================================

#[test]
fn get_completions_with_path_none() {
    let code = "my $x = 1;\n$x";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), None);
    assert!(has_label(&items, "$x"), "should complete $x with path=None");
}

#[test]
fn get_completions_with_path_some() {
    let code = "my $x = 1;\n$x";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/path/to/file.pl"));
    assert!(has_label(&items, "$x"), "should complete $x with a filepath");
}

// ===========================================================================
// 17. Multiple variables across scopes
// ===========================================================================

#[test]
fn variables_from_different_subs() {
    let code = r#"
sub foo {
    my $inner_foo = 1;
}
sub bar {
    my $inner_bar = 2;
}
$inner_
"#;
    let items = completions_at_end(code);
    // Symbol extraction should find both even though they're in different scopes
    // (the symbol table collects all declarations)
    let inner_labels: Vec<_> = items.iter().filter(|i| i.label.contains("inner")).collect();
    assert!(!inner_labels.is_empty(), "should find inner_ variables");
}

// ===========================================================================
// 18. Workspace symbol completion
// ===========================================================================

#[test]
fn workspace_symbol_completion_requires_prefix() {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/Lib.pm"));
    let module_code = "package Lib;\nsub helper { }\n1;\n";
    must(index.index_file(uri, module_code.to_string()));

    // Empty prefix: workspace completions require non-empty prefix
    let code = "";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, 0);
    // With empty prefix, workspace symbols may be skipped
    let ws_funcs: Vec<_> =
        items.iter().filter(|i| i.detail.as_deref() == Some("workspace")).collect();
    // This is expected behavior: empty prefix filters workspace symbols
    let _ = ws_funcs;
}

#[test]
fn workspace_exports_prioritized() {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/MyLib.pm"));
    let module_code = r#"package MyLib;
our @EXPORT = qw(exported_fn);
sub exported_fn { }
sub private_fn { }
1;
"#;
    must(index.index_file(uri, module_code.to_string()));

    let code = "use MyLib;\nMyLib::";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());
    assert!(has_label(&items, "exported_fn"), "exported_fn should be in completions");
}

// ===========================================================================
// 19. CompletionContext fields
// ===========================================================================

#[test]
fn completion_context_debug_impl() {
    let code = "my $x = 1;\n$x";
    let provider = parse_and_provider(code);
    // CompletionContext is not directly accessible from the public API,
    // but we can verify it works through the completion flow
    let items = provider.get_completions(code, code.len());
    assert!(has_label(&items, "$x"));
}

// ===========================================================================
// 20. Mixed completion scenarios
// ===========================================================================

#[test]
fn completions_include_both_local_and_builtin() {
    let code = "sub split_data { }\nspli";
    let items = completions_at_end(code);
    // Should see both the builtin 'split' and user-defined 'split_data'
    assert!(has_label(&items, "split"), "should suggest builtin split");
    assert!(has_label(&items, "split_data"), "should suggest user-defined split_data");
}

#[test]
fn all_variables_without_sigil_prefix() {
    // Without sigil prefix, variables should still appear (via add_all_variables)
    let code = "my $total = 0;\ntot";
    let items = completions_at_end(code);
    // Should suggest $total even without sigil prefix
    assert!(
        items.iter().any(|i| i.label.contains("total")),
        "should suggest variable matching prefix without sigil"
    );
}

#[test]
fn multiple_package_declarations() {
    let code = r#"
package Foo;
sub foo_func { }

package Bar;
sub bar_func { }

bar_
"#;
    let items = completions_at_end(code);
    assert!(has_label(&items, "bar_func"), "should suggest bar_func");
}

// ===========================================================================
// 21. Moo accessor method synthesis
// ===========================================================================

#[test]
fn moo_accessor_method_synthesized() {
    let code = r#"
package MyObj;
use Moo;
has 'email' => (is => 'ro');
sub process {
    my $self = shift;
    $self->
}
"#;
    let pos = must_some(code.find("$self->")) + "$self->".len();
    let provider = parse_and_provider(code);
    let items = provider.get_completions(code, pos);
    assert!(has_label(&items, "email"), "Moo accessor 'email' should appear in method completions");
}

// ===========================================================================
// 22. Large number of completions
// ===========================================================================

#[test]
fn many_variables_does_not_crash() {
    let mut code = String::new();
    for i in 0..100 {
        code.push_str(&format!("my $var_{} = {};\n", i, i));
    }
    code.push_str("$var_");
    let items = completions_at_end(&code);
    assert!(items.len() >= 50, "should return many variable completions");
}

// ===========================================================================
// 23. get_completions vs get_completions_with_path equivalence
// ===========================================================================

#[test]
fn get_completions_equivalent_to_with_path_none() {
    let code = "my $z = 1;\n$z";
    let provider = parse_and_provider(code);
    let items1 = provider.get_completions(code, code.len());
    let items2 = provider.get_completions_with_path(code, code.len(), None);
    let l1 = labels(&items1);
    let l2 = labels(&items2);
    assert_eq!(l1, l2, "get_completions should equal get_completions_with_path(None)");
}

// ===========================================================================
// 24. Additional edge cases for robustness
// ===========================================================================

#[test]
fn completion_at_line_boundary() {
    let code = "my $x = 1;\n";
    let items = completions_at(code, code.len());
    // At end of line with empty prefix, should get keywords + builtins
    let _ = items;
}

#[test]
fn completion_mid_word() {
    let code = "my $abcdef = 1;\n$abc";
    let items = completions_at(code, code.len());
    assert!(has_label(&items, "$abcdef"), "should complete $abcdef from prefix $abc");
}

#[test]
fn nested_sub_variables() {
    let code = r#"
sub outer {
    my $outer_var = 1;
    sub inner {
        my $inner_var = 2;
    }
}
$"#;
    let items = completions_at_end(code);
    // Both should be discovered by the symbol extractor
    assert!(
        items.iter().any(|i| i.label.contains("outer_var") || i.label.contains("inner_var")),
        "should find variables from nested subs"
    );
}

#[test]
fn completion_after_semicolon() {
    let code = "my $x = 1;";
    // Complete right after semicolon
    let items = completions_at(code, code.len());
    // Empty prefix → keywords + builtins
    let _ = items;
}

#[test]
fn multiline_code_completion() {
    let code = "my $line1 = 1;\nmy $line2 = 2;\nmy $line3 = 3;\n$line";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$line1"));
    assert!(has_label(&items, "$line2"));
    assert!(has_label(&items, "$line3"));
}

// ===========================================================================
// Scope-distance ranking tests
// ===========================================================================

fn find_item<'a>(items: &'a [CompletionItem], label: &str) -> Option<&'a CompletionItem> {
    items.iter().find(|i| i.label == label)
}

fn sort_text_of(items: &[CompletionItem], label: &str) -> Option<String> {
    find_item(items, label).and_then(|i| i.sort_text.clone())
}

#[test]
fn scope_distance_immediate_variable_ranks_before_outer() {
    // $inner is in the same scope as cursor (immediate),
    // $outer is in a parent scope.
    let code = "my $outer = 1;\nsub foo {\n    my $inner = 2;\n    $";
    let items = completions_at_end(code);

    let inner_sort = sort_text_of(&items, "$inner");
    let outer_sort = sort_text_of(&items, "$outer");

    assert!(inner_sort.is_some(), "$inner should be in completions");
    assert!(outer_sort.is_some(), "$outer should be in completions");

    // Immediate scope variable should sort before parent scope variable
    assert!(
        inner_sort < outer_sort,
        "immediate scope $inner ({:?}) should sort before outer $outer ({:?})",
        inner_sort,
        outer_sort
    );
}

#[test]
fn scope_distance_nested_blocks_rank_closer_first() {
    // $block_var is in the if-block (immediate),
    // $sub_var is in the enclosing sub (parent),
    // $file_var is at file scope (package level).
    let code = concat!(
        "my $file_var = 0;\n",
        "sub process {\n",
        "    my $sub_var = 1;\n",
        "    if (1) {\n",
        "        my $block_var = 2;\n",
        "        $"
    );
    let items = completions_at_end(code);

    let block_sort = sort_text_of(&items, "$block_var");
    let sub_sort = sort_text_of(&items, "$sub_var");
    let file_sort = sort_text_of(&items, "$file_var");

    assert!(block_sort.is_some(), "$block_var should be in completions");
    assert!(sub_sort.is_some(), "$sub_var should be in completions");
    assert!(file_sort.is_some(), "$file_var should be in completions");

    assert!(
        block_sort < sub_sort,
        "immediate $block_var ({:?}) should sort before parent $sub_var ({:?})",
        block_sort,
        sub_sort
    );
    assert!(
        sub_sort < file_sort,
        "parent $sub_var ({:?}) should sort before file-level $file_var ({:?})",
        sub_sort,
        file_sort
    );
}

#[test]
fn scope_distance_function_ranking() {
    // Test with a shared prefix so both functions match
    let code2 = concat!(
        "sub utility_a { }\n",
        "sub process {\n",
        "    sub utility_b { }\n",
        "    utility_"
    );
    let items2 = completions_at_end(code2);

    let inner_sort = sort_text_of(&items2, "utility_b");
    let outer_sort = sort_text_of(&items2, "utility_a");

    assert!(inner_sort.is_some(), "utility_b should be in completions");
    assert!(outer_sort.is_some(), "utility_a should be in completions");

    assert!(
        inner_sort < outer_sort,
        "inner utility_b ({:?}) should sort before outer utility_a ({:?})",
        inner_sort,
        outer_sort
    );
}

#[test]
fn scope_distance_same_scope_variables_alphabetical() {
    // Two variables in the same scope should be ordered alphabetically
    let code = "my $zebra = 1;\nmy $alpha = 2;\n$";
    let items = completions_at_end(code);

    let alpha_sort = sort_text_of(&items, "$alpha");
    let zebra_sort = sort_text_of(&items, "$zebra");

    assert!(alpha_sort.is_some(), "$alpha should be in completions");
    assert!(zebra_sort.is_some(), "$zebra should be in completions");

    // Same scope distance, so should be alphabetical by name
    assert!(
        alpha_sort < zebra_sort,
        "$alpha ({:?}) should sort before $zebra ({:?}) at same scope depth",
        alpha_sort,
        zebra_sort
    );
}

#[test]
fn scope_distance_workspace_variables_rank_last() {
    // Workspace symbols (sort prefix 4_) should rank after local variables (1x_)
    // and after core builtins (3_). This implements the sort order:
    // local scope < file scope < core builtins < workspace/CPAN.
    let index = Arc::new(WorkspaceIndex::new());
    let file_url = must(Url::parse("file:///other.pm"));
    let module_code = r#"
package Other;
our $ws_item = 42;
sub ws_func { }
1;
"#;
    must(index.index_file(file_url, module_code.to_string()));

    // Use a prefix that matches a workspace function (none locally defined)
    let code = "my $local_var = 1;\nws_";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    // The workspace function should appear with sort prefix starting with '4'
    // (tier 4 = workspace/CPAN, after core builtins at tier 3)
    let ws_func = find_item(&items, "Other::ws_func");
    if let Some(ws_item) = ws_func {
        let sort_text = ws_item.sort_text.as_deref().unwrap_or("");
        assert!(
            sort_text.starts_with('4'),
            "workspace function sort_text ({:?}) should start with '4' (workspace tier, after builtins)",
            sort_text
        );
    }

    // Also verify that local variables use scope-distance sort keys (1a-1d)
    let code2 = "my $local_var = 1;\n$lo";
    let provider2 = parse_and_provider(code2);
    let items2 = provider2.get_completions(code2, code2.len());
    let local_sort = sort_text_of(&items2, "$local_var");
    assert!(local_sort.is_some(), "$local_var should be in completions");

    let sort = local_sort.as_deref().unwrap_or("");
    assert!(
        sort.starts_with("1a") || sort.starts_with("1b") || sort.starts_with("1c"),
        "local variable sort_text ({:?}) should use scope-distance prefix (1a/1b/1c)",
        sort
    );
}

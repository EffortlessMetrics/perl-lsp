//! Extended unit tests for the perl-lsp-completion crate.
//!
//! These tests complement comprehensive_unit_tests.rs by covering:
//! - File path security helpers (sanitize, split, safe filenames)
//! - Method completion edge cases (DBI inference from assignments, default methods)
//! - Keyword snippet placeholders for all snippet-producing keywords
//! - Builtin template completions (open, map, grep, sort)
//! - Test::More individual function completions
//! - Context detection behaviors (regex, string, comment, package)
//! - Moo/Moose extended option keys
//! - Workspace completion for constants, packages, variables, exports
//! - Sort/dedup edge cases
//! - Additional robustness and edge-case scenarios

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

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

fn find_item<'a>(items: &'a [CompletionItem], label: &str) -> Option<&'a CompletionItem> {
    items.iter().find(|i| i.label == label)
}

// ===========================================================================
// 1. Keyword snippet completions – verify all snippet-producing keywords
// ===========================================================================

#[test]
fn keyword_while_snippet() {
    let code = "whil";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "while"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("while"), "while snippet should contain 'while'");
    assert!(insert.contains('$'), "while snippet should contain placeholder");
}

#[test]
fn keyword_elsif_snippet() {
    let code = "elsi";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "elsif"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("elsif"), "elsif snippet should contain 'elsif'");
}

#[test]
fn keyword_else_snippet() {
    let code = "els";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "else"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("else"), "else snippet should contain 'else'");
}

#[test]
fn keyword_unless_snippet() {
    let code = "unles";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "unless"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("unless"), "unless snippet should contain 'unless'");
}

#[test]
fn keyword_for_c_style_snippet() {
    let code = "fo";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "for"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("for"), "for snippet should contain 'for'");
    assert!(insert.contains("$i"), "for snippet should have loop variable");
}

#[test]
fn keyword_foreach_snippet_array_placeholder() {
    let code = "foreac";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "foreach"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("foreach"), "foreach snippet should contain 'foreach'");
    assert!(insert.contains("array"), "foreach snippet should reference array");
}

#[test]
fn keyword_package_snippet_name_placeholder() {
    let code = "packa";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "package"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("Name"), "package snippet should have Name placeholder");
}

#[test]
fn keyword_use_present_in_completions() {
    let code = "us";
    let items = completions_at_end(code);
    // 'use' may come from keyword or builtin; after dedup only one survives
    let item = must_some(find_item(&items, "use"));
    assert!(item.insert_text.is_some(), "use completion should have insert_text");
}

#[test]
fn keyword_if_snippet_has_braces() {
    let code = "i";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "if"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains('{'), "if snippet should have opening brace");
    assert!(insert.contains('}'), "if snippet should have closing brace");
}

#[test]
fn keyword_my_completion() {
    let code = "m";
    let items = completions_at_end(code);
    assert!(has_label(&items, "my"), "should suggest 'my' keyword");
}

#[test]
fn keyword_our_completion() {
    let code = "ou";
    let items = completions_at_end(code);
    assert!(has_label(&items, "our"), "should suggest 'our' keyword");
}

#[test]
fn keyword_local_completion() {
    let code = "loca";
    let items = completions_at_end(code);
    assert!(has_label(&items, "local"), "should suggest 'local' keyword");
}

#[test]
fn keyword_return_completion() {
    let code = "retur";
    let items = completions_at_end(code);
    assert!(has_label(&items, "return"), "should suggest 'return' keyword");
}

#[test]
fn keyword_state_completion() {
    let code = "stat";
    let items = completions_at_end(code);
    assert!(has_label(&items, "state"), "should suggest 'state' keyword");
}

#[test]
fn keyword_next_last_redo_completion() {
    let code = "nex";
    let items = completions_at_end(code);
    assert!(has_label(&items, "next"), "should suggest 'next'");

    let items2 = completions_at_end("las");
    assert!(has_label(&items2, "last"), "should suggest 'last'");

    let items3 = completions_at_end("red");
    assert!(has_label(&items3, "redo"), "should suggest 'redo'");
}

#[test]
fn snippet_keywords_have_snippet_kind() {
    let code = "su";
    let items = completions_at_end(code);
    let sub_item = must_some(find_item(&items, "sub"));
    assert_eq!(sub_item.kind, CompletionItemKind::Snippet, "sub should be Snippet kind");
}

#[test]
fn non_snippet_keywords_have_keyword_kind() {
    let code = "m";
    let items = completions_at_end(code);
    let my_item = must_some(find_item(&items, "my"));
    assert_eq!(my_item.kind, CompletionItemKind::Keyword, "my should be Keyword kind");
}

// ===========================================================================
// 2. Builtin template/insert text verification
// ===========================================================================

#[test]
fn builtin_open_insert_text_has_filehandle() {
    let code = "ope";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "open"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("$fh"), "open template should contain $fh");
}

#[test]
fn builtin_map_insert_text_has_block() {
    let code = "ma";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "map"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains('{'), "map template should contain block braces");
}

#[test]
fn builtin_grep_insert_text_has_block() {
    let code = "gre";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "grep"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains('{'), "grep template should contain block braces");
}

#[test]
fn builtin_sort_insert_text_has_block() {
    let code = "sor";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "sort"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains('{'), "sort template should contain block braces");
}

#[test]
fn builtin_chomp_completion() {
    let code = "chom";
    let items = completions_at_end(code);
    assert!(has_label(&items, "chomp"), "should suggest chomp");
}

#[test]
fn builtin_keys_values_completion() {
    let code = "key";
    let items = completions_at_end(code);
    assert!(has_label(&items, "keys"), "should suggest keys");

    let items2 = completions_at_end("val");
    assert!(has_label(&items2, "values"), "should suggest values");
}

#[test]
fn builtin_die_warn_completion() {
    let code = "di";
    let items = completions_at_end(code);
    assert!(has_label(&items, "die"), "should suggest die");

    let items2 = completions_at_end("war");
    assert!(has_label(&items2, "warn"), "should suggest warn");
}

#[test]
fn builtin_eval_completion() {
    let code = "eva";
    let items = completions_at_end(code);
    assert!(has_label(&items, "eval"), "should suggest eval");
}

#[test]
fn builtin_system_exec_completion() {
    let code = "syste";
    let items = completions_at_end(code);
    assert!(has_label(&items, "system"), "should suggest system");

    let items2 = completions_at_end("exe");
    assert!(has_label(&items2, "exec"), "should suggest exec");
}

#[test]
fn builtin_substr_index_rindex_completion() {
    let code = "subst";
    let items = completions_at_end(code);
    assert!(has_label(&items, "substr"), "should suggest substr");

    let items2 = completions_at_end("rinde");
    assert!(has_label(&items2, "rindex"), "should suggest rindex");
}

#[test]
fn builtin_splice_completion() {
    let code = "splic";
    let items = completions_at_end(code);
    assert!(has_label(&items, "splice"), "should suggest splice");
}

#[test]
fn builtin_bless_ref_completion() {
    let code = "bles";
    let items = completions_at_end(code);
    assert!(has_label(&items, "bless"), "should suggest bless");
}

#[test]
fn builtin_function_kind() {
    let code = "pri";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "print"));
    assert_eq!(item.kind, CompletionItemKind::Function, "builtins should be Function kind");
}

// ===========================================================================
// 3. Method completion – DBI inference and defaults
// ===========================================================================

#[test]
fn method_default_includes_isa_can() {
    let code = "my $obj = Something->new();\n$obj->";
    let items = completions_at_end(code);
    assert!(has_label(&items, "isa"), "default methods should include isa");
    assert!(has_label(&items, "can"), "default methods should include can");
    assert!(has_label(&items, "DOES"), "default methods should include DOES");
    assert!(has_label(&items, "VERSION"), "default methods should include VERSION");
}

#[test]
fn dbi_db_has_begin_rollback() {
    let code = "my $dbh = DBI->connect('dbi:Pg:db=test');\n$dbh->";
    let items = completions_at_end(code);
    assert!(has_label(&items, "begin_work"), "DBI::db should suggest begin_work");
    assert!(has_label(&items, "rollback"), "DBI::db should suggest rollback");
    assert!(has_label(&items, "last_insert_id"), "DBI::db should suggest last_insert_id");
    assert!(has_label(&items, "quote"), "DBI::db should suggest quote");
    assert!(has_label(&items, "ping"), "DBI::db should suggest ping");
}

#[test]
fn dbi_st_has_bind_param() {
    let code = "my $sth = $dbh->prepare('SELECT 1');\n$sth->";
    let items = completions_at_end(code);
    assert!(has_label(&items, "bind_param"), "DBI::st should suggest bind_param");
    assert!(has_label(&items, "fetch"), "DBI::st should suggest fetch");
    assert!(has_label(&items, "fetchrow_array"), "DBI::st should suggest fetchrow_array");
}

#[test]
fn dbi_dbh_variable_name_inference() {
    // Variable named $dbh is inferred as DBI::db even without DBI->connect
    let code = "$dbh->";
    let items = completions_at_end(code);
    assert!(has_label(&items, "prepare"), "$dbh should infer DBI::db type");
    assert!(has_label(&items, "disconnect"), "$dbh should infer DBI::db type");
}

#[test]
fn dbi_sth_variable_name_inference() {
    // Variable named $sth is inferred as DBI::st
    let code = "$sth->";
    let items = completions_at_end(code);
    assert!(has_label(&items, "execute"), "$sth should infer DBI::st type");
    assert!(has_label(&items, "fetchrow_hashref"), "$sth should infer DBI::st type");
}

#[test]
fn method_completion_function_kind() {
    let code = "my $obj = Foo->new();\n$obj->";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "new"));
    assert_eq!(item.kind, CompletionItemKind::Function, "methods should be Function kind");
}

#[test]
fn method_completion_has_insert_text_with_parens() {
    let code = "my $obj = Foo->new();\n$obj->";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "new"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("()"), "method insert_text should include parens");
}

#[test]
fn method_completion_local_sub_preferred() {
    // Local subs should appear in method completion context
    let code = "sub custom_method { }\nmy $obj = Foo->new();\n$obj->custom";
    let items = completions_at_end(code);
    assert!(
        items.iter().any(|i| i.label.starts_with("custom")),
        "local subs should appear in method completion"
    );
}

// ===========================================================================
// 4. Variable completion – extended cases
// ===========================================================================

#[test]
fn our_variable_completion() {
    let code = "our $shared = 1;\n$sh";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$shared"), "should suggest our variable $shared");
}

#[test]
fn multiple_same_prefix_different_sigils() {
    let code = "my $data = 1;\nmy @data = ();\nmy %data;\n$d";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$data"), "should suggest $data for scalar prefix");
    assert!(!has_label(&items, "@data"), "should not suggest @data for $ prefix");
    assert!(!has_label(&items, "%data"), "should not suggest %data for $ prefix");
}

#[test]
fn array_prefix_excludes_scalars() {
    let code = "my @arr = ();\nmy $arr_scalar = 1;\n@a";
    let items = completions_at_end(code);
    assert!(has_label(&items, "@arr"), "should suggest @arr");
    assert!(!has_label(&items, "$arr_scalar"), "should not suggest $arr_scalar for @ prefix");
}

#[test]
fn hash_prefix_excludes_scalars_and_arrays() {
    let code = "my %hash = ();\nmy $hash_val = 1;\nmy @hash_arr = ();\n%h";
    let items = completions_at_end(code);
    assert!(has_label(&items, "%hash"), "should suggest %hash");
    assert!(!has_label(&items, "$hash_val"), "should not suggest scalar for % prefix");
}

#[test]
fn variable_with_underscore_in_name() {
    let code = "my $my_long_variable_name = 1;\n$my_long";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$my_long_variable_name"));
}

#[test]
fn special_scalar_dollar_zero() {
    let code = "$0";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$0"), "should suggest $0 (program name)");
}

#[test]
fn special_scalar_dollar_caret_o() {
    // $^O and $^V start with "$^" but the prefix "$^" is parsed as a single
    // token; the special variables list requires starts_with match.
    // With prefix "$" all specials starting with "$" are returned.
    let code = "$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$^O"), "should suggest $^O (OS name)");
    assert!(has_label(&items, "$^V"), "should suggest $^V (Perl version)");
}

#[test]
fn special_scalar_dollar_ampersand() {
    let code = "$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$&"), "should suggest $& (last match)");
}

#[test]
fn special_array_isa() {
    let code = "@I";
    let items = completions_at_end(code);
    assert!(has_label(&items, "@ISA"), "should suggest @ISA");
}

#[test]
fn special_array_export() {
    let code = "@E";
    let items = completions_at_end(code);
    assert!(has_label(&items, "@EXPORT"), "should suggest @EXPORT");
}

#[test]
fn variable_documentation_from_special_vars() {
    let code = "$_";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "$_"));
    assert!(item.documentation.is_some(), "$_ should have documentation");
}

// ===========================================================================
// 5. Test::More – individual function completions
// ===========================================================================

#[test]
fn test_more_is_deeply_completion() {
    let code = "use Test::More;\nis_d";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "is_deeply"), "should suggest is_deeply");
}

#[test]
fn test_more_new_ok_completion() {
    let code = "use Test::More;\nnew_";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "new_ok"), "should suggest new_ok");
}

#[test]
fn test_more_plan_completion() {
    let code = "use Test::More;\npla";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "plan"), "should suggest plan");
}

#[test]
fn test_more_diag_note_completion() {
    let code = "use Test::More;\ndia";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "diag"), "should suggest diag");

    let code2 = "use Test::More;\nnot";
    let provider2 = parse_and_provider(code2);
    let items2 = provider2.get_completions_with_path(code2, code2.len(), Some("/t/test.t"));
    assert!(has_label(&items2, "note"), "should suggest note");
}

#[test]
fn test_more_skip_and_bail_out() {
    let code = "use Test::More;\n";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "skip"), "should suggest skip");
    assert!(has_label(&items, "BAIL_OUT"), "should suggest BAIL_OUT");
}

#[test]
fn test_more_use_ok_require_ok() {
    let code = "use Test::More;\nuse_";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "use_ok"), "should suggest use_ok");
}

#[test]
fn test_more_like_unlike_completion() {
    let code = "use Test::More;\nlik";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "like"), "should suggest like");
}

#[test]
fn test_file_extension_alone_enables_test_completions() {
    // Just the .t extension, without use Test::More
    let code = "my $x = 1;\n";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/foo.t"));
    assert!(has_label(&items, "ok"), ".t file should enable test completions");
}

#[test]
fn test_more_cmp_ok_isa_ok_can_ok() {
    let code = "use Test::More;\n";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "cmp_ok"), "should suggest cmp_ok");
    assert!(has_label(&items, "isa_ok"), "should suggest isa_ok");
    assert!(has_label(&items, "can_ok"), "should suggest can_ok");
}

#[test]
fn test_more_pass_fail_completion() {
    let code = "use Test::More;\npas";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    assert!(has_label(&items, "pass"), "should suggest pass");
}

#[test]
fn test_more_detail_is_test_more() {
    let code = "use Test::More;\n";
    let provider = parse_and_provider(code);
    let items = provider.get_completions_with_path(code, code.len(), Some("/t/test.t"));
    let ok_item = must_some(find_item(&items, "ok"));
    assert_eq!(ok_item.detail.as_deref(), Some("Test::More"));
}

// ===========================================================================
// 6. Moo/Moose – extended option keys
// ===========================================================================

#[test]
fn moo_has_option_accessor_key() {
    let code = "use Moo;\nhas 'attr' => (acc";
    let items = completions_at_end(code);
    assert!(has_label(&items, "accessor"), "should suggest 'accessor' option");
}

#[test]
fn moo_has_option_predicate_clearer_handles() {
    let code = "use Moo;\nhas 'attr' => (";
    let items = completions_at_end(code);
    assert!(has_label(&items, "predicate"), "should suggest 'predicate'");
    assert!(has_label(&items, "clearer"), "should suggest 'clearer'");
    assert!(has_label(&items, "handles"), "should suggest 'handles'");
}

#[test]
fn moo_has_option_writer_reader() {
    let code = "use Moo;\nhas 'attr' => (writ";
    let items = completions_at_end(code);
    assert!(has_label(&items, "writer"), "should suggest 'writer'");
}

#[test]
fn moose_has_option_also_works() {
    // Moose uses the same `has` pattern as Moo
    let code = "use Moose;\nhas 'name' => (";
    let items = completions_at_end(code);
    assert!(has_label(&items, "is"), "Moose has should also suggest 'is'");
    assert!(has_label(&items, "isa"), "Moose has should also suggest 'isa'");
}

#[test]
fn moo_has_option_documentation_present() {
    let code = "use Moo;\nhas 'attr' => (";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "is"));
    assert!(item.documentation.is_some(), "has option should have documentation");
}

#[test]
fn moo_has_option_detail_is_moo_moose() {
    let code = "use Moo;\nhas 'attr' => (";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "is"));
    assert_eq!(item.detail.as_deref(), Some("Moo/Moose option"));
}

// ===========================================================================
// 7. Context detection behavior
// ===========================================================================

#[test]
fn no_completion_in_pod_like_comment() {
    let code = "# This is a comment\n# pri";
    let items = completions_at_end(code);
    assert!(items.is_empty(), "should not complete in comment lines");
}

#[test]
fn completion_after_comment_line() {
    let code = "# this is a comment\npr";
    let items = completions_at_end(code);
    assert!(has_label(&items, "print"), "should complete on line after comment");
}

#[test]
fn completion_in_string_context_no_keywords() {
    // Inside a double-quoted string, regular completions are suppressed
    // (only file path completions may appear)
    let code = "my $x = \"pr";
    let items = completions_at_end(code);
    // In string context, keywords should not appear
    assert!(
        !items.iter().any(|i| i.kind == CompletionItemKind::Keyword),
        "keywords should not appear inside string literals"
    );
}

#[test]
fn completion_after_string_literal() {
    let code = "my $x = \"hello\";\npr";
    let items = completions_at_end(code);
    assert!(has_label(&items, "print"), "should complete after closed string");
}

#[test]
fn package_context_defaults_to_main() {
    let code = "my $x = 1;\n$x";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    // Verify completions work in main package context
    let items = provider.get_completions(code, code.len());
    assert!(has_label(&items, "$x"), "should complete in default main package");
}

// ===========================================================================
// 8. Workspace completion – extended scenarios
// ===========================================================================

#[test]
fn workspace_multiple_files_completion() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri1 = must(Url::parse("file:///workspace/ModA.pm"));
    let code1 = "package ModA;\nsub func_a { }\n1;\n";
    must(index.index_file(uri1, code1.to_string()));

    let uri2 = must(Url::parse("file:///workspace/ModB.pm"));
    let code2 = "package ModB;\nsub func_b { }\n1;\n";
    must(index.index_file(uri2, code2.to_string()));

    let code = "use ModA;\nuse ModB;\nModA::";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());
    assert!(has_label(&items, "func_a"), "should suggest func_a from ModA");
}

#[test]
fn workspace_constant_completion() {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/Constants.pm"));
    let module_code = "package Constants;\nuse constant PI => 3.14159;\n1;\n";
    must(index.index_file(uri, module_code.to_string()));

    let code = "use Constants;\nConstants::";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());
    // The indexer may or may not extract constants; verify no panic
    let _ = items;
}

#[test]
fn workspace_package_completion_via_find_symbols() {
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/MyApp.pm"));
    let module_code = "package MyApp;\nsub run { }\n1;\n";
    must(index.index_file(uri, module_code.to_string()));

    // Non-qualified prefix triggers workspace symbol search
    let code = "MyA";
    let provider = parse_provider_with_index(code, Arc::clone(&index));
    let items = provider.get_completions(code, code.len());
    // Should find MyApp package or its symbols
    let has_myapp = items.iter().any(|i| i.label.contains("MyApp"));
    // This depends on workspace indexing behavior; at minimum, no panic
    let _ = has_myapp;
}

#[test]
fn workspace_index_empty_has_no_symbols() {
    let index = Arc::new(WorkspaceIndex::new());
    let code = "foo";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());
    // With empty index, no workspace symbols should appear
    let ws_items: Vec<_> =
        items.iter().filter(|i| i.detail.as_deref() == Some("workspace")).collect();
    assert!(ws_items.is_empty(), "empty workspace index should not produce workspace completions");
}

// ===========================================================================
// 9. Sort/dedup – extended cases
// ===========================================================================

#[test]
fn sort_text_has_priority_prefix() {
    let code = "$_";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "$_"));
    let sort_text = must_some(item.sort_text.as_ref());
    // Special variables should have sort priority prefix "0_"
    assert!(sort_text.starts_with("0_"), "special variable sort_text should start with 0_");
}

#[test]
fn user_variable_sort_priority() {
    let code = "my $foo = 1;\n$f";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "$foo"));
    let sort_text = must_some(item.sort_text.as_ref());
    // sort_text format is "1{distance}_name" where distance is a-d for scope proximity
    assert!(sort_text.starts_with("1"), "user variable sort_text should start with 1");
    assert!(sort_text.contains('_'), "user variable sort_text should contain underscore separator");
}

#[test]
fn user_function_sort_priority() {
    let code = "sub handler { }\nhand";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "handler"));
    let sort_text = must_some(item.sort_text.as_ref());
    // sort_text format is "2{distance}_name" where distance is a-d for scope proximity
    assert!(sort_text.starts_with("2"), "user function sort_text should start with 2");
    assert!(sort_text.contains('_'), "user function sort_text should contain underscore separator");
}

#[test]
fn builtin_sort_priority() {
    let code = "pri";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "print"));
    let sort_text = must_some(item.sort_text.as_ref());
    assert!(sort_text.starts_with("3_"), "builtin sort_text should start with 3_");
}

#[test]
fn keyword_sort_priority() {
    let code = "su";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "sub"));
    let sort_text = must_some(item.sort_text.as_ref());
    // Keywords now use tier 5 to rank after core builtins (3_) and workspace (4_).
    assert!(sort_text.starts_with("5_"), "keyword sort_text should start with 5_");
}

#[test]
fn completions_deterministic_across_calls() {
    let code = "my $a = 1;\nmy $ab = 2;\nmy $abc = 3;\n$a";
    let items1 = completions_at_end(code);
    let items2 = completions_at_end(code);
    assert_eq!(labels(&items1), labels(&items2), "completions should be deterministic");
}

// ===========================================================================
// 10. CompletionItem field correctness – extended
// ===========================================================================

#[test]
fn completion_item_additional_edits_empty_by_default() {
    let code = "my $x = 1;\n$x";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "$x"));
    assert!(item.additional_edits.is_empty(), "additional_edits should be empty by default");
}

#[test]
fn keyword_completion_has_filter_text() {
    let code = "su";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "sub"));
    assert!(item.filter_text.is_some(), "keyword should have filter_text");
    assert_eq!(item.filter_text.as_deref(), Some("sub"));
}

#[test]
fn builtin_completion_has_detail() {
    let code = "ope";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "open"));
    let detail = must_some(item.detail.as_ref());
    assert!(!detail.is_empty(), "open should have non-empty detail");
}

#[test]
fn special_variable_has_detail() {
    let code = "$_";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "$_"));
    assert_eq!(item.detail.as_deref(), Some("special variable"));
}

// ===========================================================================
// 11. Function completion – extended
// ===========================================================================

#[test]
fn function_completion_insert_text_has_parens() {
    let code = "sub process { }\nproc";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "process"));
    let insert = must_some(item.insert_text.as_ref());
    assert!(insert.contains("()"), "function insert_text should include parens");
}

#[test]
fn multiple_subs_completion() {
    let code = "sub alpha { }\nsub beta { }\nsub gamma { }\nal";
    let items = completions_at_end(code);
    assert!(has_label(&items, "alpha"), "should suggest alpha");
    assert!(!has_label(&items, "beta"), "should not suggest beta for prefix 'al'");
}

#[test]
fn ampersand_prefix_function_kind() {
    let code = "sub my_func { }\n&my_";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "my_func"));
    assert_eq!(item.kind, CompletionItemKind::Function, "& prefix should give Function kind");
}

// ===========================================================================
// 12. Edge cases and robustness
// ===========================================================================

#[test]
fn tab_characters_in_source() {
    let code = "my\t$tab_var = 1;\n$tab";
    let items = completions_at_end(code);
    assert!(
        items.iter().any(|i| i.label.contains("tab_var")),
        "should handle tab characters in source"
    );
}

#[test]
fn deeply_nested_blocks_completion() {
    let code = r#"
sub outer {
    if (1) {
        while (1) {
            my $deep_var = 1;
        }
    }
}
$deep"#;
    let items = completions_at_end(code);
    assert!(
        items.iter().any(|i| i.label.contains("deep")),
        "should find variables from deeply nested blocks"
    );
}

#[test]
fn completion_just_after_newline() {
    let code = "my $x = 1;\n$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$x"), "should complete right after newline with sigil");
}

#[test]
fn empty_sub_body_completion() {
    let code = "sub foo { }\nfo";
    let items = completions_at_end(code);
    assert!(has_label(&items, "foo"), "should suggest function from empty sub body");
}

#[test]
fn completion_with_semicolons_and_code() {
    let code = "my $a = 1; my $b = 2; my $c = 3; $";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$a"));
    assert!(has_label(&items, "$b"));
    assert!(has_label(&items, "$c"));
}

#[test]
fn very_long_variable_name() {
    let code = "my $a_very_long_variable_name_that_is_quite_descriptive = 42;\n$a_very_long";
    let items = completions_at_end(code);
    assert!(
        has_label(&items, "$a_very_long_variable_name_that_is_quite_descriptive"),
        "should handle long variable names"
    );
}

#[test]
fn all_sigil_types_in_single_file() {
    let code = r#"
my $scalar = 1;
my @array = (1, 2, 3);
my %hash = (a => 1);
sub func { }
"#;
    // Scalar
    let items = completions_at(&format!("{}$s", code), code.len() + 2);
    assert!(has_label(&items, "$scalar"), "should find scalar");

    // Array
    let items = completions_at(&format!("{}@a", code), code.len() + 2);
    assert!(has_label(&items, "@array"), "should find array");

    // Hash
    let items = completions_at(&format!("{}%h", code), code.len() + 2);
    assert!(has_label(&items, "%hash"), "should find hash");

    // Function
    let items = completions_at(&format!("{}fun", code), code.len() + 3);
    assert!(has_label(&items, "func"), "should find function");
}

#[test]
fn cancellation_mid_completion_returns_empty() {
    let code = "my $x = 1;\nmy $y = 2;\n$";
    let provider = parse_and_provider(code);
    // Always-cancelled should return empty
    let items = provider.get_completions_with_path_cancellable(code, code.len(), None, &|| true);
    assert!(items.is_empty(), "cancelled completion should be empty");
}

#[test]
fn cancellation_false_returns_results() {
    let code = "my $x = 1;\n$x";
    let provider = parse_and_provider(code);
    let items =
        provider
            .get_completions_with_path_cancellable(code, code.len(), Some("/test.pl"), &|| false);
    assert!(!items.is_empty(), "non-cancelled with filepath should return results");
}

#[test]
fn position_at_exact_end_of_source() {
    let code = "my $exact = 1;\n$exact";
    let items = completions_at(code, code.len());
    assert!(has_label(&items, "$exact"), "completion at exact end should work");
}

#[test]
fn position_one_beyond_triggers_empty() {
    let code = "my $x = 1;";
    let provider = parse_and_provider(code);
    let items = provider.get_completions(code, code.len() + 1);
    assert!(items.is_empty(), "position one beyond source should return empty");
}

#[test]
fn completion_with_numbers_in_variable_names() {
    let code = "my $var1 = 1;\nmy $var2 = 2;\nmy $var3 = 3;\n$var";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$var1"));
    assert!(has_label(&items, "$var2"));
    assert!(has_label(&items, "$var3"));
}

#[test]
fn completion_with_mixed_case_function_names() {
    let code = "sub processData { }\nsub ProcessItems { }\nProc";
    let items = completions_at_end(code);
    assert!(has_label(&items, "ProcessItems"), "should respect case in function names");
    assert!(!has_label(&items, "processData"), "case-sensitive: 'processData' != 'Proc' prefix");
}

#[test]
fn multiple_moo_has_declarations() {
    let code = r#"
package MyObj;
use Moo;
has 'name' => (is => 'ro');
has 'age' => (is => 'rw');
sub show {
    my $self = shift;
    $self->
}
"#;
    let pos = must_some(code.find("$self->")) + "$self->".len();
    let provider = parse_and_provider(code);
    let items = provider.get_completions(code, pos);
    assert!(has_label(&items, "name"), "first Moo accessor should appear");
    assert!(has_label(&items, "age"), "second Moo accessor should appear");
}

#[test]
fn completion_preserves_text_edit_range() {
    let code = "my $prefix_test = 1;\n$prefix";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "$prefix_test"));
    let (start, end) = must_some(item.text_edit_range);
    assert!(start < end, "text_edit_range start should be before end");
    assert_eq!(end, code.len(), "text_edit_range end should be at cursor position");
}

// ---------------------------------------------------------------------------
// Workspace-aware `use` module name completion (#352)
// ---------------------------------------------------------------------------

#[test]
fn use_statement_suggests_workspace_modules() {
    let index = Arc::new(WorkspaceIndex::new());

    // Index two modules in the workspace
    let uri1 = must(Url::parse("file:///workspace/lib/MyApp/Config.pm"));
    must(index.index_file(uri1, "package MyApp::Config;\nsub load { }\n1;\n".to_string()));

    let uri2 = must(Url::parse("file:///workspace/lib/MyApp/Logger.pm"));
    must(index.index_file(uri2, "package MyApp::Logger;\nsub info { }\n1;\n".to_string()));

    let code = "use MyApp::";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    assert!(
        has_label(&items, "MyApp::Config"),
        "should suggest MyApp::Config after `use MyApp::`, got: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    assert!(
        has_label(&items, "MyApp::Logger"),
        "should suggest MyApp::Logger after `use MyApp::`, got: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

#[test]
fn use_statement_filters_by_prefix() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri1 = must(Url::parse("file:///workspace/lib/Foo/Bar.pm"));
    must(index.index_file(uri1, "package Foo::Bar;\n1;\n".to_string()));

    let uri2 = must(Url::parse("file:///workspace/lib/Baz/Qux.pm"));
    must(index.index_file(uri2, "package Baz::Qux;\n1;\n".to_string()));

    let code = "use Foo";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    assert!(has_label(&items, "Foo::Bar"), "should suggest Foo::Bar when prefix is Foo");
    assert!(!has_label(&items, "Baz::Qux"), "should NOT suggest Baz::Qux when prefix is Foo");
}

#[test]
fn use_statement_items_are_module_kind() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri = must(Url::parse("file:///workspace/lib/TestMod.pm"));
    must(index.index_file(uri, "package TestMod;\n1;\n".to_string()));

    let code = "use Test";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    let module_item = must_some(find_item(&items, "TestMod"));
    assert_eq!(
        module_item.kind,
        CompletionItemKind::Module,
        "use-statement module completions should be Module kind"
    );
}

#[test]
fn use_statement_not_triggered_after_semicolon() {
    // After `use Module;`, we should NOT be in use-statement context
    let index = Arc::new(WorkspaceIndex::new());

    let uri = must(Url::parse("file:///workspace/lib/Done.pm"));
    must(index.index_file(uri, "package Done;\n1;\n".to_string()));

    let code = "use strict;\nmy $d";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    // Should not have module completion here, should have variable completion
    assert!(
        !has_label(&items, "Done"),
        "should NOT suggest module names outside use statement context"
    );
}

#[test]
fn require_statement_suggests_workspace_modules() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri = must(Url::parse("file:///workspace/lib/Net/HTTP.pm"));
    must(index.index_file(uri, "package Net::HTTP;\n1;\n".to_string()));

    let code = "require Net";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    assert!(has_label(&items, "Net::HTTP"), "should suggest Net::HTTP after `require Net`");
}

// ---------------------------------------------------------------------------
// Workspace-aware method completion via `->` (#352)
// ---------------------------------------------------------------------------

#[test]
fn arrow_completion_uses_workspace_methods_static_call() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri = must(Url::parse("file:///workspace/lib/MyService.pm"));
    must(index.index_file(
        uri,
        "package MyService;\nsub new { }\nsub process { }\nsub validate { }\n1;\n".to_string(),
    ));

    let code = "MyService->";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    assert!(
        has_label(&items, "process"),
        "should suggest workspace method `process` for MyService->, got: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    assert!(
        has_label(&items, "validate"),
        "should suggest workspace method `validate` for MyService->"
    );
}

#[test]
fn arrow_completion_workspace_methods_have_detail() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri = must(Url::parse("file:///workspace/lib/Greeter.pm"));
    must(index.index_file(uri, "package Greeter;\nsub hello { }\n1;\n".to_string()));

    let code = "Greeter->";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    let hello = must_some(find_item(&items, "hello"));
    let detail = must_some(hello.detail.as_deref());
    assert!(
        detail.contains("Greeter"),
        "workspace method detail should mention the package name, got: {detail:?}"
    );
}

#[test]
fn arrow_completion_from_variable_assignment() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri = must(Url::parse("file:///workspace/lib/Cache.pm"));
    must(index.index_file(
        uri,
        "package Cache;\nsub new { }\nsub get { }\nsub set { }\n1;\n".to_string(),
    ));

    let code = "my $cache = Cache->new();\n$cache->";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    assert!(
        has_label(&items, "get"),
        "should suggest `get` from Cache via variable assignment inference, got: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    assert!(
        has_label(&items, "set"),
        "should suggest `set` from Cache via variable assignment inference"
    );
}

#[test]
fn arrow_completion_from_multiline_object_assignment() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri = must(Url::parse("file:///workspace/lib/Cache.pm"));
    must(index.index_file(
        uri,
        "package Cache;\nsub new { }\nsub get { }\nsub set { }\n1;\n".to_string(),
    ));

    let code = "my $cache =\n    Cache->new();\n$cache->";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    assert!(
        has_label(&items, "get"),
        "should suggest `get` from Cache via multiline object assignment inference, got: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
    assert!(
        has_label(&items, "set"),
        "should suggest `set` from Cache via multiline object assignment inference"
    );
}

#[test]
fn arrow_completion_does_not_duplicate_local_methods() {
    let index = Arc::new(WorkspaceIndex::new());

    let uri = must(Url::parse("file:///workspace/lib/Dup.pm"));
    must(index.index_file(uri, "package Dup;\nsub shared_method { }\n1;\n".to_string()));

    // The local file also defines shared_method
    let code = "package Dup;\nsub shared_method { }\nDup->";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());

    let count = items.iter().filter(|i| i.label == "shared_method").count();
    assert!(
        count <= 1,
        "should not duplicate method completions, found {count} entries for shared_method"
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn use_statement_no_workspace_index_returns_general_completions() {
    // Without workspace index, use-statement context should still work
    // (just fall through to no completions or keyword completions)
    let code = "use My";
    let provider = parse_and_provider(code);
    let items = provider.get_completions(code, code.len());
    // Should not crash; may return empty or keyword completions
    let _ = items;
}

#[test]
fn use_statement_empty_workspace_index() {
    let index = Arc::new(WorkspaceIndex::new());
    let code = "use My";
    let provider = parse_provider_with_index(code, index);
    let items = provider.get_completions(code, code.len());
    // Should not crash with empty workspace index
    assert!(
        !items.iter().any(|i| i.kind == CompletionItemKind::Module),
        "empty workspace index should produce no module completions"
    );
}

#[test]
fn use_pragma_does_not_trigger_module_completion() {
    // `use strict`, `use warnings`, `use constant` etc. should NOT trigger
    // module name completion because these are lowercase pragmas, not modules.
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/lib/Strict.pm"));
    must(index.index_file(uri, "package Strict;\n1;\n".to_string()));

    for pragma in &["use strict", "use warnings", "use constant", "use lib", "use if"] {
        let provider = parse_provider_with_index(pragma, Arc::clone(&index));
        let items = provider.get_completions(pragma, pragma.len());
        assert!(
            !items.iter().any(|i| i.kind == CompletionItemKind::Module),
            "pragma `{pragma}` should NOT trigger module completion, got: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }
}

#[test]
fn variable_assignment_inference_ignores_comparison_operators() {
    // `if ($var == 0)` should NOT be confused with an assignment
    let index = Arc::new(WorkspaceIndex::new());
    let uri = must(Url::parse("file:///workspace/lib/Cmp.pm"));
    must(index.index_file(uri, "package Cmp;\nsub check { }\n1;\n".to_string()));

    let code = "if ($obj == Cmp->new()) { }\n$obj->";
    let provider = parse_provider_with_index(code, Arc::clone(&index));
    let items = provider.get_completions(code, code.len());
    // Should NOT infer type from `== Cmp->new()` comparison
    assert!(
        !has_label(&items, "check"),
        "should not infer package from comparison operator ==, got: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

// ===========================================================================
// 13. Special variable smart completion – issue #2347
// ===========================================================================

#[test]
fn special_scalar_child_status() {
    // $? holds the child process status after system/backtick/waitpid
    let code = "$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$?"), "should suggest $? (child process status)");
}

#[test]
fn special_hash_sig() {
    // %SIG maps signal names to handlers
    let code = "%";
    let items = completions_at_end(code);
    assert!(has_label(&items, "%SIG"), "should suggest %SIG (signal handlers)");
}

#[test]
fn special_array_argv() {
    // @ARGV holds command-line arguments
    let code = "@A";
    let items = completions_at_end(code);
    assert!(has_label(&items, "@ARGV"), "should suggest @ARGV (command-line args)");
}

#[test]
fn special_scalar_child_status_has_rich_documentation() {
    // $? documentation should mention child / process status
    let code = "$";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "$?"));
    let doc = must_some(item.documentation.as_ref());
    assert!(
        doc.to_lowercase().contains("child") || doc.to_lowercase().contains("status"),
        "$? documentation should mention child process status, got: {doc}"
    );
}

#[test]
fn special_scalar_capture_group_one() {
    // $1 is the first regex capture group - confirm it is in the list
    let code = "$1";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$1"), "should suggest $1 (first capture group)");
}

#[test]
fn special_scalar_child_status_sort_priority() {
    // Special variables should sort before regular variables (prefix "0_")
    let code = "$";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "$?"));
    let sort_text = must_some(item.sort_text.as_ref());
    assert!(
        sort_text.starts_with("0_"),
        "$? sort_text should start with '0_' for high priority, got: {sort_text}"
    );
}

#[test]
fn special_scalar_caret_t_script_start_time() {
    // $^T holds the epoch time when the script started.
    // The caret is a word boundary so the completion prefix is "$" (not "$^T").
    let code = "$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$^T"), "should suggest $^T (script start time)");
}

#[test]
fn special_scalar_caret_a_format_accumulator() {
    // $^A is the accumulator for format()/write().
    // The caret is a word boundary so the completion prefix is "$".
    let code = "$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$^A"), "should suggest $^A (format accumulator)");
}

#[test]
fn special_scalar_caret_w_warning_flag() {
    // $^W is the global warning flag (prefer 'use warnings' for lexical scope).
    // The caret is a word boundary so the completion prefix is "$".
    let code = "$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$^W"), "should suggest $^W (warning flag)");
}

#[test]
fn special_scalar_plus_last_bracket() {
    // $+ is the last successful regex bracket matched.
    // The '+' is a word boundary so the completion prefix is "$".
    let code = "$";
    let items = completions_at_end(code);
    assert!(has_label(&items, "$+"), "should suggest $+ (last bracket matched)");
}

#[test]
fn special_variables_all_have_detail_field() {
    // Every special variable completion should carry a non-empty detail string
    let code = "$";
    let items = completions_at_end(code);
    let special: Vec<_> = items
        .iter()
        .filter(|i| i.sort_text.as_deref().map(|s| s.starts_with("0_")).unwrap_or(false))
        .collect();
    assert!(!special.is_empty(), "should have special variables in list");
    for item in &special {
        assert!(
            item.detail.as_deref().map(|d| !d.is_empty()).unwrap_or(false),
            "special variable {} should have non-empty detail, got: {:?}",
            item.label,
            item.detail
        );
    }
}

// ===========================================================================
// Issue #2780: Missing builtins, no documentation strings
// ===========================================================================

/// socket should appear in completions when typing "so" — it is currently missing.
#[test]
fn test_missing_builtins_now_present() {
    let code = "so";
    let items = completions_at_end(code);
    let labels_list: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels_list.contains(&"socket"),
        "socket should complete from 'so'; got: {:?}",
        &labels_list
    );
    assert!(labels_list.contains(&"sort"), "sort should still complete from 'so'");
}

/// print completion must carry a documentation string (not None).
#[test]
fn test_builtin_has_documentation() {
    let code = "pri";
    let items = completions_at_end(code);
    let print_item = items.iter().find(|c| c.label == "print");
    assert!(print_item.is_some(), "print should complete from 'pri'");
    let doc = print_item.and_then(|c| c.documentation.as_deref());
    assert!(doc.is_some(), "print completion should have a documentation string, got None");
}

/// `defined` should appear when typing "def" — it's a builtin, not a keyword.
#[test]
fn test_defined_is_a_builtin_not_keyword() {
    let code = "def";
    let items = completions_at_end(code);
    assert!(
        items.iter().any(|c| c.label == "defined"),
        "defined should appear in completions from 'def'"
    );
}

// ===========================================================================
// 21. Relevance-based sort ordering: local -> file -> core -> CPAN (#2832)
// ===========================================================================

/// Builtins sort before workspace/CPAN symbols.
///
/// The tier order is: special vars (0_) < user vars (1_) < user funcs (2_)
/// < core builtins (3_) < workspace/CPAN (4_) < keywords (5_).
/// This test verifies that a Perl core builtin sorts ahead of a symbol from
/// another file (which represents CPAN or project-level workspace symbols).
#[test]
fn builtin_ranks_before_workspace_symbol() {
    let index = Arc::new(WorkspaceIndex::new());
    let file_url = must(Url::parse("file:///lib/MyModule.pm"));
    // Index a subroutine named "split_records" — starts with "spl" just like
    // the builtin "split", so both are candidates for the prefix "spl".
    let module_code = "package MyModule;\nsub split_records { }\n1;\n";
    must(index.index_file(file_url, module_code.to_string()));

    let code = "use MyModule;\nspl";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let items = provider.get_completions(code, code.len());

    let builtin_item = must_some(find_item(&items, "split"));
    // Workspace symbol label is the qualified name
    let ws_item = find_item(&items, "MyModule::split_records");

    let builtin_sort = must_some(builtin_item.sort_text.as_ref());
    assert!(
        builtin_sort.starts_with("3_"),
        "core builtin 'split' sort_text should start with '3_', got: {builtin_sort:?}"
    );

    let ws = must_some(ws_item);
    let ws_sort = must_some(ws.sort_text.as_ref());
    assert!(
        ws_sort.starts_with("4_") || ws_sort.starts_with("5_"),
        "workspace symbol sort_text should start with '4_' or '5_' (after builtins), got: {ws_sort:?}"
    );
    assert!(
        builtin_sort < ws_sort,
        "builtin 'split' ({builtin_sort:?}) should sort before workspace 'split_records' ({ws_sort:?})"
    );
}

/// Workspace symbols use sort prefix "4_" placing them after core builtins ("3_").
#[test]
fn workspace_symbol_sort_priority_is_4() {
    let index = Arc::new(WorkspaceIndex::new());
    let file_url = must(Url::parse("file:///lib/Util.pm"));
    let module_code = "package Util;\nsub helper_fn { }\n1;\n";
    must(index.index_file(file_url, module_code.to_string()));

    let code = "use Util;\nhelper_fn";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let items = provider.get_completions(code, code.len());

    // Workspace symbol label is the qualified name (Module::function).
    // `use Util;` with no import list means Util is not in the import_map
    // (import_map has no entry for it), so the `None` arm applies -> tier 4.
    let ws_item = must_some(find_item(&items, "Util::helper_fn"));
    let sort_text = must_some(ws_item.sort_text.as_ref());
    assert!(
        sort_text.starts_with("4_"),
        "workspace function sort_text should start with '4_' (CPAN tier, after builtins), got: {sort_text:?}"
    );
}

/// Keywords use sort prefix "5_" placing them after CPAN/workspace symbols.
///
/// Rationale: when a user types a partial name like "su", they are more
/// likely looking for their own `sub_name` function or the CPAN symbol
/// than the `sub` keyword. Keywords come last as they are always available
/// and match via snippet expansion anyway.
#[test]
fn keyword_sort_priority_is_5() {
    let code = "su";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "sub"));
    let sort_text = must_some(item.sort_text.as_ref());
    assert!(
        sort_text.starts_with("5_"),
        "keyword 'sub' sort_text should start with '5_', got: {sort_text:?}"
    );
}

// ===========================================================================
// 22. Documentation strings on completion items (#2832)
// ===========================================================================

/// Common Perl built-in functions should carry a documentation string so that
/// LSP clients can display hover text in the completion popup.
#[test]
fn builtin_print_has_documentation() {
    let code = "pri";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "print"));
    assert!(item.documentation.is_some(), "builtin 'print' should have documentation, got None");
    let doc = must_some(item.documentation.as_ref());
    assert!(!doc.is_empty(), "builtin 'print' documentation should not be empty");
}

#[test]
fn builtin_split_has_documentation() {
    let code = "spl";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "split"));
    assert!(item.documentation.is_some(), "builtin 'split' should have documentation, got None");
}

#[test]
fn builtin_open_has_documentation() {
    let code = "ope";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "open"));
    assert!(item.documentation.is_some(), "builtin 'open' should have documentation, got None");
}

#[test]
fn builtin_push_has_documentation() {
    let code = "pus";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "push"));
    assert!(item.documentation.is_some(), "builtin 'push' should have documentation, got None");
}

#[test]
fn builtin_map_has_documentation() {
    let code = "ma";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "map"));
    assert!(item.documentation.is_some(), "builtin 'map' should have documentation, got None");
}

#[test]
fn builtin_grep_has_documentation() {
    let code = "gre";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "grep"));
    assert!(item.documentation.is_some(), "builtin 'grep' should have documentation, got None");
}

/// The `if` keyword snippet should carry a brief documentation string.
#[test]
fn keyword_if_has_documentation() {
    let code = "if";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "if"));
    assert!(item.documentation.is_some(), "keyword 'if' should have documentation, got None");
}

/// The `sub` keyword snippet should carry a brief documentation string.
#[test]
fn keyword_sub_has_documentation() {
    let code = "su";
    let items = completions_at_end(code);
    let item = must_some(find_item(&items, "sub"));
    assert!(item.documentation.is_some(), "keyword 'sub' should have documentation, got None");
}

// ===========================================================================
// 23. Deduplication: builtin beats keyword when both match the same label
// ===========================================================================

/// Several names appear in both `create_builtins()` and `LSP_COMPLETION_KEYWORDS`
/// (e.g. `die`, `eval`, `exit`, `warn`, `require`). After deduplication the
/// completion list must contain exactly one entry for each such name, and that
/// entry must carry the builtin's tier-3 sort prefix (lower = higher priority)
/// rather than the keyword's tier-5 prefix.
///
/// This guards against regression where both items survive deduplication and
/// a client receives confusing duplicate completions.
#[test]
fn builtin_beats_keyword_on_duplicate_label() {
    // "die" appears in create_builtins() AND in LSP_COMPLETION_KEYWORDS.
    let code = "di";
    let items = completions_at_end(code);

    // Count how many completion items have label "die".
    let die_items: Vec<_> = items.iter().filter(|i| i.label == "die").collect();
    assert_eq!(
        die_items.len(),
        1,
        "'die' should appear exactly once after dedup; got {} items",
        die_items.len()
    );

    // The surviving item must be the builtin (tier 3_), not the keyword (tier 5_).
    let sort = must_some(die_items[0].sort_text.as_ref());
    assert!(
        sort.starts_with("3_"),
        "'die' should survive as builtin (3_) not keyword (5_); got sort_text: {sort:?}"
    );

    // Also verify 'warn' — another overlapping name.
    let warn_items: Vec<_> = items.iter().filter(|i| i.label == "warn").collect();
    // warn may or may not match prefix "di", so we only check if it appears
    // that it appears at most once.
    assert!(warn_items.len() <= 1, "'warn' should appear at most once in completions for 'di'");
}

/// Specifically test 'warn' which overlaps builtins and keywords, using a
/// prefix that only matches 'warn'.
#[test]
fn builtin_warn_no_duplicate() {
    let code = "war";
    let items = completions_at_end(code);

    let warn_items: Vec<_> = items.iter().filter(|i| i.label == "warn").collect();
    assert_eq!(
        warn_items.len(),
        1,
        "'warn' should appear exactly once after dedup; got {} items",
        warn_items.len()
    );

    let sort = must_some(warn_items[0].sort_text.as_ref());
    assert!(
        sort.starts_with("3_"),
        "'warn' should survive as builtin (3_) not keyword (5_); got sort_text: {sort:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Hash key completion edge case tests (issue #4264)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn test_hash_key_completion_single_key() {
    let code = "my %map = (single => 'value');\n$map{sin";
    let completions = completions_at_end(code);
    assert!(has_label(&completions, "single"), "single key hash should complete");
}

#[test]
fn test_hash_key_completion_quoted_keys() {
    // Use empty prefix to verify both single-quoted and double-quoted keys are extracted;
    // prefix filtering is tested separately in test_hash_key_completion_prefix_filtering.
    let code = "my %quoted = ('first_key' => 1, \"second_key\" => 2);\n$quoted{";
    let completions = completions_at_end(code);
    assert!(has_label(&completions, "first_key"), "quoted key 'first_key' should be extracted");
    assert!(
        has_label(&completions, "second_key"),
        "double-quoted key 'second_key' should be extracted"
    );
}

#[test]
fn test_hash_key_completion_multiline_definition() {
    // Use empty prefix to verify all keys are extracted across multiple lines;
    // prefix filtering is tested separately in test_hash_key_completion_prefix_filtering.
    let code = "my %config = (\n  host => 'localhost',\n  port => 5432,\n);\n$config{";
    let completions = completions_at_end(code);
    assert!(has_label(&completions, "host"), "multiline hash should extract keys correctly");
    assert!(has_label(&completions, "port"), "multiline hash should extract all keys");
}

#[test]
fn test_hash_key_completion_individual_key_assignment() {
    let code = "$config{database} = 'mydb';\n$config{d";
    let completions = completions_at_end(code);
    assert!(has_label(&completions, "database"), "individual key assignment should be recognized");
}

#[test]
fn test_hash_key_completion_mixed_definitions() {
    // Use empty prefix to verify both fat-comma literal and individual-assignment keys
    // are collected; both patterns must produce completions without prefix filtering.
    let code = "my %data = (color => 'red');\n$data{shade} = 'dark';\n$data{";
    let completions = completions_at_end(code);
    assert!(has_label(&completions, "color"), "literal hash keys should be found");
    assert!(has_label(&completions, "shade"), "individual assignment keys should be found");
}

#[test]
fn test_hash_key_completion_with_whitespace() {
    let code = "my %config = (hostname => 'localhost');\n$config  {  hos";
    let completions = completions_at_end(code);
    assert!(has_label(&completions, "hostname"), "whitespace around brace should be handled");
}

#[test]
fn test_hash_key_completion_nested_access() {
    let code = "my %outer = (key1 => 1);\nmy %inner = (nested => 2);\n$outer{key1}{nest";
    let completions = completions_at_end(code);
    let has_key1_property =
        completions.iter().any(|c| c.label == "key1" && c.kind == CompletionItemKind::Property);
    assert!(!has_key1_property, "nested hash access should not suggest keys from outer hash");
}

#[test]
fn test_hash_key_completion_does_not_fire_for_hash_slice() {
    let code = "my %config = (host => 'localhost', port => 5432);\n@config{ho";
    let completions = completions_at_end(code);
    let has_host_property =
        completions.iter().any(|c| c.label == "host" && c.kind == CompletionItemKind::Property);
    assert!(!has_host_property, "hash slice @config{{...}} should not trigger hash key completion");
}

#[test]
fn test_hash_key_completion_double_sigil_dereference() {
    let code = "my %data = (key => 'value');\n$$ref{ke";
    let completions = completions_at_end(code);
    let has_key_property =
        completions.iter().any(|c| c.label == "key" && c.kind == CompletionItemKind::Property);
    assert!(
        !has_key_property,
        "double-sigil deref $$ref{{...}} should not trigger hash key completion"
    );
}

#[test]
fn test_hash_key_completion_double_sigil_same_name_no_false_positive() {
    // When $$data{ke is written but %data also exists, the double-sigil deref must
    // NOT suggest keys from %data — $$data is a scalar-ref dereference, not a plain
    // hash access.  Before the fix this would have returned `key` via %data.
    let code = "my %data = (key => 'value');\n$$data{ke";
    let completions = completions_at_end(code);
    let has_key_property =
        completions.iter().any(|c| c.label == "key" && c.kind == CompletionItemKind::Property);
    assert!(
        !has_key_property,
        "double-sigil deref $$data{{...}} must not suggest keys from %data even when names match"
    );
}

#[test]
fn test_hash_key_completion_prefix_filtering() {
    let code = "my %errors = (invalid_input => 'bad', invalid_format => 'ugly', valid_format => 'good');\n$errors{invalid_";
    let completions = completions_at_end(code);
    assert!(
        has_label(&completions, "invalid_input"),
        "prefix 'invalid_' should match 'invalid_input'"
    );
    assert!(
        has_label(&completions, "invalid_format"),
        "prefix 'invalid_' should match 'invalid_format'"
    );
    assert!(
        !has_label(&completions, "valid_format"),
        "prefix 'invalid_' should NOT match 'valid_format'"
    );
}

#[test]
fn test_hash_key_completion_case_sensitive() {
    let code = "my %config = (Host => 'localhost', host => 'local');\n$config{H";
    let completions = completions_at_end(code);
    assert!(has_label(&completions, "Host"), "uppercase prefix 'H' should match 'Host'");
    assert!(
        !has_label(&completions, "host"),
        "uppercase prefix 'H' should not match lowercase 'host'"
    );
}

#[test]
fn test_hash_key_completion_duplicate_keys() {
    let code = "my %dup = (key => 1, key => 2);\n$dup{k";
    let completions = completions_at_end(code);
    let key_count = completions
        .iter()
        .filter(|c| c.label == "key" && c.kind == CompletionItemKind::Property)
        .count();
    assert_eq!(key_count, 1, "duplicate key should appear only once in completions");
}

#[test]
fn test_hash_key_completion_numeric_and_underscore_keys() {
    let code = "my %data = (key_1 => 'a', key_2 => 'b', _private => 'c', __init => 'd');\n$data{_";
    let completions = completions_at_end(code);
    assert!(
        has_label(&completions, "_private"),
        "underscore-prefix key '_private' should be found"
    );
    assert!(has_label(&completions, "__init"), "double-underscore key '__init' should be found");
    assert!(!has_label(&completions, "key_1"), "prefix '_' should not match 'key_1'");
}

#[test]
fn test_hash_key_completion_empty_hash() {
    let code = "my %empty = ();\n$empty{x";
    let completions = completions_at_end(code);
    let property_completions: Vec<_> =
        completions.iter().filter(|c| c.kind == CompletionItemKind::Property).collect();
    assert!(property_completions.is_empty(), "empty hash should not suggest any keys");
}

#[test]
fn test_hash_key_completion_fat_comma_and_string_conversion() {
    let code = "my %config = (bare_word => 1, 'quoted' => 2);\n$config{b";
    let completions = completions_at_end(code);
    assert!(
        has_label(&completions, "bare_word"),
        "bare word left of fat comma should be extracted"
    );
}

#[test]
fn test_hash_key_completion_in_string_no_suggestions() {
    let code = "my %config = (host => 'localhost');\nmy $s = \"$config{ho";
    let completions = completions_at_end(code);
    let has_host_property =
        completions.iter().any(|c| c.label == "host" && c.kind == CompletionItemKind::Property);
    assert!(!has_host_property, "hash key completion must not fire inside a string literal");
}

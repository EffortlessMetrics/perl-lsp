//! BDD-style behavior specification tests for perl-lsp-completion
//!
//! These tests describe *what* the completion engine does from an editor
//! user's perspective.  Each test name reads as a behavior statement:
//! "completes <thing> after <trigger>" or "when <context>, suggests <items>."
//!
//! Coverage targets:
//! - Variable completion by sigil ($, @, %)
//! - Method completion after arrow (->)
//! - Module / package completion after ::
//! - Keyword and builtin function completion
//! - Context suppression (comments, strings, regex)
//! - Test::More completion in test files
//! - Moo/Moose `has` option-key completion
//! - Cancellation behaviour
//! - Edge cases: empty input, boundary positions, unicode

use perl_lsp_completion::{CompletionItem, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::sync::Arc;
use url::Url;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn completions(code: &str, position: usize) -> Vec<CompletionItem> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    provider.get_completions(code, position)
}

fn completions_at_end(code: &str) -> Vec<CompletionItem> {
    completions(code, code.len())
}

fn completions_with_path(code: &str, position: usize, filepath: &str) -> Vec<CompletionItem> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    provider.get_completions_with_path(code, position, Some(filepath))
}

fn completions_with_index(
    code: &str,
    position: usize,
    index: Arc<WorkspaceIndex>,
) -> Vec<CompletionItem> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    provider.get_completions(code, position)
}

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

// ===========================================================================
// 1. Variable completion after dollar sign
// ===========================================================================

#[test]
fn completes_scalar_variable_names_after_dollar() {
    let code = "my $username = 'alice';\nmy $user_id = 1;\n$us";
    let items = completions_at_end(code);

    assert!(has_label(&items, "$username"), "should suggest $username, got: {:?}", labels(&items));
    assert!(has_label(&items, "$user_id"), "should suggest $user_id, got: {:?}", labels(&items));
}

#[test]
fn completes_array_variable_names_after_at() {
    let code = "my @items = ();\nmy @inventory = ();\n@i";
    let items = completions_at_end(code);

    assert!(has_label(&items, "@items"), "should suggest @items, got: {:?}", labels(&items));
    assert!(
        has_label(&items, "@inventory"),
        "should suggest @inventory, got: {:?}",
        labels(&items)
    );
}

#[test]
fn completes_hash_variable_names_after_percent() {
    let code = "my %config = ();\nmy %cache = ();\n%c";
    let items = completions_at_end(code);

    assert!(has_label(&items, "%config"), "should suggest %config, got: {:?}", labels(&items));
    assert!(has_label(&items, "%cache"), "should suggest %cache, got: {:?}", labels(&items));
}

#[test]
fn does_not_cross_sigil_types_for_scalar_prefix() {
    let code = "my $alpha = 1;\nmy @alpha_list;\n$al";
    let items = completions_at_end(code);

    assert!(has_label(&items, "$alpha"), "should suggest scalar $alpha");
    assert!(
        !has_label(&items, "@alpha_list"),
        "should NOT suggest array @alpha_list for scalar prefix, got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// 2. Method completion after arrow
// ===========================================================================

#[test]
fn completes_method_names_after_arrow() {
    let code = r#"
package Calculator;
sub add { }
sub subtract { }
sub multiply { }

my $calc = Calculator->new();
$calc->
"#;
    // Position right after "->"
    let pos = must_some(code.rfind("->")) + 2;
    let items = completions(code, pos);

    // Should include user-defined methods
    assert!(
        items.iter().any(|c| c.label == "add" || c.label == "subtract" || c.label == "multiply"),
        "should suggest defined methods after ->, got: {:?}",
        labels(&items)
    );
}

#[test]
fn completes_dbi_methods_after_arrow_for_dbh_variable() {
    let code = "my $dbh = DBI->connect('dbi:SQLite:');\n$dbh->";
    let pos = code.len();
    let items = completions(code, pos);

    assert!(
        has_label(&items, "prepare"),
        "should suggest DBI prepare for $dbh, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "disconnect"),
        "should suggest DBI disconnect for $dbh, got: {:?}",
        labels(&items)
    );
}

#[test]
fn completes_dbi_statement_methods_for_sth_variable() {
    let code = "my $sth = $dbh->prepare('SELECT 1');\n$sth->";
    let pos = code.len();
    let items = completions(code, pos);

    assert!(
        has_label(&items, "execute"),
        "should suggest execute for $sth, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "fetchrow_hashref"),
        "should suggest fetchrow_hashref for $sth, got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// 3. Module / package completion after ::
// ===========================================================================

#[test]
fn completes_module_names_after_use() {
    // Create workspace with an indexed module
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = must(Url::parse("file:///workspace/MyApp/Config.pm"));
    let module_code = "package MyApp::Config;\nsub load { }\n1;";
    must(index.index_file(module_uri, module_code.to_string()));

    let code = "use MyApp::Config;\nMyApp::Config::";
    let items = completions_with_index(code, code.len(), index);

    assert!(
        has_label(&items, "load"),
        "should suggest exported function from indexed module, got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// 4. Keyword completion
// ===========================================================================

#[test]
fn completes_keywords_for_partial_input() {
    let code = "fo";
    let items = completions_at_end(code);

    assert!(
        items.iter().any(|c| c.label == "for" || c.label == "foreach"),
        "should suggest for/foreach keywords, got: {:?}",
        labels(&items)
    );
}

#[test]
fn completes_sub_keyword() {
    let code = "su";
    let items = completions_at_end(code);

    assert!(has_label(&items, "sub"), "should suggest 'sub' keyword, got: {:?}", labels(&items));
}

// ===========================================================================
// 5. Builtin function completion
// ===========================================================================

#[test]
fn completes_builtin_print_functions() {
    let code = "pri";
    let items = completions_at_end(code);

    assert!(has_label(&items, "print"), "should suggest print, got: {:?}", labels(&items));
    assert!(has_label(&items, "printf"), "should suggest printf, got: {:?}", labels(&items));
}

#[test]
fn completes_builtin_open_function() {
    let code = "ope";
    let items = completions_at_end(code);

    assert!(has_label(&items, "open"), "should suggest open, got: {:?}", labels(&items));
}

#[test]
fn completes_builtin_chomp_and_chop() {
    let code = "cho";
    let items = completions_at_end(code);

    assert!(has_label(&items, "chomp"), "should suggest chomp, got: {:?}", labels(&items));
    assert!(has_label(&items, "chop"), "should suggest chop, got: {:?}", labels(&items));
}

// ===========================================================================
// 6. Context suppression
// ===========================================================================

#[test]
fn does_not_complete_inside_comments() {
    let code = "# pri";
    let items = completions_at_end(code);

    assert!(
        items.is_empty(),
        "should suppress completions inside comments, got: {:?}",
        labels(&items)
    );
}

#[test]
fn does_not_complete_inside_end_of_line_comment() {
    let code = "my $x = 1; # pr";
    let items = completions_at_end(code);

    assert!(
        items.is_empty(),
        "should suppress completions in trailing comments, got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// 7. Test::More completion in test context
// ===========================================================================

#[test]
fn completes_test_more_functions_in_test_file() {
    let code = "use Test::More;\nis";
    let items = completions_with_path(code, code.len(), "/project/t/basic.t");

    assert!(
        items.iter().any(|c| c.label == "is" || c.label == "is_deeply"),
        "should suggest Test::More functions in .t files, got: {:?}",
        labels(&items)
    );
}

#[test]
fn completes_test_more_when_source_uses_test_module() {
    let code = "use Test::More tests => 3;\nok";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "ok"),
        "should suggest ok() when Test::More is imported, got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// 8. Moo/Moose `has` option-key completion
// ===========================================================================

#[test]
fn completes_moo_has_option_keys_inside_parentheses() {
    let code = "use Moo;\nhas 'name' => (is => 'ro', ";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "isa"),
        "should suggest 'isa' inside has() options, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "required"),
        "should suggest 'required' inside has() options, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "default"),
        "should suggest 'default' inside has() options, got: {:?}",
        labels(&items)
    );
}

#[test]
fn completes_moo_has_option_keys_with_prefix_filter() {
    let code = "use Moo;\nhas 'name' => (re";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "required"),
        "should suggest 'required' matching prefix 're', got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "reader"),
        "should suggest 'reader' matching prefix 're', got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// 9. Moo/Moose accessor completion
// ===========================================================================

#[test]
fn completes_moo_accessor_methods_after_self_arrow() {
    let code = r#"
package User;
use Moo;

has 'email' => (is => 'ro', isa => 'Str');

sub display {
    my $self = shift;
    $self->
}
"#;
    let pos = must_some(code.find("$self->")) + "$self->".len();
    let items = completions(code, pos);

    assert!(
        has_label(&items, "email"),
        "should suggest Moo accessor 'email' after $self->, got: {:?}",
        labels(&items)
    );
}

#[test]
fn moo_accessor_completion_includes_type_documentation() {
    let code = r#"
package Config;
use Moo;

has 'timeout' => (is => 'rw', isa => 'Int');

sub check {
    my $self = shift;
    $self->
}
"#;
    let pos = must_some(code.find("$self->")) + "$self->".len();
    let items = completions(code, pos);

    let timeout_item = must_some(items.iter().find(|c| c.label == "timeout"));
    let doc = must_some(timeout_item.documentation.as_deref());
    assert!(
        doc.contains("Int"),
        "accessor documentation should include the isa type, got: {:?}",
        doc
    );
}

// ===========================================================================
// 10. Cancellation
// ===========================================================================

#[test]
fn returns_empty_when_immediately_cancelled() {
    let code = "my $x = 1;\n$x";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let items = provider.get_completions_with_path_cancellable(
        code,
        code.len(),
        None,
        &|| true, // always cancelled
    );

    assert!(items.is_empty(), "should return empty when cancelled immediately");
}

// ===========================================================================
// 11. Edge cases
// ===========================================================================

#[test]
fn empty_source_returns_only_keywords_or_nothing() {
    let items = completions_at_end("");
    // Empty source may still yield keyword completions; verify each item
    // has a non-empty label (structural validity check).
    for item in &items {
        assert!(!item.label.is_empty(), "completion labels must be non-empty");
    }
}

#[test]
fn position_at_zero_returns_valid_completions_or_empty() {
    let code = "my $x = 1;";
    let items = completions(code, 0);
    // Position zero is before any typed text; completions may include keywords.
    for item in &items {
        assert!(!item.label.is_empty(), "completion labels must be non-empty at position 0");
    }
}

#[test]
fn position_beyond_source_returns_empty() {
    let code = "my $x = 1;";
    let items = completions(code, code.len() + 100);
    assert!(items.is_empty(), "out-of-bounds position should return empty");
}

#[test]
fn user_defined_subroutine_appears_in_completions() {
    let code = r#"
sub calculate_total { }
sub calculate_tax { }
calc
"#;
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "calculate_total"),
        "should suggest user-defined sub calculate_total, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "calculate_tax"),
        "should suggest user-defined sub calculate_tax, got: {:?}",
        labels(&items)
    );
}

#[test]
fn completions_are_deduplicated() {
    let code = "my $count = 1;\n$c";
    let items = completions_at_end(code);

    let count_items: Vec<_> = items.iter().filter(|i| i.label == "$count").collect();
    assert!(
        count_items.len() <= 1,
        "should not have duplicate completions, found {} entries for $count",
        count_items.len()
    );
}

#[test]
fn special_variables_appear_for_dollar_prefix() {
    let code = "my $x = 1;\n$";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "$_"),
        "should suggest special variable $_, got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// 12. Workspace-aware completion (issue #1653)
// ===========================================================================

fn workspace_with_modules() -> Arc<WorkspaceIndex> {
    let index = Arc::new(WorkspaceIndex::new());
    let uri1 = must(Url::parse("file:///workspace/MyApp/Config.pm"));
    must(
        index.index_file(
            uri1,
            "package MyApp::Config;
sub load_config { }
sub save_config { }
1;
"
            .to_string(),
        ),
    );
    let uri2 = must(Url::parse("file:///workspace/MyApp/Logger.pm"));
    must(
        index.index_file(
            uri2,
            "package MyApp::Logger;
sub log_info { }
sub log_error { }
sub log_debug { }
1;
"
            .to_string(),
        ),
    );
    let uri3 = must(Url::parse("file:///workspace/Utils.pm"));
    must(
        index.index_file(
            uri3,
            "package Utils;
use Exporter 'import';
our @EXPORT_OK = qw(trim_string format_date);
sub trim_string { }
sub format_date { }
1;
"
            .to_string(),
        ),
    );
    index
}

#[test]
fn workspace_use_suggests_module_names() {
    let index = workspace_with_modules();
    let code = "use MyApp";
    let items = completions_with_index(code, code.len(), index);
    assert!(
        has_label(&items, "MyApp::Config"),
        "should suggest MyApp::Config, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "MyApp::Logger"),
        "should suggest MyApp::Logger, got: {:?}",
        labels(&items)
    );
}

#[test]
fn workspace_use_suggests_at_least_three_modules() {
    let index = workspace_with_modules();
    let code = "use ";
    let items = completions_with_index(code, code.len(), index);
    let module_items: Vec<_> =
        items.iter().filter(|i| i.detail.as_deref() == Some("module")).collect();
    assert!(
        module_items.len() >= 3,
        "should suggest >=3 modules, found {}: {:?}",
        module_items.len(),
        module_items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

#[test]
fn workspace_subroutine_completion_in_general_context() {
    let index = workspace_with_modules();
    let code = "log_i";
    let items = completions_with_index(code, code.len(), index);
    assert!(
        items.iter().any(|i| i.label.contains("log_info")),
        "should suggest log_info, got: {:?}",
        labels(&items)
    );
}

#[test]
fn workspace_method_completion_with_inferred_type() {
    let index = workspace_with_modules();
    let code = "my $cfg = MyApp::Config->new;\n$cfg->";
    let items = completions_with_index(code, code.len(), index);
    assert!(
        items.iter().any(|i| i.label == "load_config"),
        "should suggest load_config, got: {:?}",
        labels(&items)
    );
    assert!(
        items.iter().any(|i| i.label == "save_config"),
        "should suggest save_config, got: {:?}",
        labels(&items)
    );
}

#[test]
fn workspace_package_member_completion_after_colons() {
    let index = workspace_with_modules();
    let code = "MyApp::Logger::";
    let items = completions_with_index(code, code.len(), index);
    assert!(has_label(&items, "log_info"), "should suggest log_info, got: {:?}", labels(&items));
    assert!(has_label(&items, "log_error"), "should suggest log_error, got: {:?}", labels(&items));
}

#[test]
fn workspace_completions_dedup_with_local() {
    let index = workspace_with_modules();
    let code = "sub log_info { }
log_i";
    let items = completions_with_index(code, code.len(), index);
    let log_info_items: Vec<_> = items.iter().filter(|i| i.label == "log_info").collect();
    assert!(
        log_info_items.len() <= 1,
        "should not duplicate local log_info, found {}",
        log_info_items.len()
    );
}

#[test]
fn workspace_completions_sort_after_local() {
    let index = workspace_with_modules();
    let code = "sub load_local { }
load";
    let items = completions_with_index(code, code.len(), index);
    let local_item = items.iter().find(|i| i.label == "load_local");
    let ws_item = items.iter().find(|i| i.label.contains("load_config"));
    if let (Some(local), Some(ws)) = (local_item, ws_item) {
        let local_sort = local.sort_text.as_deref().unwrap_or(&local.label);
        let ws_sort = ws.sort_text.as_deref().unwrap_or(&ws.label);
        assert!(
            local_sort < ws_sort,
            "local should sort before workspace: local={local_sort:?} ws={ws_sort:?}"
        );
    }
}

#[test]
fn workspace_use_qw_suggests_module_exports() {
    let index = workspace_with_modules();
    let code = "use Utils qw(tri";
    let items = completions_with_index(code, code.len(), index);
    assert!(
        has_label(&items, "trim_string"),
        "should suggest trim_string, got: {:?}",
        labels(&items)
    );
}

#[test]
fn workspace_empty_prefix_does_not_suggest_all_symbols() {
    let index = workspace_with_modules();
    let code = "";
    let items = completions_with_index(code, code.len(), index);
    let ws_items: Vec<_> =
        items.iter().filter(|i| i.detail.as_deref() == Some("workspace")).collect();
    assert!(
        ws_items.is_empty(),
        "should not suggest workspace symbols with empty prefix, found: {:?}",
        ws_items.iter().map(|i| &i.label).collect::<Vec<_>>()
    );
}

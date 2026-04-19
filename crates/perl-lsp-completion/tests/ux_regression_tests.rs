//! UX regression tests for common Perl editing completion scenarios.
//!
//! These tests lock the completion behavior for real-world editing patterns.
//! If completion quality degrades — wrong completions, missing completions, or
//! wrong item kinds — these tests fail and catch the regression before release.
//!
//! ## Scenarios
//!
//! | # | Trigger | Expected |
//! |---|---------|----------|
//! | a | `pri` | suggests `print` (Function), `printf` (Function) |
//! | b | `$f` after `my $foo = 1; my $bar = 2;` | suggests `$foo` (Variable) |
//! | c | `whi` | suggests `while` (Keyword) |
//! | d | `use Str` (uppercase) | suggests workspace modules starting with `Str` (Module) |
//! | d2 | `use str` (lowercase) | does NOT suggest module completions (pragma guard) |
//! | e | `$self->` after declared methods | offers declared methods (Function) |
//! | f | `# pri` | no completions (comment context) |
//! | g | `"pri` inside string | no keyword completions (string context) |

use perl_lsp_completion::{CompletionItem, CompletionItemKind, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::sync::Arc;
use url::Url;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn completions_at_end(code: &str) -> Vec<CompletionItem> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    provider.get_completions(code, code.len())
}

fn completions_at(code: &str, pos: usize) -> Vec<CompletionItem> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    provider.get_completions(code, pos)
}

fn completions_with_index(
    code: &str,
    pos: usize,
    index: Arc<WorkspaceIndex>,
) -> Vec<CompletionItem> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    provider.get_completions(code, pos)
}

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

// ===========================================================================
// Scenario (a): Builtin function completion — `pri` → `print`, `printf`
// ===========================================================================

#[test]
fn builtin_pri_suggests_print() {
    let code = "pri";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "print"),
        "typing 'pri' must suggest 'print'; got: {:?}",
        labels(&items)
    );
}

#[test]
fn builtin_pri_suggests_printf() {
    let code = "pri";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "printf"),
        "typing 'pri' must suggest 'printf'; got: {:?}",
        labels(&items)
    );
}

#[test]
fn builtin_print_has_function_kind() {
    let code = "pri";
    let items = completions_at_end(code);

    let print_item = must_some(items.iter().find(|i| i.label == "print"));
    assert_eq!(
        print_item.kind,
        CompletionItemKind::Function,
        "'print' completion must have Function kind; got: {:?}",
        print_item.kind
    );
}

#[test]
fn builtin_printf_has_function_kind() {
    let code = "pri";
    let items = completions_at_end(code);

    let printf_item = must_some(items.iter().find(|i| i.label == "printf"));
    assert_eq!(
        printf_item.kind,
        CompletionItemKind::Function,
        "'printf' completion must have Function kind; got: {:?}",
        printf_item.kind
    );
}

// ===========================================================================
// Scenario (b): Variable completion — `$f` → `$foo` after declarations
// ===========================================================================

#[test]
fn variable_dollar_f_suggests_foo() {
    let code = "my $foo = 1;\nmy $bar = 2;\n$f";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "$foo"),
        "typing '$f' after 'my $foo = 1' must suggest '$foo'; got: {:?}",
        labels(&items)
    );
}

#[test]
fn variable_dollar_f_does_not_suggest_bar() {
    let code = "my $foo = 1;\nmy $bar = 2;\n$f";
    let items = completions_at_end(code);

    assert!(
        !has_label(&items, "$bar"),
        "typing '$f' must not suggest '$bar' (wrong prefix); got: {:?}",
        labels(&items)
    );
}

#[test]
fn variable_foo_has_variable_kind() {
    let code = "my $foo = 1;\nmy $bar = 2;\n$f";
    let items = completions_at_end(code);

    let foo_item = must_some(items.iter().find(|i| i.label == "$foo"));
    assert_eq!(
        foo_item.kind,
        CompletionItemKind::Variable,
        "'$foo' completion must have Variable kind; got: {:?}",
        foo_item.kind
    );
}

// ===========================================================================
// Scenario (c): Keyword completion — `whi` → `while`
// ===========================================================================

#[test]
fn keyword_whi_suggests_while() {
    let code = "whi";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "while"),
        "typing 'whi' must suggest 'while' keyword; got: {:?}",
        labels(&items)
    );
}

#[test]
fn keyword_while_has_snippet_kind() {
    // `while` has a snippet expansion (body with cursor placeholder), so the
    // engine classifies it as Snippet rather than plain Keyword.  This is
    // intentional: editors use Snippet kind to enable placeholder tabstops.
    let code = "whi";
    let items = completions_at_end(code);

    let while_item = must_some(items.iter().find(|i| i.label == "while"));
    assert_eq!(
        while_item.kind,
        CompletionItemKind::Snippet,
        "'while' completion must have Snippet kind (it has a snippet body); got: {:?}",
        while_item.kind
    );
}

#[test]
fn keyword_while_has_snippet_with_content() {
    let code = "whi";
    let items = completions_at_end(code);

    let while_item = must_some(items.iter().find(|i| i.label == "while"));
    let insert = must_some(while_item.insert_text.as_ref());
    assert!(
        insert.contains("while"),
        "'while' snippet must contain the keyword 'while'; got insert_text: {insert:?}"
    );
}

// ===========================================================================
// Scenario (d): Use statement completion
//
// Perl module names start with uppercase by convention.  Lowercase tokens
// after `use` (e.g. `use strict`, `use warnings`) are treated as pragmas and
// do NOT trigger workspace module-name completion.  This is intentional.
//
// For uppercase-first prefixes (e.g. `use Str`) workspace modules are offered.
// ===========================================================================

#[test]
fn use_uppercase_prefix_suggests_workspace_module() -> Result<(), Box<dyn std::error::Error>> {
    // Populate a workspace module whose name starts with `Str` so the
    // non-vacuous assertion confirms module-name completions are triggered.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///lib/Strings/Utils.pm")?,
        "package Strings::Utils;\nsub trim { }\n1;\n".to_string(),
    )?;
    index.index_file(
        Url::parse("file:///lib/Config/Loader.pm")?,
        "package Config::Loader;\nsub load { }\n1;\n".to_string(),
    )?;

    let code = "use Str";
    let items = completions_with_index(code, code.len(), index);

    assert!(
        items
            .iter()
            .any(|i| i.label.starts_with("Str") && i.kind == CompletionItemKind::Module),
        "typing 'use Str' (uppercase) must suggest matching workspace modules; got: {:?}",
        labels(&items)
    );
    assert!(
        !has_label(&items, "Config::Loader"),
        "typing 'use Str' must not suggest unrelated workspace modules; got: {:?}",
        labels(&items)
    );
    Ok(())
}

#[test]
fn use_lowercase_pragma_does_not_suggest_module_completions()
-> Result<(), Box<dyn std::error::Error>> {
    // `use str` has a lowercase-first token.  The engine guards against
    // suggesting module-kind completions for pragmas (like `use strict`,
    // `use warnings`).  This must hold regardless of what the workspace index
    // contains.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///lib/Strict.pm")?,
        "package Strict;\n1;\n".to_string(),
    )?;

    let code = "use str";
    let items = completions_with_index(code, code.len(), index);

    assert!(
        !items.iter().any(|i| i.kind == CompletionItemKind::Module),
        "typing 'use str' (lowercase) must not produce Module-kind completions; got: {:?}",
        items
            .iter()
            .map(|i| (&i.label, &i.kind))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ===========================================================================
// Scenario (e): Method completion after `$self->` with declared methods
// ===========================================================================

#[test]
fn self_arrow_suggests_declared_methods() {
    let code = "package Greeter;\nsub greet   { }\nsub farewell { }\nsub run {\n    my $self = shift;\n    $self->\n}";
    // Position right after the final `->`
    let pos = must_some(code.rfind("->")) + "->".len();
    let items = completions_at(code, pos);

    assert!(
        has_label(&items, "greet") || has_label(&items, "farewell"),
        "typing '$self->' must suggest declared methods in the package; got: {:?}",
        labels(&items)
    );
}

#[test]
fn self_arrow_method_has_function_kind() {
    let code = "package Notifier;\nsub send_email { }\nsub notify {\n    my $self = shift;\n    $self->\n}";
    let pos = must_some(code.rfind("->")) + "->".len();
    let items = completions_at(code, pos);

    // At least one method item should be of Function kind.
    assert!(
        items.iter().any(|i| i.kind == CompletionItemKind::Function),
        "method completions after '$self->' must include Function-kind items; got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// Scenario (f): No completions inside comments — `# pri`
// ===========================================================================

#[test]
fn no_completions_inside_line_comment() {
    let code = "# pri";
    let items = completions_at_end(code);

    assert!(
        items.is_empty(),
        "no completions inside a line comment; got: {:?}",
        labels(&items)
    );
}

#[test]
fn no_completions_inside_trailing_comment() {
    let code = "my $x = 1; # pri";
    let items = completions_at_end(code);

    assert!(
        items.is_empty(),
        "no completions inside a trailing comment; got: {:?}",
        labels(&items)
    );
}

// ===========================================================================
// Scenario (g): String context — `"pri` does not suggest keywords or builtins
// ===========================================================================

#[test]
fn no_keyword_completions_inside_double_quoted_string() {
    // Inside an open double-quoted string the engine suppresses keyword
    // completions.  Variable interpolation completions ($var inside "") are
    // allowed but keywords such as 'while' must not appear.
    let code = "my $x = \"pri";
    let items = completions_at_end(code);

    assert!(
        !items.iter().any(|i| i.kind == CompletionItemKind::Keyword),
        "no Keyword-kind completions inside a double-quoted string; got: {:?}",
        items
            .iter()
            .map(|i| (&i.label, &i.kind))
            .collect::<Vec<_>>()
    );
}

#[test]
fn no_builtin_completions_inside_double_quoted_string() {
    // Builtin functions (print, printf, …) must also be suppressed inside strings.
    let code = "my $x = \"pri";
    let items = completions_at_end(code);

    assert!(
        !has_label(&items, "print"),
        "'print' must not be suggested inside a string literal; got: {:?}",
        labels(&items)
    );
    assert!(
        !has_label(&items, "printf"),
        "'printf' must not be suggested inside a string literal; got: {:?}",
        labels(&items)
    );
}

#[test]
fn no_completions_inside_single_quoted_string() {
    // Single-quoted strings do not interpolate, so the engine should suppress
    // the usual completion categories entirely.
    let code = "my $x = 'pri";
    let items = completions_at_end(code);

    assert!(
        items.is_empty(),
        "no completions should appear inside a single-quoted string; got: {:?}",
        labels(&items)
    );
}

#[test]
fn completions_resume_after_closed_string() {
    // After the closing quote, normal completions (e.g. builtins) must return.
    let code = "my $x = \"hello\";\npri";
    let items = completions_at_end(code);

    assert!(
        has_label(&items, "print"),
        "'print' must be suggested on the line after a closed string; got: {:?}",
        labels(&items)
    );
}

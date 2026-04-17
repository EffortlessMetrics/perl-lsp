//! Completion behavior tests for Rose::DB::Object ORM.
//!
//! These tests verify:
//! - AC2: Column accessor completions appear when typing `$obj->` on Rose::DB::Object subclass
//! - Rose::DB::Object column accessor documentation

use perl_lsp_completion::{CompletionItem, CompletionProvider};
use perl_parser_core::Parser;
use perl_tdd_support::must;

fn completions(code: &str, position: usize) -> Vec<CompletionItem> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    provider.get_completions(code, position)
}

fn has_label(items: &[CompletionItem], label: &str) -> bool {
    items.iter().any(|i| i.label == label)
}

fn labels(items: &[CompletionItem]) -> Vec<String> {
    items.iter().map(|i| i.label.clone()).collect()
}

// =============================================================================
// AC2: Column Accessor Completion Tests
// =============================================================================

#[test]
fn rose_db_object_column_accessor_completion_appears() {
    // AC2: Given a Rose::DB::Object subclass with `columns => [qw(id name email)]`
    // When the user types `$user->` and triggers completion
    // Then completion items include: `id()`, `name()`, `email()`
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    table => 'users',
    columns => [qw(id name email)],
    primary_key_columns => ['id'],
);

sub new { shift->SUPER::new(@_) }
"#;

    // Find the position after "$user->"
    let pos = code.len();
    let items = completions(code, pos);

    // Check that column accessor completions appear
    assert!(
        has_label(&items, "id"),
        "expected 'id' column accessor in completion, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "name"),
        "expected 'name' column accessor in completion, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "email"),
        "expected 'email' column accessor in completion, got: {:?}",
        labels(&items)
    );
}

#[test]
fn rose_db_object_column_completion_documented() {
    // AC2: Each completion item should show documentation "Column accessor (Rose::DB::Object)"
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id name)],
);
"#;

    let pos = code.len();
    let items = completions(code, pos);

    // Find the 'id' completion item
    let id_item = items.iter().find(|i| i.label == "id");
    assert!(id_item.is_some(), "expected 'id' completion item, got: {:?}", labels(&items));

    let id_item = id_item.unwrap();
    let doc = id_item.documentation.as_deref().unwrap_or("");
    assert!(
        doc.contains("Rose::DB::Object"),
        "expected 'id' documentation to mention Rose::DB::Object, got: {doc}"
    );
}

#[test]
fn rose_db_object_completion_via_use_base() {
    // Same test but using `use base` instead of `use parent`
    let code = r#"
package MyApp::Article;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id title body)],
);
"#;

    let pos = code.len();
    let items = completions(code, pos);

    assert!(
        has_label(&items, "id"),
        "expected 'id' column accessor in completion, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "title"),
        "expected 'title' column accessor in completion, got: {:?}",
        labels(&items)
    );
    assert!(
        has_label(&items, "body"),
        "expected 'body' column accessor in completion, got: {:?}",
        labels(&items)
    );
}

#[test]
fn rose_db_object_no_columns_no_accessor_completion() {
    // Without columns => [...], no accessor completions should appear
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

sub new { }
"#;

    let pos = code.len();
    let items = completions(code, pos);

    // Should NOT have synthesized accessors without meta->setup
    assert!(
        !has_label(&items, "id"),
        "did not expect 'id' accessor without meta->setup columns, got: {:?}",
        labels(&items)
    );
}

#[test]
fn rose_db_object_mixed_with_inherited_methods() {
    // Rose::DB::Object column accessors should appear alongside inherited methods
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id name)],
);

sub custom_method { }
"#;

    let pos = code.len();
    let items = completions(code, pos);

    // Should have both column accessors and user-defined methods
    assert!(has_label(&items, "id"), "expected 'id' column accessor, got: {:?}", labels(&items));
    assert!(
        has_label(&items, "custom_method"),
        "expected 'custom_method' user method, got: {:?}",
        labels(&items)
    );
}

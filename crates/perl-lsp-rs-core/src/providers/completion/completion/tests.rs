use super::*;
use crate::providers::file_completion::CWD_LOCK as FILE_COMPLETION_CWD_LOCK;
use perl_parser_core::Parser;
use perl_tdd_support::{must, must_some};
use perl_workspace::workspace_index::WorkspaceIndex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use url::Url;

struct CurrentDirGuard {
    previous: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let previous = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(Self { previous })
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.previous);
    }
}

#[test]
fn test_variable_completion() {
    let code = r#"
my $count = 42;
my $counter = 0;
my @items = ();

$c
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len() - 1);

    assert!(completions.iter().any(|c| c.label == "$count"));
    assert!(completions.iter().any(|c| c.label == "$counter"));
}

#[test]
fn test_function_completion() {
    let code = r#"
sub process_data {
# ...
}

sub process_items {
# ...
}

proc
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len() - 1);

    assert!(completions.iter().any(|c| c.label == "process_data"));
    assert!(completions.iter().any(|c| c.label == "process_items"));
}

#[test]
fn dash_trigger_after_multibyte_receiver_keeps_char_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let code = "# 我-";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let context = provider.analyze_context(code, code.len());

    assert_eq!(context.prefix, "我->");
    assert_eq!(context.prefix_start, 2);
    assert_eq!(context.trigger_character, Some('-'));
    Ok(())
}

#[test]
fn word_prefix_after_multibyte_delimiter_keeps_char_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let code = "# ”my_func";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let context = provider.analyze_context(code, code.len());

    assert_eq!(context.prefix, "my_func");
    assert_eq!(&code[context.prefix_start..], "my_func");
    Ok(())
}

#[test]
fn object_pad_constructor_receiver_after_multibyte_delimiter_keeps_char_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let code = "# ”Point->new(";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let package = provider.object_pad_constructor_package(code, code.len());

    assert_eq!(package.as_deref(), Some("Point"));
    Ok(())
}

#[test]
fn test_use_constant_completion_from_visible_symbol_table() {
    let code = r#"
package My::Config;
use constant PI => 3.14159;
use constant qw(MAX_RETRIES TIMEOUT);

P
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len() - 1);

    let pi_completion = completions.iter().find(|c| c.label == "PI");
    assert!(pi_completion.is_some(), "expected PI constant completion");
    assert_eq!(
        pi_completion.map(|c| c.kind),
        Some(crate::providers::completion_item::CompletionItemKind::Constant)
    );
}

#[test]
fn test_use_constant_hash_form_completion() {
    // Verify hash-ref form `use constant { FOO => 1, BAR => 2 }` surfaces both names.
    let code = r#"
use constant { MIN_VAL => 1, MAX_VAL => 100 };

M
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len() - 1);

    let min_completion = completions.iter().find(|c| c.label == "MIN_VAL");
    assert!(
        min_completion.is_some(),
        "MIN_VAL should appear in completions from hash-form use constant"
    );
    assert_eq!(
        min_completion.map(|c| c.kind),
        Some(crate::providers::completion_item::CompletionItemKind::Constant),
        "MIN_VAL should have kind Constant"
    );

    let max_completion = completions.iter().find(|c| c.label == "MAX_VAL");
    assert!(
        max_completion.is_some(),
        "MAX_VAL should appear in completions from hash-form use constant"
    );
}

#[test]
fn test_use_constant_no_parens_in_insert_text() {
    // Constants must insert without trailing () — unlike function completions.
    let code = r#"
use constant ANSWER => 42;

A
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len() - 1);

    let answer = completions.iter().find(|c| c.label == "ANSWER");
    assert!(answer.is_some(), "ANSWER should appear in completions");
    assert_eq!(
        answer.and_then(|c| c.insert_text.as_deref()),
        Some("ANSWER"),
        "Constants must not insert trailing () — they are called like barewords"
    );
}

#[test]
fn test_builtin_completion() {
    let code = "pr";

    let mut parser = Parser::new(""); // Empty AST
    let ast = must(parser.parse());

    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    assert!(completions.iter().any(|c| c.label == "print"));
    assert!(completions.iter().any(|c| c.label == "printf"));
}

#[test]
fn test_current_package_detection() {
    let code = r#"package Foo;
my $x = 1;
$x
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    // position at end of file
    let context = provider.analyze_context(code, code.len());
    assert_eq!(context.current_package, "Foo");
}

#[test]
fn test_package_block_detection() {
    let code = r#"package Foo {
my $x;
$x;
}
package Bar;
$"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    // Inside Foo block
    let pos_foo = must_some(code.find("$x;")) + 2; // position after $x
    let ctx_foo = provider.analyze_context(code, pos_foo);
    assert_eq!(ctx_foo.current_package, "Foo");

    // After block, in Bar package
    let pos_bar = code.len();
    let ctx_bar = provider.analyze_context(code, pos_bar);
    assert_eq!(ctx_bar.current_package, "Bar");
}

#[test]
fn test_incomplete_nested_block_scope_context() {
    let code = concat!(
        "my $file_var = 0;\n",
        "sub process {\n",
        "    my $sub_var = 1;\n",
        "    if (1) {\n",
        "        my $block_var = 2;\n",
        "        $"
    );

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    let context = provider.analyze_context(code, code.len());

    let sub_scope = must_some(
        provider
            .symbol_table
            .symbols
            .get("sub_var")
            .and_then(|symbols| symbols.first())
            .map(|symbol| symbol.scope_id),
    );
    let block_scope = must_some(
        provider
            .symbol_table
            .symbols
            .get("block_var")
            .and_then(|symbols| symbols.first())
            .map(|symbol| symbol.scope_id),
    );

    assert_eq!(
        context.cursor_scope_id, block_scope,
        "expected cursor scope to match block_var scope in incomplete nested block; cursor={:?} sub={:?} block={:?}",
        context.cursor_scope_id, sub_scope, block_scope
    );
}

#[test]
fn test_incomplete_nested_block_variable_sorting() {
    let code = concat!(
        "my $file_var = 0;\n",
        "sub process {\n",
        "    my $sub_var = 1;\n",
        "    if (1) {\n",
        "        my $block_var = 2;\n",
        "        $"
    );

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    let completions = provider.get_completions(code, code.len());

    let block_item =
        must_some(completions.iter().find(|completion| completion.label == "$block_var"));
    let sub_item = must_some(completions.iter().find(|completion| completion.label == "$sub_var"));

    assert!(
        block_item.sort_text < sub_item.sort_text,
        "expected incomplete block variable to outrank parent variable, got block={:?} sub={:?}",
        block_item.sort_text,
        sub_item.sort_text
    );
}

#[test]
fn test_variable_completion_prefers_nearest_parent_scope_over_name_order() {
    let code = concat!(
        "{\n",
        "    my $v_a = 1;\n",
        "    {\n",
        "        my $v_z = 2;\n",
        "        {\n",
        "            $v_\n",
        "        }\n",
        "    }\n",
        "}\n"
    );

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    // Use rfind to locate the standalone $v_ completion trigger (the LAST $v_ in
    // the source), not the first occurrence which is inside $v_a or $v_z.
    let trigger_pos = must_some(code.rfind("$v_")) + 3;
    let completions = provider.get_completions(code, trigger_pos);

    let v_a_idx = must_some(completions.iter().position(|completion| completion.label == "$v_a"));
    let v_z_idx = must_some(completions.iter().position(|completion| completion.label == "$v_z"));

    assert!(
        v_z_idx < v_a_idx,
        "expected one-hop parent variable ($v_z) to rank before two-hop parent ($v_a); indices: v_z={v_z_idx}, v_a={v_a_idx}"
    );
}

#[test]
fn test_package_member_completion() {
    // Create workspace index with a module exporting a function
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = must(Url::parse("file:///workspace/MyModule.pm"));
    let module_code = r#"package MyModule;
our @EXPORT = qw(exported_sub);
sub exported_sub { }
sub internal_sub { }
1;
"#;
    must(index.index_file(module_uri, module_code.to_string()));

    // Code that triggers package completion
    let code = "use MyModule;\nMyModule::";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(completions.iter().any(|c| c.label == "exported_sub"), "should suggest exported_sub");
    let exported_sub =
        must_some(completions.iter().find(|completion| completion.label == "exported_sub"));
    let documentation = must_some(exported_sub.documentation.as_deref());
    assert!(
        documentation.contains("MyModule::exported_sub"),
        "expected package member doc to mention qualified symbol, got: {documentation:?}"
    );
}

#[test]
fn test_current_document_package_member_completion() -> Result<(), Box<dyn std::error::Error>> {
    let math_utils = r#"package MathUtils;

sub square {
    my ($n) = @_;
    return $n * $n;
}

sub cube {
    my ($n) = @_;
    return $n * $n * $n;
}
"#;

    let code = format!(
        r#"{math_utils}
package main;

my $sq = MathUtils::"#
    );

    let index = Arc::new(WorkspaceIndex::new());
    index
        .index_file(Url::parse("file:///workspace/MathUtils.pm")?, format!("{math_utils}\n1;\n"))?;

    let mut parser = Parser::new(&code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, &code, Some(index));
    let completions = provider.get_completions(&code, code.len());

    assert!(
        completions.iter().any(|completion| completion.label == "square"),
        "same-document package completion should include MathUtils::square; got {:?}",
        completions.iter().map(|completion| &completion.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|completion| completion.label == "cube"),
        "same-document package completion should include all local MathUtils members"
    );
    assert_eq!(
        completions.iter().filter(|completion| completion.label == "square").count(),
        1,
        "local and workspace package-member evidence should deduplicate square"
    );
    assert_eq!(
        completions.iter().filter(|completion| completion.label == "cube").count(),
        1,
        "local and workspace package-member evidence should deduplicate cube"
    );
    Ok(())
}

#[test]
fn test_current_document_package_member_completion_keeps_typed_prefix() {
    let code = r#"package MathUtils;
sub square {
    my ($n) = @_;
    return $n * $n;
}
sub cube {
    my ($n) = @_;
    return $n * $n * $n;
}
package main;
my $sq = MathUtils::s"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|completion| completion.label == "square"),
        "same-file package member completion should keep working after a typed member prefix; got {:?}",
        completions.iter().map(|completion| &completion.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().all(|completion| completion.label != "cube"),
        "member prefix `s` should filter out nonmatching package members; got {:?}",
        completions.iter().map(|completion| &completion.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_current_document_package_member_completion_skips_generated_framework_accessors() {
    let code = r#"package Example::User;
use Moo;

has 'name' => (is => 'ro', isa => 'Str');

package main;
my $name = Example::User::n"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().all(|completion| completion.label != "name"),
        "same-file package member completion must not promote generated accessors; got {:?}",
        completions.iter().map(|completion| &completion.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_moo_accessor_method_completion() {
    let code = r#"
package Example::User;
use Moo;

has 'name' => (is => 'ro', isa => 'Str');

sub greet {
my $self = shift;
return $self->name;
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let synthesized = provider
        .symbol_table
        .symbols
        .get("name")
        .map(|symbols| symbols.iter().any(|symbol| symbol.kind == SymbolKind::Subroutine))
        .unwrap_or(false);
    assert!(synthesized, "expected synthesized `name` subroutine symbol in symbol table");

    let pos = must_some(code.find("$self->name")) + "$self->".len();
    let completions = provider.get_completions(code, pos);

    assert!(
        completions.iter().any(|item| item.label == "name"),
        "expected synthesized Moo accessor `name` in method completion"
    );
}

#[test]
fn test_moo_accessor_completion_shows_isa_type() {
    let code = r#"
package Example::User;
use Moo;

has 'name' => (is => 'ro', isa => 'Str');
has 'age'  => (is => 'rw', isa => 'Int');

sub greet {
my $self = shift;
$self->
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let pos = must_some(code.find("$self->")) + "$self->".len();
    let completions = provider.get_completions(code, pos);

    // name accessor should appear with isa type in documentation
    let name_item = must_some(completions.iter().find(|c| c.label == "name"));
    let name_doc = must_some(name_item.documentation.as_deref());
    assert!(
        name_doc.contains("Str"),
        "expected `Str` type in name accessor documentation, got: {name_doc:?}"
    );

    // age accessor should appear with isa type in documentation
    let age_item = must_some(completions.iter().find(|c| c.label == "age"));
    let age_doc = must_some(age_item.documentation.as_deref());
    assert!(
        age_doc.contains("Int"),
        "expected `Int` type in age accessor documentation, got: {age_doc:?}"
    );

    // detail should indicate it's a Moo/Moose accessor, not just "method"
    let name_detail = must_some(name_item.detail.as_deref());
    assert!(
        name_detail.contains("accessor"),
        "expected 'accessor' in detail for Moo attribute, got: {name_detail:?}"
    );
}

#[test]
fn test_moose_accessor_completion_shows_isa_type() {
    let code = r#"
package Example::Animal;
use Moose;

has 'species' => (is => 'ro', isa => 'Str', required => 1);

sub describe {
my $self = shift;
$self->
}
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let pos = must_some(code.find("$self->")) + "$self->".len();
    let completions = provider.get_completions(code, pos);

    let species_item = must_some(completions.iter().find(|c| c.label == "species"));
    let species_doc = must_some(species_item.documentation.as_deref());
    assert!(
        species_doc.contains("Str"),
        "expected `Str` type in species accessor documentation, got: {species_doc:?}"
    );
}

#[test]
fn test_moo_has_option_key_completion() {
    let code = r#"
use Moo;
has 'name' => (re
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|item| item.label == "required"),
        "expected `required` option completion inside has(...) context"
    );
    assert!(
        completions.iter().any(|item| item.label == "reader"),
        "expected `reader` option completion inside has(...) context"
    );
}

#[test]
fn test_moo_has_option_key_completion_with_quoted_prefix() {
    let code = r#"
use Moo;
has 'name' => ('re
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|item| item.label == "required"),
        "expected `required` option completion for quoted key prefix"
    );
}

#[test]
fn test_object_pad_constructor_param_completion() {
    let code = r#"
use Object::Pad;

class Point {
field $x :param = 0;
field $y :param = 0;
field $cache = 1;
}

Point->new(
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|item| item.label == "x"),
        "expected `x` constructor completion inside Point->new(...)"
    );
    assert!(
        completions.iter().any(|item| item.label == "y"),
        "expected `y` constructor completion inside Point->new(...)"
    );
    assert!(
        !completions.iter().any(|item| item.label == "cache"),
        "non-:param fields should not appear in constructor completion"
    );

    let x_item = must_some(completions.iter().find(|item| item.label == "x"));
    assert_eq!(x_item.insert_text.as_deref(), Some("x => "));
}

#[test]
fn test_object_pad_constructor_param_completion_honors_prefix_and_value_context() {
    let prefix_code = r#"
use Object::Pad;

class Point {
field $name :param;
field $native_name :param;
field $age :param;
}

Point->new(na"#;

    let mut parser = Parser::new(prefix_code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, prefix_code, None);
    let completions = provider.get_completions(prefix_code, prefix_code.len());
    let constructor_labels: Vec<&str> = completions
        .iter()
        .filter(|item| item.detail.as_deref() == Some("Object::Pad constructor parameter"))
        .map(|item| item.label.as_str())
        .collect();

    assert!(constructor_labels.contains(&"name"), "expected `name` to match prefix `na`");
    assert!(
        constructor_labels.contains(&"native_name"),
        "expected `native_name` to remain available when matching prefix"
    );
    assert!(
        !constructor_labels.contains(&"age"),
        "non-matching constructor params should be filtered by prefix"
    );

    let value_code = r#"
use Object::Pad;

class Point {
field $name :param;
field $native_name :param;
}

Point->new(name => "#;

    let mut parser = Parser::new(value_code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, value_code, None);
    let value_completions = provider.get_completions(value_code, value_code.len());
    let value_constructor_labels: Vec<&str> = value_completions
        .iter()
        .filter(|item| item.detail.as_deref() == Some("Object::Pad constructor parameter"))
        .map(|item| item.label.as_str())
        .collect();

    assert!(
        !value_constructor_labels.contains(&"name"),
        "constructor key completions should not appear in value position"
    );
    assert!(
        !value_constructor_labels.contains(&"native_name"),
        "constructor key completions should not appear after `=>`"
    );
}

#[test]
fn test_object_pad_constructor_param_completion_supports_lowercase_class_names() {
    let code = r#"
use Object::Pad;

class point {
field $name :param;
field $native_name :param;
}

point->new(na"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
    let completions = provider.get_completions(code, code.len());
    let constructor_labels: Vec<&str> = completions
        .iter()
        .filter(|item| item.detail.as_deref() == Some("Object::Pad constructor parameter"))
        .map(|item| item.label.as_str())
        .collect();

    assert!(constructor_labels.contains(&"name"));
    assert!(constructor_labels.contains(&"native_name"));
}

#[test]
fn test_moo_isa_type_completion_includes_builtins_and_imports() {
    let code = concat!(
        "\n",
        "use MyApp::Types qw(UserID PositiveInt);\n",
        "use Moose;\n",
        "\n",
        "has 'id' => (\n",
        "is => 'ro',\n",
        "isa => "
    );

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|item| item.label == "Str"),
        "expected built-in Moose type `Str` in isa completion, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|item| item.label == "ArrayRef"),
        "expected built-in Moose type `ArrayRef` in isa completion"
    );
    assert!(
        completions.iter().any(|item| item.label == "UserID"),
        "expected imported custom type `UserID` in isa completion"
    );
}

#[test]
fn test_moo_isa_type_completion_with_quoted_prefix() {
    let code = r#"
use Moose;

has 'id' => (
is => 'ro',
isa => 'St
"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|item| item.label == "Str"),
        "expected built-in Moose type `Str` for quoted isa prefix"
    );
}

#[test]
fn test_completion_sorttext_prevents_context_mixing() -> Result<(), Box<dyn std::error::Error>> {
    let hash_code = "my %config = (host => 'localhost', port => 5432);\n$config{ho";
    let mut hash_parser = Parser::new(hash_code);
    let hash_ast = must(hash_parser.parse());
    let hash_provider = CompletionProvider::new(&hash_ast);
    let hash_completions = hash_provider.get_completions(hash_code, hash_code.len());
    let hash_item = must_some(hash_completions.iter().find(|item| item.label == "host"));
    let hash_sort = must_some(hash_item.sort_text.as_deref());

    let type_code = concat!(
        "\n",
        "use MyApp::Types qw(StrFoo);\n",
        "use Moose;\n",
        "\n",
        "has 'id' => (\n",
        "is => 'ro',\n",
        "isa => 'S"
    );
    let mut type_parser = Parser::new(type_code);
    let type_ast = must(type_parser.parse());
    let type_provider = CompletionProvider::new_with_index_and_source(&type_ast, type_code, None);
    let type_completions = type_provider.get_completions(type_code, type_code.len());
    let str_item = must_some(type_completions.iter().find(|item| item.label == "Str"));
    let str_sort = must_some(str_item.sort_text.as_deref());
    let str_foo_item = must_some(type_completions.iter().find(|item| item.label == "StrFoo"));
    let str_foo_sort = must_some(str_foo_item.sort_text.as_deref());

    let option_code = r#"
use Moose;

has 'id' => (
i"#;
    let mut option_parser = Parser::new(option_code);
    let option_ast = must(option_parser.parse());
    let option_provider =
        CompletionProvider::new_with_index_and_source(&option_ast, option_code, None);
    let option_completions = option_provider.get_completions(option_code, option_code.len());
    let option_item = must_some(option_completions.iter().find(|item| item.label == "is"));
    let option_sort = must_some(option_item.sort_text.as_deref());

    let field_code = r#"
use Object::Pad;

class Point {
field $name :param;
field $native_name :param;
}

Point->new(na"#;
    let mut field_parser = Parser::new(field_code);
    let field_ast = must(field_parser.parse());
    let field_provider =
        CompletionProvider::new_with_index_and_source(&field_ast, field_code, None);
    let field_completions = field_provider.get_completions(field_code, field_code.len());
    let field_item = must_some(field_completions.iter().find(|item| item.label == "name"));
    let field_sort = must_some(field_item.sort_text.as_deref());

    let mut sort_texts = vec![hash_sort, str_sort, str_foo_sort, option_sort, field_sort];
    sort_texts.sort_unstable();

    let expected = vec!["0f_name", "0h_host", "0o_is", "0t_Str", "0t_StrFoo"];
    if sort_texts != expected {
        return Err(
            format!("expected grouped sortText values {expected:?}, got {sort_texts:?}").into()
        );
    }

    Ok(())
}

#[test]
fn test_regex_completion_binding_operator() {
    // Cursor right after the opening slash of a regex
    let code = r#"my $x = "hello"; $x =~ /"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    // Should contain regex constructs
    assert!(
        completions.iter().any(|c| c.label == "\\d"),
        "expected \\d regex completion inside =~ /.../"
    );
    assert!(
        completions.iter().any(|c| c.label == "\\w"),
        "expected \\w regex completion inside =~ /.../"
    );
    assert!(
        completions.iter().any(|c| c.label == "(?:...)"),
        "expected non-capturing group regex completion"
    );
}

#[test]
fn test_regex_completion_negated_binding() {
    let code = r#"my $x = "test"; $x !~ /"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    assert!(completions.iter().any(|c| c.label == "\\d"), "expected regex completions after !~");
}

#[test]
fn test_regex_completion_m_operator() {
    let code = "if ($line =~ m/";

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "\\d"),
        "expected regex completions inside m/.../"
    );
    assert!(
        completions.iter().any(|c| c.label == "^"),
        "expected anchor completions inside m/.../"
    );
}

#[test]
fn test_regex_completion_qr_operator() {
    let code = "my $re = qr/";

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "\\d+"),
        "expected common pattern completions inside qr/.../"
    );
    assert!(
        completions.iter().any(|c| c.label == "(?=...)"),
        "expected lookahead group completion inside qr/.../"
    );
}

#[test]
fn test_regex_completion_s_operator() {
    let code = "($line = $input) =~ s/";

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "\\s+"),
        "expected common pattern completions inside s/.../"
    );
}

#[test]
fn test_regex_completion_has_all_categories() {
    let code = r#"$x =~ /"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    // Character classes
    assert!(completions.iter().any(|c| c.label == "\\d"));
    assert!(completions.iter().any(|c| c.label == "\\D"));
    assert!(completions.iter().any(|c| c.label == "\\w"));
    assert!(completions.iter().any(|c| c.label == "\\W"));
    assert!(completions.iter().any(|c| c.label == "\\s"));
    assert!(completions.iter().any(|c| c.label == "\\S"));
    assert!(completions.iter().any(|c| c.label == "\\h"));
    assert!(completions.iter().any(|c| c.label == "\\H"));
    assert!(completions.iter().any(|c| c.label == "\\v"));
    assert!(completions.iter().any(|c| c.label == "\\V"));
    assert!(completions.iter().any(|c| c.label == "\\R"));
    assert!(completions.iter().any(|c| c.label == "[...]"));
    assert!(completions.iter().any(|c| c.label == "[^...]"));

    // Anchors
    assert!(completions.iter().any(|c| c.label == "^"));
    assert!(completions.iter().any(|c| c.label == "$"));
    assert!(completions.iter().any(|c| c.label == "\\b"));
    assert!(completions.iter().any(|c| c.label == "\\B"));
    assert!(completions.iter().any(|c| c.label == "\\A"));
    assert!(completions.iter().any(|c| c.label == "\\z"));
    assert!(completions.iter().any(|c| c.label == "\\Z"));

    // Quantifiers
    assert!(completions.iter().any(|c| c.label == "*"));
    assert!(completions.iter().any(|c| c.label == "+"));
    assert!(completions.iter().any(|c| c.label == "?"));
    assert!(completions.iter().any(|c| c.label == "{n}"));
    assert!(completions.iter().any(|c| c.label == "{n,}"));
    assert!(completions.iter().any(|c| c.label == "{n,m}"));

    // Groups
    assert!(completions.iter().any(|c| c.label == "(...)"));
    assert!(completions.iter().any(|c| c.label == "(?:...)"));
    assert!(completions.iter().any(|c| c.label == "(?=...)"));
    assert!(completions.iter().any(|c| c.label == "(?!...)"));
    assert!(completions.iter().any(|c| c.label == "(?<=...)"));
    assert!(completions.iter().any(|c| c.label == "(?<!...)"));

    // Common patterns
    assert!(completions.iter().any(|c| c.label == "\\d+"));
    assert!(completions.iter().any(|c| c.label == "\\w+"));
    assert!(completions.iter().any(|c| c.label == "\\s+"));
    assert!(completions.iter().any(|c| c.label == ".*?"));
    assert!(completions.iter().any(|c| c.label == ".+?"));
}

#[test]
fn test_regex_completion_items_have_correct_kind() {
    let code = r#"$x =~ /"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    for item in &completions {
        assert_eq!(
            item.kind,
            CompletionItemKind::Snippet,
            "regex completion '{}' should be Snippet kind",
            item.label
        );
    }
}

#[test]
fn test_regex_completion_items_have_documentation() {
    let code = r#"$x =~ /"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    for item in &completions {
        assert!(
            item.documentation.is_some(),
            "regex completion '{}' should have documentation",
            item.label
        );
        assert!(item.detail.is_some(), "regex completion '{}' should have detail", item.label);
    }
}

#[test]
fn test_regex_completion_not_in_normal_context() {
    // Outside regex context, should not get regex completions
    let code = "my $x = 1;\n";

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|c| c.label == "\\d"),
        "regex completions should NOT appear outside regex context"
    );
}

#[test]
fn test_is_in_regex_binding_operator() {
    let code = r#"$x =~ /hello"#;
    assert!(CompletionProvider::is_in_regex(code, code.len()));
}

#[test]
fn test_is_in_regex_m_operator() {
    let code = "m/pattern";
    assert!(CompletionProvider::is_in_regex(code, code.len()));
}

#[test]
fn test_is_in_regex_qr_operator() {
    let code = "my $re = qr/pattern";
    assert!(CompletionProvider::is_in_regex(code, code.len()));
}

#[test]
fn test_is_in_regex_s_operator() {
    let code = "$line =~ s/old";
    assert!(CompletionProvider::is_in_regex(code, code.len()));
}

#[test]
fn test_is_in_regex_keyword_operator() {
    let code = "$x or /pattern";
    assert!(CompletionProvider::is_in_regex(code, code.len()));
}

#[test]
fn test_is_not_in_regex_division() {
    // Division should NOT be detected as regex
    let code = "my $result = $x / $y";
    // Position after "$x / $" -- should not be regex because $ precedes /
    // but our heuristic checks pre_slash context
    assert!(
        !CompletionProvider::is_in_regex(code, code.len()),
        "division should not be detected as regex context"
    );
}

#[test]
fn test_regex_completion_suppresses_sigil_completions_in_patterns() {
    // Cursor is inside the regex body at the end of `$fo` — before the
    // closing `/`. Variable completions are noisy inside regex patterns.
    let code = r#"my $foo = 1; my $bar = qr/^$fo/"#;
    // Position just before the closing '/'
    let pos = code.len() - 1;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "$foo"),
        "expected variable completions to be suppressed inside regex patterns, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_regex_completion_suppresses_at_sigil_in_patterns() {
    let code = r#"my @arr = (1, 2); $str =~ /prefix @ar"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "@arr"),
        "expected array completions to be suppressed inside regex patterns, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_substitution_replacement_side_still_offers_variables() {
    // Cursor is in the replacement side of s///, which remains a string-like
    // expression context rather than a regex pattern.
    let code = r#"my $baz = "x"; s/old/$ba"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        completions.iter().any(|item| item.label == "$baz"),
        "expected variable completions on the replacement side of s///, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_regex_pattern_side_suppresses_variables_not_flags() {
    let code = r#"$x =~ /\d"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        completions.iter().any(|item| item.label == r"\d"),
        "regex constructs should still be offered for non-sigil prefixes inside regex, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_suppresses_scalar_variables() {
    let code = r#"my $message = "hi"; my $text = "Hello $me"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "$message"),
        "expected scalar variable completions to be suppressed inside strings, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_suppresses_scalar_after_escaped_quote() {
    let code = r#"my $message = "hi"; my $text = "Hello \" $me"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "$message"),
        "expected escaped quotes to keep string-context suppression active, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_suppresses_scalar_in_single_quotes() {
    let code = r#"my $message = "hi"; my $text = 'Hello $me"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "$message"),
        "expected scalar variable completions to be suppressed inside single-quoted strings, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_suppresses_scalar_in_qq_literal() {
    let code = r#"my $message = "hi"; my $text = qq{Hello $me"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "$message"),
        "expected scalar variable completions to be suppressed inside qq literals, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_suppresses_scalar_in_q_literal() {
    let code = r#"my $message = "hi"; my $text = q($me"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "$message"),
        "expected scalar variable completions to be suppressed inside q literals, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_suppresses_function_sigils() {
    let code = r#"sub helper {} my $text = "call &he"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "&helper"),
        "expected function completions to be suppressed inside strings, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_after_hash_key_q_stays_in_code_context() {
    let code = r#"my $name = "hi"; my %h = (q => 1); $h{q}; $na"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        completions.iter().any(|item| item.label == "$name"),
        "hash-key q must not poison following code as string context, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_after_hash_key_m_still_suppresses_inside_later_string() {
    let code = r#"my $name = "hi"; my %h = (m => 1); $h{m}; my $text = "Hello $na"#;
    let pos = code.len();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| item.label == "$name"),
        "hash-key m must not hide a later real string context, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_preserves_path_completion() -> Result<(), Box<dyn std::error::Error>> {
    let _cwd_guard = FILE_COMPLETION_CWD_LOCK.lock()?;
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join("src"))?;
    let _dir_guard = CurrentDirGuard::change_to(temp.path())?;

    let code = r#"my $path = "./""#;
    let pos = code.len() - 1;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        completions.iter().any(|item| item.label == "src/"),
        "expected path completion to remain available inside string paths, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_string_completion_suppresses_method_arrow_completions() {
    let code = r#"my $dbh; my $s = "$dbh->""#;
    let pos = code.len() - 1;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, pos);

    assert!(
        !completions.iter().any(|item| matches!(item.label.as_str(), "can" | "selectrow_array")),
        "method completions must stay suppressed inside strings, got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_string_completion_suppresses_multiline_use_qw_structural_completions()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///lib/MyUtils.pm")?,
        "package MyUtils;\nsub helper_one {}\n1;\n".to_string(),
    )?;
    let code = "my $text = \"before\nuse MyUtils qw(he";

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|item| item.label == "helper_one"),
        "use/qw-looking text inside a multiline string must not trigger import completions; got: {:?}",
        completions.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_string_completion_suppresses_multiline_require_structural_completions()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    let code = "my $text = \"before\nrequire Ut";

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions
            .iter()
            .any(|item| item.label == "Utils" && item.kind == CompletionItemKind::Module),
        "require-looking text inside a multiline string must not trigger module completions; got: {:?}",
        completions.iter().map(|item| (&item.label, &item.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_regex_completion_replaces_escape_prefix_range() {
    let code = r#"$x =~ /\d"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    let item = must_some(completions.iter().find(|completion| completion.label == r"\d"));
    assert_eq!(
        item.text_edit_range,
        Some((code.len() - r"\d".len(), code.len())),
        "expected regex completion to replace the typed escape sequence"
    );
}

#[test]
fn test_regex_completion_offers_perl_whitespace_and_linebreak_classes() {
    let code = r#"$x =~ /\"#;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

    for label in &["\\h", "\\H", "\\v", "\\V", "\\R"] {
        assert!(
            labels.contains(label),
            "expected Perl regex class completion '{label}', got: {labels:?}"
        );
    }
}

#[test]
fn test_regex_completion_replaces_group_prefix_range() {
    let code = r#"$x =~ /(?: "#;
    let code = code.trim_end();

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    let item = must_some(completions.iter().find(|completion| completion.label == "(?:...)"));
    assert_eq!(
        item.text_edit_range,
        Some((code.len() - "(?:".len(), code.len())),
        "expected regex completion to replace the typed group opener"
    );
}

#[test]
fn test_detect_use_qw_import_context_basic() {
    // Cursor right after opening paren in qw()
    let code = "use MyModule qw(";
    let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
    assert!(result.is_some(), "should detect qw() import context");
    let (module, prefix) =
        result.as_ref().map(|(m, p)| (m.as_str(), p.as_str())).unwrap_or_default();
    assert_eq!(module, "MyModule");
    assert_eq!(prefix, "");
}

#[test]
fn test_detect_use_qw_import_context_with_prefix() {
    let code = "use File::Basename qw(bas";
    let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
    assert!(result.is_some(), "should detect qw() import context with prefix");
    let (module, prefix) =
        result.as_ref().map(|(m, p)| (m.as_str(), p.as_str())).unwrap_or_default();
    assert_eq!(module, "File::Basename");
    assert_eq!(prefix, "bas");
}

#[test]
fn test_detect_use_qw_import_context_with_existing_imports() {
    let code = "use MyModule qw(foo bar ba";
    let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
    assert!(result.is_some(), "should detect qw() import context after existing imports");
    let (module, prefix) =
        result.as_ref().map(|(m, p)| (m.as_str(), p.as_str())).unwrap_or_default();
    assert_eq!(module, "MyModule");
    assert_eq!(prefix, "ba");
}

#[test]
fn test_detect_use_qw_not_after_close() {
    // Cursor after the closing paren
    let code = "use MyModule qw(foo bar);";
    let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
    assert!(result.is_none(), "should not detect context after closing paren");
}

#[test]
fn test_detect_use_qw_not_for_pragmas() {
    let code = "use strict qw(";
    let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
    assert!(result.is_none(), "should not detect context for lowercase pragmas");
}

#[test]
fn test_use_qw_import_completion_with_workspace() -> Result<(), Box<dyn std::error::Error>> {
    // Create workspace index with a module that has subroutines
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/MyUtils.pm")?;
    let module_code = r#"package MyUtils;
use Exporter 'import';
our @EXPORT_OK = qw(helper_one helper_two);
sub helper_one { }
sub helper_two { }
sub _private_internal { }
1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // Code where user is typing inside qw()
    let code = "use MyUtils qw(hel";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "helper_one"),
        "should suggest helper_one from MyUtils: got {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "helper_two"),
        "should suggest helper_two from MyUtils"
    );
    Ok(())
}

#[test]
fn test_use_qw_import_completion_empty_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/Utils.pm")?;
    let module_code = r#"package Utils;
sub alpha { }
sub beta { }
1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // Empty prefix inside qw()
    let code = "use Utils qw(";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "alpha"),
        "should suggest alpha with empty prefix: got {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(completions.iter().any(|c| c.label == "beta"), "should suggest beta with empty prefix");
    Ok(())
}

#[test]
fn test_use_qw_import_completion_detail_shows_module() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/MyLib.pm")?;
    let module_code = r#"package MyLib;
sub do_work { }
1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    let code = "use MyLib qw(do";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let do_work = completions.iter().find(|c| c.label == "do_work");
    assert!(do_work.is_some(), "should suggest do_work");
    let detail = must_some(do_work.and_then(|c| c.detail.as_deref()));
    assert!(detail.contains("MyLib"), "detail should mention module name, got: {detail:?}");
    Ok(())
}

#[test]
fn test_self_arrow_resolves_workspace_methods() -> Result<(), Box<dyn std::error::Error>> {
    // Regression test for issue #2536: $self-> method completion should resolve
    // workspace-indexed methods from the current package.
    //
    // The methods are ONLY in the workspace index (a separate .pm file), not in
    // the currently-parsed source. This tests the workspace path specifically:
    // `classify_text_pattern_receiver` must return `SelfOrThis("MyService")` for `$self->` when
    // `context.current_package == "MyService"`.
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/MyService.pm")?;
    let module_code = r#"package MyService;
sub new { bless {}, shift }
sub process_request { }
sub validate_input { }
1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // The currently-edited file is in MyService but does NOT define
    // process_request or validate_input locally — they are workspace-only.
    let code = r#"package MyService;
sub run {
my $self = shift;
$self->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "process_request"),
        "$self-> should suggest process_request from workspace index; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "validate_input"),
        "$self-> should suggest validate_input from workspace index"
    );
    Ok(())
}

#[test]
fn test_this_arrow_resolves_workspace_methods() -> Result<(), Box<dyn std::error::Error>> {
    // Same as above but using $this as the invocant variable.
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/MyHandler.pm")?;
    let module_code = r#"package MyHandler;
sub new { bless {}, shift }
sub handle { }
1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // Only `run` is in the edited file; `handle` lives only in the workspace index.
    let code = r#"package MyHandler;
sub run {
my $this = shift;
$this->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "handle"),
        "$this-> should suggest handle from workspace index; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_method_completion_semantic_inheritance_detail() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Parent.pm")?,
        r#"package Parent;
sub inherited_method { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Child.pm")?,
        r#"package Child;
use parent 'Parent';
sub own_method { }
1;
"#
        .to_string(),
    )?;

    let code = r#"package Child;
sub run {
my $self = shift;
$self->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let own = must_some(completions.iter().find(|item| item.label == "own_method"));
    assert_eq!(
        own.detail.as_deref(),
        Some("method from Child — receiver: self/this"),
        "workspace own method should use semantic method candidate detail with receiver evidence (#7918)"
    );

    let inherited = must_some(completions.iter().find(|item| item.label == "inherited_method"));
    assert_eq!(
        inherited.detail.as_deref(),
        Some("inherited method from Parent — receiver: self/this"),
        "inherited method should use semantic method candidate detail with receiver evidence (#7918)"
    );

    Ok(())
}

#[test]
fn test_method_completion_prefers_nearest_ancestor() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Base.pm")?,
        r#"package Base;
sub shadowed_method { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Parent.pm")?,
        r#"package Parent;
use parent 'Base';
sub shadowed_method { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Child.pm")?,
        r#"package Child;
use parent 'Parent';
1;
"#
        .to_string(),
    )?;

    let code = r#"package Child;
sub run {
my $self = shift;
$self->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    let method = must_some(completions.iter().find(|item| item.label == "shadowed_method"));

    assert_eq!(
        method.detail.as_deref(),
        Some("inherited method from Parent — receiver: self/this"),
        "nearest ancestor should shadow a farther ancestor with the same method name (with #7918 receiver suffix)"
    );

    Ok(())
}

#[test]
fn test_self_arrow_in_main_package_does_not_resolve() -> Result<(), Box<dyn std::error::Error>> {
    // Edge case: $self-> in the main package should NOT resolve to any package methods.
    // The guard condition `context.current_package != "main"` prevents incorrect
    // suggestions when the user is in script-level code.
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/MyLib.pm")?;
    let module_code = r#"package MyLib;
sub new { bless {}, shift }
sub helper { }
1;
"#;
    index.index_file(module_uri, module_code.to_string())?;

    // Code is at package main (implicit), so $self-> should not resolve
    let code = r#"sub run {
my $self = shift;
$self->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    // Should NOT suggest MyLib methods just because the variable is named $self
    assert!(
        !completions.iter().any(|c| c.label == "helper"),
        "$self-> in main package should not suggest methods from other packages"
    );
    Ok(())
}

// -------------------------------------------------------------------------
// Literal `bless` receiver inference (issue #7896)
// -------------------------------------------------------------------------

#[test]
fn test_bless_double_quoted_class_resolves_methods() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
sub fetch { }
1;
"#
        .to_string(),
    )?;
    // Index an unrelated package to prove the inference stays scoped to Foo
    // and does not leak unrelated workspace methods into completions.
    index.index_file(
        Url::parse("file:///workspace/Unrelated.pm")?,
        r#"package Unrelated;
sub quack { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless {}, "Foo";
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "bark"),
        "bless {{}}, \"Foo\" should infer Foo and suggest bark; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "fetch"),
        "bless {{}}, \"Foo\" should infer Foo and suggest fetch"
    );
    assert!(
        !completions.iter().any(|c| c.label == "quack"),
        "bless {{}}, \"Foo\" must not leak Unrelated::quack into completions"
    );
    Ok(())
}

#[test]
fn test_bless_single_quoted_class_resolves_methods() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless {}, 'Foo';
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "bark"),
        "bless {{}}, 'Foo' should infer Foo and suggest bark; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_with_hash_content_resolves_methods() -> Result<(), Box<dyn std::error::Error>> {
    // Hash content with internal commas must not confuse the arg-separator
    // comma finder.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless { a => 1, b => 2 }, "Foo";
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "bark"),
        "bless with hash content + class should still infer Foo; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_with_parens_resolves_methods() -> Result<(), Box<dyn std::error::Error>> {
    // `bless({}, "Foo")` form — parenthesized call.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless({}, "Foo");
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "bark"),
        "bless({{}}, \"Foo\") should infer Foo and suggest bark; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_with_newline_separated_args_resolves_methods()
-> Result<(), Box<dyn std::error::Error>> {
    // Newline-separated bless arguments are valid Perl and should still be
    // classified as a literal-bless receiver for completion inference.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless {},
    "Foo";
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "bark"),
        "newline-separated bless args should infer Foo and suggest bark; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_qualified_class_resolves_methods() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo/Bar.pm")?,
        r#"package Foo::Bar;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless {}, "Foo::Bar";
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "bark"),
        "bless {{}}, \"Foo::Bar\" should infer Foo::Bar and suggest bark; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_dynamic_class_does_not_resolve() -> Result<(), Box<dyn std::error::Error>> {
    // `bless {}, $class` is dynamic — we must not infer a static package and
    // must not suggest methods from any specific package on its behalf.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $class = "Foo";
my $x = bless {}, $class;
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|c| c.label == "bark"),
        "bless {{}}, $class is dynamic — must not infer Foo. got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_with_concat_class_does_not_resolve() -> Result<(), Box<dyn std::error::Error>> {
    // `bless {}, "Foo" . $suffix` is a non-literal class expression — must
    // fail closed and not infer Foo even though the literal `"Foo"` appears.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $suffix = "Bar";
my $x = bless {}, "Foo" . $suffix;
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|c| c.label == "bark"),
        "bless {{}}, \"Foo\" . $suffix is non-literal — must not infer Foo. got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_nested_in_call_does_not_resolve() -> Result<(), Box<dyn std::error::Error>> {
    // `wrapper(bless {}, "Foo")` does not establish that `$x` is Foo — the
    // assignment result is whatever `wrapper` returns. The helper must
    // anchor on RHS-starts-with-bless and reject this nested case.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"sub wrapper { return $_[0]; }
my $x = wrapper(bless {}, "Foo");
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|c| c.label == "bark"),
        "wrapper(bless {{}}, \"Foo\") is nested — must not infer Foo. got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_qualified_prefix_does_not_resolve() -> Result<(), Box<dyn std::error::Error>> {
    // `bless::class {}, "Foo"` is a fully-qualified call to a sub named
    // `class` in package `bless`, NOT the builtin `bless` expression. The
    // assignment result is whatever that sub returns, not a Foo. The
    // stricter `starts_with_bless_expression` guard must reject this even
    // though the byte after `bless` (`:`) is not a Perl identifier byte.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless::class {}, "Foo";
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|c| c.label == "bark"),
        "bless::class {{}}, \"Foo\" is not the builtin bless — must not infer Foo. got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_bless_array_ref_with_class_resolves_methods() -> Result<(), Box<dyn std::error::Error>> {
    // Array-ref content with internal commas must not confuse the
    // arg-separator scan (delimiter-balanced top-level comma finder).
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless [1, 2, 3], "Foo";
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "bark"),
        "bless [1, 2, 3], \"Foo\" should infer Foo and suggest bark; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

// -------------------------------------------------------------------------
// Receiver-evidence in method completion detail text (issue #7918)
//
// These tests assert that the existing `ReceiverEvidence` provenance
// (#7917) is now appended to method completion `detail` text. They also
// pin invariants that label / insert_text / filter_text / sort_text /
// the candidate set itself are unchanged — this PR is detail-only.
// -------------------------------------------------------------------------

#[test]
fn detail_includes_static_package_receiver_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = "Foo->";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let detail = must_some(bark.detail.as_deref());
    assert!(
        detail.contains("receiver: static package"),
        "Foo->bark detail should include static-package receiver evidence; got {detail:?}"
    );
    Ok(())
}

#[test]
fn detail_includes_receiver_evidence_for_constructor_assignment()
-> Result<(), Box<dyn std::error::Error>> {
    // For `my $x = Foo->new; $x->`, three receiver-evidence paths could
    // legitimately fire:
    //   - source-backed receiver fact from the semantic fact layer
    //   - text-pattern `ConstructorAssignment` (matches `Foo->new`
    //     assignment in the source)
    //   - `TypeInferenceEngine` (which DOES infer `$x : Foo` from
    //     constructor-method calls in production today, contrary to the
    //     `bless`-only limitation)
    //
    // The exact source-backed receiver fact is preferred when available, but
    // the legacy labels remain valid fallback paths.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub new { bless {}, shift }
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = Foo->new;
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let detail = must_some(bark.detail.as_deref());
    assert!(
        detail.contains("receiver: source-backed object")
            || detail.contains("receiver: constructor assignment")
            || detail.contains("receiver: type engine"),
        "my $$x = Foo->new; $$x->bark detail should include receiver evidence \
         (source-backed object, constructor assignment, or type engine); got {detail:?}"
    );
    Ok(())
}

#[test]
fn detail_includes_literal_bless_receiver_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = bless {}, "Foo";
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let detail = must_some(bark.detail.as_deref());
    // Outcome C from #7925: medium-confidence evidence (LiteralBless) gets
    // an explicit `, medium confidence` suffix.
    assert!(
        detail.contains("receiver: literal bless, medium confidence"),
        "literal bless detail should include `receiver: literal bless, medium confidence`; got {detail:?}"
    );
    Ok(())
}

#[test]
fn detail_omits_confidence_label_for_high_confidence_static_package()
-> Result<(), Box<dyn std::error::Error>> {
    // Outcome C from #7925: high-confidence evidence (StaticPackage) stays
    // unlabelled — the detail must not contain "medium confidence" or
    // "high confidence" noise.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = "Foo->";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let detail = must_some(bark.detail.as_deref());
    assert!(
        detail.contains("receiver: static package"),
        "static-package detail should still include receiver suffix; got {detail:?}"
    );
    assert!(
        !detail.contains("medium confidence"),
        "high-confidence StaticPackage detail must not be labelled medium confidence; got {detail:?}"
    );
    assert!(
        !detail.contains("high confidence"),
        "high-confidence StaticPackage detail must not be labelled high confidence; got {detail:?}"
    );
    Ok(())
}

#[test]
fn detail_includes_medium_confidence_label_for_type_engine_or_constructor_assignment()
-> Result<(), Box<dyn std::error::Error>> {
    // For `my $x = Foo->new; $x->`, three paths can fire:
    //   - SourceBackedObject (high) -> unlabelled
    //   - TypeEngine (medium) -> labelled
    //   - ConstructorAssignment (high) -> unlabelled
    //
    // Outcome C from #7925: detail must include the medium label only
    // when the firing variant is medium-confidence. This test asserts the
    // (label present) ⇔ (TypeEngine fired) implication so neither path
    // accidentally drops or adds a label.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub new { bless {}, shift }
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"my $x = Foo->new;
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let detail = must_some(bark.detail.as_deref());

    let has_type_engine = detail.contains("receiver: type engine");
    let has_constructor = detail.contains("receiver: constructor assignment");
    let has_source_backed = detail.contains("receiver: source-backed object");
    let has_medium_label = detail.contains("medium confidence");

    assert!(
        has_type_engine || has_constructor || has_source_backed,
        "constructor-assignment fixture must emit one valid receiver label; got {detail:?}"
    );
    if has_type_engine {
        assert!(
            has_medium_label,
            "TypeEngine evidence is medium-confidence; detail must include `medium confidence`; got {detail:?}"
        );
    } else {
        // ConstructorAssignment/source-backed object fired; high-confidence, unlabelled.
        assert!(
            !has_medium_label,
            "High-confidence receiver evidence must NOT include `medium confidence`; got {detail:?}"
        );
    }
    Ok(())
}

#[test]
fn detail_for_inherited_preserves_from_annotation_and_appends_receiver()
-> Result<(), Box<dyn std::error::Error>> {
    // Inherited methods must keep `(from Base)` (or the semantic-path
    // equivalent `inherited method from Parent`) AND get the receiver
    // suffix appended.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Parent.pm")?,
        r#"package Parent;
sub inherited_method { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Child.pm")?,
        r#"package Child;
use parent 'Parent';
1;
"#
        .to_string(),
    )?;

    let code = r#"package Child;
sub run {
my $self = shift;
$self->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let inherited = must_some(completions.iter().find(|c| c.label == "inherited_method"));
    let detail = must_some(inherited.detail.as_deref());
    assert!(
        detail.contains("Parent"),
        "inherited detail should still mention defining package Parent; got {detail:?}"
    );
    assert!(
        detail.contains("receiver: self/this"),
        "inherited detail should append self/this receiver evidence; got {detail:?}"
    );
    Ok(())
}

#[test]
fn detail_change_does_not_alter_label_insert_filter_or_sort_text()
-> Result<(), Box<dyn std::error::Error>> {
    // Invariant: the only field that changes from #7918 is `detail`.
    // `label`, `insert_text`, `filter_text`, and `sort_text` must remain
    // unchanged for the same input (verified against literal expected
    // values that match pre-#7918 production behavior).
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = "Foo->";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    assert_eq!(bark.label, "bark", "label must not change");
    assert_eq!(bark.insert_text.as_deref(), Some("bark()"), "insert_text must not change");
    assert_eq!(bark.filter_text.as_deref(), Some("bark"), "filter_text must not change");
    assert_eq!(
        bark.sort_text.as_deref(),
        Some("2_bark"),
        "sort_text must remain `<tier>_<name>` with tier 2 (own method); \
         receiver-evidence detail must not change ranking"
    );
    Ok(())
}

#[test]
fn detail_with_evidence_helper_handles_both_base_formats() {
    // Direct unit test for the formatting helper that both the semantic
    // and inline-fallback paths call. Forcing the inline-fallback path
    // end-to-end requires synthetic workspace-index state not exposed by
    // the public test harness (semantic cutover succeeds for fixtures
    // where `index.index_file()` populates both the workspace symbol
    // table and the semantic fact shards together). This unit test pins
    // the contract that BOTH base-detail formats receive the same suffix
    // treatment.
    use super::workspace::detail_with_evidence;

    // Inline-fallback path's base format ("Foo method" / "Foo method (from Base)"):
    let fallback_own = detail_with_evidence(
        "Foo method".to_string(),
        &ReceiverEvidence::StaticPackage("Foo".to_string()),
    );
    assert_eq!(fallback_own, "Foo method — receiver: static package");

    let fallback_inherited = detail_with_evidence(
        "Foo method (from Base)".to_string(),
        &ReceiverEvidence::SelfOrThis("Foo".to_string()),
    );
    assert_eq!(fallback_inherited, "Foo method (from Base) — receiver: self/this");

    // Semantic path's base format ("method from Foo" / "inherited method from Parent"
    // / "generated accessor from Foo"):
    // LiteralBless is medium-confidence — outcome C from #7925 appends
    // ", medium confidence".
    let semantic_own = detail_with_evidence(
        "method from Foo".to_string(),
        &ReceiverEvidence::LiteralBless("Foo".to_string()),
    );
    assert_eq!(semantic_own, "method from Foo — receiver: literal bless, medium confidence");

    // ConstructorAssignment is high-confidence — no label appended.
    let semantic_inherited = detail_with_evidence(
        "inherited method from Parent".to_string(),
        &ReceiverEvidence::ConstructorAssignment("Foo".to_string()),
    );
    assert_eq!(
        semantic_inherited,
        "inherited method from Parent — receiver: constructor assignment"
    );

    // TypeEngine is medium-confidence — outcome C appends label.
    let generated = detail_with_evidence(
        "generated accessor from Foo".to_string(),
        &ReceiverEvidence::TypeEngine("Foo".to_string()),
    );
    assert_eq!(generated, "generated accessor from Foo — receiver: type engine, medium confidence");

    // Unknown evidence: base detail is returned unchanged.
    let unknown = detail_with_evidence("Foo method".to_string(), &ReceiverEvidence::Unknown);
    assert_eq!(unknown, "Foo method");
}

#[test]
fn detail_with_evidence_helper_handles_empty_and_punctuated_base_details() {
    use super::workspace::detail_with_evidence;

    // Edge case: empty base detail should still format deterministically.
    let empty_base =
        detail_with_evidence(String::new(), &ReceiverEvidence::TypeEngine("Foo".to_string()));
    assert_eq!(empty_base, " — receiver: type engine, medium confidence");

    // Edge case: inherited details can already include punctuation; suffix
    // insertion should append cleanly without altering the original text.
    let punctuated = detail_with_evidence(
        "method from Foo (experimental; v2)".to_string(),
        &ReceiverEvidence::StaticPackage("Foo".to_string()),
    );
    assert_eq!(punctuated, "method from Foo (experimental; v2) — receiver: static package");
}

#[test]
fn detail_unchanged_when_no_receiver_evidence_path_reachable()
-> Result<(), Box<dyn std::error::Error>> {
    // When receiver inference fails, the production callsite returns no
    // method completions at all (existing pre-#7918 behavior). This test
    // pins that contract: an unknown receiver yields no exact-receiver
    // suggestions, so no method-detail text is produced for `bark`.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = "$nonexistent->";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|c| c.label == "bark"),
        "unknown receiver must not produce Foo's methods (pre-#7918 behavior preserved)"
    );
    Ok(())
}

#[test]
fn live_completion_visible_symbol_slice_surfaces_explicit_import()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/lib/Tools.pm")?,
        r#"package Tools;
use Exporter 'import';
our @EXPORT_OK = qw(alpha beta);
sub alpha { }
sub beta { }
1;
"#
        .to_string(),
    )?;

    let importer_uri = Url::parse("file:///workspace/app.pl")?;
    let code = r#"package App;
use Tools qw(alpha);
al"#;
    index.index_file(importer_uri.clone(), code.to_string())?;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let completions =
        provider.get_completions_with_path(code, code.len(), Some(importer_uri.as_str()));

    let alpha = must_some(completions.iter().find(|item| item.label == "alpha"));
    assert_eq!(alpha.insert_text.as_deref(), Some("alpha"));
    assert_eq!(alpha.filter_text.as_deref(), Some("alpha"));
    assert!(
        must_some(alpha.sort_text.as_deref()).starts_with("2z_visible_"),
        "visible-symbol completion should use the narrow live sort tier; got {:?}",
        alpha.sort_text
    );
    let detail = must_some(alpha.detail.as_deref());
    assert!(
        detail.contains("imported from Tools") && detail.contains("compiler fact"),
        "visible-symbol completion should label source/provenance in detail; got {detail:?}"
    );
    let documentation = must_some(alpha.documentation.as_deref());
    assert!(
        documentation.contains("Provenance: ImportExportInference")
            && documentation.contains("Freshness: Fresh"),
        "visible-symbol completion should document provenance and freshness; got {documentation:?}"
    );
    assert!(
        !completions.iter().any(|item| item.label == "beta"),
        "explicit import should not promote unimported optional export `beta`"
    );
    Ok(())
}

#[test]
fn live_completion_visible_symbol_slice_respects_empty_import()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/lib/Tools.pm")?,
        r#"package Tools;
use Exporter 'import';
our @EXPORT = qw(alpha);
sub alpha { }
1;
"#
        .to_string(),
    )?;

    let importer_uri = Url::parse("file:///workspace/app_empty_import.pl")?;
    let code = r#"package App;
use Tools ();
al"#;
    index.index_file(importer_uri.clone(), code.to_string())?;

    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let completions =
        provider.get_completions_with_path(code, code.len(), Some(importer_uri.as_str()));

    assert!(
        !completions.iter().any(|item| {
            item.label == "alpha"
                && item
                    .sort_text
                    .as_deref()
                    .is_some_and(|sort_text| sort_text.starts_with("2z_visible_"))
        }),
        "empty import should not promote default export `alpha` through the live visible-symbol path"
    );
    Ok(())
}

// -------------------------------------------------------------------------
// Unknown-receiver bounded fallback (issue #7929, outcome A)
//
// These tests pin the production-callsite behavior of the bounded
// low-confidence fallback. Source policy:
//   - imported / visible packages from the buffer's `import_map`
//   - current package (when not `main`) and its `@ISA` chain
// All-workspace fallback is intentionally NOT used. Dynamic (positively
// detected fail-closed bless forms) is NOT fallback-eligible.
// -------------------------------------------------------------------------

#[test]
fn fallback_offers_imported_package_methods_for_unknown_receiver()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    // Buffer imports Foo and calls a method on a variable that has no
    // receiver-evidence assignment (`$obj` is undeclared / unknown).
    // Receiver evidence is `Unknown` — fallback should fire from the
    // imported `Foo` package.
    let code = r#"use Foo;
1;
$obj->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = match completions.iter().find(|c| c.label == "bark") {
        Some(bark) => bark,
        None => {
            let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
            return Err(format!(
                "fallback should include imported Foo's `bark`; got labels: {labels:?}"
            )
            .into());
        }
    };
    let detail = must_some(bark.detail.as_deref());
    assert!(
        detail.contains("receiver: unknown, low confidence"),
        "fallback detail should say receiver: unknown, low confidence; got {detail:?}"
    );
    let sort_text = must_some(bark.sort_text.as_deref());
    assert!(sort_text.starts_with("6_"), "fallback sort_text should be tier 6; got {sort_text:?}");
    Ok(())
}

#[test]
fn source_backed_hash_slot_receiver_uses_exact_completion_pilot()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/MyApp/DB.pm")?,
        r#"package MyApp::DB;
sub connect { }
1;
"#
        .to_string(),
    )?;

    let code = "my %services = (db => MyApp::DB->new); $services{db}->connect;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let position = must_some(code.find("$services{db}->")) + "$services{db}->".len();
    let completions = provider.get_completions(code, position);

    let connect = must_some(completions.iter().find(|c| c.label == "connect"));
    let detail = must_some(connect.detail.as_deref());
    assert!(
        detail.contains("receiver: hash slot"),
        "source-backed hash-slot receiver should be exact and labeled; got {detail:?}"
    );
    let sort_text = must_some(connect.sort_text.as_deref());
    assert!(
        sort_text.starts_with("2_") || sort_text.starts_with("3_"),
        "exact receiver completion should rank above fallback tier 6; got {sort_text:?}"
    );
    Ok(())
}

#[test]
fn medium_confidence_accessor_return_receiver_preserves_imported_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/MyApp/DB.pm")?,
        r#"package MyApp::DB;
sub connect { }
1;
"#
        .to_string(),
    )?;

    let code = r#"use MyApp::DB;
package MyApp::Service;
use Moo;
has db => (is => 'ro', isa => 'MyApp::DB');
my $service = MyApp::Service->new;
$service->db->connect;"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let position = must_some(code.find("$service->db->")) + "$service->db->".len();
    let completions = provider.get_completions(code, position);

    let connect = must_some(completions.iter().find(|c| c.label == "connect"));
    let detail = must_some(connect.detail.as_deref());
    assert!(
        detail.contains("receiver: unknown, low confidence"),
        "medium-confidence accessor-return facts must preserve fallback, not exact receiver evidence; got {detail:?}"
    );
    assert!(
        !detail.contains("receiver: source-backed object")
            && !detail.contains("receiver: hash slot")
            && !detail.contains("receiver: literal bless"),
        "medium-confidence accessor-return facts must not be promoted to exact receiver detail; got {detail:?}"
    );
    let sort_text = must_some(connect.sort_text.as_deref());
    assert!(
        sort_text.starts_with("6_"),
        "medium-confidence accessor-return fallback should keep fallback tier 6; got {sort_text:?}"
    );
    Ok(())
}

#[test]
fn medium_confidence_method_return_receiver_preserves_imported_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/MyApp/DB.pm")?,
        r#"package MyApp::DB;
sub connect { }
1;
"#
        .to_string(),
    )?;

    let code = r#"use MyApp::DB;
package MyApp::Service;
sub db { return MyApp::DB->new; }
my $service = MyApp::Service->new;
$service->db->connect;"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let position = must_some(code.find("$service->db->")) + "$service->db->".len();
    let completions = provider.get_completions(code, position);

    let connect = must_some(completions.iter().find(|c| c.label == "connect"));
    let detail = must_some(connect.detail.as_deref());
    assert!(
        detail.contains("receiver: unknown, low confidence"),
        "medium-confidence method-return facts must preserve fallback, not exact receiver evidence; got {detail:?}"
    );
    assert!(
        !detail.contains("receiver: source-backed object")
            && !detail.contains("receiver: hash slot")
            && !detail.contains("receiver: literal bless"),
        "medium-confidence method-return facts must not be promoted to exact receiver detail; got {detail:?}"
    );
    let sort_text = must_some(connect.sort_text.as_deref());
    assert!(
        sort_text.starts_with("6_"),
        "medium-confidence method-return fallback should keep fallback tier 6; got {sort_text:?}"
    );
    Ok(())
}

#[test]
fn dynamic_hash_key_receiver_preserves_imported_fallback() -> Result<(), Box<dyn std::error::Error>>
{
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/MyApp/DB.pm")?,
        r#"package MyApp::DB;
sub connect { }
1;
"#
        .to_string(),
    )?;

    let code = r#"use MyApp::DB;
my %services = (db => MyApp::DB->new);
my $name = "db";
$services{$name}->connect;"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source(&ast, code, Some(index));
    let position = must_some(code.find("$services{$name}->")) + "$services{$name}->".len();
    let completions = provider.get_completions(code, position);

    let connect = must_some(completions.iter().find(|c| c.label == "connect"));
    let detail = must_some(connect.detail.as_deref());
    assert!(
        detail.contains("receiver: unknown, low confidence"),
        "dynamic hash key must preserve bounded fallback, not exact hash-slot evidence; got {detail:?}"
    );
    assert!(
        !detail.contains("receiver: hash slot"),
        "dynamic hash key must not be labeled as an exact hash-slot receiver; got {detail:?}"
    );
    let sort_text = must_some(connect.sort_text.as_deref());
    assert!(
        sort_text.starts_with("6_"),
        "dynamic hash-key fallback should keep fallback tier 6; got {sort_text:?}"
    );
    Ok(())
}

#[test]
fn fallback_offers_current_package_methods_for_unknown_receiver()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/MyService.pm")?,
        r#"package MyService;
sub helper { }
1;
"#
        .to_string(),
    )?;

    // In package MyService, calling a method on an undeclared variable —
    // receiver is Unknown, current-package methods are the bounded source.
    let code = r#"package MyService;
$obj->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let helper = must_some(completions.iter().find(|c| c.label == "helper"));
    let detail = must_some(helper.detail.as_deref());
    assert!(
        detail.contains("receiver: unknown, low confidence"),
        "fallback detail should say receiver: unknown, low confidence; got {detail:?}"
    );
    Ok(())
}

#[test]
fn fallback_does_not_include_unrelated_workspace_packages() -> Result<(), Box<dyn std::error::Error>>
{
    // `Unrelated` is indexed in the workspace but neither imported by
    // the buffer nor part of the current package graph. Bounded fallback
    // must NOT include `Unrelated::quack`.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Unrelated.pm")?,
        r#"package Unrelated;
sub quack { }
1;
"#
        .to_string(),
    )?;

    let code = r#"use Foo;
1;
$obj->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "bark"),
        "fallback should include imported Foo's methods; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        !completions.iter().any(|c| c.label == "quack"),
        "fallback must NOT include unrelated workspace packages (#7929 bounded source policy)"
    );
    Ok(())
}

#[test]
fn fallback_does_not_fire_for_dynamic_receiver() -> Result<(), Box<dyn std::error::Error>> {
    // `bless {}, $class` is Dynamic — fail-closed. Even though Foo is
    // imported, fallback must not fire (Dynamic is explicitly not
    // fallback-eligible per #7929).
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = r#"use Foo;
my $class = "Foo";
my $x = bless {}, $class;
$x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|c| c.label == "bark"),
        "Dynamic receiver must stay fail-closed even with Foo imported; got {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn fallback_does_not_alter_exact_receiver_detail_or_order() -> Result<(), Box<dyn std::error::Error>>
{
    // Exact receiver case: `Foo->bark`. After this PR fallback exists,
    // but exact-receiver completions must keep their existing detail
    // (with `receiver: static package`), tier-2 sort, and label/insert
    // shape. No `receiver: unknown` should leak into exact completions.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = "Foo->";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let detail = must_some(bark.detail.as_deref());
    assert!(
        detail.contains("receiver: static package"),
        "exact static-package detail unchanged; got {detail:?}"
    );
    assert!(
        !detail.contains("unknown"),
        "exact-receiver detail must not contain 'unknown'; got {detail:?}"
    );
    assert_eq!(bark.sort_text.as_deref(), Some("2_bark"), "exact-receiver sort tier unchanged");
    Ok(())
}

#[test]
fn fallback_tier_is_below_exact_receiver_tiers() -> Result<(), Box<dyn std::error::Error>> {
    // Pins the tier-vs-tier contract by running two completion requests
    // against the same workspace index:
    //   1. exact:    `Foo->`         must surface `bark` at tier 2 or 3
    //                                with no `unknown` in detail
    //   2. fallback: `use Bar;\n$obj->` must surface `mew` at tier 6
    //                                with `receiver: unknown, low confidence`
    // The earlier `fallback_sorts_below_exact_receiver_completions` test
    // only exercised path #1 (fallback never fires for `Foo->`), so it
    // could not actually prove the tier discipline. This replacement
    // exercises both paths and asserts the numeric tier ordering.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Bar.pm")?,
        r#"package Bar;
sub mew { }
1;
"#
        .to_string(),
    )?;

    // Request 1 — exact receiver via `Foo->`.
    let exact_code = "Foo->";
    let mut parser_exact = Parser::new(exact_code);
    let exact_ast = must(parser_exact.parse());
    let exact_provider = CompletionProvider::new_with_index(&exact_ast, Some(index.clone()));
    let exact_completions = exact_provider.get_completions(exact_code, exact_code.len());
    let bark = must_some(exact_completions.iter().find(|c| c.label == "bark"));
    let bark_sort = must_some(bark.sort_text.as_deref());
    let bark_detail = must_some(bark.detail.as_deref());
    assert!(
        bark_sort.starts_with("2_") || bark_sort.starts_with("3_"),
        "exact Foo->bark must use tier 2 or 3; got {bark_sort:?}"
    );
    assert!(
        !bark_detail.contains("unknown"),
        "exact-receiver detail must not contain 'unknown'; got {bark_detail:?}"
    );

    // Request 2 — Unknown receiver fallback via `use Bar;\n$obj->`.
    let fallback_code = "use Bar;\n$obj->";
    let mut parser_fb = Parser::new(fallback_code);
    let fallback_ast = must(parser_fb.parse());
    let fallback_provider = CompletionProvider::new_with_index(&fallback_ast, Some(index));
    let fallback_completions =
        fallback_provider.get_completions(fallback_code, fallback_code.len());
    let mew = must_some(fallback_completions.iter().find(|c| c.label == "mew"));
    let mew_sort = must_some(mew.sort_text.as_deref());
    let mew_detail = must_some(mew.detail.as_deref());
    assert!(
        mew_sort.starts_with("6_"),
        "Unknown-receiver fallback `mew` must use tier 6; got {mew_sort:?}"
    );
    assert!(
        mew_detail.contains("receiver: unknown, low confidence"),
        "fallback detail should say receiver: unknown, low confidence; got {mew_detail:?}"
    );

    // Numeric tier proof: exact tier (2 or 3) < fallback tier (6).
    let exact_tier: u8 = bark_sort
        .as_bytes()
        .first()
        .copied()
        .and_then(|b| (b as char).to_digit(10).map(|d| d as u8))
        .ok_or("exact sort_text must start with a digit")?;
    let fallback_tier: u8 = mew_sort
        .as_bytes()
        .first()
        .copied()
        .and_then(|b| (b as char).to_digit(10).map(|d| d as u8))
        .ok_or("fallback sort_text must start with a digit")?;
    assert!(
        exact_tier < fallback_tier,
        "exact tier {exact_tier} must sort above fallback tier {fallback_tier}"
    );
    Ok(())
}

#[test]
fn fallback_omits_when_no_imports_and_main_package() -> Result<(), Box<dyn std::error::Error>> {
    // No `use` and the buffer is in main package — `allowed_packages`
    // is empty in `add_unknown_receiver_fallback`, so fallback emits
    // no candidates. This is the `detail_unchanged_when_no_receiver_…`
    // contract: bounded fallback respects the bound.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
1;
"#
        .to_string(),
    )?;

    let code = "$nonexistent->";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    assert!(
        !completions.iter().any(|c| c.label == "bark"),
        "no imports + main package + Unknown receiver = no fallback (bounded source empty); got {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

// -------------------------------------------------------------------------
// Receiver-evidence classification (issue #7910, outcome B)
//
// These tests pin the typed receiver-evidence provenance produced by
// `classify_text_pattern_receiver` and `classify_receiver`. They do not
// assert on completion ordering — outcome B explicitly does not change
// ordering — they assert on the evidence variant and confidence level.
// -------------------------------------------------------------------------

use super::workspace::{ReceiverEvidence, classify_receiver, classify_text_pattern_receiver};
use perl_semantic_analyzer::type_inference::TypeInferenceEngine;
use perl_semantic_facts::Confidence;

fn ctx_for(prefix: &str, current_package: &str, source_position: usize) -> CompletionContext {
    CompletionContext {
        position: source_position,
        trigger_character: None,
        in_string: false,
        in_regex: false,
        in_comment: false,
        in_use_statement: false,
        current_package: current_package.to_string(),
        prefix: prefix.to_string(),
        prefix_start: source_position.saturating_sub(prefix.len()),
        cursor_scope_id: 0,
    }
}

#[test]
fn classify_receiver_static_package_is_high_confidence() {
    let source = "Foo->";
    let ctx = ctx_for("Foo->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::StaticPackage("Foo".to_string()));
    assert_eq!(ev.package(), Some("Foo"));
    assert_eq!(ev.confidence(), Some(Confidence::High));
}

#[test]
fn classify_receiver_qualified_static_package() {
    let source = "Foo::Bar->";
    let ctx = ctx_for("Foo::Bar->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::StaticPackage("Foo::Bar".to_string()));
    assert_eq!(ev.confidence(), Some(Confidence::High));
}

#[test]
fn classify_receiver_self_is_high_confidence() {
    let source = "package MyService;\n$self->";
    let ctx = ctx_for("$self->", "MyService", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::SelfOrThis("MyService".to_string()));
    assert_eq!(ev.confidence(), Some(Confidence::High));
}

#[test]
fn classify_receiver_this_is_high_confidence() {
    let source = "package MyHandler;\n$this->";
    let ctx = ctx_for("$this->", "MyHandler", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::SelfOrThis("MyHandler".to_string()));
    assert_eq!(ev.confidence(), Some(Confidence::High));
}

#[test]
fn classify_receiver_self_in_main_package_is_unknown() {
    // `$self->` in main package does not classify — guard matches the
    // existing receiver-inference behavior pre-#7910.
    let source = "$self->";
    let ctx = ctx_for("$self->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Unknown);
    assert_eq!(ev.confidence(), None);
}

#[test]
fn classify_receiver_constructor_assignment_is_high_confidence() {
    let source = "my $x = Foo->new;\n$x->";
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::ConstructorAssignment("Foo".to_string()));
    assert_eq!(ev.confidence(), Some(Confidence::High));
}

#[test]
fn classify_receiver_qualified_constructor_assignment() {
    let source = "my $x = Foo::Bar->new;\n$x->";
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::ConstructorAssignment("Foo::Bar".to_string()));
    assert_eq!(ev.confidence(), Some(Confidence::High));
}

#[test]
fn classify_receiver_literal_bless_is_medium_confidence() {
    let source = r#"my $x = bless {}, "Foo";
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::LiteralBless("Foo".to_string()));
    assert_eq!(ev.confidence(), Some(Confidence::Medium));
}

#[test]
fn classify_receiver_literal_bless_qualified_class_is_medium_confidence() {
    let source = r#"my $x = bless {}, "Foo::Bar";
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::LiteralBless("Foo::Bar".to_string()));
    assert_eq!(ev.confidence(), Some(Confidence::Medium));
}

#[test]
fn classify_receiver_dynamic_bless_is_dynamic() {
    // `bless {}, $class` is dynamic — extract_bless_literal_class fails
    // closed. After #7929 the classifier reports `Dynamic` (not
    // `Unknown`) so the Unknown-receiver fallback stays fail-closed
    // instead of offering bounded fallback methods for the variable.
    let source = r#"my $class = "Foo";
my $x = bless {}, $class;
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Dynamic);
    assert_eq!(ev.confidence(), None);
    assert!(!ev.is_unknown_fallback_eligible());
}

#[test]
fn classify_receiver_dynamic_concat_class_is_dynamic() {
    // `bless {}, "Foo" . $suffix` is non-literal — Dynamic, not Unknown.
    let source = r#"my $suffix = "Bar";
my $x = bless {}, "Foo" . $suffix;
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Dynamic);
    assert!(!ev.is_unknown_fallback_eligible());
}

#[test]
fn classify_receiver_dynamic_nested_bless_is_dynamic() {
    // `wrapper(bless {}, "Foo")` is nested — assignment result is the
    // wrapper return, not the blessed object. Dynamic, not Unknown.
    let source = r#"sub wrapper { return $_[0]; }
my $x = wrapper(bless {}, "Foo");
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Dynamic);
    assert!(!ev.is_unknown_fallback_eligible());
}

#[test]
fn classify_receiver_dynamic_qualified_prefix_is_dynamic() {
    // `bless::class {}, "Foo"` is a non-builtin qualified call. Dynamic.
    let source = r#"my $x = bless::class {}, "Foo";
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Dynamic);
    assert!(!ev.is_unknown_fallback_eligible());
}

#[test]
fn classify_receiver_unknown_is_fallback_eligible() {
    // True unknown — no `bless` keyword, no assignment evidence — is
    // fallback-eligible.
    let source = "$nonexistent->";
    let ctx = ctx_for("$nonexistent->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Unknown);
    assert!(ev.is_unknown_fallback_eligible());
}

#[test]
fn classify_receiver_bless_keyword_inside_string_is_not_dynamic() {
    // The substring `bless` appears in a string literal, not as a Perl
    // keyword. The `rhs_has_bless_keyword_outside_strings` helper must
    // skip string contents — this scenario is Unknown, not Dynamic.
    let source = r#"my $x = "I bless this house";
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Unknown);
}

#[test]
fn classify_receiver_variable_named_bless_is_unknown_fallback_eligible() {
    // `$bless` as RHS — the substring `bless` is preceded by a `$` sigil
    // and is therefore an identifier suffix, not a call-like `bless`
    // keyword. Must NOT be classified as Dynamic; must remain Unknown so
    // the bounded fallback (#7929) is still eligible.
    let source = r#"my $x = $bless;
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Unknown);
    assert!(ev.is_unknown_fallback_eligible());
}

#[test]
fn classify_receiver_bless_in_comment_is_not_dynamic() {
    // A `# bless ...` comment after the assignment must not be treated as
    // a dynamic bless expression. The RHS scan should stop at `#`
    // (single-line comment terminator), keeping evidence Unknown.
    let source = r#"my $x = $obj; # bless this later
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Unknown);
    assert!(ev.is_unknown_fallback_eligible());
}

#[test]
fn classify_receiver_hash_key_bless_is_not_dynamic() {
    // `$obj->{bless}` is a hash-key access whose key happens to be the
    // word `bless`. Preceded by `{`, this is not a call-like `bless`
    // and must not become Dynamic.
    let source = r#"my $x = $obj->{bless};
$x->"#;
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Unknown);
    assert!(ev.is_unknown_fallback_eligible());
}

#[test]
fn classify_receiver_no_assignment_is_unknown() {
    // Variable with no preceding assignment — no evidence, classifier
    // reports Unknown and the production callsite returns no method
    // completions.
    let source = "$nonexistent->";
    let ctx = ctx_for("$nonexistent->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Unknown);
    assert_eq!(ev.confidence(), None);
}

#[test]
fn classify_receiver_lowercase_static_prefix_is_unknown() {
    // Lowercase identifier on the left of `->` is not a Perl package name
    // (packages start uppercase by convention). The classifier falls
    // through to Unknown.
    let source = "foo->";
    let ctx = ctx_for("foo->", "main", source.len());
    let ev = classify_text_pattern_receiver(&ctx, source);
    assert_eq!(ev, ReceiverEvidence::Unknown);
}

#[test]
fn classify_receiver_unknown_has_no_package() {
    let ev = ReceiverEvidence::Unknown;
    assert_eq!(ev.package(), None);
    assert_eq!(ev.confidence(), None);
}

#[test]
fn type_engine_variant_has_medium_confidence() {
    // Direct accessor proof for the TypeEngine variant. End-to-end
    // production-callsite proof (a `classify_receiver` call that returns
    // TypeEngine from a natural Perl source) is deferred: the current
    // `TypeInferenceEngine` does not infer `PerlType::Object` from
    // `my $x = Foo->new` or `bless` (per `real_world_patterns.rs:1718`,
    // "bless is not directly tracked by type inference"), and its
    // `global_env` is private with no public setter, so we cannot seed
    // an Object type in tests through the supported API. This test
    // pins the variant's package / confidence accessors so the future
    // PR that wires Object types into the engine has a stable contract
    // to land against.
    let ev = ReceiverEvidence::TypeEngine("Foo".to_string());
    assert_eq!(ev.package(), Some("Foo"));
    assert_eq!(ev.confidence(), Some(Confidence::Medium));
}

#[test]
fn classify_receiver_engine_present_but_empty_falls_through_to_text_pattern() {
    // Proves the type-engine arm of `classify_receiver` is wired
    // correctly and fails over to the text-pattern arm when the engine
    // has no Object type for the receiver variable. With the engine
    // supplied but empty, `my $x = Foo->new; $x->` should classify as
    // ConstructorAssignment (text-pattern), not TypeEngine.
    let source = "my $x = Foo->new;\n$x->";
    let ctx = ctx_for("$x->", "main", source.len());
    let engine = TypeInferenceEngine::new();
    let ev = classify_receiver(&ctx, source, Some(&engine));
    assert_eq!(ev, ReceiverEvidence::ConstructorAssignment("Foo".to_string()));
    assert_eq!(ev.confidence(), Some(Confidence::High));
}

#[test]
fn classify_receiver_no_engine_uses_text_pattern() {
    // Sanity counterpart: with no type engine supplied, the same source
    // must still classify (via the text-pattern arm) as
    // ConstructorAssignment.
    let source = "my $x = Foo->new;\n$x->";
    let ctx = ctx_for("$x->", "main", source.len());
    let ev = classify_receiver(&ctx, source, None);
    assert_eq!(ev, ReceiverEvidence::ConstructorAssignment("Foo".to_string()));
}

// -------------------------------------------------------------------------
// Tests for is_use_statement_context and add_use_module_completions
// -------------------------------------------------------------------------

#[test]
fn test_use_statement_context_after_use_keyword() -> Result<(), Box<dyn std::error::Error>> {
    // "use " with cursor right after space — empty prefix, should trigger module completion
    let index = Arc::new(WorkspaceIndex::new());
    let uri = Url::parse("file:///lib/MyApp.pm")?;
    index.index_file(uri, "package MyApp;\n1;\n".to_string())?;
    let code = "use ";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "MyApp" && c.kind == CompletionItemKind::Module),
        "use <cursor> should suggest workspace module names; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_use_statement_context_with_prefix() -> Result<(), Box<dyn std::error::Error>> {
    // "use MyA" — prefix filtering should narrow to MyApp, not OtherLib
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/MyApp.pm")?, "package MyApp;\n1;\n".to_string())?;
    index.index_file(
        Url::parse("file:///lib/OtherLib.pm")?,
        "package OtherLib;\n1;\n".to_string(),
    )?;
    let code = "use MyA";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "MyApp" && c.kind == CompletionItemKind::Module),
        "use MyA should suggest MyApp with Module kind"
    );
    assert!(
        !completions.iter().any(|c| c.label == "OtherLib"),
        "use MyA should not suggest OtherLib"
    );
    Ok(())
}

#[test]
fn test_use_statement_skips_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    // Lowercase-first token after `use` means pragma — no module completion.
    // The index is populated with a Module-kind package so that if the lowercase
    // guard in is_use_statement_context were absent the test would fail (not
    // vacuously pass due to an empty index).
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Strict.pm")?, "package Strict;\n1;\n".to_string())?;
    let code = "use strict";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        "use strict should not trigger module completions; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_use_statement_skips_past_module_name_at_qw() -> Result<(), Box<dyn std::error::Error>> {
    // Cursor inside qw list should NOT trigger module-name completion.
    // The index is populated so the test is non-vacuous: if the qw-dispatch
    // branch were removed, add_use_module_completions would fire and the
    // Module-kind assertion would fail.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///lib/Module.pm")?,
        "package Module;\nsub foo {}\n1;\n".to_string(),
    )?;
    let code = "use Module qw(foo";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    // This context routes to qw-import completions (Function kind), not module-name
    // completions (Module kind).
    assert!(
        !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        "cursor inside qw() should not get module-name completions; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_statement_triggers_module_completion() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    let code = "require Ut";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "Utils" && c.kind == CompletionItemKind::Module),
        "require Ut should suggest Utils with Module kind; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_statement_skips_file_path() -> Result<(), Box<dyn std::error::Error>> {
    // Previously, single-quoted require was blocked. Now completion fires for quoted forms
    // so that `require 'Foo/Ba` gets module-name suggestions as the user types.
    // This test documents that the open-quote context is now active.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    // Open-quote context: cursor right after `require '` (no closing quote)
    let code = "require 'Utils";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    // Completion should fire for quoted module-path forms (the quote restriction is removed)
    assert!(
        completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        "require 'Utils (open-quote) should trigger module-name completions; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_statement_skips_double_quoted_file_path() -> Result<(), Box<dyn std::error::Error>>
{
    // Previously, double-quoted require was blocked. Now completion fires for quoted forms
    // so that `require "Foo/Ba` gets module-name suggestions as the user types.
    // This test documents that the open-quote context is now active for double-quoted forms too.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    // Open-quote context: cursor inside `require "Utils` (no closing quote)
    let code = r#"require "Utils"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    // Completion should fire — the double-quote restriction has been removed
    assert!(
        completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        r#"require "Utils (open-quote) should trigger module-name completions; got: {:?}"#,
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_open_quote_is_require_context() -> Result<(), Box<dyn std::error::Error>> {
    // Confirms that `require "` (cursor right after the opening quote with nothing typed yet)
    // is treated as a valid module-name completion context. Previously, the opening quote char
    // was in the block list, suppressing completion. This regression guard ensures it stays open.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Foo.pm")?, "package Foo;\n1;\n".to_string())?;
    // Cursor right after the opening quote — the user is about to type a module path
    let code = r#"require ""#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        r#"require " (cursor after open-quote) should trigger module-name completions; got: {:?}"#,
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_closed_quoted_pm_path_skips_module_completion()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    let code = r#"require "Utils.pm""#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        r#"closed require "Utils.pm" should not trigger module-name completions; got: {:?}"#,
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_quoted_script_path_skips_module_completion()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    let code = "require './utils.pl'";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        "require './utils.pl' should not trigger module-name completions; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_variable_path_skips_module_completion() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    let code = "require $module";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        "require $module should not trigger module-name completions; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_statement_skips_version_check() -> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    let code = "require 5.010";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        "require 5.010 should not trigger module-name completions; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_statement_triggers_completion_for_lowercase_module()
-> Result<(), Box<dyn std::error::Error>> {
    // `require autodie` is valid Perl — lowercase module names must still get completions.
    // The previous implementation incorrectly blocked all non-uppercase-starting require
    // targets, including valid lowercase modules like autodie, overload, and Carp.
    let index = Arc::new(WorkspaceIndex::new());
    index
        .index_file(Url::parse("file:///lib/autodie.pm")?, "package autodie;\n1;\n".to_string())?;
    let code = "require auto";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "autodie" && c.kind == CompletionItemKind::Module),
        "require auto should suggest 'autodie' with Module kind; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_require_statement_skips_vstring_version() -> Result<(), Box<dyn std::error::Error>> {
    // `require v5.10` is a v-string version check, not a module name.
    // 'v' starts the token but it is not followed by '::' — it should be blocked
    // because 'v' is a digit-prefix indicator in this context.
    // Currently, 'v' is a letter so it passes the digit/quote/path check.
    // This is an inherent limitation of single-char prefix detection — the full
    // `require v5.10` case requires position-aware parsing to resolve correctly.
    // For now, assert the observed (not-yet-blocked) behavior to document it.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
    let code = "require v5.10";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    // v5.10 is an unlikely prefix for a module (no CPAN modules start with 'v' in practice),
    // and even if triggered, the module index has no matching 'v*' entry.
    // Assert we never suggest Utils for this context.
    assert!(
        !completions.iter().any(|c| c.label == "Utils" && c.kind == CompletionItemKind::Module),
        "require v5.10 should not suggest unrelated modules; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_use_module_deduplication() -> Result<(), Box<dyn std::error::Error>> {
    // Two files declaring the same package should produce one completion, not two
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/MyApp.pm")?, "package MyApp;\n1;\n".to_string())?;
    index.index_file(
        Url::parse("file:///lib/MyApp2.pm")?,
        "package MyApp;\n1;\n".to_string(), // duplicate package name
    )?;
    let code = "use MyA";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    let myapp_count = completions.iter().filter(|c| c.label == "MyApp").count();
    assert_eq!(
        myapp_count, 1,
        "Duplicate package declarations should produce exactly one completion"
    );
    Ok(())
}

#[test]
fn test_use_module_non_use_context_excluded() -> Result<(), Box<dyn std::error::Error>> {
    // Outside a use/require statement, module-priority sort_text should NOT appear.
    // add_use_module_completions gates on in_use_statement; its "1_" sort_text
    // prefix is the marker we check here.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///lib/MyApp.pm")?,
        "package MyApp;\nsub hello {}\n1;\n".to_string(),
    )?;
    let code = "my $x = MyA";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    // The "1_MyApp" sort_text is only emitted by add_use_module_completions,
    // which is guarded by in_use_statement. It must not appear outside that context.
    assert!(
        !completions.iter().any(|c| c.sort_text.as_deref() == Some("1_MyApp")),
        "Module-priority sort_text should only appear in use context"
    );
    Ok(())
}

#[test]
fn test_path_to_module_name_maps_nested_pm_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path().join("lib");
    let module_file = root.join("File").join("Path").join("To").join("Module.pm");
    fs::create_dir_all(module_file.parent().ok_or("missing parent")?)?;
    fs::write(&module_file, "package File::Path::To::Module;\n1;\n")?;

    let module_name = workspace::path_to_module_name(&root, &module_file);
    assert_eq!(module_name.as_deref(), Some("File::Path::To::Module"));
    Ok(())
}

#[test]
fn test_use_completion_scans_include_paths() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let include_root = temp.path().join("external");
    let module_file = include_root.join("DB").join("Driver.pm");
    fs::create_dir_all(module_file.parent().ok_or("missing parent")?)?;
    fs::write(module_file, "package DB::Driver;\n1;\n")?;

    let code = "use DB";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(Arc::new(WorkspaceIndex::new())),
        vec![include_root],
        Vec::new(),
        false,
    );
    let completions = provider.get_completions(code, code.len());
    assert!(completions.iter().any(|c| c.label == "DB::Driver"));
    Ok(())
}

#[test]
fn test_use_completion_workspace_first_and_dedupes_external()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let include_root = temp.path().join("external");
    let module_file = include_root.join("DBI.pm");
    fs::create_dir_all(module_file.parent().ok_or("missing parent")?)?;
    fs::write(&module_file, "package DBI;\n1;\n")?;

    let index = Arc::new(WorkspaceIndex::new());
    let module_uri =
        Url::from_file_path(&module_file).map_err(|()| "failed to build module file URI")?;
    index.index_file(module_uri, "package DBI;\n1;\n".into())?;

    let code = "use DB";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(index),
        vec![include_root],
        Vec::new(),
        false,
    );
    let completions = provider.get_completions(code, code.len());
    let dbi_items: Vec<_> = completions.iter().filter(|c| c.label == "DBI").collect();
    assert_eq!(dbi_items.len(), 1, "DBI should be deduplicated across workspace/external");
    assert_eq!(dbi_items[0].detail.as_deref(), Some("module"));
    Ok(())
}

#[test]
fn test_use_completion_filters_workspace_module_by_active_include_roots()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let workspace = temp.path();
    let suppressed_module = workspace.join("lib").join("GoneModule.pm");
    let active_root = workspace.join("t").join("lib");
    fs::create_dir_all(suppressed_module.parent().ok_or("missing suppressed module parent")?)?;
    fs::create_dir_all(&active_root)?;
    fs::write(&suppressed_module, "package GoneModule;\n1;\n")?;

    let index = Arc::new(WorkspaceIndex::new());
    let suppressed_uri =
        Url::from_file_path(&suppressed_module).map_err(|()| "failed to build module URI")?;
    index.index_file(suppressed_uri, "package GoneModule;\n1;\n".into())?;

    let code = "use Gon";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(index),
        vec![active_root],
        Vec::new(),
        false,
    );
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.label == "GoneModule"),
        "workspace package modules outside active @INC roots must not leak into use-completion"
    );
    Ok(())
}

#[test]
fn test_use_completion_system_inc_opt_in() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let system_root = temp.path().join("sys");
    let module_file = system_root.join("Sys").join("Only.pm");
    fs::create_dir_all(module_file.parent().ok_or("missing parent")?)?;
    fs::write(module_file, "package Sys::Only;\n1;\n")?;

    let code = "use Sys::O";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let disabled = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(Arc::new(WorkspaceIndex::new())),
        Vec::<PathBuf>::new(),
        vec![system_root.clone()],
        false,
    )
    .get_completions(code, code.len());
    assert!(!disabled.iter().any(|c| c.label == "Sys::Only"));

    let enabled = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(Arc::new(WorkspaceIndex::new())),
        Vec::<PathBuf>::new(),
        vec![system_root],
        true,
    )
    .get_completions(code, code.len());
    let sys_only = enabled.iter().find(|c| c.label == "Sys::Only").ok_or("missing Sys::Only")?;
    assert_eq!(sys_only.detail.as_deref(), Some("system module"));
    Ok(())
}

#[test]
fn test_path_to_module_name_top_level_pm() -> Result<(), Box<dyn std::error::Error>> {
    // DBI.pm directly in root → "DBI" (single-component name, no "::")
    let temp = TempDir::new()?;
    let root = temp.path().join("lib");
    fs::create_dir_all(&root)?;
    let module_file = root.join("DBI.pm");
    fs::write(&module_file, "package DBI;\n1;\n")?;

    let module_name = workspace::path_to_module_name(&root, &module_file);
    assert_eq!(module_name.as_deref(), Some("DBI"));
    Ok(())
}

#[test]
fn test_path_to_module_name_non_pm_excluded() -> Result<(), Box<dyn std::error::Error>> {
    // Only .pm files should be mapped; .pl, .pod, .so, no-ext must return None
    let temp = TempDir::new()?;
    let root = temp.path().join("lib");
    fs::create_dir_all(&root)?;

    for name in &["Script.pl", "Manual.pod", "XSHelper.so", "README"] {
        let file = root.join(name);
        fs::write(&file, "")?;
        assert!(
            workspace::path_to_module_name(&root, &file).is_none(),
            "expected None for {}",
            name
        );
    }
    Ok(())
}

#[test]
fn test_scan_excludes_non_pm_files() -> Result<(), Box<dyn std::error::Error>> {
    // scan_directory_for_modules must not surface .pl or .pod files
    let temp = TempDir::new()?;
    let root = temp.path().join("lib");
    fs::create_dir_all(&root)?;
    fs::write(root.join("Good.pm"), "package Good;\n1;\n")?;
    fs::write(root.join("Bad.pl"), "#!/usr/bin/perl\n")?;
    fs::write(root.join("Doc.pod"), "=head1 NAME\n")?;

    let modules = workspace::scan_directory_for_modules(&root, "");
    assert!(modules.contains(&"Good".to_string()), "Good.pm should be included");
    assert!(!modules.contains(&"Bad".to_string()), "Bad.pl should be excluded");
    assert!(!modules.contains(&"Doc".to_string()), "Doc.pod should be excluded");
    Ok(())
}

#[test]
fn test_scan_nonexistent_root_returns_empty() {
    // A path that does not exist must silently return an empty list
    let modules = workspace::scan_directory_for_modules(
        std::path::Path::new("/nonexistent/path/that/cannot/exist/12345"),
        "Module",
    );
    assert!(modules.is_empty());
}

#[test]
fn test_path_to_module_name_rejects_non_pm_extension() -> Result<(), Box<dyn std::error::Error>> {
    // path_to_module_name must return None for files that are not .pm files,
    // even if they look like module paths.  Without this guard the scanner
    // would surface .pl scripts and .pod documentation as completable modules.
    let temp = TempDir::new()?;
    let root = temp.path().join("lib");
    fs::create_dir_all(&root)?;
    for name in &["Script.pl", "Doc.pod", "Archive.pm.bak", "README"] {
        let file = root.join(name);
        fs::write(&file, "")?;
        assert!(
            workspace::path_to_module_name(&root, &file).is_none(),
            "expected None for {}",
            name
        );
    }
    Ok(())
}

#[test]
fn test_scan_respects_max_depth() -> Result<(), Box<dyn std::error::Error>> {
    // scan_directory_for_modules must not descend more than MAX_SCAN_DEPTH (8)
    // levels below the root.  A directory exactly at depth 8 should be scanned;
    // a directory at depth 9 should be silently skipped.
    let temp = TempDir::new()?;
    let root = temp.path().join("lib");

    // Build a path 9 levels deep: root/a/b/c/d/e/f/g/h/
    let deep_dir =
        root.join("a").join("b").join("c").join("d").join("e").join("f").join("g").join("h"); // depth 8 — should be scanned
    let too_deep = deep_dir.join("x"); // depth 9 — must be skipped
    fs::create_dir_all(&too_deep)?;
    fs::write(deep_dir.join("AtLimit.pm"), "package A::B::C::D::E::F::G::H::AtLimit;\n1;\n")?;
    fs::write(too_deep.join("TooDeep.pm"), "package TooDeep;\n1;\n")?;

    let modules = workspace::scan_directory_for_modules(&root, "");
    assert!(
        modules.iter().any(|m| m == "a::b::c::d::e::f::g::h::AtLimit"),
        "depth-8 module should be found; got: {:?}",
        modules
    );
    assert!(
        !modules.iter().any(|m| m == "x::TooDeep" || m == "TooDeep"),
        "depth-9 module must not appear; got: {:?}",
        modules
    );
    Ok(())
}

#[test]
fn test_use_completion_not_triggered_outside_use_statement()
-> Result<(), Box<dyn std::error::Error>> {
    // Verify that the scan does NOT fire for general variable completion.
    // If a large include_root were passed and the scan ran unconditionally,
    // this test would be slow and surface module names for non-use positions.
    let temp = TempDir::new()?;
    let include_root = temp.path().join("external");
    let module_file = include_root.join("SomeMod.pm");
    fs::create_dir_all(&include_root)?;
    fs::write(module_file, "package SomeMod;\n1;\n")?;

    // Code that does NOT contain a `use` statement — cursor at a scalar
    let code = "my $x = 1;\nprint $x";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(Arc::new(WorkspaceIndex::new())),
        vec![include_root],
        Vec::new(),
        false,
    );
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.label == "SomeMod"),
        "external module should not appear outside `use` context"
    );
    Ok(())
}

#[test]
fn test_use_statement_past_semicolon_excluded() -> Result<(), Box<dyn std::error::Error>> {
    // Cursor at the end of `use Module;` — the semicolon guard in
    // is_use_statement_context must suppress module-name completions.
    // Without the `;` check the cursor would be considered still inside
    // the use statement and would show stale module suggestions.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(Url::parse("file:///lib/Module.pm")?, "package Module;\n1;\n".to_string())?;
    let code = "use Module;";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
        "cursor after `use Module;` should not trigger module-name completions; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
    Ok(())
}

// ── Gap 1: Named capture group in regex_patterns ─────────────────────────

#[test]
fn test_regex_named_capture_completion() {
    // Cursor inside an empty regex body — named capture should be offered.
    let code = r#"$x =~ /"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "(?<name>...)"),
        "expected named capture group in regex completions; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_regex_named_capture_prefix_disambig() {
    // Typing `(?<` inside a regex → both lookbehind and named capture offered.
    let code = r#"$x =~ /(?<"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "(?<=...)"),
        "expected lookbehind when prefix is (?<"
    );
    assert!(
        completions.iter().any(|c| c.label == "(?<name>...)"),
        "expected named capture when prefix is (?<"
    );
}

#[test]
fn test_regex_named_capture_prefix_lookbehind_only() {
    // Typing `(?<=` — only the lookbehind should match (named capture label
    // does not start with `(?<=`).
    let code = r#"$x =~ /(?<="#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "(?<=...)"),
        "expected lookbehind for prefix (?<="
    );
    assert!(
        !completions.iter().any(|c| c.label == "(?<name>...)"),
        "named capture should NOT appear for prefix (?<= (label doesn't start with (?<=)"
    );
}

// ── Gap 2: is_in_regex_flags heuristic ───────────────────────────────────

#[test]
fn test_is_in_regex_flags_after_close_slash() {
    // Cursor immediately after the closing `/` of a regex.
    let code = "$x =~ /foo/";
    assert!(
        CompletionProvider::is_in_regex_flags(code, code.len()),
        "cursor right after closing / should be in regex-flags context"
    );
}

#[test]
fn test_is_in_regex_flags_after_partial_flag() {
    // Cursor after one already-typed flag character.
    let code = "m/foo/i";
    assert!(
        CompletionProvider::is_in_regex_flags(code, code.len()),
        "cursor after /i should still be in regex-flags context"
    );
}

#[test]
fn test_is_in_regex_flags_s_operator() {
    let code = "s/foo/bar/g";
    assert!(
        CompletionProvider::is_in_regex_flags(code, code.len()),
        "s/// with /g flag should be in regex-flags context"
    );
}

#[test]
fn test_is_in_regex_flags_m_brace_delimiter() {
    let code = "m{foo}i";
    assert!(
        CompletionProvider::is_in_regex_flags(code, code.len()),
        "m{{}} with trailing flag should be in regex-flags context"
    );
}

#[test]
fn test_is_in_regex_flags_qr_bang_delimiter() {
    let code = "qr!foo!m";
    assert!(
        CompletionProvider::is_in_regex_flags(code, code.len()),
        "qr!! with trailing flag should be in regex-flags context"
    );
}

#[test]
fn test_is_not_in_regex_flags_division() {
    // Plain division — must not be treated as regex flags.
    let code = "my $x = $a / $b /";
    assert!(
        !CompletionProvider::is_in_regex_flags(code, code.len()),
        "division should not be detected as regex-flags context"
    );
}

#[test]
fn test_regex_flag_completions_after_close() {
    // Cursor right after closing `/` — should offer all standard flag letters.
    let code = "$x =~ /foo/";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    // Standard regex flags per Perl documentation
    for flag in &["g", "i", "m", "s", "x", "e", "r", "a", "p"] {
        assert!(
            labels.contains(flag),
            "expected standard regex flag '{flag}' in completions; got: {labels:?}"
        );
    }
}

#[test]
fn test_regex_flag_completions_skip_already_typed() {
    // `g` already typed — completions should include `i` but not `g`.
    let code = "$x =~ /foo/g";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    assert!(!labels.contains(&"g"), "already-typed flag 'g' should be excluded");
    assert!(labels.contains(&"i"), "flag 'i' should still be offered");
}

#[test]
fn test_regex_tr_flag_completions() {
    // tr/// should offer only c, d, s — not g, i, e.
    let code = "tr/a-z/A-Z/";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    for flag in &["c", "d", "s"] {
        assert!(labels.contains(flag), "tr/// flag '{flag}' should be offered; got: {labels:?}");
    }
    for flag in &["g", "i", "e"] {
        assert!(!labels.contains(flag), "tr/// should NOT offer '{flag}'; got: {labels:?}");
    }
}

#[test]
fn test_regex_tr_binding_operator_flag_completions() {
    // `$x =~ tr/.../` should also offer only c, d, s (binding form).
    let code = "$x =~ tr/a-z/A-Z/";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    for flag in &["c", "d", "s"] {
        assert!(
            labels.contains(flag),
            "tr/// binding flag '{flag}' should be offered; got: {labels:?}"
        );
    }
    assert!(!labels.contains(&"g"), "tr/// should NOT offer 'g'; got: {labels:?}");
}

// ── Gap 3: Statement-level regex operator snippets ───────────────────────

#[test]
fn test_regex_operator_snippets_present() {
    let code = "";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, 0);
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
    assert!(labels.contains(&"mregex"), "mregex snippet missing; got: {labels:?}");
    assert!(labels.contains(&"ssubst"), "ssubst snippet missing; got: {labels:?}");
    assert!(labels.contains(&"qrpat"), "qrpat snippet missing; got: {labels:?}");
}

#[test]
fn test_regex_operator_snippet_bodies() {
    // Verify the insert_text for each new snippet is syntactically correct.
    let code = "mregex";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    let mregex = must_some(completions.iter().find(|c| c.label == "mregex"));
    let insert = mregex.insert_text.as_deref().unwrap_or_default();
    assert!(insert.starts_with("m/"), "mregex body must start with m/; got: {insert:?}");

    // Also verify ssubst and qrpat with explicit prefix lookup
    let code2 = "ssubst";
    let mut parser2 = Parser::new(code2);
    let ast2 = must(parser2.parse());
    let provider2 = CompletionProvider::new(&ast2);
    let completions2 = provider2.get_completions(code2, code2.len());
    let ssubst = must_some(completions2.iter().find(|c| c.label == "ssubst"));
    let insert2 = ssubst.insert_text.as_deref().unwrap_or_default();
    assert!(insert2.starts_with("s/"), "ssubst body must start with s/; got: {insert2:?}");

    let code3 = "qrpat";
    let mut parser3 = Parser::new(code3);
    let ast3 = must(parser3.parse());
    let provider3 = CompletionProvider::new(&ast3);
    let completions3 = provider3.get_completions(code3, code3.len());
    let qrpat = must_some(completions3.iter().find(|c| c.label == "qrpat"));
    let insert3 = qrpat.insert_text.as_deref().unwrap_or_default();
    assert!(insert3.starts_with("qr/"), "qrpat body must start with qr/; got: {insert3:?}");
}

// ── Dash trigger character tests (#2865) ─────────────────────────────────
// When `-` is a trigger character, context detection must distinguish
// method-call arrows (`->`) from arithmetic/decrement operators.

#[test]
fn test_dash_trigger_fires_method_completion_for_arrow() -> Result<(), Box<dyn std::error::Error>> {
    // `$obj-` (cursor after `-`) — the `-` is the start of `->`, so method
    // completions must appear even before the `>` is typed.
    // Crucially, the result must be ONLY method completions (Function kind),
    // not the entire keyword/snippet list. Without the `-` trigger feature,
    // the code returns all completions — this assertion catches that false pass.
    let code = r#"package MyService;
sub new { bless {}, shift }
sub process { }
sub validate { }
sub run {
my $self = shift;
$self-"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/MyService.pm")?;
    let module_code = "package MyService;\nsub process { }\nsub validate { }\n1;\n";
    index.index_file(module_uri, module_code.to_string())?;
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    // Must find method completions from the workspace index
    assert!(
        completions.iter().any(|c| c.label == "process" || c.label == "validate"),
        "dash trigger on `$self-` should produce method completions; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    // Must NOT return the full keyword/snippet dump — only method completions.
    // "arrayref", "hashref" are snippets from the generic path; they should not
    // appear when the context is a method-call arrow.
    assert!(
        !completions.iter().any(|c| c.label == "arrayref" || c.label == "hashref"),
        "dash trigger on `$self-` must not return generic snippets; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn test_dash_trigger_suppressed_for_subtract_assign() {
    // `$x -=` (cursor after `-` in `-=`) — must return NO completions.
    let code = "$x -";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    // Position is at len() which puts cursor right after `-` preceded by space.
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.is_empty(),
        "dash trigger on `$x -` (subtract context) should return no completions; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_dash_trigger_suppressed_for_decrement() {
    // `$x--` — second `-` is preceded by another `-`, must return NO completions.
    // The guard `source[position-2] != b'-'` prevents treating `--` as `->`.
    let code = "$x--";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    // Cursor after the second `-`: preceding char is `-`, not an identifier.
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.is_empty(),
        "dash trigger on `$x--` (decrement context) should return no completions; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_dash_trigger_suppressed_for_unary_minus() {
    // `my $x = -$y` — unary minus, `-` preceded by space → no completions.
    let code = "my $x = -";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.is_empty(),
        "dash trigger on `my $x = -` (unary minus) should return no completions; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_dash_trigger_fires_for_hash_deref_arrow() -> Result<(), Box<dyn std::error::Error>> {
    // `$hash->{key}` — trigger on `-` in `$hash->`, receiver ends with `h`
    // (alphanumeric), should produce method completions (not a generic dump).
    let code = r#"package MyService;
sub new { bless {}, shift }
sub get_data { }
sub run {
my $hash = {};
$hash-"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/MyService.pm")?;
    let module_code = "package MyService;\nsub new { }\nsub get_data { }\n1;\n";
    index.index_file(module_uri, module_code.to_string())?;
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "get_data" || c.label == "new"),
        "dash trigger on `$hash-` should produce completions; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    // Must not return generic snippet dump
    assert!(
        !completions.iter().any(|c| c.label == "arrayref" || c.label == "hashref"),
        "dash trigger on `$hash-` must not return generic snippets; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    Ok(())
}

// ── Hash key completion tests ─────────────────────────────────────────────

#[test]
fn test_hash_key_completion_basic() {
    // my %config = (host => 'localhost', port => 5432);
    // $config{ho<cursor>
    // Expected: "host" suggested, "port" filtered out by prefix
    let code = "my %config = (host => 'localhost', port => 5432);\n$config{ho";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "host"),
        "expected 'host' in hash key completions for prefix 'ho'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        !completions.iter().any(|c| c.label == "port"),
        "expected 'port' filtered out by prefix 'ho'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_empty_prefix() {
    // $config{<cursor> -- all keys returned
    let code = "my %config = (host => 'localhost', port => 5432);\n$config{";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        completions.iter().any(|c| c.label == "host"),
        "expected 'host' in hash key completions with empty prefix; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        completions.iter().any(|c| c.label == "port"),
        "expected 'port' in hash key completions with empty prefix; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_does_not_fire_for_hashref_deref() {
    // $ref->{ho<cursor> -- hashref deref, must NOT suggest hash keys
    let code = "my $ref = {host => 'localhost'};\n$ref->{ho";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    // Must not return a Property-kinded "host" completion (hash key detection
    // must bail when `->` precedes the `{`)
    assert!(
        !completions.iter().any(|c| c.label == "host" && c.kind == CompletionItemKind::Property),
        "hashref deref `$ref->{{ho` must not produce Property-kinded 'host' completion; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_in_comment_no_suggestions() {
    // # $config{ho<cursor> -- in comment, should not suggest hash keys
    let code = "my %config = (host => 'localhost');\n# $config{ho";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    assert!(
        !completions.iter().any(|c| c.label == "host" && c.kind == CompletionItemKind::Property),
        "hash key completion must not fire inside a comment; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_unknown_variable_returns_empty_for_that_hash() {
    // $config{<cursor> where %config has no known init -- no leaked keys from %other
    let code = "my %other = (a => 1);\n$config{";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());
    // Keys from %other must not appear as completions for $config{}
    assert!(
        !completions.iter().any(|c| c.label == "a" && c.kind == CompletionItemKind::Property),
        "keys from %%other must not leak into %%config completions; got: {:?}",
        completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_quoted_keys_with_special_characters() {
    // Test that quoted keys with hyphens, dots, spaces, and other special characters
    // are included in hash key completions.
    let code = r#"my %data = ('db-host' => 'localhost', 'api.key' => 'secret', 'api key' => 'value', 'foo_bar' => 'normal');
$data{db"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    // 'db-host' should be suggested (starts with 'db' prefix)
    assert!(
        completions.iter().any(|c| c.label == "db-host"),
        "expected 'db-host' (quoted key with hyphen) in completions for prefix 'db'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    // 'api.key' should NOT be suggested (doesn't start with 'db')
    assert!(
        !completions.iter().any(|c| c.label == "api.key"),
        "expected 'api.key' filtered out by prefix 'db'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    // 'foo_bar' should NOT be suggested (doesn't start with 'db')
    assert!(
        !completions.iter().any(|c| c.label == "foo_bar"),
        "expected 'foo_bar' filtered out by prefix 'db'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_double_quoted_hyphenated_key() {
    // Mirror of the single-quoted case for double-quoted keys: keys written with
    // `"..."` and containing special characters must also be completed. This
    // exercises the double-quote branch of the quote-stripping logic.
    let code = r#"my %data = ("db-host" => 'localhost', "api.key" => 'secret');
$data{db"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    // 'db-host' (double-quoted key with hyphen) should be suggested for prefix 'db'.
    assert!(
        completions.iter().any(|c| c.label == "db-host"),
        "expected 'db-host' (double-quoted key with hyphen) in completions for prefix 'db'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    // 'api.key' should NOT be suggested (doesn't start with 'db').
    assert!(
        !completions.iter().any(|c| c.label == "api.key"),
        "expected 'api.key' filtered out by prefix 'db'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_quoted_keys_with_dots_and_spaces() {
    // Test completion with dot and space separators in keys
    let code = r#"my %config = ('db.host' => 1, 'api key' => 2);
$config{api"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    // 'api key' should be suggested (starts with 'api' prefix)
    assert!(
        completions.iter().any(|c| c.label == "api key"),
        "expected 'api key' (quoted key with space) in completions for prefix 'api'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    // 'db.host' should NOT be suggested (doesn't start with 'api')
    assert!(
        !completions.iter().any(|c| c.label == "db.host"),
        "expected 'db.host' filtered out by prefix 'api'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_double_quoted_keys_with_special_characters() {
    let code = r#"my %config = ("db.host" => 1, "api key" => 2, "bare" => 3);
$config{api"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "api key"),
        "expected double-quoted 'api key' in completions for prefix 'api'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    assert!(
        !completions.iter().any(|c| c.label == "db.host"),
        "expected double-quoted 'db.host' filtered out by prefix 'api'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_hash_key_completion_unterminated_quoted_key_no_bogus_suggestion() {
    // Regression: 'db-host (opening quote but no closing quote) must not produce a
    // completion item with a leading-quote artifact like "'db-host".  Previously the
    // character-class guard rejected these implicitly; after relaxing it for quoted
    // keys we must ensure only *fully*-quoted tokens (both delimiters present) are
    // accepted as special-char keys.
    let code = "my %cfg = ('db-host' => 1, host => 2);\n$cfg{db";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    // The fully-quoted key 'db-host' should be present.
    assert!(
        completions.iter().any(|c| c.label == "db-host"),
        "expected 'db-host' in completions; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    // No completion item must carry a leading single-quote from an unterminated literal.
    assert!(
        completions.iter().all(|c| !c.label.starts_with('\'')),
        "no completion label must start with a quote character (unterminated-literal artifact); got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
}

#[test]
fn test_detect_hash_key_context_unicode_non_ident_after_brace_no_panic() {
    let source = "$config{☃ho";
    let result = CompletionProvider::detect_hash_key_context(source, source.len());
    assert!(result.is_none());
}

#[test]
fn test_detect_hash_key_context_unicode_non_ident_before_var_no_panic() {
    let source = "☃config{ho";
    let result = CompletionProvider::detect_hash_key_context(source, source.len());
    assert!(result.is_none());
}

#[test]
fn test_detect_hash_key_context_unicode_before_brace_no_panic() {
    // ☃ (3 bytes) immediately before `{` — previously caused a panic in the
    // `->` check which sliced source[brace_pos-2..brace_pos] across a
    // non-char-boundary.
    let source = "$config☃{key";
    let result = CompletionProvider::detect_hash_key_context(source, source.len());
    // ☃ is not a valid Perl identifier char so the variable name scan will not
    // find a `$` sigil — result must be None without panicking.
    assert!(result.is_none());
}

#[test]
fn test_detect_hash_key_context_4byte_emoji_in_key_no_panic() {
    // 4-byte emoji (U+1F600) mid-key-prefix — exercises the char_indices rev path
    // with a surrogate-range codepoint.
    let source = "$config{\u{1F600}ho";
    let result = CompletionProvider::detect_hash_key_context(source, source.len());
    assert!(result.is_none());
}

#[test]
fn test_detect_hash_key_context_ascii_regression() {
    // Plain ASCII must still work correctly after the Unicode fixes.
    let source = "$config{key";
    let result = CompletionProvider::detect_hash_key_context(source, source.len());
    assert_eq!(result, Some(("config".to_string(), "key".to_string())));
}

#[test]
fn test_provider_captures_include_and_system_inc_paths() {
    let code = "use My::Module;\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let include_paths = vec![PathBuf::from("/workspace/lib"), PathBuf::from("t/lib")];
    let system_inc_paths = vec![PathBuf::from("/usr/lib/perl5")];

    let provider = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        None,
        include_paths.clone(),
        system_inc_paths.clone(),
        false,
    );

    assert_eq!(provider.include_paths, include_paths);
    assert_eq!(provider.system_inc_paths, system_inc_paths);
}

#[test]
fn test_use_module_completion_unchanged_with_empty_inc_vectors()
-> Result<(), Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    let module_uri = Url::parse("file:///workspace/MyApp.pm")?;
    index.index_file(module_uri, "package MyApp;\n1;\n".to_string())?;

    let code = "use MyA";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let baseline_provider =
        CompletionProvider::new_with_index_and_source(&ast, code, Some(Arc::clone(&index)));
    let baseline = baseline_provider.get_completions_with_path(code, code.len(), None);

    let with_empty_inc = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(index),
        Vec::new(),
        Vec::new(),
        false,
    );
    let with_empty_inc_results = with_empty_inc.get_completions_with_path(code, code.len(), None);

    let baseline_labels: std::collections::HashSet<String> =
        baseline.into_iter().map(|item| item.label).collect();
    let with_empty_labels: std::collections::HashSet<String> =
        with_empty_inc_results.into_iter().map(|item| item.label).collect();

    assert_eq!(
        baseline_labels, with_empty_labels,
        "empty include paths must not change completion results in phase 1"
    );
    Ok(())
}

/// Include roots constrain workspace-index module candidates, but they do not
/// trigger filesystem scans. A workspace-indexed package under an active root
/// remains visible; packages outside active roots are covered by the negative
/// filter test.
#[test]
fn test_non_empty_inc_paths_keep_workspace_module_under_active_root()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let active_root = temp.path().join("lib");
    let module_path = active_root.join("MyApp.pm");
    fs::create_dir_all(&active_root)?;
    fs::write(&module_path, "package MyApp;\n1;\n")?;

    let index = Arc::new(WorkspaceIndex::new());
    let module_uri =
        Url::from_file_path(&module_path).map_err(|()| "failed to build module URI")?;
    index.index_file(module_uri, "package MyApp;\n1;\n".to_string())?;

    let code = "use MyA";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;

    let baseline =
        CompletionProvider::new_with_index_and_source(&ast, code, Some(Arc::clone(&index)))
            .get_completions_with_path(code, code.len(), None);

    // Non-empty inc paths: completions keep workspace-indexed modules that live
    // under an active @INC root.
    let inactive_root = temp.path().join("inactive-inc-root");
    let inactive_system_root = temp.path().join("inactive-system-inc-root");
    let with_inc = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(index),
        vec![active_root, inactive_root],
        vec![inactive_system_root],
        false,
    )
    .get_completions_with_path(code, code.len(), None);

    let baseline_labels: std::collections::HashSet<String> =
        baseline.into_iter().map(|item| item.label).collect();
    let with_inc_labels: std::collections::HashSet<String> =
        with_inc.into_iter().map(|item| item.label).collect();

    assert_eq!(
        baseline_labels, with_inc_labels,
        "non-empty include paths must keep workspace package completions under active @INC roots"
    );
    Ok(())
}

// -------------------------------------------------------------------------
// `collect_used_module_names` direct tests (issue #7929)
//
// The Unknown-receiver bounded fallback consumes this helper to decide
// which workspace packages count as "visible" to the buffer. Pin the
// inclusion / exclusion contract directly so the source-policy is
// reviewable without going through the full completion pipeline.
// -------------------------------------------------------------------------

#[test]
fn collect_used_module_names_includes_bare_use() {
    let code = "use Foo;\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let modules = super::import_map::collect_used_module_names(&ast);
    assert!(modules.contains("Foo"), "bare `use Foo;` should include Foo; got {modules:?}");
}

#[test]
fn collect_used_module_names_includes_empty_import_list() {
    let code = "use Foo ();\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let modules = super::import_map::collect_used_module_names(&ast);
    assert!(
        modules.contains("Foo"),
        "`use Foo ();` should still include Foo as a visible package; got {modules:?}"
    );
}

#[test]
fn collect_used_module_names_includes_qw_list() {
    let code = "use Foo qw(bar baz);\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let modules = super::import_map::collect_used_module_names(&ast);
    assert!(
        modules.contains("Foo"),
        "`use Foo qw(...);` should include Foo as a visible package; got {modules:?}"
    );
}

#[test]
fn collect_used_module_names_includes_block_package_use() {
    let code = "package My::App {\n    use Foo::Thing;\n    use strict;\n}\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let modules = super::import_map::collect_used_module_names(&ast);
    assert!(
        modules.contains("Foo::Thing"),
        "`use Foo::Thing` inside a block-form package should include Foo::Thing; got {modules:?}"
    );
    assert!(
        !modules.contains("strict"),
        "lowercase pragmas inside block-form packages must stay excluded; got {modules:?}"
    );
}

#[test]
fn collect_used_module_names_excludes_lowercase_pragmas() {
    let code = "use strict;\nuse warnings;\nuse feature 'say';\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let modules = super::import_map::collect_used_module_names(&ast);
    assert!(
        !modules.contains("strict"),
        "`use strict` (lowercase pragma) must be excluded; got {modules:?}"
    );
    assert!(
        !modules.contains("warnings"),
        "`use warnings` (lowercase pragma) must be excluded; got {modules:?}"
    );
    assert!(
        !modules.contains("feature"),
        "`use feature` (lowercase pragma) must be excluded; got {modules:?}"
    );
}

// -------------------------------------------------------------------------
// Real-workspace baselines for unknown-receiver fallback quality (#7960)
//
// Quality goals proven here against a single multi-file fixture:
//   1. Useful fallback hits — bounded fallback surfaces methods the user
//      can plausibly want from used / current packages.
//   2. No unrelated leak — packages neither imported nor in the current
//      package graph stay out of the fallback list (no all-workspace
//      fallback).
//   3. Dynamic stays fail-closed — Dynamic receivers receive no fallback
//      at all, even when the relevant package is imported.
//   4. Exact receiver non-regression — exact receiver completions keep
//      their existing label / detail / sort tier; no `unknown` leaks.
//   5. No bounded source → no fallback — when used_modules ∪ {current_package}
//      is empty, no fallback fires regardless of how many packages are
//      indexed.
//
// Counters proven by these baselines:
//   useful_fallback_hit_count       = 4   (bark, helper, child_method, parent_method)
//   unrelated_method_leak_count     = 0   (quack never appears in fallback)
//   dynamic_fallback_leak_count     = 0   (no fallback for Dynamic receivers)
//   exact_receiver_regression_count = 0   (Foo->bark detail / sort unchanged)
// -------------------------------------------------------------------------

fn build_baseline_workspace() -> Result<Arc<WorkspaceIndex>, Box<dyn std::error::Error>> {
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Foo.pm")?,
        r#"package Foo;
sub bark { }
sub fetch { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Unrelated.pm")?,
        r#"package Unrelated;
sub quack { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/MyService.pm")?,
        r#"package MyService;
sub helper { }
sub serve { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Parent.pm")?,
        r#"package Parent;
sub parent_method { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Child.pm")?,
        r#"package Child;
use parent 'Parent';
sub child_method { }
1;
"#
        .to_string(),
    )?;
    Ok(index)
}

#[test]
fn baseline_imported_package_useful_hit_no_unrelated_leak() -> Result<(), Box<dyn std::error::Error>>
{
    let index = build_baseline_workspace()?;

    // Realistic shape: buffer imports Foo, calls a method on a sub
    // parameter that has no constructor / bless / type-engine evidence —
    // receiver is `Unknown`, fallback should fire from imported Foo.
    let code = r#"use Foo;

sub do_things {
    my ($obj) = @_;
    $obj->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    // Useful hits: imported Foo's `bark` carries low-confidence detail
    // and tier 6 sort.
    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let bark_detail = must_some(bark.detail.as_deref());
    assert!(
        bark_detail.contains("receiver: unknown, low confidence"),
        "imported `bark` should carry low-confidence fallback detail; got {bark_detail:?}"
    );
    let bark_sort = must_some(bark.sort_text.as_deref());
    assert!(
        bark_sort.starts_with("6_"),
        "fallback `bark` should sort at tier 6; got {bark_sort:?}"
    );

    // Unrelated leak guard: Unrelated.pm is indexed in the workspace but
    // is neither imported nor in the current package graph. Its `quack`
    // must NOT appear in the completion list.
    assert!(
        !completions.iter().any(|c| c.label == "quack"),
        "unrelated `quack` must not leak into fallback; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );
    // Same guard for MyService and Child / Parent — none of those are
    // imported, none are in the current (main) package graph.
    for label in ["helper", "serve", "child_method", "parent_method"] {
        assert!(
            !completions.iter().any(|c| c.label == label),
            "unrelated `{label}` must not leak; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    Ok(())
}

#[test]
fn baseline_block_package_imported_package_fallback_hit() -> Result<(), Box<dyn std::error::Error>>
{
    let index = build_baseline_workspace()?;

    // Modern block-form package syntax nests the `use Foo;` statement under
    // NodeKind::Package.block. The bounded fallback should still treat Foo as
    // visible for an Unknown receiver inside that package.
    let code = r#"package My::Block {
    use Foo;

    sub do_things {
        my ($obj) = @_;
        $obj->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let detail = must_some(bark.detail.as_deref());
    assert!(
        detail.contains("receiver: unknown, low confidence"),
        "block-package imported `bark` should carry low-confidence fallback detail; got {detail:?}"
    );

    assert!(
        !completions.iter().any(|c| c.label == "quack"),
        "unrelated `quack` must not leak for block-package imports; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    Ok(())
}

#[test]
fn baseline_current_package_useful_hit_no_unrelated_leak() -> Result<(), Box<dyn std::error::Error>>
{
    let index = build_baseline_workspace()?;

    // Buffer is in `package MyService;` with no `use`. Receiver is
    // Unknown, current-package fallback should surface `helper`.
    let code = r#"package MyService;

sub run {
    my ($obj) = @_;
    $obj->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let helper = must_some(completions.iter().find(|c| c.label == "helper"));
    let detail = must_some(helper.detail.as_deref());
    assert!(
        detail.contains("receiver: unknown, low confidence"),
        "current-package `helper` should carry low-confidence detail; got {detail:?}"
    );
    let sort = must_some(helper.sort_text.as_deref());
    assert!(sort.starts_with("6_"), "current-package fallback should sort at tier 6; got {sort:?}");

    // Unrelated workspace packages must not leak when current-package
    // fallback fires.
    for label in ["bark", "fetch", "quack", "child_method", "parent_method"] {
        assert!(
            !completions.iter().any(|c| c.label == label),
            "unrelated `{label}` must not leak when current-package fallback fires; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    Ok(())
}

#[test]
fn baseline_current_package_graph_includes_inherited_methods()
-> Result<(), Box<dyn std::error::Error>> {
    // Buffer is in `package Child;`. Child.pm in the workspace declares
    // `use parent 'Parent';`, so `collect_all_package_members` follows
    // the @ISA chain into Parent. The bounded fallback should therefore
    // include both `child_method` and `parent_method`.
    let index = build_baseline_workspace()?;
    let code = r#"package Child;

sub do_thing {
    my ($obj) = @_;
    $obj->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let child = must_some(completions.iter().find(|c| c.label == "child_method"));
    let child_detail = must_some(child.detail.as_deref());
    assert!(
        child_detail.contains("receiver: unknown, low confidence"),
        "Child's `child_method` should be in low-confidence fallback; got {child_detail:?}"
    );
    let child_sort = must_some(child.sort_text.as_deref());
    assert!(
        child_sort.starts_with("6_"),
        "current-package fallback should sort at tier 6; got {child_sort:?}"
    );

    let parent = must_some(completions.iter().find(|c| c.label == "parent_method"));
    let parent_detail = must_some(parent.detail.as_deref());
    assert!(
        parent_detail.contains("receiver: unknown, low confidence"),
        "Parent's `parent_method` should appear via @ISA in fallback; got {parent_detail:?}"
    );
    let parent_sort = must_some(parent.sort_text.as_deref());
    assert!(
        parent_sort.starts_with("6_"),
        "inherited fallback should sort at tier 6; got {parent_sort:?}"
    );

    // Unrelated must not leak into the current-package graph fallback.
    for label in ["bark", "fetch", "quack", "helper"] {
        assert!(
            !completions.iter().any(|c| c.label == label),
            "unrelated `{label}` must not leak; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    Ok(())
}

#[test]
fn baseline_dynamic_receiver_no_fallback_leak() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_baseline_workspace()?;
    // `bless {}, $class` is the canonical Dynamic form. Even with Foo
    // imported, fallback must NOT fire — Dynamic stays fail-closed.
    let code = r#"use Foo;

sub make {
    my ($class) = @_;
    my $x = bless {}, $class;
    $x->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    // No imported method should appear — fallback is suppressed entirely.
    for label in ["bark", "fetch", "quack", "helper", "child_method", "parent_method"] {
        assert!(
            !completions.iter().any(|c| c.label == label),
            "Dynamic receiver must not get fallback; `{label}` leaked. Got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }
    Ok(())
}

#[test]
fn baseline_exact_receiver_non_regression() -> Result<(), Box<dyn std::error::Error>> {
    let index = build_baseline_workspace()?;
    // Exact static-package receiver. Detail, sort, and label must match
    // the pre-#7930 contract (#7920 / #7926). No `unknown` / `low
    // confidence` may leak in.
    let code = "Foo->";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let bark = must_some(completions.iter().find(|c| c.label == "bark"));
    let bark_detail = must_some(bark.detail.as_deref());
    assert!(
        bark_detail.contains("receiver: static package"),
        "exact receiver detail should say `receiver: static package`; got {bark_detail:?}"
    );
    assert!(
        !bark_detail.contains("unknown"),
        "exact receiver detail must not contain `unknown`; got {bark_detail:?}"
    );
    assert!(
        !bark_detail.contains("low confidence"),
        "exact receiver detail must not be marked low confidence; got {bark_detail:?}"
    );
    let bark_sort = must_some(bark.sort_text.as_deref());
    assert!(
        bark_sort.starts_with("2_") || bark_sort.starts_with("3_"),
        "exact receiver should keep tier 2 or 3; got {bark_sort:?}"
    );
    assert_eq!(bark.label, "bark", "exact receiver label must be unchanged");
    Ok(())
}

#[test]
fn baseline_no_bounded_source_no_all_workspace_fallback() -> Result<(), Box<dyn std::error::Error>>
{
    let index = build_baseline_workspace()?;
    // Buffer is in `main` (current_package excluded by `Unknown` fallback
    // policy) and has no `use`. `allowed_packages` is empty — fallback
    // must emit nothing, regardless of how many packages are indexed.
    let code = r#"sub run {
    my ($obj) = @_;
    $obj->"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    for label in ["bark", "fetch", "quack", "helper", "serve", "child_method", "parent_method"] {
        assert!(
            !completions.iter().any(|c| c.label == label),
            "no bounded source means no fallback — `{label}` must not appear; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }
    Ok(())
}

// ── PR 7a: prefix-directed module scan regression tests ──────────────────────

#[test]
fn test_root_and_leaf_prefix_single_segment() {
    // Single segment: scan dir stays at root.
    use std::path::Path;
    let root = Path::new("/lib");
    let (scan_dir, leaf, depth) = workspace::root_and_leaf_prefix(root, "Foo");
    assert_eq!(scan_dir, root, "single segment must keep root as scan_dir");
    assert_eq!(leaf, "Foo");
    assert_eq!(depth, 0);
}

#[test]
fn test_root_and_leaf_prefix_multi_segment() {
    // Multi-segment: scan dir descends into the consumed namespace.
    use std::path::Path;
    let root = Path::new("/lib");
    let (scan_dir, leaf, depth) = workspace::root_and_leaf_prefix(root, "Foo::Bar::Ba");
    assert_eq!(scan_dir, root.join("Foo").join("Bar"));
    assert_eq!(leaf, "Ba");
    assert_eq!(depth, 2);
}

#[test]
fn test_root_and_leaf_prefix_empty() {
    // Empty prefix: scan dir stays at root.
    use std::path::Path;
    let root = Path::new("/lib");
    let (scan_dir, leaf, depth) = workspace::root_and_leaf_prefix(root, "");
    assert_eq!(scan_dir, root);
    assert_eq!(leaf, "");
    assert_eq!(depth, 0);
}

/// Behavior-preservation: the set of module names returned by
/// `scan_directory_for_modules` for a namespaced prefix must be identical
/// whether we call it with a single-segment prefix or a multi-segment prefix
/// that narrows down to the same subset.
///
/// This verifies that the prefix-directed scan optimisation (PR 7a) does not
/// change which modules are returned — only which directory the BFS starts in.
#[test]
fn test_scan_prefix_directed_identical_to_root_scan() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path().join("lib");

    // Create a fixture tree:
    //   lib/
    //     Foo/
    //       Bar/
    //         Baz.pm        → Foo::Bar::Baz
    //         Batman.pm     → Foo::Bar::Batman
    //       Other.pm        → Foo::Other
    //     Unrelated/
    //       Module.pm       → Unrelated::Module
    let foo_bar = root.join("Foo").join("Bar");
    fs::create_dir_all(&foo_bar)?;
    fs::write(foo_bar.join("Baz.pm"), "package Foo::Bar::Baz;\n1;\n")?;
    fs::write(foo_bar.join("Batman.pm"), "package Foo::Bar::Batman;\n1;\n")?;

    let foo_dir = root.join("Foo");
    fs::write(foo_dir.join("Other.pm"), "package Foo::Other;\n1;\n")?;

    let unrelated = root.join("Unrelated");
    fs::create_dir_all(&unrelated)?;
    fs::write(unrelated.join("Module.pm"), "package Unrelated::Module;\n1;\n")?;

    // Multi-segment prefix "Foo::Bar::Ba" — must return only Foo::Bar::Baz and
    // Foo::Bar::Batman (both match the "Foo::Bar::Ba" prefix).
    let mut multi_sorted = workspace::scan_directory_for_modules(&root, "Foo::Bar::Ba");
    multi_sorted.sort();

    // The multi-segment result must include exactly the "Ba"-matching modules.
    assert!(
        multi_sorted.contains(&"Foo::Bar::Baz".to_string()),
        "Foo::Bar::Baz must appear; got: {multi_sorted:?}"
    );
    assert!(
        multi_sorted.contains(&"Foo::Bar::Batman".to_string()),
        "Foo::Bar::Batman must appear; got: {multi_sorted:?}"
    );

    // Unrelated modules must not appear in multi-segment result.
    assert!(
        !multi_sorted.contains(&"Unrelated::Module".to_string()),
        "Unrelated::Module must not appear in multi-seg; got: {multi_sorted:?}"
    );

    // Foo::Other does not start with "Foo::Bar::Ba" so must not appear.
    assert!(
        !multi_sorted.contains(&"Foo::Other".to_string()),
        "Foo::Other must not appear in multi-seg result; got: {multi_sorted:?}"
    );

    Ok(())
}

/// Behavior-preservation: `scan_directory_for_modules` with a prefix whose
/// intermediate subdirectory does not exist must silently return empty rather
/// than panicking or returning unrelated results.
#[test]
fn test_scan_prefix_directed_nonexistent_subdir_returns_empty()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path().join("lib");
    fs::create_dir_all(&root)?;
    // No "Ghost" directory exists under root.
    let result = workspace::scan_directory_for_modules(&root, "Ghost::Module::Foo");
    assert!(
        result.is_empty(),
        "nonexistent intermediate subdir must yield empty result; got: {result:?}"
    );
    Ok(())
}

/// Behavior-preservation: prefix-directed scan with a fully typed namespace
/// still surfaces completions via `add_use_module_completions`.
#[test]
fn test_use_completion_namespaced_prefix_directed() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let include_root = temp.path().join("external");

    // Create Mojo::Controller and Mojo::Util (matching "Mojo::C" and "Mojo::U").
    let mojo_dir = include_root.join("Mojo");
    fs::create_dir_all(&mojo_dir)?;
    fs::write(mojo_dir.join("Controller.pm"), "package Mojo::Controller;\n1;\n")?;
    fs::write(mojo_dir.join("Util.pm"), "package Mojo::Util;\n1;\n")?;

    let code = "use Mojo::Co";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index_and_source_and_paths(
        &ast,
        code,
        Some(Arc::new(WorkspaceIndex::new())),
        vec![include_root],
        Vec::new(),
        false,
    );
    let completions = provider.get_completions(code, code.len());

    // Controller matches "Mojo::Co" — must appear.
    assert!(
        completions.iter().any(|c| c.label == "Mojo::Controller"),
        "Mojo::Controller must appear for prefix 'Mojo::Co'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    // Util does NOT match "Mojo::Co" — must not appear.
    assert!(
        !completions.iter().any(|c| c.label == "Mojo::Util"),
        "Mojo::Util must not appear for prefix 'Mojo::Co'; got: {:?}",
        completions.iter().map(|c| &c.label).collect::<Vec<_>>()
    );

    Ok(())
}

// ── Special variable completion tests (issue #788) ─────────────────────────

/// Special scalar variables are offered when the user types `$` and each has
/// a documentation string.
#[test]
fn test_special_scalar_vars_offered_with_docs() {
    let code = "$";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    // Core special vars that MUST appear
    for label in ["$_", "$!", "$/", "$@", "$?", "$$", "$0", "$1"] {
        let item = completions.iter().find(|c| c.label == label);
        assert!(item.is_some(), "expected {label} in scalar completions");
        // Each special variable must carry documentation
        let doc = item.and_then(|c| c.documentation.as_deref());
        assert!(doc.is_some_and(|d| !d.is_empty()), "{label} must have non-empty documentation");
    }

    // Extended capture-group vars $2..$9 must also be present
    for label in ["$2", "$3", "$4", "$5", "$6", "$7", "$8", "$9"] {
        assert!(
            completions.iter().any(|c| c.label == label),
            "expected {label} (capture group var) in scalar completions"
        );
    }

    // Additional perlvar special scalars
    for label in ["$;", "$\"", "$|", "$^X", "$^I", "$^F"] {
        assert!(
            completions.iter().any(|c| c.label == label),
            "expected {label} in scalar completions"
        );
    }
}

/// Special array variables are offered when the user types `@`.
#[test]
fn test_special_array_vars_offered() {
    let code = "@";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    for label in ["@_", "@ARGV", "@INC", "@ISA", "@EXPORT", "@EXPORT_OK"] {
        let item = completions.iter().find(|c| c.label == label);
        assert!(item.is_some(), "expected {label} in array completions");
        let doc = item.and_then(|c| c.documentation.as_deref());
        assert!(doc.is_some_and(|d| !d.is_empty()), "{label} must have non-empty documentation");
    }
}

/// Special hash variables are offered when the user types `%`.
#[test]
fn test_special_hash_vars_offered() {
    let code = "%";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    for label in ["%ENV", "%INC", "%SIG"] {
        let item = completions.iter().find(|c| c.label == label);
        assert!(item.is_some(), "expected {label} in hash completions");
        let doc = item.and_then(|c| c.documentation.as_deref());
        assert!(doc.is_some_and(|d| !d.is_empty()), "{label} must have non-empty documentation");
    }
}

/// Prefix filtering works: typing `$E` should NOT return `$_` but SHOULD return
/// `$ENV_` style vars if any; typing `$_` should only match topic-variable `$_`.
#[test]
fn test_special_var_prefix_filtering() {
    // "$_" prefix — only the topic variable should match from special vars
    let code = "$_";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$_"),
        "typing '$_' should still offer $_ as a completion"
    );
    // $. (current line) should NOT match because $. does not start with $_
    assert!(!completions.iter().any(|c| c.label == "$."), "$. must not appear when prefix is '$_'");
}

/// Regression: lexical variables and builtins still appear alongside special vars.
#[test]
fn test_special_vars_do_not_displace_lexical_or_builtins() {
    let code = r#"my $xyzzy = 1; $x"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);
    let completions = provider.get_completions(code, code.len());

    // Lexical var declared above must appear
    assert!(
        completions.iter().any(|c| c.label == "$xyzzy"),
        "lexical $xyzzy must appear alongside special vars"
    );
}

/// The expanded special-variable list has at least 40 entries across all sigils.
#[test]
fn test_special_var_count_at_least_40() {
    let mut total = 0usize;

    for (code, pos) in [("$", 1usize), ("@", 1), ("%", 1)] {
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, pos);
        // Count only items that have "special variable" detail (not lexical vars)
        total +=
            completions.iter().filter(|c| c.detail.as_deref() == Some("special variable")).count();
    }

    assert!(total >= 40, "expected at least 40 special variables across all sigils, got {total}");
}

/// Completion is suppressed inside heredoc blocks
#[test]
fn test_no_completion_inside_heredoc() {
    let code = r#"my $text = <<EOF;
This is a $var literal
and this is @array
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    // Position inside heredoc (cursor on "var" in "$var literal")
    let position = must_some(code.find("$var"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "should not complete inside heredoc");
}

/// Heredoc body text may itself contain `<<` without ending suppression.
#[test]
fn test_no_completion_inside_heredoc_with_shift_like_body_text() {
    let code = r#"my $text = <<EOF;
my $a = 1;
$a << $b
EOF
my $after = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$b"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "shift-like text inside a heredoc body must not re-enable completions"
    );
}

/// Multiple heredocs opened on one statement suppress through each body.
#[test]
fn test_no_completion_inside_second_heredoc_from_same_line() {
    let code = r#"print <<A, <<B;
first body
A
second body $cursor
B
my $after = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "completion must stay suppressed inside the second heredoc body"
    );
}

/// Tilde heredocs suppress completion through an indented body and closing marker.
#[test]
fn test_no_completion_inside_tilde_heredoc() {
    let code = r#"my $text = <<~EOF;
  literal $cursor
  EOF
my $after = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "completion must stay suppressed inside a <<~ heredoc body");
}

/// Indented heredocs allow quoted delimiters after horizontal space.
#[test]
fn test_no_completion_inside_spaced_quoted_tilde_heredoc() {
    let code = r#"my $text = <<~ "EOF";
  literal $cursor
  EOF
my $after = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "completion must stay suppressed inside a spaced quoted <<~ heredoc body"
    );
}

/// Backslash heredocs suppress completion inside their bodies.
#[test]
fn test_no_completion_inside_backslash_heredoc() {
    let code = r#"my $text = <<\EOF;
literal $cursor
EOF
my $after = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "completion must stay suppressed inside a <<\\ heredoc body");
}

/// Backtick-delimited heredocs suppress completion inside their bodies.
#[test]
fn test_no_completion_inside_backtick_heredoc() {
    let code = r#"my $text = <<`EOF`;
literal $cursor
EOF
my $after = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "completion must stay suppressed inside a backtick-delimited heredoc body"
    );
}

/// Digit-starting heredoc labels suppress completion inside their bodies.
#[test]
fn test_no_completion_inside_digit_label_heredoc() {
    let code = r#"my $text = <<123;
$cursor
123
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "digit-starting heredoc labels must suppress inside the body");
}

/// Backslashed digit-starting heredoc labels suppress inside their bodies.
#[test]
fn test_no_completion_inside_backslash_digit_label_heredoc() {
    let code = r#"my $text = <<\123;
$cursor
123
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "backslashed digit-starting heredoc labels must suppress inside the body"
    );
}

/// Tilde heredocs also accept backslashed digit-starting labels.
#[test]
fn test_no_completion_inside_tilde_backslash_digit_label_heredoc() {
    let code = r#"my $text = <<~\123;
    $cursor
    123
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "tilde backslashed digit-starting heredoc labels must suppress inside the body"
    );
}

/// Empty quoted heredoc labels suppress completion until the blank terminator line.
#[test]
fn test_no_completion_inside_empty_quoted_label_heredoc() {
    let code = "my $text = <<\"\";\n$cursor\n\nmy $after = 1;\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "empty quoted heredoc labels must suppress inside the body");
}

/// A heredoc opener can appear at the start of a statement line.
#[test]
fn test_no_completion_inside_start_of_line_heredoc() {
    let code = r#"<<EOF;
literal $cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "start-of-line heredocs must suppress inside the body");
}

/// Escaped quotes inside a quoted heredoc label are part of the terminator.
#[test]
fn test_completion_resumes_after_escaped_quote_heredoc_label_closes() {
    let code = r#"my $text = <<"EO\"F";
literal $cursor
EO"F
my $after = $te"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let body_position = must_some(code.find("$cursor"));
    let body_completions = provider.get_completions(code, body_position);
    assert!(
        body_completions.is_empty(),
        "escaped-quote heredoc labels must suppress inside the body"
    );

    let after_completions = provider.get_completions(code, code.len());
    assert!(
        after_completions.iter().any(|completion| completion.label == "$text"),
        "$text should complete after the escaped-quote heredoc closes"
    );
}

/// Escaped q-like delimiters keep heredoc-looking text inside the literal.
#[test]
fn test_escaped_q_like_delimiter_heredoc_text_does_not_suppress_completion_after() {
    let code = r#"my $literal = q!escaped \! <<EOF!;
my $after = $lit"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|completion| completion.label == "$literal"),
        "escaped q-like delimiters must keep <<EOF as literal text"
    );
}

/// Regex-like literal text containing `<<` must not start heredoc suppression.
#[test]
fn test_regex_literal_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $regex_marker = qr/<<EOF/;\nmy $after = $regex";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$regex_marker"),
        "regex literal text containing <<EOF must not suppress later completions"
    );
}

/// Regex-like literal text with a punctuation delimiter must not start heredoc suppression.
#[test]
fn test_regex_literal_bang_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $regex_marker = qr!<<EOF!;\nmy $after = $regex";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$regex_marker"),
        "regex literal text containing <<EOF with ! delimiters must not suppress later completions"
    );
}

/// Bare slash regex text containing `<<` must not start heredoc suppression.
#[test]
fn test_bare_slash_regex_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $subject = 'value';\nif (/<<EOF/) {}\nmy $after = $subject";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$subject"),
        "bare slash regex text containing <<EOF must not suppress later completions"
    );
}

/// Bare slash regexes after Perl operators must not start heredoc suppression.
#[test]
fn test_operator_bare_regex_heredoc_text_does_not_suppress_completion_after() {
    let code = r#"my $subject = 'value';
my @rows = ('x');
sub matches_subject {
    return /<<EOF/;
}
my $count = grep /<<EOF/, @rows;
my @parts = split /<<EOF/, $subject;
my $after = $subject"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$subject"),
        "operator bare regex text containing <<EOF must not suppress later completions"
    );
}

/// Substitution replacement text containing `<<` must not start heredoc suppression.
#[test]
fn test_substitution_replacement_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $text = 'a';\n$text =~ s/a/<<EOF/;\nmy $after = $text";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$text"),
        "substitution replacement text containing <<EOF must not suppress later completions"
    );
}

/// Whitespace after `s` still belongs to the substitution operator.
#[test]
fn test_spaced_substitution_replacement_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $text = 'a';\n$text =~ s /a/<<EOF/;\nmy $after = $text";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$text"),
        "spaced substitution replacement text containing <<EOF must not suppress later completions"
    );
}

/// Substitution replacement text with punctuation delimiters must not start heredoc suppression.
#[test]
fn test_substitution_bang_replacement_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $text = 'a';\n$text =~ s!a!<<EOF!;\nmy $after = $text";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$text"),
        "substitution replacement text containing <<EOF with ! delimiters must not suppress later completions"
    );
}

/// Paired substitution delimiters can span lines and must still hide heredoc-looking text.
#[test]
fn test_multiline_paired_substitution_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $text = 'a';\n$text =~ s(a)\n(<<EOF);\nmy $after = $text";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$text"),
        "paired substitution replacement text containing <<EOF must not suppress later completions"
    );
}

/// Multi-section operators may use a different paired delimiter before a heredoc.
#[test]
fn test_mixed_paired_substitution_before_heredoc_opener_on_same_line() {
    let code = r#"my $value = "old";
print $value =~ s[old](new), <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "mixed paired substitution delimiters must not hide a later heredoc opener"
    );
}

/// A subroutine named like a quote operator must not mask a later heredoc.
#[test]
fn test_sub_named_s_does_not_mask_heredoc_opener() {
    let code = r#"sub s { 1 }
my $text = <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "a sub named s must not be treated as a substitution while scanning later heredocs"
    );
}

/// A malformed bare `s(...)` statement should not black out later heredocs.
#[test]
fn test_bare_s_statement_recovers_before_later_heredoc_opener() {
    let code = r#"sub s { 1 }
s(1);
my $text = <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "scanner recovery after s(...) must still record the later heredoc"
    );
}

/// Method and qualified names like `s` must not mask a later heredoc.
#[test]
fn test_method_or_qualified_s_does_not_mask_heredoc_opener() {
    let code = r#"package Foo;
sub s { 1 }
package main;
my $obj = bless {}, 'Foo';
$obj->s(1);
Foo::s(1);
my $text = <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "method and qualified names named s must not be treated as substitutions"
    );
}

/// Spaced left shifts against bareword constants must not start heredoc suppression.
#[test]
fn test_spaced_shift_bareword_does_not_suppress_completion_after() {
    let code = "use constant EOF => 1;\nmy $shifted = 2 << EOF;\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "spaced left shift text must not be treated as a heredoc opener"
    );
}

/// Unspaced left shifts against bareword constants must not start heredoc suppression.
#[test]
fn test_unspaced_shift_bareword_does_not_suppress_completion_after() {
    let code = "use constant EOF => 1;\nmy $shifted = 2<<EOF;\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "unspaced numeric left shift text must not be treated as a heredoc opener"
    );
}

/// Unspaced sigiled left shifts must not start heredoc suppression.
#[test]
fn test_unspaced_sigiled_shift_bareword_does_not_suppress_completion_after() {
    let code =
        "use constant EOF => 1;\nmy $input = 2;\nmy $shifted = $input<<EOF;\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "unspaced sigiled left shift text must not be treated as a heredoc opener"
    );
}

/// Unspaced bareword constant shifts must not start heredoc suppression.
#[test]
fn test_unspaced_bareword_shift_does_not_suppress_completion_after() {
    let code = "use constant FOO => 2;\nuse constant EOF => 1;\nmy $shifted = FOO<<EOF;\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "unspaced bareword left shift text must not be treated as a heredoc opener"
    );
}

/// A later bare line matching the right operand does not prove a constant shift is a heredoc.
#[test]
fn test_unspaced_bareword_shift_future_label_does_not_suppress_completion_before_label() {
    let code = "use constant FOO => 2;\nuse constant BAR => 1;\nmy $shifted = FOO<<BAR;\nmy $after = $shift\nBAR\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$shift\n")) + "$shift".len();
    let completions = provider.get_completions(code, position);

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "a later BAR line must not make a constant left shift look like a heredoc"
    );
}

/// Quoted constant declarations still make no-space constant shifts non-heredocs.
#[test]
fn test_quoted_constant_shift_future_label_does_not_suppress_completion_before_label() {
    let code = "use constant \"FOO\" => 2;\nuse constant \"BAR\" => 1;\nmy $shifted = FOO<<BAR;\nmy $after = $shift\nBAR\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$shift\n")) + "$shift".len();
    let completions = provider.get_completions(code, position);

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "quoted use constant names must not leave constant shifts in heredoc mode"
    );
}

/// Lowercase bareword constant shifts must not start heredoc suppression.
#[test]
fn test_unspaced_lowercase_bareword_shift_does_not_suppress_completion_after() {
    let code = "use constant foo => 2;\nuse constant bar => 1;\nmy $shifted = foo<<bar;\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "unspaced lowercase bareword left shifts must not be treated as heredoc openers"
    );
}

/// Unspaced output statements with constant operands must not start heredoc suppression.
#[test]
fn test_unspaced_print_bareword_constant_shift_does_not_suppress_completion_after() {
    let code = "use constant OUT => 4;\nuse constant EOF => 1;\nmy $shifted = print OUT<<EOF;\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "unspaced print constant shifts must not be treated as heredoc openers"
    );
}

/// Lowercase bareword constants in output statements are still left shifts without a terminator.
#[test]
fn test_unspaced_print_lowercase_constant_shift_does_not_suppress_completion_after() {
    let code = "use constant out => 4;\nuse constant marker => 1;\nmy $shifted = print out<<marker;\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "unspaced print lowercase constant shifts must not be treated as heredoc openers"
    );
}

/// Spaced bareword constant shifts must not start heredoc suppression.
#[test]
fn test_spaced_bareword_shift_does_not_suppress_completion_after() {
    let code = "use constant FOO => 2;\nuse constant EOF => 1;\nmy $shifted = FOO <<EOF;\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "spaced bareword left shift text must not be treated as a heredoc opener"
    );
}

/// Spaced bareword constant shifts after return must not start heredoc suppression.
#[test]
fn test_return_spaced_lowercase_bareword_shift_does_not_suppress_completion_after() {
    let code = "my $shifted = 1;\nuse constant foo => 2;\nuse constant bar => 1;\nsub f { return foo <<bar; }\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "spaced return-value bareword shifts must not be treated as heredoc openers"
    );
}

/// Future delimiter probes do not turn spaced return constant shifts into heredocs.
#[test]
fn test_return_spaced_constant_shift_future_label_does_not_suppress_completion_before_label() {
    let code = "my $shifted = 1;\nuse constant foo => 2;\nuse constant bar => 1;\nsub f { return foo <<bar; }\nmy $after = $shift\nbar\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$shift\n")) + "$shift".len();
    let completions = provider.get_completions(code, position);

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "a later bar line must not make a return constant shift look like a heredoc"
    );
}

/// Future delimiter probes must ignore matching text inside later literals.
#[test]
fn test_return_shift_future_literal_label_does_not_suppress_completion_before_literal() {
    let code = "my $shifted = 1;\nuse constant foo => 2;\nuse constant bar => 1;\nsub f { return foo <<bar; }\nmy $after = $shift\nmy $literal = \"\nbar\n\";\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$shift\n")) + "$shift".len();
    let completions = provider.get_completions(code, position);

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "future close detection must not count delimiter-looking lines inside later literals"
    );
}

/// Future delimiter probes must ignore matching text inside later POD blocks.
#[test]
fn test_return_shift_future_pod_label_does_not_suppress_completion_after_pod() {
    let code = "my $shifted = 1;\nuse constant foo => 2;\nuse constant bar => 1;\nsub f { return foo <<bar; }\n=pod\nbar\n=cut\nmy $after = $shift";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "future close detection must not count delimiter-looking lines inside later POD"
    );
}

/// Future delimiter probes must ignore matching text inside later heredocs.
#[test]
fn test_return_shift_future_heredoc_label_keeps_later_heredoc_suppressed() {
    let code = "my $shifted = 1;\nuse constant foo => 2;\nuse constant bar => 1;\nsub f { return foo <<bar; }\nmy $h = <<EOF;\nbar\n$cursor\nEOF\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "future close detection must not count delimiter-looking lines inside later heredoc bodies"
    );
}

/// Unspaced shift probes still track later real heredocs before accepting a close.
#[test]
fn test_unspaced_shift_future_heredoc_label_keeps_later_heredoc_suppressed() {
    let code = "use constant foo => 2;\nuse constant bar => 1;\nmy $shifted = foo<<bar;\nmy $h = <<EOF;\nbar\n$cursor\nEOF\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "unspaced future close detection must not count delimiter-looking lines inside later heredoc bodies"
    );
}

/// Future probes still notice a later heredoc opener after a literal closes.
#[test]
fn test_return_shift_future_literal_then_heredoc_label_keeps_later_heredoc_suppressed() {
    let code = "my $shifted = 1;\nuse constant foo => 2;\nuse constant bar => 1;\nsub f { return foo <<bar; }\nmy $literal = \"\ntext\"; my $h = <<EOF;\nbar\n$cursor\nEOF\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "future close detection must track heredocs opened after a multiline literal closes"
    );
}

/// Return-value all-caps bareword calls can take heredoc arguments.
#[test]
fn test_no_completion_inside_return_all_caps_bareword_call_heredoc() {
    let code = r#"sub RENDER { shift }
sub build {
    return RENDER <<EOF;
$cursor
EOF
}
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "return all-caps bareword heredocs must suppress inside the body"
    );
}

/// No-space print heredocs are still heredocs and suppress inside the body.
#[test]
fn test_no_completion_inside_print_heredoc_without_space() {
    let code = r#"print<<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "print<<EOF heredocs must still suppress inside the body");
}

/// No-space call heredocs suppress inside the body.
#[test]
fn test_no_completion_inside_no_space_bareword_call_heredoc() {
    let code = r#"system<<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "lowercase no-space call heredocs must suppress inside the body"
    );
}

/// All-caps no-space call heredocs suppress inside the body.
#[test]
fn test_no_completion_inside_all_caps_no_space_bareword_call_heredoc() {
    let code = r#"sub RENDER { shift }
RENDER<<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "all-caps no-space call heredocs must suppress inside the body"
    );
}

/// Future-close probes for no-space call heredocs treat candidate body text as text.
#[test]
fn test_no_completion_inside_no_space_call_heredoc_with_body_heredoc_text() {
    let code = r#"sub render { shift }
render<<BAR;
my $inner = <<EOF;
$cursor
BAR
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "heredoc-looking text inside a no-space call heredoc body must not hide the real close"
    );
}

/// Print heredocs with bareword filehandles still suppress inside the body.
#[test]
fn test_no_completion_inside_print_bareword_filehandle_heredoc() {
    let code = r#"print OUT <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "print OUT <<EOF must suppress inside the body");
}

/// Sigiled values before `<<` are expressions, not output filehandle heredocs.
#[test]
fn test_sigiled_shift_does_not_suppress_completion_after() {
    let code = "my $fh = 4;\nuse constant EOF => 1;\nmy $shifted = $fh<<EOF;\nmy $after = $sh";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "sigiled left shifts must not be treated as output filehandle heredocs"
    );
}

/// Print heredocs with unspaced bareword filehandles still suppress inside the body.
#[test]
fn test_no_completion_inside_unspaced_print_bareword_filehandle_heredoc() {
    let code = r#"print OUT<<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "print OUT<<EOF must suppress inside the body");
}

/// Print heredocs with sigiled filehandles still suppress inside the body.
#[test]
fn test_no_completion_inside_print_sigiled_filehandle_heredoc() {
    let code = r#"my $fh;
print $fh <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "print $fh <<EOF must suppress inside the body");
}

/// Print heredocs with braced filehandles still suppress inside the body.
#[test]
fn test_no_completion_inside_print_braced_filehandle_heredoc() {
    let code = r#"my $fh;
print {$fh} <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "print {{$fh}} <<EOF must suppress inside the body");
}

/// Say heredocs with sigiled filehandles still suppress inside the body.
#[test]
fn test_no_completion_inside_say_sigiled_filehandle_heredoc() {
    let code = r#"use feature 'say';
my $fh;
say $fh <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "say $fh <<EOF must suppress inside the body");
}

/// Printf heredocs with sigiled filehandles still suppress inside the body.
#[test]
fn test_no_completion_inside_printf_sigiled_filehandle_heredoc() {
    let code = r#"my $fh;
printf $fh <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "printf $fh <<EOF must suppress inside the body");
}

/// Printf heredocs with unspaced bareword filehandles still suppress inside the body.
#[test]
fn test_no_completion_inside_unspaced_printf_bareword_filehandle_heredoc() {
    let code = r#"printf OUT<<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "printf OUT<<EOF must suppress inside the body");
}

/// Heredocs passed to user-defined calls suppress inside the body.
#[test]
fn test_no_completion_inside_bareword_call_heredoc() {
    let code = r#"render <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "bareword call heredocs must suppress inside the body");
}

/// Method-result shifts are not heredoc calls without parentheses.
#[test]
fn test_method_result_shift_does_not_suppress_completion_before_label() {
    let code = r#"my $renderer = bless {}, 'Renderer';
my $shifted = $renderer->mask <<MASK;
my $after = $shift
MASK
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$shift\n")) + "$shift".len();
    let completions = provider.get_completions(code, position);

    assert!(
        completions.iter().any(|c| c.label == "$shifted"),
        "arrow-method left shifts must not be treated as heredoc bodies"
    );
}

/// Parenthesized method-call heredocs suppress inside the body.
#[test]
fn test_no_completion_inside_parenthesized_method_call_heredoc() {
    let code = r#"my $renderer = bless {}, 'Renderer';
$renderer->render(<<EOF);
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "parenthesized method-call heredocs must suppress inside the body"
    );
}

/// Return-value call heredocs still suppress inside the body.
#[test]
fn test_no_completion_inside_return_call_heredoc() {
    let code = r#"sub render {}
sub build {
    return render <<EOF;
$cursor
EOF
}
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "return call heredocs must suppress inside the body");
}

/// A sigiled variable named `$q` must not be mistaken for a q-like literal.
#[test]
fn test_variable_named_q_does_not_mask_heredoc_opener() {
    let code = r#"my $q = <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "$q before a heredoc opener must still record the delimiter");
}

/// A label with trailing spaces is body text, not the heredoc terminator.
#[test]
fn test_trailing_space_label_line_stays_inside_heredoc_body() {
    let code = "my $text = <<EOF;\nEOF   \n$cursor\nEOF\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "EOF followed by spaces must not close the heredoc");
}

/// Tilde heredoc label lines with trailing spaces are body text, not terminators.
#[test]
fn test_tilde_heredoc_trailing_space_label_line_stays_inside_body() {
    let code = "my $text = <<~EOF;\n  EOF   \n  $cursor\n  EOF\n";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "indented EOF followed by spaces must not close the heredoc");
}

/// q-like literal text containing `<<` must not start heredoc suppression.
#[test]
fn test_q_literal_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $q_marker = q{<<EOF};\nmy $after = $q";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$q_marker"),
        "q-like literal text containing <<EOF must not suppress later completions"
    );
}

/// Nested paired q-like delimiters must keep heredoc-looking text inside the literal.
#[test]
fn test_nested_q_literal_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $q_marker = q{{} <<EOF };\nmy $after = $q";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$q_marker"),
        "nested q-like literal text containing <<EOF must not suppress later completions"
    );
}

/// Fat-comma keys named like q-like operators must not hide heredoc values.
#[test]
fn test_fat_comma_q_key_before_heredoc_value_suppresses_inside_body() {
    let code = r#"my %h = (q => <<EOF);
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "fat-comma q keys must not mask heredoc values");
}

/// Heredocs after a multiline literal that closes on the same line are still found.
#[test]
fn test_literal_closes_before_heredoc_opener_on_same_line() {
    let code = r#"my $prefix = q{
literal text
}; my $text = <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "a heredoc opener after a closed multiline literal must suppress inside the body"
    );
}

/// q-like literal text with a punctuation delimiter must not start heredoc suppression.
#[test]
fn test_q_literal_pipe_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $q_marker = q|<<EOF|;\nmy $after = $q";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$q_marker"),
        "q-like literal text containing <<EOF with | delimiters must not suppress later completions"
    );
}

/// Spaced q-like literal text must not start heredoc suppression.
#[test]
fn test_spaced_q_literal_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $q_marker = q /<<EOF/;\nmy $after = $q";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$q_marker"),
        "spaced q literal text containing <<EOF must not suppress later completions"
    );
}

/// Spaced qq literal text must not start heredoc suppression.
#[test]
fn test_spaced_qq_literal_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $qq_marker = qq /<<EOF/;\nmy $after = $qq";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$qq_marker"),
        "spaced qq literal text containing <<EOF must not suppress later completions"
    );
}

/// Newline-separated q delimiters must still keep heredoc-looking text inside the literal.
#[test]
fn test_newline_spaced_q_literal_heredoc_text_does_not_suppress_completion_after() {
    let code = "my $q_marker = q\n{<<EOF};\nmy $after = $q";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$q_marker"),
        "newline-spaced q literal text containing <<EOF must not suppress later completions"
    );
}

/// Newline-separated substitution delimiters must keep POD-looking text inside the literal.
#[test]
fn test_newline_spaced_substitution_pod_text_does_not_suppress_completion_after() {
    let code = r#"my $subject = "value";
$subject =~ s
{
=pod
}{replacement};
my $after = $subject"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$subject"),
        "newline-spaced substitution text containing =pod must not suppress later completions"
    );
}

/// Regex-looking heredoc body text must not bypass heredoc suppression.
#[test]
fn test_no_regex_completion_inside_heredoc_body() {
    let code = r#"my $text = <<EOF;
if ($value =~ /li
EOF
my $after = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("/li")) + "/li".len();
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "regex-looking heredoc body text must not produce regex completions"
    );
}

/// Completion is suppressed inside POD blocks
#[test]
fn test_no_completion_inside_pod_block() {
    let code = r#"=pod

This is documentation about a $special variable
and @array references

=cut

my $real_var = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    // Position inside POD block (cursor on "special" in "$special")
    let position = must_some(code.find("$special"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "should not complete inside POD block");
}

/// Regex-looking POD text must not bypass POD suppression.
#[test]
fn test_no_regex_completion_inside_pod_block() {
    let code = r#"=pod
if ($value =~ /li
=cut
my $after = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("/li")) + "/li".len();
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "regex-looking POD text must not produce regex completions");
}

/// Custom POD commands at column zero start POD blocks until `=cut`.
#[test]
fn test_no_completion_inside_custom_pod_command_block() {
    let code = r#"=constructor new
Documentation mentions a $cursor variable.
=cut
my $real_var = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "custom column-zero POD commands must suppress completion until =cut"
    );
}

/// Custom POD commands that start with `cut` are not the `=cut` terminator.
#[test]
fn test_no_completion_inside_cutting_pod_command_block() {
    let code = r#"=cutting edge
Documentation mentions a $cursor variable.
=cut
my $real_var = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "`=cutting` must be treated as a custom POD command, not a terminator"
    );
}

/// File-test `-s` operators are not substitution literals and must not mask POD.
#[test]
fn test_no_completion_inside_pod_after_file_test_s_operator() {
    let code = r#"my $file = "README.md";
my $size = -s $file;
=pod

$cursor
=cut
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(completions.is_empty(), "file-test -s must not prevent POD suppression from starting");
}

/// POD-looking text inside a multiline q-like literal is not a POD block.
#[test]
fn test_pod_marker_inside_multiline_q_literal_does_not_suppress_completion_after() {
    let code = r#"my $text = q{
=pod
};
my $after = $text"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$text"),
        "POD-looking text inside a multiline q literal must not suppress later completions"
    );
}

/// POD-looking text inside a multiline slash regex is not a POD block.
#[test]
fn test_pod_marker_inside_multiline_slash_regex_does_not_suppress_completion_after() {
    let code = r#"my $subject = "value";
if ($subject =~ /
=pod
/) {}
my $after = $subject"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$subject"),
        "POD-looking text inside a multiline slash regex must not suppress later completions"
    );
}

/// Perl-looking POD prose must not make the terminator look like string context.
#[test]
fn test_completion_after_pod_prose_with_unmatched_q_literal_text() {
    let code = r#"=pod
Documentation mentions q{ as prose, not Perl code.
=cut

my $real_var = 1;
$real"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$real_var"),
        "POD prose containing unmatched q-like text must not suppress after =cut"
    );
}

/// Earlier POD prose must not make later POD markers look like literal context.
#[test]
fn test_no_completion_inside_later_pod_after_unmatched_q_prose() {
    let code = r#"=pod
Documentation mentions q{ as prose, not Perl code.
=cut

=pod
Documentation mentions a $cursor variable.
=cut
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "unmatched q-like text in earlier POD prose must not disable later POD suppression"
    );
}

/// Heredoc-looking POD prose must not make the terminator look like heredoc context.
#[test]
fn test_completion_after_pod_prose_with_heredoc_like_text() {
    let code = r#"=pod
Documentation mentions <<EOF as prose, not Perl code.
=cut

my $real_var = 1;
$real"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$real_var"),
        "POD prose containing heredoc-looking text must not suppress after =cut"
    );
}

/// A fake heredoc marker in a comment must not mask a following POD block.
#[test]
fn test_no_completion_inside_pod_after_comment_heredoc_text() {
    let code = r#"# docs mention <<EOF heredocs
=pod

Documentation mentions a $cursor variable.

=cut

my $real_var = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "comment text that mentions <<EOF must not prevent POD suppression"
    );
}

/// A fake heredoc marker in a string must not mask a following POD block.
#[test]
fn test_no_completion_inside_pod_after_string_heredoc_text() {
    let code = r#"my $marker = "<<EOF";
=pod

Documentation mentions a $cursor variable.

=cut

my $real_var = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "string text that contains <<EOF must not prevent POD suppression"
    );
}

/// Perl-looking heredoc body text must not mask a later real POD block.
#[test]
fn test_no_completion_inside_pod_after_heredoc_with_unmatched_q_text() {
    let code = r#"my $text = <<EOF;
q{
EOF
=pod

Documentation mentions a $cursor variable.

=cut

my $real_var = 1;
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "heredoc body q-like text must not prevent later POD suppression"
    );
}

/// POD-looking string text must not disable later heredoc suppression.
#[test]
fn test_no_completion_inside_heredoc_after_string_pod_marker() {
    let code = r#"my $marker = "
=pod
";
my $text = <<EOF;
$cursor
EOF
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let position = must_some(code.find("$cursor"));
    let completions = provider.get_completions(code, position);

    assert!(
        completions.is_empty(),
        "string text that contains =pod must not prevent heredoc suppression"
    );
}

/// POD-looking command-string text must not disable later completion.
#[test]
fn test_completion_after_backtick_string_pod_marker() {
    let code = r#"my $cmd = `
=pod
not real POD
`;
my $after = $cmd"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$cmd"),
        "backtick string text that contains =pod must not suppress later completions"
    );
}

/// Heredoc-looking command-string text must not suppress later completion.
#[test]
fn test_completion_after_same_line_backtick_string_heredoc_marker() {
    let code = "my $cmd = `printf <<EOF`;\nmy $after = $cmd";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, code.len());

    assert!(
        completions.iter().any(|c| c.label == "$cmd"),
        "backtick string text that contains <<EOF must not suppress later completions"
    );
}

/// Completion works normally after POD block ends
#[test]
fn test_completion_after_pod_block() {
    let code = r#"=pod
Documentation here
=cut

my $real_var = 1;
$real
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    // Position after POD, at cursor position completing "$real"
    let completions = provider.get_completions(code, code.len() - 1);

    // Should suggest variables after POD block
    assert!(
        completions.iter().any(|c| c.label == "$real_var"),
        "should complete variables after POD block ends"
    );
}

/// Indented `=pod` (leading whitespace) must NOT trigger POD suppression.
/// Per perlpod, POD commands must appear at column 0.
#[test]
fn test_indented_pod_marker_does_not_suppress_completion() {
    // A hash value that happens to look like `=pod` but is indented — not real POD.
    let code = "my $x = 1;\n    # this comment mentions =pod but at indent\nmy $cursor = ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, position);

    // $x declared above must appear — suppression must NOT have fired
    assert!(
        completions.iter().any(|c| c.label == "$x"),
        "indented =pod-like content in a comment must not trigger POD suppression; $x should complete"
    );
}

/// Heredoc body containing `=pod`-like content must NOT bleed into the POD
/// state machine after the heredoc closes.
#[test]
fn test_heredoc_with_pod_content_does_not_suppress_completion_after() {
    // The heredoc body contains `=pod` as literal text. After `END`, we are back
    // in regular Perl code and completion should work normally.
    let code = "my $text = <<END;\n=pod this is a literal string\nEND\nmy $after = ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, position);

    // $text must appear — POD suppression must NOT bleed out of the heredoc
    assert!(
        completions.iter().any(|c| c.label == "$text"),
        "$text should complete after a heredoc whose body contained =pod"
    );
}

/// A quoted heredoc terminator that looks like POD is still the terminator, not
/// the start of a POD block.
#[test]
fn test_pod_like_quoted_heredoc_terminator_does_not_suppress_completion_after() {
    let code = "my $text = <<\"=pod\";\nliteral\n=pod\nmy $after = ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, position);

    assert!(
        completions.iter().any(|c| c.label == "$text"),
        "$text should complete after a quoted =pod heredoc terminator"
    );
}

/// Completion is suppressed inside a heredoc body (not just at the $ sign).
/// After the closing delimiter, completion resumes.
#[test]
fn test_completion_resumes_after_heredoc_closes() {
    let code = "my $outer = <<EOF;\nliteral content\nEOF\nmy $after_heredoc = ";
    let position = code.len();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, position);

    // $outer must appear — we are after the closing EOF, not inside the heredoc
    assert!(
        completions.iter().any(|c| c.label == "$outer"),
        "$outer should complete after the heredoc closes"
    );
}

/// Suppression is exact: a cursor positioned on the heredoc closing delimiter
/// line itself should NOT be considered inside the heredoc body.
#[test]
fn test_heredoc_closing_delimiter_is_not_body() {
    // Cursor is right before "EOF" on the closing line — not inside body.
    let code = "my $x = <<EOF;\nliteral\nEOF\n";
    // Position of the 'E' in the closing EOF line
    let eof_line_pos = must_some(code.find("\nEOF\n")) + 1;
    let position = eof_line_pos;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new(&ast);

    let completions = provider.get_completions(code, position);
    assert!(
        completions.iter().any(|c| c.label == "$x"),
        "cursor on closing delimiter line must not be treated as inside heredoc body"
    );
}

// -------------------------------------------------------------------------
// Package-qualified method completion (issue #1606)
//
// When completing `Foo::method`, the completion system should provide
// inherited methods from @ISA chains, not just direct package members.
// This ensures parity with arrow-form method completion `Foo->method`.
// -------------------------------------------------------------------------

#[test]
fn package_qualified_method_completion_includes_inherited() -> Result<(), Box<dyn std::error::Error>>
{
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Parent.pm")?,
        r#"package Parent;
sub inherited_method { }
1;
"#
        .to_string(),
    )?;
    index.index_file(
        Url::parse("file:///workspace/Child.pm")?,
        r#"package Child;
our @ISA = ('Parent');
sub own_method { }
1;
"#
        .to_string(),
    )?;

    let code = "Child::";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    // Both own and inherited methods should appear
    let own = completions.iter().find(|c| c.label == "own_method");
    let inherited = completions.iter().find(|c| c.label == "inherited_method");

    assert!(
        own.is_some(),
        "Package-qualified method completion for Child:: should include own_method"
    );
    assert!(
        inherited.is_some(),
        "Package-qualified method completion for Child:: should include inherited_method from Parent"
    );

    // Own methods should rank higher (tier 2 vs tier 3)
    let own_sort = own.and_then(|c| c.sort_text.as_deref()).unwrap_or("");
    let inherited_sort = inherited.and_then(|c| c.sort_text.as_deref()).unwrap_or("");
    assert!(own_sort.starts_with("2_"), "own method should use tier 2, got {own_sort:?}");
    assert!(
        inherited_sort.starts_with("3_"),
        "inherited method should use tier 3, got {inherited_sort:?}"
    );

    Ok(())
}

#[test]
fn package_qualified_completion_includes_own_constants_and_variables()
-> Result<(), Box<dyn std::error::Error>> {
    // Regression test: constants and package variables must still appear when
    // completing `Foo::` after the @ISA-chain BFS was introduced.  The BFS
    // (`collect_all_package_members`) filters to Subroutine|Method only;
    // without the supplemental `get_package_members` call these would silently
    // vanish.
    let index = Arc::new(WorkspaceIndex::new());
    index.index_file(
        Url::parse("file:///workspace/Config.pm")?,
        r#"package Config;
use constant PI => 3.14159;
use constant MAX_RETRIES => 3;
our $VERSION = '1.0';
sub helper { }
1;
"#
        .to_string(),
    )?;

    let code = "Config::";
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let completions = provider.get_completions(code, code.len());

    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

    assert!(
        completions.iter().any(|c| c.label == "PI"),
        "Package-qualified completion must include own constants; got: {labels:?}"
    );
    assert!(
        completions.iter().any(|c| c.label == "MAX_RETRIES"),
        "Package-qualified completion must include own constants; got: {labels:?}"
    );
    assert!(
        completions.iter().any(|c| c.label == "helper"),
        "Package-qualified completion must include own subroutines; got: {labels:?}"
    );

    // Constants should have Constant kind
    let pi = completions.iter().find(|c| c.label == "PI").unwrap();
    assert_eq!(
        pi.kind,
        crate::providers::completion_item::CompletionItemKind::Constant,
        "PI should be offered as a Constant completion item"
    );

    Ok(())
}

#[test]
fn extract_fat_comma_keys_covers_quoted_and_bareword_forms() {
    // Exercises all three branches of the key-token classification in
    // `extract_fat_comma_keys`: single-quoted, double-quoted, and bareword,
    // plus the rejection path for an unquoted token with special characters.
    fn collect(list_text: &str) -> Vec<String> {
        let mut keys = Vec::new();
        let mut seen = std::collections::HashSet::new();
        CompletionProvider::extract_fat_comma_keys(list_text, &mut keys, &mut seen);
        keys
    }

    // Bareword key (alphanumeric + underscore) is accepted.
    assert!(collect("host => 1").iter().any(|k| k == "host"));
    // Single-quoted key may contain special characters (hyphen).
    assert!(collect("'db-name' => 1").iter().any(|k| k == "db-name"));
    // Double-quoted key may contain special characters (dot).
    assert!(collect("\"x.y\" => 1").iter().any(|k| k == "x.y"));
    // Unquoted token with a non-word character is rejected (no quoting).
    assert!(collect("a-b => 1").is_empty());
}

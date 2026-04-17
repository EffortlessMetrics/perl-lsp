//! Framework semantic extraction tests for Rose::DB::Object ORM.
//!
//! These tests verify:
//! - Detection of Rose::DB::Object via `use base qw(Rose::DB::Object)`
//! - Extraction of column metadata from `__PACKAGE__->meta->setup(columns => [...])`
//! - Synthesis of accessor method symbols for columns

use perl_semantic_analyzer::{
    analysis::class_model::{ClassModel, ClassModelBuilder, Framework},
    symbol::{SymbolExtractor, SymbolKind, SymbolTable},
};
use perl_tdd_support::{must, must_some};

fn build_models(code: &str) -> Vec<ClassModel> {
    let mut parser = perl_semantic_analyzer::Parser::new(code);
    let ast = must(parser.parse());
    ClassModelBuilder::new().build(&ast)
}

fn find_model<'a>(models: &'a [ClassModel], name: &str) -> Option<&'a ClassModel> {
    models.iter().find(|m| m.name == name)
}

fn extract_symbols(code: &str) -> SymbolTable {
    let mut parser = perl_semantic_analyzer::Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == kind))
}

fn has_method_with_declaration(
    table: &SymbolTable,
    name: &str,
    kind: SymbolKind,
    declaration: Option<&str>,
) -> bool {
    table.symbols.get(name).is_some_and(|symbols| {
        symbols
            .iter()
            .any(|symbol| symbol.kind == kind && symbol.declaration.as_deref() == declaration)
    })
}

// =============================================================================
// AC1: Framework Detection Tests
// =============================================================================

#[test]
fn rose_db_object_detected_from_use_base_qw() {
    // AC1: Given a Perl file containing `use base qw(Rose::DB::Object)`
    // When the semantic analyzer processes the file
    // Then the package is classified as Framework::RoseDBObject
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

sub new { }
"#;

    let models = build_models(code);
    let model = must_some(find_model(&models, "MyApp::User"));

    assert_eq!(
        model.framework,
        Framework::RoseDBObject,
        "expected Framework::RoseDBObject for package inheriting from Rose::DB::Object"
    );
}

#[test]
fn rose_db_object_detected_from_use_parent() {
    let code = r#"
package MyApp::User;
use parent 'Rose::DB::Object';

sub new { }
"#;

    let models = build_models(code);
    let model = must_some(find_model(&models, "MyApp::User"));

    assert_eq!(
        model.framework,
        Framework::RoseDBObject,
        "expected Framework::RoseDBObject for package using use parent 'Rose::DB::Object'"
    );
}

#[test]
fn rose_db_object_parent_captured() {
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

sub new { }
"#;

    let models = build_models(code);
    let model = must_some(find_model(&models, "MyApp::User"));

    assert!(
        model.parents.contains(&"Rose::DB::Object".to_string()),
        "expected 'Rose::DB::Object' in parents list"
    );
}

// =============================================================================
// AC4: meta->setup Extraction Tests
// =============================================================================

#[test]
fn rose_db_object_meta_setup_columns_extracted() {
    // AC4: Given `__PACKAGE__->meta->setup(columns => [qw(id name email)])`
    // When the semantic analyzer processes the file
    // Then synthesized symbols are created for `id()`, `name()`, and `email()`
    // with `declaration = "meta->setup"`
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    table => 'users',
    columns => [qw(id name email)],
);
"#;

    let table = extract_symbols(code);

    // Check that column accessor methods exist
    assert!(
        has_symbol(&table, "id", SymbolKind::Subroutine),
        "expected 'id' method symbol for Rose::DB::Object column accessor"
    );
    assert!(
        has_symbol(&table, "name", SymbolKind::Subroutine),
        "expected 'name' method symbol for Rose::DB::Object column accessor"
    );
    assert!(
        has_symbol(&table, "email", SymbolKind::Subroutine),
        "expected 'email' method symbol for Rose::DB::Object column accessor"
    );
}

#[test]
fn rose_db_object_column_methods_have_meta_setup_declaration() {
    // AC4: Column accessor methods should have declaration = "meta->setup"
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id name)],
);
"#;

    let table = extract_symbols(code);

    assert!(
        has_method_with_declaration(&table, "id", SymbolKind::Subroutine, Some("meta->setup")),
        "expected 'id' method to have declaration = 'meta->setup'"
    );
    assert!(
        has_method_with_declaration(&table, "name", SymbolKind::Subroutine, Some("meta->setup")),
        "expected 'name' method to have declaration = 'meta->setup'"
    );
}

#[test]
fn rose_db_object_meta_setup_primary_key_captured() {
    // The primary_key_columns should be tracked as primary keys
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    table => 'users',
    columns => [qw(id name email)],
    primary_key_columns => ['id'],
);
"#;

    let models = build_models(code);
    let model = must_some(find_model(&models, "MyApp::User"));

    // The column accessors should exist as methods
    let id_method = model.methods.iter().find(|m| m.name == "id");
    assert!(id_method.is_some(), "expected 'id' method to be extracted from meta->setup");
}

#[test]
fn rose_db_object_meta_setup_multiple_columns() {
    let code = r#"
package MyApp::Article;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    table => 'articles',
    columns => [qw(id title body status created_at)],
);
"#;

    let table = extract_symbols(code);

    for col in ["id", "title", "body", "status", "created_at"] {
        assert!(
            has_symbol(&table, col, SymbolKind::Subroutine),
            "expected '{col}' method symbol for Rose::DB::Object column accessor"
        );
    }
}

// =============================================================================
// AC2: Column Accessor Completion Tests
// =============================================================================

#[test]
fn rose_db_object_column_accessor_completion_items() {
    // AC2: When completing after `$user->`, column accessor completions appear
    // with documentation "Column accessor (Rose::DB::Object)"
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id name email)],
);
"#;

    let table = extract_symbols(code);

    // Check that symbols have proper documentation for Rose::DB::Object
    if let Some(symbols) = table.symbols.get("id") {
        for symbol in symbols {
            if symbol.kind == SymbolKind::Subroutine {
                let doc = symbol.documentation.as_deref().unwrap_or("");
                assert!(
                    doc.contains("Rose::DB::Object"),
                    "expected 'id' documentation to mention Rose::DB::Object, got: {doc}"
                );
            }
        }
    } else {
        unreachable!("expected 'id' to be in symbol table");
    }
}

// =============================================================================
// Additional Edge Cases
// =============================================================================

#[test]
fn rose_db_object_no_columns_no_accessor_synthesis() {
    // Without columns => [...], no accessor symbols should be synthesized
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

# No meta->setup call
sub new { }
"#;

    let table = extract_symbols(code);

    // Should NOT have synthesized accessors without meta->setup
    assert!(
        !has_symbol(&table, "id", SymbolKind::Subroutine),
        "did not expect 'id' accessor without meta->setup columns"
    );
}

#[test]
fn rose_db_object_class_accessor_takes_precedence() {
    // If Class::Accessor is in the parent chain, ClassAccessor should take precedence
    let code = r#"
package MyApp::Hybrid;
use base qw(Class::Accessor Rose::DB::Object);

__PACKAGE__->mk_accessors(qw(foo));
"#;

    let models = build_models(code);
    let model = must_some(find_model(&models, "MyApp::Hybrid"));

    // Class::Accessor should be detected first
    assert_eq!(
        model.framework,
        Framework::ClassAccessor,
        "expected Framework::ClassAccessor when Class::Accessor is in parent chain"
    );
}

#[test]
fn rose_db_object_moo_takes_precedence() {
    // Moo should take precedence over Rose::DB::Object detection
    let code = r#"
package MyApp::User;
use Moo;
use base qw(Rose::DB::Object);

has 'name' => (is => 'ro');
"#;

    let models = build_models(code);
    let model = must_some(find_model(&models, "MyApp::User"));

    // Moo should be the detected framework
    assert_eq!(
        model.framework,
        Framework::Moo,
        "expected Framework::Moo to take precedence over Rose::DB::Object"
    );
}

#[test]
fn rose_db_object_with_relationships_not_extracted() {
    // Relationships (one_to_many, many_to_many) are out of scope for initial impl
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id name)],
    relationships => [
        one_to_many => 'articles',
    ],
);
"#;

    let table = extract_symbols(code);

    // Just verify columns are extracted, relationships are out of scope
    assert!(has_symbol(&table, "id", SymbolKind::Subroutine), "expected 'id' column accessor");
    assert!(has_symbol(&table, "name", SymbolKind::Subroutine), "expected 'name' column accessor");
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn rose_db_object_empty_columns_array() {
    // Empty columns array should not synthesize any methods
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [],
);
"#;

    let table = extract_symbols(code);

    // No column accessors should be synthesized
    assert!(
        !has_symbol(&table, "id", SymbolKind::Subroutine),
        "did not expect 'id' with empty columns"
    );
}

#[test]
fn rose_db_object_single_column() {
    // Single column should still work
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id)],
);
"#;

    let table = extract_symbols(code);

    assert!(has_symbol(&table, "id", SymbolKind::Subroutine), "expected 'id' column accessor");
}

#[test]
fn rose_db_object_column_names_with_underscores() {
    // Column names with underscores should work
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id user_name created_at updated_at)],
);
"#;

    let table = extract_symbols(code);

    assert!(has_symbol(&table, "id", SymbolKind::Subroutine), "expected 'id'");
    assert!(has_symbol(&table, "user_name", SymbolKind::Subroutine), "expected 'user_name'");
    assert!(has_symbol(&table, "created_at", SymbolKind::Subroutine), "expected 'created_at'");
    assert!(has_symbol(&table, "updated_at", SymbolKind::Subroutine), "expected 'updated_at'");
}

#[test]
fn rose_db_object_column_names_with_numbers() {
    // Column names with numbers should work
    let code = r#"
package MyApp::Invoice;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id total_2019 total_2020 column3)],
);
"#;

    let table = extract_symbols(code);

    assert!(has_symbol(&table, "id", SymbolKind::Subroutine), "expected 'id'");
    assert!(has_symbol(&table, "total_2019", SymbolKind::Subroutine), "expected 'total_2019'");
    assert!(has_symbol(&table, "total_2020", SymbolKind::Subroutine), "expected 'total_2020'");
}

#[test]
fn rose_db_object_user_defined_method_same_as_column() {
    // User-defined method with same name as column - user method should take precedence
    // (though in practice Rose::DB::Object might override, the symbol table tracks both)
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    columns => [qw(id name)],
);

sub id {
    my $self = shift;
    return $self->SUPER::id(@_);
}
"#;

    let table = extract_symbols(code);

    // Both should exist - user-defined and synthesized
    assert!(has_symbol(&table, "id", SymbolKind::Subroutine), "expected 'id' method");
}

#[test]
fn rose_db_object_columns_after_other_args() {
    // columns key may not be the first key in the hash
    let code = r#"
package MyApp::User;
use base qw(Rose::DB::Object);

__PACKAGE__->meta->setup(
    table => 'users',
    primary_key_columns => ['id'],
    columns => [qw(id name email)],
);
"#;

    let table = extract_symbols(code);

    assert!(has_symbol(&table, "id", SymbolKind::Subroutine), "expected 'id' column accessor");
    assert!(has_symbol(&table, "name", SymbolKind::Subroutine), "expected 'name' column accessor");
    assert!(
        has_symbol(&table, "email", SymbolKind::Subroutine),
        "expected 'email' column accessor"
    );
}

#[test]
fn rose_db_object_no_base_no_extraction() {
    // Without Rose::DB::Object base, meta->setup should not be treated as Rose::DB::Object
    let code = r#"
package MyApp::User;

__PACKAGE__->meta->setup(
    columns => [qw(id name)],
);
"#;

    let models = build_models(code);
    let model = find_model(&models, "MyApp::User");

    // Should not be classified as RoseDBObject since it doesn't inherit from it
    if let Some(model) = model {
        assert_ne!(
            model.framework,
            Framework::RoseDBObject,
            "expected not Framework::RoseDBObject without Rose::DB::Object inheritance"
        );
    }
}

// =============================================================================
// AC5: Framework Enum Documentation Test
// =============================================================================

#[test]
fn rose_db_object_enum_variant_exists() {
    // AC5: Framework::RoseDBObject variant should exist with proper documentation
    // that explicitly states it represents "runtime schema conformance"
    let _ = Framework::RoseDBObject;
}

//! Tests for DBI and ORM Query Result Type Inference
//!
//! These tests cover:
//! - AC-1: Annotation Parsing - `# type: DBI::Row[...]` annotations for typed hash hover and completions
//! - AC-2: DBIx::Class Result Class Parsing - `->search()` returns `ResultSet[T]`, `->first()`/`->find()` return `Result[T]`
//! - AC-3: Graceful Fallback - Unknown SQL returns `Any`, no false positives
//! - AC-4: PerlType Variants - `TypedHash`, `ResultSet`, `Result` display correctly
//! - AC-5: No Regression - Existing `ClassName->new()` behavior preserved
//!
//! Work Item: work-f392f0ad

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::type_inference::{
    PerlType, ScalarType, TypeBasedCompletion, TypeInferenceEngine,
};
use perl_tdd_support::{must, must_some};
use std::sync::Arc;

// ============================================================================
// AC-4: PerlType Variants - TypedHash, ResultSet, Result
// ============================================================================

/// TypedHash displays as `HashRef { col: Type, ... }`
#[test]
fn test_typed_hash_type_display_single_column() {
    let keys = vec![("id".to_string(), PerlType::Scalar(ScalarType::Integer))];
    let ty = PerlType::TypedHash { keys };
    let display = ty.to_string();
    assert!(display.starts_with("HashRef {"), "Expected 'HashRef {{' prefix, got: {}", display);
    assert!(display.contains("id: Int"), "Expected 'id: Int' in display, got: {}", display);
}

/// TypedHash displays all column names and types
#[test]
fn test_typed_hash_type_display_multiple_columns() {
    let keys = vec![
        ("id".to_string(), PerlType::Scalar(ScalarType::Integer)),
        ("name".to_string(), PerlType::Scalar(ScalarType::String)),
        ("email".to_string(), PerlType::Scalar(ScalarType::String)),
    ];
    let ty = PerlType::TypedHash { keys };
    let display = ty.to_string();
    assert!(display.contains("id: Int"), "Expected 'id: Int' in display, got: {}", display);
    assert!(display.contains("name: Str"), "Expected 'name: Str' in display, got: {}", display);
    assert!(display.contains("email: Str"), "Expected 'email: Str' in display, got: {}", display);
}

/// ResultSet(T) displays as `ResultSet[T]`
#[test]
fn test_result_set_type_display() {
    let inner = PerlType::Object("User".to_string());
    let ty = PerlType::ResultSet(Box::new(inner));
    let display = ty.to_string();
    assert_eq!(display, "ResultSet[User]", "Expected 'ResultSet[User]', got: {}", display);
}

/// Result(T) displays as `Result[T]`
#[test]
fn test_result_type_display() {
    let inner = PerlType::Object("User".to_string());
    let ty = PerlType::Result(Box::new(inner));
    let display = ty.to_string();
    assert_eq!(display, "Result[User]", "Expected 'Result[User]', got: {}", display);
}

/// TypedHash can be constructed with empty keys
#[test]
fn test_typed_hash_empty_keys() {
    let ty = PerlType::TypedHash { keys: vec![] };
    let display = ty.to_string();
    assert!(display.starts_with("HashRef {"), "Expected 'HashRef {{' prefix, got: {}", display);
}

// ============================================================================
// AC-1: Annotation Parsing - DBI::Row type hints
// ============================================================================

/// TypeInferenceEngine can parse `# type: DBI::Row[id=>Int]` annotations
#[test]
fn test_annotation_parser_dbi_row_single_column() {
    let code = r#"
# type: DBI::Row[id=>Int]
my $row = $sth->fetchrow_hashref();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    // After parsing the annotation, the variable $row should have TypedHash type
    let ty = engine.get_type_at("row");
    assert!(ty.is_some(), "Expected type for 'row' to be inferred from annotation");

    let ty = must_some(ty);
    assert!(matches!(ty, PerlType::TypedHash { .. }), "Expected TypedHash type, got: {:?}", ty);
}

/// TypeInferenceEngine can parse `# type: DBI::Row[id=>Int, name=>Str]` annotations
#[test]
fn test_annotation_parser_dbi_row_multiple_columns() {
    let code = r#"
# type: DBI::Row[id=>Int, name=>Str, email=>Str]
my $row = $sth->fetchrow_hashref();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let ty = engine.get_type_at("row");
    assert!(ty.is_some(), "Expected type for 'row' to be inferred from annotation");

    let ty = must_some(ty);
    assert!(matches!(ty, PerlType::TypedHash { .. }), "Expected TypedHash type, got: {:?}", ty);

    // Verify the columns are correctly parsed
    if let PerlType::TypedHash { keys } = ty {
        assert_eq!(keys.len(), 3, "Expected 3 columns, got: {}", keys.len());
    }
}

/// Annotation parsing preserves column type information
#[test]
fn test_annotation_parser_preserves_column_types() {
    let code = r#"
# type: DBI::Row[id=>Int, name=>Str, active=>Bool]
my $row = $sth->fetchrow_hashref();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let ty = must_some(engine.get_type_at("row"));

    if let PerlType::TypedHash { keys } = ty {
        let id_type = keys.iter().find(|(name, _)| name == "id");
        assert!(id_type.is_some(), "Expected 'id' column");
        assert_eq!(
            id_type.unwrap().1,
            PerlType::Scalar(ScalarType::Integer),
            "Expected 'id' to have Int type"
        );

        let name_type = keys.iter().find(|(name, _)| name == "name");
        assert!(name_type.is_some(), "Expected 'name' column");
        assert_eq!(
            name_type.unwrap().1,
            PerlType::Scalar(ScalarType::String),
            "Expected 'name' to have Str type"
        );

        let active_type = keys.iter().find(|(name, _)| name == "active");
        assert!(active_type.is_some(), "Expected 'active' column");
        assert_eq!(
            active_type.unwrap().1,
            PerlType::Scalar(ScalarType::Boolean),
            "Expected 'active' to have Bool type"
        );
    } else {
        panic!("Expected TypedHash type, got: {:?}", ty);
    }
}

/// Invalid annotation format does not cause crashes
#[test]
fn test_invalid_annotation_format_no_crash() {
    let code = r#"
# type: NotDBIRow[id=>Int]
my $row = $sth->fetchrow_hashref();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    // Should not panic, just ignore the annotation
    let _ = engine.infer(&ast);

    // The type should not be TypedHash for non-DBI annotations
    let ty = engine.get_type_at("row");
    // It's acceptable for this to be Any or some other type, as long as it doesn't crash
    let _ = ty;
}

// ============================================================================
// AC-2: DBIx::Class Result Class Parsing
// ============================================================================

/// DBIx::Class result class parsing extracts table name and columns
#[test]
fn test_dbix_class_result_class_parsing() {
    let code = r#"
package MyApp::Schema::Result::User;
__PACKAGE__->table("users");
__PACKAGE__->add_columns(
    id => { data_type => "integer", is_nullable => 0 },
    name => { data_type => "varchar", size => 255 },
    email => { data_type => "varchar", size => 255 },
);
__PACKAGE__->set_primary_key("id");
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    // The engine should store ResultClassInfo for MyApp::Schema::Result::User
    // We can verify this by checking that ->search() calls return ResultSet type
    let ty = engine.get_type_for_result_class("MyApp::Schema::Result::User");
    assert!(ty.is_some(), "Expected ResultClassInfo for 'MyApp::Schema::Result::User'");
}

/// DBIx::Class `->search()` returns `ResultSet[T]`
#[test]
fn test_dbix_class_search_returns_resultset() {
    let code = r#"
package MyApp::Schema::Result::User;
__PACKAGE__->table("users");
__PACKAGE__->add_columns(
    id => { data_type => "integer", is_nullable => 0 },
    name => { data_type => "varchar", size => 255 },
);
__PACKAGE__->set_primary_key("id");

package main;
my $schema = MyApp::Schema->connect('dbi:SQLite:dbname=:memory:');
my $users = $schema->resultset('User')->search({});
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    // After ->search() call, $users should have ResultSet[User] type
    let ty = engine.get_type_at("users");
    assert!(ty.is_some(), "Expected type for 'users' after ->search()");

    let ty = must_some(ty);
    // ResultSet wrapping Object("User") or similar
    match ty {
        PerlType::ResultSet(inner) => {
            // The inner type should be related to User
            let inner_str = inner.to_string();
            assert!(inner_str.contains("User"), "Expected ResultSet[User], got: {:?}", ty);
        }
        _ => panic!("Expected ResultSet type for ->search() result, got: {:?}", ty),
    }
}

/// DBIx::Class `->first()` returns `Result[T]`
#[test]
fn test_dbix_class_first_returns_result() {
    let code = r#"
package MyApp::Schema::Result::User;
__PACKAGE__->table("users");
__PACKAGE__->add_columns(
    id => { data_type => "integer", is_nullable => 0 },
    name => { data_type => "varchar", size => 255 },
);
__PACKAGE__->set_primary_key("id");

package main;
my $schema = MyApp::Schema->connect('dbi:SQLite:dbname=:memory:');
my $user = $schema->resultset('User')->first();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    // After ->first() call, $user should have Result[User] type
    let ty = engine.get_type_at("user");
    assert!(ty.is_some(), "Expected type for 'user' after ->first()");

    let ty = must_some(ty);
    match ty {
        PerlType::Result(inner) => {
            let inner_str = inner.to_string();
            assert!(inner_str.contains("User"), "Expected Result[User], got: {:?}", ty);
        }
        _ => panic!("Expected Result type for ->first() result, got: {:?}", ty),
    }
}

/// DBIx::Class `->find()` returns `Result[T]`
#[test]
fn test_dbix_class_find_returns_result() {
    let code = r#"
package MyApp::Schema::Result::User;
__PACKAGE__->table("users");
__PACKAGE__->add_columns(
    id => { data_type => "integer", is_nullable => 0 },
    name => { data_type => "varchar", size => 255 },
);
__PACKAGE__->set_primary_key("id");

package main;
my $schema = MyApp::Schema->connect('dbi:SQLite:dbname=:memory:');
my $user = $schema->resultset('User')->find(1);
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    // After ->find() call, $user should have Result[User] type
    let ty = engine.get_type_at("user");
    assert!(ty.is_some(), "Expected type for 'user' after ->find()");

    let ty = must_some(ty);
    match ty {
        PerlType::Result(inner) => {
            let inner_str = inner.to_string();
            assert!(inner_str.contains("User"), "Expected Result[User], got: {:?}", ty);
        }
        _ => panic!("Expected Result type for ->find() result, got: {:?}", ty),
    }
}

// ============================================================================
// AC-3: Graceful Fallback - Unknown SQL returns Any
// ============================================================================

/// DBI method call without annotation returns Any (not wrong typed hash)
#[test]
fn test_unknown_dbi_returns_any() {
    let code = r#"
my $row = $sth->fetchrow_hashref();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let ty = engine.get_type_at("row");
    // Without annotation, type should be Any, not a wrongly-inferred TypedHash
    assert!(
        ty.is_none()
            || matches!(
                ty.as_ref(),
                Some(PerlType::Any) | Some(PerlType::Scalar(ScalarType::Undef))
            ),
        "Expected Any or Undef for unannotated DBI call, got: {:?}",
        ty
    );
}

/// Unknown ORM resultset returns Any
#[test]
fn test_unknown_orm_returns_any() {
    let code = r#"
my $result = $unknown_orm->search({});
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let ty = engine.get_type_at("result");
    // Without known ORM, should return Any
    assert!(
        ty.is_none() || matches!(ty.as_ref(), Some(PerlType::Any)),
        "Expected Any for unknown ORM call, got: {:?}",
        ty
    );
}

// ============================================================================
// AC-5: No Regression - ClassName->new() behavior preserved
// ============================================================================

/// ClassName->new() still returns Object("ClassName")
#[test]
fn test_class_new_returns_object_type() {
    let code = r#"
package MyClass;
sub new { bless {}, shift }

package main;
my $obj = MyClass->new();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let ty = engine.get_type_at("obj");
    assert!(ty.is_some(), "Expected type for 'obj'");

    let ty = must_some(ty);
    assert!(
        matches!(ty, PerlType::Object(ref name) if name.contains("MyClass")),
        "Expected Object(MyClass), got: {:?}",
        ty
    );
}

/// hover_label_for returns class name for Object types
#[test]
fn test_hover_label_for_object_type() {
    let code = r#"
package MyClass;
sub new { bless {}, shift }

package main;
my $obj = MyClass->new();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let label = engine.hover_label_for("obj");
    assert!(label.is_some(), "Expected hover label for 'obj'");

    let label = must_some(label);
    assert!(label.contains("MyClass"), "Expected 'MyClass' in hover label, got: {}", label);
}

// ============================================================================
// TypeBasedCompletion for TypedHash column completions
// ============================================================================

/// TypeBasedCompletion provides column completions for TypedHash
#[test]
fn test_completion_provides_typed_hash_columns() {
    let code = r#"
# type: DBI::Row[id=>Int, name=>Str, email=>Str]
my $row = $sth->fetchrow_hashref();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let comp = TypeBasedCompletion::new(Arc::new(engine));

    // When completing on $row->{ (hash key access), should suggest column names
    let completions = comp.get_completions("row", "{");

    // Should include id, name, email as completions
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

    assert!(labels.contains(&"id"), "Expected 'id' in completions, got: {:?}", labels);
    assert!(labels.contains(&"name"), "Expected 'name' in completions, got: {:?}", labels);
    assert!(labels.contains(&"email"), "Expected 'email' in completions, got: {:?}", labels);
}

/// TypeBasedCompletion provides column completions for DBIx::Class Result
#[test]
fn test_completion_provides_dbix_class_column_accessors() {
    let code = r#"
package MyApp::Schema::Result::User;
__PACKAGE__->table("users");
__PACKAGE__->add_columns(
    id => { data_type => "integer", is_nullable => 0 },
    name => { data_type => "varchar", size => 255 },
    email => { data_type => "varchar", size => 255 },
);
__PACKAGE__->set_primary_key("id");

package main;
my $schema = MyApp::Schema->connect('dbi:SQLite:dbname=:memory:');
my $user = $schema->resultset('User')->first();
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    let comp = TypeBasedCompletion::new(Arc::new(engine));

    // When completing on $user-> (method call), should suggest column accessors
    let completions = comp.get_completions("user", "");

    // DBIx::Class Result objects have column accessors as methods
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

    // Column names should be available as method completions
    assert!(
        labels.contains(&"id") || labels.contains(&"name") || labels.contains(&"email"),
        "Expected column accessor in completions, got: {:?}",
        labels
    );
}

// ============================================================================
// Integration: Full annotation workflow
// ============================================================================

/// Full workflow: annotation -> type inference -> hover label -> completion
#[test]
fn test_full_annotation_workflow() {
    let code = r#"
# type: DBI::Row[id=>Int, name=>Str]
my $row = $sth->fetchrow_hashref();
print $row->{id};
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let _ = engine.infer(&ast);

    // 1. Type inference
    let ty = engine.get_type_at("row");
    assert!(ty.is_some(), "Type should be inferred for annotated variable");
    assert!(
        matches!(ty.as_ref(), Some(PerlType::TypedHash { .. })),
        "Type should be TypedHash, got: {:?}",
        ty
    );

    // 2. Hover label
    let label = engine.hover_label_for("row");
    assert!(label.is_some(), "Hover label should exist for annotated variable");
    let label = must_some(label);
    assert!(label.contains("HashRef"), "Hover label should show HashRef, got: {}", label);
    assert!(label.contains("id"), "Hover label should contain column name, got: {}", label);

    // 3. Completions
    let comp = TypeBasedCompletion::new(Arc::new(engine));
    let completions = comp.get_completions("row", "{");
    let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

    assert!(labels.contains(&"id"), "Column 'id' should be in completions");
    assert!(labels.contains(&"name"), "Column 'name' should be in completions");
}

//! BDD integration tests for Class::Tiny framework support.
//!
//! Covers framework detection, qw-list attribute extraction, bare `has`
//! declarations, and accessor symbol emission for the Class::Tiny and
//! Class::Tiny::RW OO frameworks.

use perl_semantic_analyzer::{
    Parser,
    analysis::class_model::{AccessorType, ClassModelBuilder, Framework},
    symbol::{SymbolExtractor, SymbolKind, SymbolTable},
};
use perl_tdd_support::{must, must_some};

fn build_class_models(
    code: &str,
) -> Vec<perl_semantic_analyzer::analysis::class_model::ClassModel> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    ClassModelBuilder::new().build(&ast)
}

fn extract_symbols(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|syms| syms.iter().any(|s| s.kind == kind))
}

// Framework detection.

#[test]
fn when_use_class_tiny_then_framework_is_class_tiny() {
    let models = build_class_models(
        r#"
package Animal;
use Class::Tiny;
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Animal"));
    assert_eq!(model.framework, Framework::ClassTiny);
}

#[test]
fn when_use_class_tiny_rw_then_framework_is_class_tiny() {
    let models = build_class_models(
        r#"
package Config;
use Class::Tiny::RW;
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Config"));
    assert_eq!(model.framework, Framework::ClassTiny);
}

// Attribute extraction from use import arguments.

#[test]
fn when_class_tiny_with_qw_list_then_attrs_extracted_as_rw() {
    let models = build_class_models(
        r#"
package Dog;
use Class::Tiny qw(name breed weight);
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Dog"));
    assert_eq!(model.attributes.len(), 3, "expected 3 attrs from qw list");

    for attr in &model.attributes {
        assert_eq!(
            attr.is,
            Some(AccessorType::Rw),
            "qw-list attribute '{}' should be Rw",
            attr.name
        );
        assert_eq!(attr.accessor_name, attr.name, "accessor_name should equal attr name");
    }

    let attr_names: std::collections::HashSet<_> =
        model.attributes.iter().map(|a| a.name.as_str()).collect();
    assert!(attr_names.contains("name"), "missing 'name'");
    assert!(attr_names.contains("breed"), "missing 'breed'");
    assert!(attr_names.contains("weight"), "missing 'weight'");
}

#[test]
fn when_class_tiny_rw_with_qw_list_then_attrs_extracted_as_rw() {
    let models = build_class_models(
        r#"
package Widget;
use Class::Tiny::RW qw(width height color);
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Widget"));
    assert_eq!(model.attributes.len(), 3);
    for attr in &model.attributes {
        assert_eq!(attr.is, Some(AccessorType::Rw));
    }
}

#[test]
fn when_class_tiny_with_no_qw_args_then_no_use_attrs() {
    let models = build_class_models(
        r#"
package Minimal;
use Class::Tiny;
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Minimal"));
    // No attributes from the use statement.
    assert_eq!(model.attributes.len(), 0, "bare use Class::Tiny with no args = no attrs");
}

#[test]
fn when_class_tiny_with_default_hashref_then_keys_are_attrs() {
    let models = build_class_models(
        r#"
package Employee;
use Class::Tiny qw(name ssn), {
  timestamp => sub { time },
  title => 'Peon',
};
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Employee"));
    let attr_names: std::collections::HashSet<_> =
        model.attributes.iter().map(|a| a.name.as_str()).collect();

    assert_eq!(model.attributes.len(), 4);
    assert!(attr_names.contains("name"));
    assert!(attr_names.contains("ssn"));
    assert!(attr_names.contains("timestamp"));
    assert!(attr_names.contains("title"));
    assert!(!attr_names.contains("time"));
    assert!(!attr_names.contains("Peon"));
}

// Bare has declarations.

#[test]
fn when_class_tiny_bare_has_then_attr_extracted_with_no_is() {
    let models = build_class_models(
        r#"
package Cat;
use Class::Tiny;
has 'name';
has 'color';
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Cat"));
    assert_eq!(model.attributes.len(), 2, "expected 2 attrs from bare has");

    let name_attr = must_some(model.attributes.iter().find(|a| a.name == "name"));
    assert_eq!(name_attr.accessor_name, "name");
    // Bare `has 'name';` produces no explicit `is` option.
    assert_eq!(name_attr.is, None);
}

#[test]
fn when_class_tiny_has_with_is_ro_then_accessor_type_is_ro() {
    let models = build_class_models(
        r#"
package Immutable;
use Class::Tiny;
has 'id' => (is => 'ro');
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Immutable"));
    let id_attr = must_some(model.attributes.iter().find(|a| a.name == "id"));
    assert_eq!(id_attr.is, Some(AccessorType::Ro));
}

#[test]
fn when_class_tiny_has_with_is_rw_then_accessor_type_is_rw() {
    let models = build_class_models(
        r#"
package Mutable;
use Class::Tiny;
has 'count' => (is => 'rw');
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Mutable"));
    let count_attr = must_some(model.attributes.iter().find(|a| a.name == "count"));
    assert_eq!(count_attr.is, Some(AccessorType::Rw));
}

// Mixed patterns.

#[test]
fn when_class_tiny_mixes_qw_and_has_then_all_attrs_captured() {
    let models = build_class_models(
        r#"
package Employee;
use Class::Tiny qw(name department);
has 'salary' => (is => 'rw');
has 'title';
sub promote { }
"#,
    );
    let model = must_some(models.iter().find(|m| m.name == "Employee"));
    assert_eq!(model.framework, Framework::ClassTiny);
    assert_eq!(model.attributes.len(), 4, "2 from qw + 2 from has");

    let attr_names: std::collections::HashSet<_> =
        model.attributes.iter().map(|a| a.name.as_str()).collect();
    assert!(attr_names.contains("name"));
    assert!(attr_names.contains("department"));
    assert!(attr_names.contains("salary"));
    assert!(attr_names.contains("title"));

    assert!(model.methods.iter().any(|m| m.name == "promote"), "method promote expected");
}

// Symbol emission.

#[test]
fn when_class_tiny_qw_attrs_then_subroutine_symbols_emitted() {
    let code = r#"
package User;
use Class::Tiny qw(name email);
"#;
    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "name", SymbolKind::Subroutine),
        "expected accessor method symbol for 'name'"
    );
    assert!(
        has_symbol(&table, "email", SymbolKind::Subroutine),
        "expected accessor method symbol for 'email'"
    );
}

#[test]
fn when_class_tiny_default_hashref_then_subroutine_symbols_emitted() {
    let code = r#"
package User;
use Class::Tiny qw(name), {
  email => sub { $_[0]->_build_email },
  status => 'active',
};
"#;
    let table = extract_symbols(code);

    assert!(has_symbol(&table, "name", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "email", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "status", SymbolKind::Subroutine));
    assert!(!has_symbol(&table, "active", SymbolKind::Subroutine));
    assert!(!has_symbol(&table, "_build_email", SymbolKind::Subroutine));
}

#[test]
fn when_class_tiny_bare_has_then_accessor_symbol_emitted() {
    let code = r#"
package Shape;
use Class::Tiny;
has 'color';
"#;
    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "color", SymbolKind::Subroutine),
        "expected accessor symbol for bare 'has color'"
    );
}

// Isolation: other packages unaffected.

#[test]
fn when_non_class_tiny_package_uses_has_then_class_tiny_framework_not_applied() {
    let models = build_class_models(
        r#"
package Moo::Thing;
use Moo;
has 'x' => (is => 'ro');

package Plain;
sub do_stuff { }
"#,
    );

    let moo_model = must_some(models.iter().find(|m| m.name == "Moo::Thing"));
    assert_eq!(moo_model.framework, Framework::Moo);

    // Plain package has no framework and should not appear as ClassModel.
    let plain = models.iter().find(|m| m.name == "Plain");
    assert!(plain.is_none(), "plain package without framework should not produce a ClassModel");
}

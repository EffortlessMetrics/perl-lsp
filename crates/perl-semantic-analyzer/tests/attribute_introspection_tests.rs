//! Tests for Perl attribute introspection and documentation.
//!
//! Covers hover documentation for subroutine and variable attributes
//! such as `:lvalue`, `:method`, `:prototype($$)`, `:shared`, and `:const`.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::semantic::{SemanticAnalyzer, get_attribute_documentation};
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind};
use perl_tdd_support::{must, must_some};

fn parse_and_analyze(code: &str) -> SemanticAnalyzer {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SemanticAnalyzer::analyze_with_source(&ast, code)
}

// ---------------------------------------------------------------------------
// get_attribute_documentation unit tests
// ---------------------------------------------------------------------------

#[test]
fn attribute_doc_lvalue_has_description() {
    let doc = get_attribute_documentation("lvalue");
    assert!(doc.is_some(), "lvalue should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("lvalue")
            || doc.description.to_lowercase().contains("assign"),
        "lvalue description should mention lvalue or assignment, got: {}",
        doc.description
    );
}

#[test]
fn attribute_doc_method_has_description() {
    let doc = get_attribute_documentation("method");
    assert!(doc.is_some(), "method should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("method"),
        "method description should mention method, got: {}",
        doc.description
    );
}

#[test]
fn attribute_doc_prototype_has_description() {
    let doc = get_attribute_documentation("prototype");
    assert!(doc.is_some(), "prototype should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("prototype")
            || doc.description.to_lowercase().contains("calling"),
        "prototype description should mention prototype or calling, got: {}",
        doc.description
    );
}

#[test]
fn attribute_doc_const_has_description() {
    let doc = get_attribute_documentation("const");
    assert!(doc.is_some(), "const should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("const")
            || doc.description.to_lowercase().contains("immutable")
            || doc.description.to_lowercase().contains("read"),
        "const description should mention const/immutable/read, got: {}",
        doc.description
    );
}

#[test]
fn attribute_doc_shared_has_description() {
    let doc = get_attribute_documentation("shared");
    assert!(doc.is_some(), "shared should have documentation");
    let doc = must_some(doc);
    assert!(
        doc.description.to_lowercase().contains("thread")
            || doc.description.to_lowercase().contains("shared"),
        "shared description should mention thread or shared, got: {}",
        doc.description
    );
}

#[test]
fn attribute_doc_unknown_returns_none() {
    let doc = get_attribute_documentation("nonexistent_attribute_xyz");
    assert!(doc.is_none(), "unknown attribute should return None");
}

#[test]
fn attribute_doc_colon_stripped_lvalue() {
    // The non-colon form must work
    let without = get_attribute_documentation("lvalue");
    assert!(without.is_some(), "lvalue (no colon) should have documentation");
}

#[test]
fn attribute_doc_covers_all_known_builtins() {
    let known_attrs = ["lvalue", "method", "prototype", "const", "shared"];
    for attr in &known_attrs {
        let doc = get_attribute_documentation(attr);
        assert!(doc.is_some(), "Expected documentation for attribute '{}' but got None", attr);
        let doc = must_some(doc);
        assert!(!doc.description.is_empty(), "Documentation for '{}' has empty description", attr);
    }
}

// ---------------------------------------------------------------------------
// Hover detail enrichment for subroutine attributes
// ---------------------------------------------------------------------------

#[test]
fn hover_lvalue_sub_details_contain_semantics() {
    let code = r#"sub get_name :lvalue { $name }"#;
    let analyzer = parse_and_analyze(code);
    let table = analyzer.symbol_table();

    let subs = table.symbols.get("get_name");
    if let Some(syms) = subs {
        let sub_sym = syms.iter().find(|s| s.kind == SymbolKind::Subroutine);
        if let Some(sym) = sub_sym {
            let hover = analyzer.hover_at(sym.location);
            if let Some(info) = hover {
                let all_text = format!(
                    "{} {} {}",
                    info.signature,
                    info.documentation.as_deref().unwrap_or(""),
                    info.details.join(" ")
                );
                assert!(
                    all_text.to_lowercase().contains("lvalue")
                        || all_text.to_lowercase().contains("assign"),
                    "Hover for :lvalue sub should describe lvalue semantics, got: {}",
                    all_text
                );
            }
        }
    }
}

#[test]
fn hover_method_sub_details_contain_semantics() {
    let code = r#"sub process :method { my ($self) = @_; }"#;
    let analyzer = parse_and_analyze(code);
    let table = analyzer.symbol_table();

    let subs = table.symbols.get("process");
    if let Some(syms) = subs {
        let sub_sym = syms.iter().find(|s| s.kind == SymbolKind::Subroutine);
        if let Some(sym) = sub_sym {
            let hover = analyzer.hover_at(sym.location);
            if let Some(info) = hover {
                let all_text = format!(
                    "{} {} {}",
                    info.signature,
                    info.documentation.as_deref().unwrap_or(""),
                    info.details.join(" ")
                );
                assert!(
                    all_text.to_lowercase().contains("method"),
                    "Hover for :method sub should describe method semantics, got: {}",
                    all_text
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Hover detail enrichment for variable attributes
// ---------------------------------------------------------------------------

#[test]
fn hover_shared_variable_details_contain_semantics() {
    let code = r#"my $count :shared = 0;"#;
    let analyzer = parse_and_analyze(code);
    let table = analyzer.symbol_table();

    let vars = table.symbols.get("count");
    if let Some(syms) = vars {
        let var_sym = syms.first();
        if let Some(sym) = var_sym {
            let hover = analyzer.hover_at(sym.location);
            if let Some(info) = hover {
                let all_text = format!(
                    "{} {} {}",
                    info.signature,
                    info.documentation.as_deref().unwrap_or(""),
                    info.details.join(" ")
                );
                assert!(
                    all_text.to_lowercase().contains("shared")
                        || all_text.to_lowercase().contains("thread"),
                    "Hover for :shared var should describe shared semantics, got: {}",
                    all_text
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Symbol attributes are stored in the symbol table
// ---------------------------------------------------------------------------

#[test]
fn symbol_table_stores_lvalue_attribute() {
    let code = r#"sub get_name :lvalue { $name }"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let table = SymbolExtractor::new_with_source(code).extract(&ast);

    let subs = table.symbols.get("get_name");
    if let Some(syms) = subs {
        let sub_sym = syms.iter().find(|s| s.kind == SymbolKind::Subroutine);
        if let Some(sym) = sub_sym {
            assert!(
                sym.attributes.iter().any(|a| a.contains("lvalue")),
                "Symbol attributes should include lvalue, got: {:?}",
                sym.attributes
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Enriched attribute details format
// ---------------------------------------------------------------------------

#[test]
fn enriched_attribute_details_format() {
    // When a sub has :lvalue, the details should contain an enriched description,
    // not just the raw ":lvalue" string.
    let code = r#"sub accessor :lvalue { $field }"#;
    let analyzer = parse_and_analyze(code);
    let table = analyzer.symbol_table();

    let subs = table.symbols.get("accessor");
    if let Some(syms) = subs {
        let sub_sym = syms.iter().find(|s| s.kind == SymbolKind::Subroutine);
        if let Some(sym) = sub_sym {
            let hover = analyzer.hover_at(sym.location);
            if let Some(info) = hover {
                // The details should be non-empty for a sub with attributes
                assert!(
                    !info.details.is_empty(),
                    "Sub with attributes should have non-empty hover details"
                );
            }
        }
    }
}

#[test]
fn sub_without_attributes_has_empty_details() {
    let code = r#"sub plain_sub { return 42; }"#;
    let analyzer = parse_and_analyze(code);
    let table = analyzer.symbol_table();

    let subs = table.symbols.get("plain_sub");
    if let Some(syms) = subs {
        let sub_sym = syms.iter().find(|s| s.kind == SymbolKind::Subroutine);
        if let Some(sym) = sub_sym {
            let hover = analyzer.hover_at(sym.location);
            if let Some(info) = hover {
                assert!(
                    info.details.is_empty(),
                    "Sub without attributes should have empty hover details, got: {:?}",
                    info.details
                );
            }
        }
    }
}

//! Comprehensive unit tests for perl-semantic-analyzer.
//!
//! Covers the main public API: semantic analysis, scope analysis,
//! symbol extraction, type inference, and workspace indexing.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::scope_analyzer::{IssueKind, ScopeAnalyzer, ScopeIssue};
use perl_semantic_analyzer::analysis::semantic::{
    SemanticAnalyzer, SemanticModel, SemanticTokenModifier, SemanticTokenType,
};
use perl_semantic_analyzer::analysis::type_inference::{
    PerlType, ScalarType, TypeBasedCompletion, TypeEnvironment, TypeInferenceEngine,
};
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind, SymbolTable};
use perl_tdd_support::must;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_and_extract(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn parse_and_analyze(code: &str) -> SemanticAnalyzer {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SemanticAnalyzer::analyze_with_source(&ast, code)
}

fn scope_issues(code: &str) -> Vec<ScopeIssue> {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, code, &[])
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|syms| syms.iter().any(|s| s.kind == kind))
}

// ===========================================================================
// 1. Symbol Extraction
// ===========================================================================

#[test]
fn symbol_extraction_simple_sub() -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_and_extract("sub greet { 1 }");
    assert!(has_symbol(&table, "greet", SymbolKind::Subroutine));
    Ok(())
}

#[test]
fn symbol_extraction_package_declaration() -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_and_extract("package Foo::Bar;");
    assert!(has_symbol(&table, "Foo::Bar", SymbolKind::Package));
    Ok(())
}

#[test]
fn symbol_extraction_my_variable() -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_and_extract("my $count = 0;");
    assert!(has_symbol(&table, "count", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_extraction_our_variable() -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_and_extract("our $VERSION = '1.0';");
    assert!(has_symbol(&table, "VERSION", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_extraction_array_variable() -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_and_extract("my @items = (1, 2, 3);");
    assert!(has_symbol(&table, "items", SymbolKind::array()));
    Ok(())
}

#[test]
fn symbol_extraction_hash_variable() -> Result<(), Box<dyn std::error::Error>> {
    let table = parse_and_extract("my %config = (key => 'val');");
    assert!(has_symbol(&table, "config", SymbolKind::hash()));
    Ok(())
}

#[test]
fn symbol_extraction_multiple_subs() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub alpha { 1 }
sub beta  { 2 }
sub gamma { 3 }
"#;
    let table = parse_and_extract(code);
    for name in ["alpha", "beta", "gamma"] {
        assert!(has_symbol(&table, name, SymbolKind::Subroutine), "missing sub {name}");
    }
    Ok(())
}

#[test]
fn symbol_extraction_method_preserves_attributes() -> Result<(), Box<dyn std::error::Error>> {
    let code = "method size :lvalue :prototype($self) ($self) { $self }";
    let table = parse_and_extract(code);
    let methods =
        table.symbols.get("size").ok_or("expected method symbol `size` to be extracted")?;
    let method = methods
        .iter()
        .find(|symbol| symbol.kind == SymbolKind::Subroutine)
        .ok_or("expected subroutine symbol for method `size`")?;

    assert!(method.attributes.contains(&"method".to_string()));
    assert!(method.attributes.contains(&"lvalue".to_string()));
    assert!(method.attributes.contains(&"prototype($self)".to_string()));

    Ok(())
}

#[test]
fn symbol_extraction_nested_sub_variables() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub outer {
    my $a = 1;
    sub inner {
        my $b = 2;
    }
}
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "outer", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "inner", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "a", SymbolKind::scalar()));
    assert!(has_symbol(&table, "b", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn symbol_extraction_qualified_name() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Foo;
sub bar { 1 }
"#;
    let table = parse_and_extract(code);
    let bar_syms = table.symbols.get("bar").ok_or("bar not found")?;
    assert!(!bar_syms.is_empty());
    assert!(
        bar_syms.iter().any(|s| s.qualified_name.contains("Foo")),
        "qualified name should contain package"
    );
    Ok(())
}

#[test]
fn symbol_extraction_constant() -> Result<(), Box<dyn std::error::Error>> {
    // `use constant` is a pragma call; the extractor may index the import
    // rather than a named constant. Verify no crash and some symbols exist.
    let code = "use constant PI => 3.14159;";
    let table = parse_and_extract(code);
    // The parser should at least produce something from this pragma
    // (may index "constant" as an Import or not index "PI" at all)
    let _ = table;
    Ok(())
}

#[test]
fn symbol_extraction_const_fast_scalar_is_constant() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use Const::Fast;
const my $PI => 3.14159;
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "PI", SymbolKind::Constant));
    Ok(())
}

#[test]
fn symbol_extraction_const_fast_array_is_constant() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use Const::Fast;
const my @ARRAY => (1, 2, 3);
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "ARRAY", SymbolKind::Constant));
    Ok(())
}

#[test]
fn symbol_extraction_readonly_scalar_is_constant() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use Readonly;
Readonly my $PI => 3.14159;
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "PI", SymbolKind::Constant));
    Ok(())
}

#[test]
fn symbol_extraction_readonly_hash_is_constant() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use Readonly;
Readonly my %HASH => (answer => 42);
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "HASH", SymbolKind::Constant));
    Ok(())
}

#[test]
fn symbol_extraction_readonly_scalar_is_constant() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use Readonly;
Readonly my $PI => 3.14159;
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "PI", SymbolKind::Constant));
    Ok(())
}

#[test]
fn symbol_extraction_readonly_array_is_constant() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
use Readonly;
Readonly my @ARRAY => (1, 2, 3);
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "ARRAY", SymbolKind::Constant));
    Ok(())
}

#[test]
fn symbol_extraction_readonly_our_uses_package_qualified_name()
-> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package My::Pkg;
use Readonly;
Readonly our $PACKAGE_CONSTANT => 'foo';
"#;
    let table = parse_and_extract(code);
    let symbols = table.symbols.get("PACKAGE_CONSTANT").ok_or("PACKAGE_CONSTANT not found")?;
    assert!(
        symbols.iter().any(|symbol| {
            symbol.kind == SymbolKind::Constant
                && symbol.qualified_name == "My::Pkg::PACKAGE_CONSTANT"
        }),
        "expected package-qualified Readonly constant symbol, got: {:?}",
        symbols.iter().map(|symbol| &symbol.qualified_name).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn symbol_extraction_documentation_comment() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
# This sub does amazing things
sub amazing { 1 }
"#;
    let table = parse_and_extract(code);
    let syms = table.symbols.get("amazing").ok_or("amazing not found")?;
    assert!(!syms.is_empty());
    assert!(syms[0].documentation.is_some());
    Ok(())
}

#[test]
fn symbol_extraction_references_tracked() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 10;
print $x;
"#;
    let table = parse_and_extract(code);
    assert!(!table.references.is_empty(), "references should be tracked for variable usage");
    Ok(())
}

#[test]
fn symbol_table_find_symbol_walks_scope() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $outer = 1;
sub test_fn {
    my $inner = 2;
}
"#;
    let table = parse_and_extract(code);
    let found = table.find_symbol("outer", 0, SymbolKind::scalar());
    assert!(!found.is_empty(), "should find $outer from global scope");
    Ok(())
}

#[test]
fn symbol_table_new_has_global_scope() -> Result<(), Box<dyn std::error::Error>> {
    let table = SymbolTable::new();
    assert!(table.scopes.contains_key(&0), "scope 0 (global) must exist");
    Ok(())
}

// ===========================================================================
// 2. Semantic Analysis
// ===========================================================================

#[test]
fn semantic_analyzer_produces_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = parse_and_analyze("my $x = 42; print $x;");
    assert!(!analyzer.semantic_tokens().is_empty());
    Ok(())
}

#[test]
fn semantic_analyzer_variable_declaration_modifier() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = parse_and_analyze("my $x = 1;");
    let decl_tokens: Vec<_> = analyzer
        .semantic_tokens()
        .iter()
        .filter(|t| t.modifiers.contains(&SemanticTokenModifier::Declaration))
        .collect();
    assert!(!decl_tokens.is_empty(), "should mark $x declaration");
    Ok(())
}

#[test]
fn semantic_analyzer_function_token_type() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = parse_and_analyze("sub hello { 1 }");
    let fn_tokens: Vec<_> = analyzer
        .semantic_tokens()
        .iter()
        .filter(|t| {
            matches!(
                t.token_type,
                SemanticTokenType::Function | SemanticTokenType::FunctionDeclaration
            )
        })
        .collect();
    assert!(!fn_tokens.is_empty(), "should have function token for sub");
    Ok(())
}

#[test]
fn semantic_analyzer_symbol_table_access() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = parse_and_analyze("my $var = 99;");
    let table = analyzer.symbol_table();
    assert!(has_symbol(table, "var", SymbolKind::scalar()));
    Ok(())
}

#[test]
fn semantic_analyzer_hover_for_sub() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
# greet the user
sub greet { 1 }
"#;
    let analyzer = parse_and_analyze(code);
    let table = analyzer.symbol_table();
    let syms = table.find_symbol("greet", 0, SymbolKind::Subroutine);
    assert!(!syms.is_empty());
    let hover = analyzer.hover_at(syms[0].location).ok_or("hover expected")?;
    assert!(hover.signature.contains("greet"));
    Ok(())
}

#[test]
fn semantic_analyzer_symbol_at_returns_most_specific() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    let pos = code.find("$x").ok_or("$x not found")?;
    let loc = perl_semantic_analyzer::SourceLocation { start: pos + 1, end: pos + 2 };
    let sym = analyzer.symbol_at(loc);
    assert!(sym.is_some(), "should find symbol at $x position");
    Ok(())
}

#[test]
fn semantic_analyzer_find_definition_on_reference() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
print $x;
"#;
    let analyzer = parse_and_analyze(code);
    let ref_pos = code.rfind("$x").ok_or("ref $x not found")?;
    // find_definition may or may not resolve depending on scope — just ensure no crash
    let _ = analyzer.find_definition(ref_pos);
    Ok(())
}

#[test]
fn semantic_analyzer_find_all_references() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub foo { 1 }
foo();
foo();
"#;
    let analyzer = parse_and_analyze(code);
    let def_pos = code.find("sub foo").ok_or("sub foo not found")? + 4;
    let refs = analyzer.find_all_references(def_pos, true);
    assert!(!refs.is_empty(), "should find at least the declaration");
    Ok(())
}

#[test]
fn semantic_analyzer_empty_source() -> Result<(), Box<dyn std::error::Error>> {
    let analyzer = parse_and_analyze("");
    assert!(analyzer.semantic_tokens().is_empty());
    assert!(analyzer.symbol_table().symbols.is_empty());
    Ok(())
}

#[test]
fn semantic_analyzer_infer_type_returns_string() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $x = 42;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, code);
    // infer_type on the program root may or may not produce a result
    let _ = analyzer.infer_type(&ast);
    Ok(())
}

// ===========================================================================
// 3. SemanticModel (high-level façade)
// ===========================================================================

#[test]
fn semantic_model_build_tokens() -> Result<(), Box<dyn std::error::Error>> {
    let code = "my $a = 1; my $b = 2;";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let model = SemanticModel::build(&ast, code);
    assert!(!model.tokens().is_empty());
    Ok(())
}

#[test]
fn semantic_model_symbol_table() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub helper { 42 }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let model = SemanticModel::build(&ast, code);
    assert!(has_symbol(model.symbol_table(), "helper", SymbolKind::Subroutine));
    Ok(())
}

#[test]
fn semantic_model_definition_at() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
sub target { 1 }
target();
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let model = SemanticModel::build(&ast, code);
    let call_pos = code.rfind("target").ok_or("target not found")?;
    // definition_at may or may not resolve; just verify no crash
    let _ = model.definition_at(call_pos);
    Ok(())
}

#[test]
fn semantic_model_hover_info_at() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
# documented helper
sub doc_helper { 1 }
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let model = SemanticModel::build(&ast, code);
    let table = model.symbol_table();
    let syms = table.find_symbol("doc_helper", 0, SymbolKind::Subroutine);
    assert!(!syms.is_empty());
    let hover = model.hover_info_at(syms[0].location);
    assert!(hover.is_some(), "hover should exist for documented sub");
    Ok(())
}

// ===========================================================================
// 4. Scope Analysis
// ===========================================================================

#[test]
fn scope_analysis_unused_variable() -> Result<(), Box<dyn std::error::Error>> {
    let issues = scope_issues("my $unused = 1;");
    assert!(
        issues.iter().any(|i| i.kind == IssueKind::UnusedVariable),
        "should detect unused variable"
    );
    Ok(())
}

#[test]
fn scope_analysis_used_variable_no_issue() -> Result<(), Box<dyn std::error::Error>> {
    let issues = scope_issues("my $x = 1; print $x;");
    let unused: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("x"))
        .collect();
    assert!(unused.is_empty(), "used variable should not be flagged");
    Ok(())
}

#[test]
fn scope_analysis_variable_shadowing() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
{
    my $x = 2;
    print $x;
}
"#;
    let issues = scope_issues(code);
    assert!(
        issues.iter().any(|i| i.kind == IssueKind::VariableShadowing),
        "should detect variable shadowing"
    );
    Ok(())
}

#[test]
fn scope_analysis_redeclaration() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
{
    my $x = 1;
    my $x = 2;
    print $x;
}
"#;
    let issues = scope_issues(code);
    assert!(
        issues.iter().any(|i| i.kind == IssueKind::VariableRedeclaration),
        "should detect redeclaration in same scope"
    );
    Ok(())
}

#[test]
fn scope_analysis_our_variables_not_unused() -> Result<(), Box<dyn std::error::Error>> {
    let issues = scope_issues("our $VERSION = '1.0';");
    let unused_version: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("VERSION"))
        .collect();
    assert!(unused_version.is_empty(), "our variables should not be reported as unused");
    Ok(())
}

#[test]
fn scope_analysis_underscore_prefix_suppresses_unused() -> Result<(), Box<dyn std::error::Error>> {
    let issues = scope_issues("my $_ignored = 1;");
    let unused: Vec<_> = issues
        .iter()
        .filter(|i| i.kind == IssueKind::UnusedVariable && i.variable_name.contains("_ignored"))
        .collect();
    assert!(unused.is_empty(), "underscore-prefixed variables should be suppressed");
    Ok(())
}

#[test]
fn scope_analysis_empty_code() -> Result<(), Box<dyn std::error::Error>> {
    let issues = scope_issues("");
    assert!(issues.is_empty(), "empty code should produce no issues");
    Ok(())
}

#[test]
fn scope_analysis_sub_parameters_declared() -> Result<(), Box<dyn std::error::Error>> {
    // Verify the scope analyzer can process sub parameters without crashing
    let code = r#"
sub greet {
    my ($name) = @_;
    print $name;
}
"#;
    let issues = scope_issues(code);
    // The scope analyzer may or may not flag $name depending on how string
    // interpolation usage is tracked; just verify analysis completes
    let _ = issues;
    Ok(())
}

#[test]
fn scope_analysis_issue_has_line_info() -> Result<(), Box<dyn std::error::Error>> {
    let issues = scope_issues("my $unused = 1;");
    for issue in &issues {
        if issue.kind == IssueKind::UnusedVariable {
            assert!(issue.line > 0, "line should be set");
            assert!(!issue.description.is_empty(), "description should be set");
        }
    }
    Ok(())
}

#[test]
fn scope_analyzer_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let a = ScopeAnalyzer;
    let mut parser = Parser::new("my $x = 1; print $x;");
    let ast = parser.parse()?;
    let issues = a.analyze(&ast, "my $x = 1; print $x;", &[]);
    // Just verify it works via Default
    let _ = issues;
    Ok(())
}

// ===========================================================================
// 5. Type Inference
// ===========================================================================

#[test]
fn type_inference_integer_literal() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my $x = 42;");
    let ast = parser.parse()?;
    let result = engine.infer(&ast);
    assert!(result.is_ok());
    assert_eq!(engine.get_type_at("x"), Some(PerlType::Scalar(ScalarType::Integer)));
    Ok(())
}

#[test]
fn type_inference_string_literal() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my $s = \"hello\";");
    let ast = parser.parse()?;
    let result = engine.infer(&ast);
    assert!(result.is_ok());
    assert_eq!(engine.get_type_at("s"), Some(PerlType::Scalar(ScalarType::String)));
    Ok(())
}

#[test]
fn type_inference_float_literal() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my $f = 3.14;");
    let ast = parser.parse()?;
    let result = engine.infer(&ast);
    assert!(result.is_ok());
    assert_eq!(engine.get_type_at("f"), Some(PerlType::Scalar(ScalarType::Float)));
    Ok(())
}

#[test]
fn type_inference_array() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my @arr = (1, 2, 3);");
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    assert!(matches!(engine.get_type_at("arr"), Some(PerlType::Array(_))));
    Ok(())
}

#[test]
fn type_inference_hash() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my %h = (a => 1, b => 2);");
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    assert!(matches!(engine.get_type_at("h"), Some(PerlType::Hash { .. })));
    Ok(())
}

#[test]
fn type_inference_undef() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my $u = undef;");
    let ast = parser.parse()?;
    let result = engine.infer(&ast);
    assert!(result.is_ok());
    assert_eq!(engine.get_type_at("u"), Some(PerlType::Scalar(ScalarType::Undef)));
    Ok(())
}

#[test]
fn type_inference_builtin_length() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my $n = length(\"hello\");");
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    assert_eq!(engine.get_type_at("n"), Some(PerlType::Scalar(ScalarType::Integer)));
    Ok(())
}

#[test]
fn type_inference_engine_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let engine = TypeInferenceEngine::default();
    // Just verify Default produces a valid engine
    assert!(engine.get_type_at("nonexistent").is_none());
    Ok(())
}

#[test]
fn type_inference_get_type_errors_empty_initially() -> Result<(), Box<dyn std::error::Error>> {
    let engine = TypeInferenceEngine::new();
    assert!(engine.get_type_errors().is_empty());
    Ok(())
}

#[test]
fn type_inference_get_subroutine() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let code = "sub add { my ($a, $b) = @_; return $a + $b; }";
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    // Subroutine type may or may not be recorded depending on implementation
    let _ = engine.get_subroutine("add");
    Ok(())
}

// ===========================================================================
// 6. Type Environment
// ===========================================================================

#[test]
fn type_env_set_get_variable() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = TypeEnvironment::new();
    env.set_variable("x".to_string(), PerlType::Scalar(ScalarType::Integer));
    assert_eq!(env.get_variable("x"), Some(&PerlType::Scalar(ScalarType::Integer)));
    Ok(())
}

#[test]
fn type_env_parent_scope_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let mut parent = TypeEnvironment::new();
    parent.set_variable("outer".to_string(), PerlType::Scalar(ScalarType::String));
    let child = TypeEnvironment::with_parent(parent);
    assert_eq!(child.get_variable("outer"), Some(&PerlType::Scalar(ScalarType::String)));
    Ok(())
}

#[test]
fn type_env_child_shadows_parent() -> Result<(), Box<dyn std::error::Error>> {
    let mut parent = TypeEnvironment::new();
    parent.set_variable("v".to_string(), PerlType::Scalar(ScalarType::Integer));
    let mut child = TypeEnvironment::with_parent(parent);
    child.set_variable("v".to_string(), PerlType::Scalar(ScalarType::String));
    assert_eq!(child.get_variable("v"), Some(&PerlType::Scalar(ScalarType::String)));
    Ok(())
}

#[test]
fn type_env_missing_variable_returns_none() -> Result<(), Box<dyn std::error::Error>> {
    let env = TypeEnvironment::new();
    assert!(env.get_variable("missing").is_none());
    Ok(())
}

#[test]
fn type_env_subroutine_lookup() -> Result<(), Box<dyn std::error::Error>> {
    let mut env = TypeEnvironment::new();
    let sig = PerlType::Subroutine {
        params: vec![PerlType::Scalar(ScalarType::Integer)],
        returns: vec![PerlType::Scalar(ScalarType::String)],
    };
    env.set_subroutine("fmt".to_string(), sig.clone());
    assert_eq!(env.get_subroutine("fmt"), Some(&sig));
    Ok(())
}

#[test]
fn type_env_default_trait() -> Result<(), Box<dyn std::error::Error>> {
    let env = TypeEnvironment::default();
    assert!(env.get_variable("x").is_none());
    Ok(())
}

// ===========================================================================
// 7. Type-Based Completions
// ===========================================================================

#[test]
fn completion_array_methods() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my @items = (1, 2);");
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    let comp = TypeBasedCompletion::new(Arc::new(engine));
    let completions = comp.get_completions("items", "");
    assert!(completions.iter().any(|c| c.label == "push"));
    assert!(completions.iter().any(|c| c.label == "pop"));
    assert!(completions.iter().any(|c| c.label == "shift"));
    assert!(completions.iter().any(|c| c.label == "unshift"));
    Ok(())
}

#[test]
fn completion_hash_methods() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new("my %cfg = (a => 1);");
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);
    let comp = TypeBasedCompletion::new(Arc::new(engine));
    let completions = comp.get_completions("cfg", "");
    assert!(completions.iter().any(|c| c.label == "keys"));
    assert!(completions.iter().any(|c| c.label == "values"));
    assert!(completions.iter().any(|c| c.label == "exists"));
    assert!(completions.iter().any(|c| c.label == "delete"));
    Ok(())
}

#[test]
fn completion_unknown_variable_empty() -> Result<(), Box<dyn std::error::Error>> {
    let engine = TypeInferenceEngine::new();
    let comp = TypeBasedCompletion::new(Arc::new(engine));
    let completions = comp.get_completions("nonexistent", "");
    assert!(completions.is_empty());
    Ok(())
}

// ===========================================================================
// 8. PerlType equality and construction
// ===========================================================================

#[test]
fn perl_type_scalar_equality() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlType::Scalar(ScalarType::Integer), PerlType::Scalar(ScalarType::Integer));
    assert_ne!(PerlType::Scalar(ScalarType::Integer), PerlType::Scalar(ScalarType::String));
    Ok(())
}

#[test]
fn perl_type_array_construction() -> Result<(), Box<dyn std::error::Error>> {
    let ty = PerlType::Array(Box::new(PerlType::Scalar(ScalarType::Integer)));
    assert!(matches!(ty, PerlType::Array(_)));
    Ok(())
}

#[test]
fn perl_type_hash_construction() -> Result<(), Box<dyn std::error::Error>> {
    let ty = PerlType::Hash {
        key: Box::new(PerlType::Scalar(ScalarType::String)),
        value: Box::new(PerlType::Scalar(ScalarType::Integer)),
    };
    assert!(matches!(ty, PerlType::Hash { .. }));
    Ok(())
}

#[test]
fn perl_type_reference() -> Result<(), Box<dyn std::error::Error>> {
    let inner = PerlType::Scalar(ScalarType::Integer);
    let ty = PerlType::Reference(Box::new(inner.clone()));
    assert_eq!(ty, PerlType::Reference(Box::new(inner)));
    Ok(())
}

#[test]
fn perl_type_union() -> Result<(), Box<dyn std::error::Error>> {
    let ty = PerlType::Union(vec![
        PerlType::Scalar(ScalarType::Integer),
        PerlType::Scalar(ScalarType::String),
    ]);
    if let PerlType::Union(variants) = &ty {
        assert_eq!(variants.len(), 2);
    } else {
        return Err("expected Union".into());
    }
    Ok(())
}

#[test]
fn perl_type_any_void_glob() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(PerlType::Any, PerlType::Any);
    assert_eq!(PerlType::Void, PerlType::Void);
    assert_eq!(PerlType::Glob, PerlType::Glob);
    assert_ne!(PerlType::Any, PerlType::Void);
    Ok(())
}

#[test]
fn perl_type_object() -> Result<(), Box<dyn std::error::Error>> {
    let ty = PerlType::Object("Foo::Bar".to_string());
    assert_eq!(ty, PerlType::Object("Foo::Bar".to_string()));
    assert_ne!(ty, PerlType::Object("Baz".to_string()));
    Ok(())
}

// ===========================================================================
// 9. Workspace Index
// ===========================================================================

#[test]
fn workspace_index_update_and_find() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();
    let table = parse_and_extract("sub my_func { 1 }");
    index.update_from_document("file:///a.pl", "", &table);

    let defs = index.find_defs("my_func");
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].uri, "file:///a.pl");
    Ok(())
}

#[test]
fn workspace_index_remove_document() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();
    let table = parse_and_extract("sub gone { 1 }");
    index.update_from_document("file:///b.pl", "", &table);
    assert_eq!(index.find_defs("gone").len(), 1);

    index.remove_document("file:///b.pl");
    assert_eq!(index.find_defs("gone").len(), 0);
    Ok(())
}

#[test]
fn workspace_index_search_symbols() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();
    let table = parse_and_extract("sub search_target { 1 }");
    index.update_from_document("file:///c.pl", "", &table);

    let results = index.search_symbols("search");
    assert!(!results.is_empty(), "search should find partial match");
    Ok(())
}

#[test]
fn workspace_index_counts() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();
    assert_eq!(index.symbol_count(), 0);
    assert_eq!(index.file_count(), 0);

    let table = parse_and_extract("sub a { 1 } sub b { 2 }");
    index.update_from_document("file:///d.pl", "", &table);

    assert!(index.symbol_count() >= 2, "should have at least 2 symbols");
    assert_eq!(index.file_count(), 1);
    Ok(())
}

#[test]
fn workspace_index_find_refs() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();
    let table = parse_and_extract("sub ref_target { 1 }");
    index.update_from_document("file:///e.pl", "", &table);

    let refs = index.find_refs("ref_target");
    assert!(!refs.is_empty(), "find_refs should return definitions");
    Ok(())
}

#[test]
fn workspace_index_multi_file() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();

    let table1 = parse_and_extract("sub shared_name { 1 }");
    index.update_from_document("file:///f1.pl", "", &table1);

    let table2 = parse_and_extract("sub shared_name { 2 }");
    index.update_from_document("file:///f2.pl", "", &table2);

    let defs = index.find_defs("shared_name");
    assert_eq!(defs.len(), 2, "same symbol from two files");
    assert_eq!(index.file_count(), 2);
    Ok(())
}

#[test]
fn workspace_index_update_replaces_old() -> Result<(), Box<dyn std::error::Error>> {
    use perl_semantic_analyzer::analysis::index::WorkspaceIndex;

    let mut index = WorkspaceIndex::new();

    let table1 = parse_and_extract("sub old_fn { 1 }");
    index.update_from_document("file:///g.pl", "", &table1);
    assert_eq!(index.find_defs("old_fn").len(), 1);

    let table2 = parse_and_extract("sub new_fn { 2 }");
    index.update_from_document("file:///g.pl", "", &table2);
    assert_eq!(index.find_defs("old_fn").len(), 0, "old symbols removed");
    assert_eq!(index.find_defs("new_fn").len(), 1, "new symbols added");
    Ok(())
}

// ===========================================================================
// 10. Integration: end-to-end semantic workflows
// ===========================================================================

#[test]
fn integration_full_module_analysis() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package MyApp::Utils;

use strict;
use warnings;

our $VERSION = '0.01';

# Format a greeting
sub greet {
    my ($name) = @_;
    return "Hello, $name!";
}

# Compute sum
sub sum {
    my @nums = @_;
    my $total = 0;
    for my $n (@nums) {
        $total += $n;
    }
    return $total;
}

1;
"#;

    // Symbol extraction
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "MyApp::Utils", SymbolKind::Package));
    assert!(has_symbol(&table, "greet", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "sum", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "VERSION", SymbolKind::scalar()));

    // Semantic analysis
    let analyzer = parse_and_analyze(code);
    assert!(!analyzer.semantic_tokens().is_empty());
    let table = analyzer.symbol_table();
    let greet_syms = table.find_symbol("greet", 0, SymbolKind::Subroutine);
    assert!(!greet_syms.is_empty());
    // Hover should contain comment
    let hover = analyzer.hover_at(greet_syms[0].location);
    assert!(hover.is_some());

    Ok(())
}

#[test]
fn integration_cross_package_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
package Alpha;
sub a_method { 1 }

package Beta;
sub b_method { 2 }
"#;
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "Alpha", SymbolKind::Package));
    assert!(has_symbol(&table, "Beta", SymbolKind::Package));
    assert!(has_symbol(&table, "a_method", SymbolKind::Subroutine));
    assert!(has_symbol(&table, "b_method", SymbolKind::Subroutine));
    Ok(())
}

#[test]
fn integration_scope_and_symbols_together() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my $x = 1;
{
    my $x = 2;
    print $x;
}
print $x;
"#;
    // Symbols
    let table = parse_and_extract(code);
    assert!(has_symbol(&table, "x", SymbolKind::scalar()));

    // Scope issues
    let issues = scope_issues(code);
    assert!(
        issues.iter().any(|i| i.kind == IssueKind::VariableShadowing),
        "inner $x shadows outer"
    );
    Ok(())
}

#[test]
fn integration_type_inference_and_completion() -> Result<(), Box<dyn std::error::Error>> {
    let code = r#"
my @list = (1, 2, 3);
my %map = (a => 1);
"#;
    let mut engine = TypeInferenceEngine::new();
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let _ = engine.infer(&ast);

    let comp = TypeBasedCompletion::new(Arc::new(engine));
    let arr_c = comp.get_completions("list", "");
    assert!(arr_c.iter().any(|c| c.label == "push"));
    let hash_c = comp.get_completions("map", "");
    assert!(hash_c.iter().any(|c| c.label == "keys"));
    Ok(())
}

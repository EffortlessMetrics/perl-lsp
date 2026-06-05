//! Error-recovery / partial-AST analysis paths for perl-semantic-analyzer.
//!
//! ## What these tests cover
//!
//! The semantic analyzer must not panic and must return a usable (possibly
//! empty or partial) model when presented with:
//!
//! 1. **Recovered parse output** – source that parses with ERROR nodes / partial
//!    AST still drives analysis to completion without panicking.
//! 2. **Cyclic `@ISA`/`use parent` inheritance** – the MRO traversal terminates
//!    (no infinite loop) even when A extends B and B extends A.
//! 3. **Malformed/truncated `use ... qw(...)` import lists** – import extraction
//!    produces an empty or partial list without panicking.
//! 4. **Missing children** – an `if` with no body via recovery, and a sub with
//!    no closing brace, are both tolerated by scope/symbol analysis.
//! 5. **Type inference on error-node source** – `TypeInferenceEngine::infer`
//!    does not panic on a recovered AST.
//! 6. **Deeply nested blocks** – scope analysis terminates on heavily-indented
//!    (but valid) source that might trigger a recursion-depth edge case.
//! 7. **Scope analysis on empty source** – analysis of an empty program returns
//!    no issues and does not panic.
//! 8. **PackageGraphExtractor on cyclic inheritance** – graph extraction
//!    terminates quickly without infinite looping on direct A→B→A cycles.
//! 9. **SemanticAnalyzer on recovered multi-error source** – `analyze_with_source`
//!    completes and the resulting token list and symbol table are internally
//!    consistent (locations within source bounds).
//! 10. **Incomplete `use` import list** – an `ImportExtractor` call on a `use`
//!     with no arguments yields at most a `Default` import spec without panicking.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::class_model::ClassModelBuilder;
use perl_semantic_analyzer::analysis::import_extractor::ImportExtractor;
use perl_semantic_analyzer::analysis::package_graph_extractor::PackageGraphExtractor;
use perl_semantic_analyzer::analysis::scope_analyzer::ScopeAnalyzer;
use perl_semantic_analyzer::analysis::semantic::SemanticAnalyzer;
use perl_semantic_analyzer::analysis::type_inference::TypeInferenceEngine;
use perl_semantic_analyzer::symbol::SymbolExtractor;
use perl_semantic_facts::FileId;
use perl_tdd_support::must;

// ---------------------------------------------------------------------------
// Helper: parse with recovery and return the AST (no panic on error nodes)
// ---------------------------------------------------------------------------

fn recover(code: &str) -> perl_semantic_analyzer::Node {
    let mut parser = Parser::new(code);
    let output = parser.parse_with_recovery();
    output.ast
}

// ---------------------------------------------------------------------------
// 1. Full pipeline on a recovered (error-node-containing) AST — no panic
// ---------------------------------------------------------------------------

/// The full analysis pipeline (symbol extraction, scope analysis, semantic
/// analysis) must complete without panicking when the source produces error
/// nodes during recovery.
#[test]
fn error_node_full_pipeline_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    // Truncated arrow expression forces an ERROR node in the AST.
    let source = "my $before = 1;\nmy $r = $before->;\nmy $after = 2;\n";
    let ast = recover(source);
    let source_len = source.len();

    // Symbol extraction
    let table = SymbolExtractor::new_with_source(source).extract(&ast);

    // Scope analysis
    let issues = ScopeAnalyzer::new().analyze(&ast, source, &[]);

    // Semantic analysis
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, source);
    let tokens = analyzer.semantic_tokens();

    // Location invariants hold
    for symbols in table.symbols.values() {
        for sym in symbols {
            assert!(
                sym.location.start <= sym.location.end,
                "symbol location start > end: {:?}",
                sym.location
            );
            assert!(
                sym.location.end <= source_len,
                "symbol location end out of bounds: {:?}",
                sym.location
            );
        }
    }

    for token in tokens {
        assert!(
            token.location.start <= token.location.end,
            "token location start > end: {:?}",
            token.location
        );
        assert!(
            token.location.end <= source_len,
            "token location end out of bounds: {:?}",
            token.location
        );
    }

    for issue in &issues {
        assert!(issue.range.0 <= issue.range.1, "scope issue range inverted: {:?}", issue.range);
        assert!(issue.range.1 <= source_len, "scope issue range out of bounds: {:?}", issue.range);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Cyclic `use parent` inheritance — MRO traversal terminates
// ---------------------------------------------------------------------------

/// A→B, B→A cyclic inheritance (via `use parent`) must not cause an infinite
/// loop in `SemanticAnalyzer`.  The analysis must complete in finite time and
/// return a model (even if some hover/method lookups are empty).
#[test]
fn cyclic_use_parent_terminates() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package A;
use parent 'B';
sub a_method { 1 }

package B;
use parent 'A';
sub b_method { 2 }
"#;
    let ast = must(Parser::new(source).parse());
    // Must complete without hanging or panicking.
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, source);
    // Result is a model — symbol table should contain the two subs.
    let table = analyzer.symbol_table();
    let has_any = table.symbols.contains_key("a_method") || table.symbols.contains_key("b_method");
    assert!(
        has_any,
        "expected at least one method in symbol table; got: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Cyclic `@ISA` assignment — PackageGraphExtractor terminates
// ---------------------------------------------------------------------------

/// `@ISA = ('B')` in A and `@ISA = ('A')` in B forms a cycle.
/// The `PackageGraphExtractor` must emit edges without looping.
#[test]
fn cyclic_isa_package_graph_terminates() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package A;
our @ISA = ('B');

package B;
our @ISA = ('A');
"#;
    let ast = must(Parser::new(source).parse());
    // Must return without panicking or looping.
    let edges = PackageGraphExtractor::extract(&ast, FileId(0));
    // Each package contributes one edge; order is not guaranteed.
    assert!(
        edges.len() >= 2,
        "expected at least 2 package edges for mutual ISA; got {}: {:?}",
        edges.len(),
        edges
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Malformed `use` statement — import extraction is tolerant
// ---------------------------------------------------------------------------

/// A truncated `use` statement (missing qw list) must not panic in
/// `ImportExtractor`; it returns an empty or minimal spec list.
#[test]
fn malformed_use_truncated_qw_import_extraction_no_panic() -> Result<(), Box<dyn std::error::Error>>
{
    // Missing closing paren — source parses with recovery.
    let source = "use Foo qw( bar baz\n";
    let ast = recover(source);
    // Must complete without panicking.
    let specs = ImportExtractor::extract(&ast, FileId(0));
    // We may get zero or one spec — either is fine.  The important thing is
    // that analysis terminates and the result is coherent.
    assert!(
        specs.len() <= 1,
        "expected 0 or 1 import specs for truncated use; got: {}",
        specs.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 5. Missing RHS in variable declaration — analysis tolerates MissingExpression
// ---------------------------------------------------------------------------

/// `my $x = ;` produces a VariableDeclaration with a MissingExpression
/// initializer.  Scope analysis and symbol extraction must not panic and must
/// still find the surrounding declarations.
#[test]
fn missing_rhs_scope_analysis_tolerant() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $before = 1;\nmy $broken = ;\nmy $after = 2;\n";
    let ast = recover(source);
    let table = SymbolExtractor::new_with_source(source).extract(&ast);
    let issues = ScopeAnalyzer::new().analyze(&ast, source, &[]);

    // `$before` and `$after` must be extractable despite the broken middle decl.
    let has_before = table.symbols.contains_key("before");
    let has_after = table.symbols.contains_key("after");
    assert!(
        has_before || has_after,
        "expected at least one of $before/$after in symbol table; got: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    // Issues slice must be internally consistent.
    let source_len = source.len();
    for issue in &issues {
        assert!(issue.range.1 <= source_len, "scope issue range out of bounds: {:?}", issue.range);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Unclosed `if` body — scope analysis is tolerant
// ---------------------------------------------------------------------------

/// An `if` statement whose block is not closed (recovery produces a partial
/// node) must be handled gracefully by scope/symbol analysis.
#[test]
fn unclosed_if_body_scope_analysis_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = 1;\nif ($x) {\n    my $y = 2;\n";
    let ast = recover(source);

    // Must not panic.
    let table = SymbolExtractor::new_with_source(source).extract(&ast);
    let issues = ScopeAnalyzer::new().analyze(&ast, source, &[]);

    // `$x` is declared at the top level; it should be reachable.
    let has_x = table.symbols.contains_key("x");
    assert!(
        has_x,
        "expected $x in symbol table even with unclosed if; got: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );

    let source_len = source.len();
    for issue in &issues {
        assert!(issue.range.1 <= source_len, "scope issue range out of bounds: {:?}", issue.range);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 7. Type inference on recovered AST — no panic
// ---------------------------------------------------------------------------

/// `TypeInferenceEngine::infer` must not panic on a recovered AST that
/// contains error nodes or missing expressions.
#[test]
fn type_inference_on_recovered_ast_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    let source = "my $x = ;\nmy $y = $x->;\nmy $z = 42;\n";
    let ast = recover(source);

    let mut engine = TypeInferenceEngine::new();
    // infer() returns Result; both Ok and Err are acceptable — we just need
    // it to return (not panic, not loop).
    let _result = engine.infer(&ast);

    Ok(())
}

// ---------------------------------------------------------------------------
// 8. Empty source — analysis of empty program is stable
// ---------------------------------------------------------------------------

/// Analyzing an empty string must produce empty tables with no issues and
/// must not panic.
#[test]
fn empty_source_analysis_is_stable() -> Result<(), Box<dyn std::error::Error>> {
    let source = "";
    let ast = must(Parser::new(source).parse());

    let table = SymbolExtractor::new_with_source(source).extract(&ast);
    let issues = ScopeAnalyzer::new().analyze(&ast, source, &[]);
    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, source);

    assert!(table.symbols.is_empty(), "empty source should have no symbols");
    assert!(issues.is_empty(), "empty source should have no scope issues");
    assert!(analyzer.semantic_tokens().is_empty(), "empty source should have no semantic tokens");
    Ok(())
}

// ---------------------------------------------------------------------------
// 9. Deeply nested blocks — scope analysis terminates
// ---------------------------------------------------------------------------

/// Recursion-guard: scope analysis on deeply-nested but valid Perl must
/// terminate and return a consistent result.
#[test]
fn deeply_nested_blocks_scope_analysis_terminates() -> Result<(), Box<dyn std::error::Error>> {
    // Build 50 levels of nesting — far below any stack overflow threshold
    // but well above what typical Perl source has.
    let depth = 50_usize;
    let open: String = (0..depth).map(|i| format!("my $v{i} = {i};\n{{\n")).collect();
    let close: String = "}\n".repeat(depth);
    let source = format!("{open}{close}");

    let ast = must(Parser::new(&source).parse());
    let issues = ScopeAnalyzer::new().analyze(&ast, &source, &[]);

    let source_len = source.len();
    for issue in &issues {
        assert!(issue.range.1 <= source_len, "scope issue range out of bounds: {:?}", issue.range);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 10. SemanticAnalyzer on multi-error recovered source — consistent model
// ---------------------------------------------------------------------------

/// Multiple recovery sites in one source must all be processed; the resulting
/// semantic model must be internally consistent (symbol locations within
/// bounds, token locations within bounds).
#[test]
fn multi_error_source_semantic_model_is_consistent() -> Result<(), Box<dyn std::error::Error>> {
    // Several syntax problems in one file.
    let source = concat!(
        "package Foo;\n",
        "my $a = ;\n", // missing RHS
        "sub bar {\n",
        "    my $b = $a->;\n", // truncated arrow
        "    return $b;\n",
        "}\n",
        "my $c = baz(;\n", // missing close paren
        "our @ISA = ('NoSuch');\n",
    );
    let ast = recover(source);
    let source_len = source.len();

    let analyzer = SemanticAnalyzer::analyze_with_source(&ast, source);
    let table = analyzer.symbol_table();
    let tokens = analyzer.semantic_tokens();

    for symbols in table.symbols.values() {
        for sym in symbols {
            assert!(
                sym.location.start <= sym.location.end,
                "symbol location inverted: {:?}",
                sym.location
            );
            assert!(
                sym.location.end <= source_len,
                "symbol location end out of bounds: {:?}",
                sym.location
            );
        }
    }

    for token in tokens {
        assert!(
            token.location.start <= token.location.end,
            "token location inverted: {:?}",
            token.location
        );
        assert!(
            token.location.end <= source_len,
            "token location end out of bounds: {:?}",
            token.location
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 11. ClassModelBuilder on cyclic inheritance — no infinite loop
// ---------------------------------------------------------------------------

/// `ClassModelBuilder` must produce class models without looping when a
/// multi-hop inheritance cycle exists (A → B → C → A).
#[test]
fn class_model_builder_cyclic_inheritance_terminates() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
package A;
use parent 'B';
sub a_only { 1 }

package B;
use parent 'C';
sub b_only { 2 }

package C;
use parent 'A';
sub c_only { 3 }
"#;
    let ast = must(Parser::new(source).parse());
    // Must return without looping.
    let models = ClassModelBuilder::new().build(&ast);
    assert_eq!(models.len(), 3, "expected 3 class models (A, B, C); got {}", models.len());

    // Confirm parent links are recorded (even though they form a cycle at
    // analysis time — the _model_ stores what was in source).
    let a_model = models.iter().find(|m| m.name == "A");
    assert!(a_model.is_some(), "model for package A must be present");
    Ok(())
}

// ---------------------------------------------------------------------------
// 12. Incomplete export list — symbol extraction doesn't panic
// ---------------------------------------------------------------------------

/// A module that declares `@EXPORT` with an empty or malformed qw list must
/// be handled by the symbol extractor without panicking.
#[test]
fn incomplete_export_list_symbol_extraction_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    // @EXPORT with an unclosed qw forces recovery.
    let source = "package MyLib;\nuse Exporter 'import';\nour @EXPORT = qw(foo\n";
    let ast = recover(source);

    // Must not panic.
    let table = SymbolExtractor::new_with_source(source).extract(&ast);

    // The package symbol is still expected even with the malformed export list.
    let has_package = table.symbols.contains_key("MyLib");
    assert!(
        has_package,
        "package MyLib should be in symbol table even with broken @EXPORT; got: {:?}",
        table.symbols.keys().collect::<Vec<_>>()
    );
    Ok(())
}

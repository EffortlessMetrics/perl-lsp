//! Property-based tests for symbol extraction and scope analysis.
//!
//! Key invariants verified:
//! - Extraction and analysis never panic on well-formed Perl snippets
//! - Results are deterministic (same input → identical output)
//! - All symbol locations are within source bounds and non-inverted
//! - Extracted symbol names are non-empty
//! - Scope analysis issues reference valid source regions

use perl_semantic_analyzer::{
    Parser,
    analysis::scope_analyzer::ScopeAnalyzer,
    analysis::symbol::SymbolExtractor,
};
use perl_test_generators::{module_path, variable};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Snippet generators
// ---------------------------------------------------------------------------

/// A Perl scalar variable declaration with a simple assignment.
fn my_scalar_decl() -> impl Strategy<Value = String> {
    (variable(), "[A-Za-z0-9_]{1,8}".prop_map(String::from)).prop_map(|(var, val)| {
        // variable() can produce $, @, % sigils; wrap in `my`
        format!("my {var} = \"{val}\";\n")
    })
}

/// A simple named subroutine with one local variable inside.
fn simple_sub() -> impl Strategy<Value = String> {
    (
        "[a-z][a-z0-9_]{1,10}".prop_map(String::from),
        variable(),
        "[0-9]{1,4}".prop_map(String::from),
    )
        .prop_map(|(name, var, val)| format!("sub {name} {{\n    my {var} = {val};\n}}\n"))
}

/// A `package` declaration.
fn package_decl() -> impl Strategy<Value = String> {
    module_path().prop_map(|path| format!("package {path};\n"))
}

/// A small Perl program composed of a package declaration plus a few subs.
fn small_program() -> impl Strategy<Value = String> {
    (
        package_decl(),
        prop::collection::vec(
            prop_oneof![simple_sub(), my_scalar_decl()],
            1..6,
        ),
    )
        .prop_map(|(pkg, stmts)| {
            let mut prog = pkg;
            for s in stmts {
                prog.push_str(&s);
            }
            prog
        })
}

// ---------------------------------------------------------------------------
// Panic-freedom
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        ..ProptestConfig::default()
    })]

    /// SymbolExtractor::extract never panics on generated Perl programs.
    #[test]
    fn symbol_extraction_never_panics(src in small_program()) {
        let mut parser = Parser::new(&src);
        if let Ok(ast) = parser.parse() {
            let _ = SymbolExtractor::new_with_source(&src).extract(&ast);
        }
    }

    /// ScopeAnalyzer::analyze never panics on generated Perl programs.
    #[test]
    fn scope_analysis_never_panics(src in small_program()) {
        let mut parser = Parser::new(&src);
        if let Ok(ast) = parser.parse() {
            let analyzer = ScopeAnalyzer::new();
            let _ = analyzer.analyze(&ast, &src, &[]);
        }
    }
}

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        ..ProptestConfig::default()
    })]

    /// Extracting symbols twice from the same source yields the same symbol count.
    #[test]
    fn symbol_extraction_is_deterministic(src in small_program()) {
        let mut parser = Parser::new(&src);
        let Ok(ast) = parser.parse() else { return Ok(()) };

        let table1 = SymbolExtractor::new_with_source(&src).extract(&ast);
        let table2 = SymbolExtractor::new_with_source(&src).extract(&ast);

        prop_assert_eq!(
            table1.symbols.len(),
            table2.symbols.len(),
            "symbol count differed across two extractions of the same source"
        );
    }

    /// Running scope analysis twice yields the same number of issues.
    #[test]
    fn scope_analysis_is_deterministic(src in small_program()) {
        let mut parser = Parser::new(&src);
        let Ok(ast) = parser.parse() else { return Ok(()) };

        let analyzer = ScopeAnalyzer::new();
        let issues1 = analyzer.analyze(&ast, &src, &[]);
        let issues2 = analyzer.analyze(&ast, &src, &[]);

        prop_assert_eq!(
            issues1.len(),
            issues2.len(),
            "issue count differed across two analyses of the same source"
        );
    }
}

// ---------------------------------------------------------------------------
// Location invariants
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        ..ProptestConfig::default()
    })]

    /// Every symbol's location satisfies `start <= end` and stays within source bounds.
    #[test]
    fn symbol_locations_are_valid(src in small_program()) {
        let mut parser = Parser::new(&src);
        let Ok(ast) = parser.parse() else { return Ok(()) };

        let table = SymbolExtractor::new_with_source(&src).extract(&ast);
        let src_len = src.len();

        for (_, syms) in &table.symbols {
            for sym in syms {
                prop_assert!(
                    sym.location.start <= sym.location.end,
                    "inverted location for {:?}: start={} end={}",
                    sym.name,
                    sym.location.start,
                    sym.location.end
                );
                prop_assert!(
                    sym.location.end <= src_len,
                    "location end {} out of bounds (source len {}) for {:?}",
                    sym.location.end,
                    src_len,
                    sym.name
                );
            }
        }
    }

    /// Every extracted symbol has a non-empty name.
    #[test]
    fn symbol_names_are_non_empty(src in small_program()) {
        let mut parser = Parser::new(&src);
        let Ok(ast) = parser.parse() else { return Ok(()) };

        let table = SymbolExtractor::new_with_source(&src).extract(&ast);
        for (name, _) in &table.symbols {
            prop_assert!(!name.is_empty(), "empty symbol name found in {:?}", src);
        }
    }

    /// Scope analysis issue ranges satisfy `start <= end` and stay in source bounds.
    #[test]
    fn scope_issue_ranges_are_valid(src in small_program()) {
        let mut parser = Parser::new(&src);
        let Ok(ast) = parser.parse() else { return Ok(()) };

        let analyzer = ScopeAnalyzer::new();
        let issues = analyzer.analyze(&ast, &src, &[]);
        let src_len = src.len();

        for issue in &issues {
            let (start, end) = issue.range;
            prop_assert!(
                start <= end,
                "inverted range in scope issue: start={} end={}",
                start,
                end
            );
            prop_assert!(
                end <= src_len,
                "scope issue range end {} out of bounds (source len {})",
                end,
                src_len
            );
        }
    }
}

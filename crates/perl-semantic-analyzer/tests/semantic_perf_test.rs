//! Performance test for semantic analysis
//! Run with: cargo test -p perl-semantic-analyzer --test semantic_perf_test -- --nocapture --ignored

use perl_semantic_analyzer::{Parser, semantic::{SemanticAnalyzer, SemanticTokenType, SemanticTokenModifier}};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_semantic_analysis_builtins() {
    let mut code = String::from("use strict;\nuse warnings;\n");

    // Generate many calls to builtins and non-builtins
    // We want a mix to exercise the branch prediction and lookup logic
    for i in 0..10000 {
        code.push_str("print \"hello\";\n");
        code.push_str("say \"world\";\n");
        code.push_str("my $x = abs(-1);\n");
        code.push_str("defined($x);\n");
        code.push_str("undef($x);\n");
        code.push_str("blessed($x);\n"); // Test the special case
        code.push_str(&format!("sub test_{} {{ return 1; }}\n", i));
        code.push_str(&format!("test_{}();\n", i));
    }

    // Warmup
    for _ in 0..3 {
        let mut parser = Parser::new(&code);
        if let Ok(ast) = parser.parse() {
            let _analyzer = SemanticAnalyzer::analyze(&ast);
        }
    }

    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");

    let start = Instant::now();
    let _analyzer = SemanticAnalyzer::analyze(&ast);
    let duration = start.elapsed();

    println!("Semantic analysis (10k iterations of 7 statements + sub def) took: {:?}", duration);
}

#[test]
fn test_blessed_is_builtin() {
    let code = r#"
use Scalar::Util qw(blessed);
blessed($x);
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse failed");
    let analyzer = SemanticAnalyzer::analyze(&ast);

    // Find the token for 'blessed'
    let tokens = analyzer.semantic_tokens();
    let _blessed_token = tokens.iter().find(|t| {
        // Find the token corresponding to the function call 'blessed'
        // 'blessed' starts at line 3.
        // Approximate location check or just token type check if unique enough
        // simpler: check if ANY token is blessed and has type Function and modifier DefaultLibrary
        // But we need to be sure it's the function call.
        // Let's just iterate and check.
        matches!(t.token_type, SemanticTokenType::Function) &&
        t.modifiers.contains(&SemanticTokenModifier::DefaultLibrary)
    });

    // In the current implementation (before change), blessed is in the match list, so it should be a Function + DefaultLibrary.
    // Wait, let's verify if 'DefaultLibrary' modifier is applied.
    // semantic.rs:
    // modifiers: if is_builtin_function(name) && !is_control_keyword(name) {
    //    vec![SemanticTokenModifier::DefaultLibrary]
    // }

    // So if blessed is in is_builtin_function, it gets DefaultLibrary.

    // Note: The analyzer generates tokens for the USE statement too, but those are Namespace/Keyword.
    // The call `blessed($x)` is a FunctionCall.

    // We need to ensure we found it.
    // Let's count how many DefaultLibrary functions we found.
    let builtin_calls = tokens.iter().filter(|t|
        t.token_type == SemanticTokenType::Function &&
        t.modifiers.contains(&SemanticTokenModifier::DefaultLibrary)
    ).count();

    // There should be at least one (blessed).
    assert!(builtin_calls >= 1, "Should find 'blessed' as a builtin function");
}

#[test]
fn test_new_builtins_recognized() {
    let code = r#"
my $x = abs(-5);
my $y = cos(0);
mkdir("foo");
"#;
    let mut parser = Parser::new(code);
    let ast = parser.parse().expect("parse failed");
    let analyzer = SemanticAnalyzer::analyze(&ast);

    let tokens = analyzer.semantic_tokens();

    // Check if 'abs', 'cos', 'mkdir' are recognized as Function + DefaultLibrary
    // Before optimization, these were NOT recognized.

    let builtin_count = tokens.iter().filter(|t|
        t.token_type == SemanticTokenType::Function &&
        t.modifiers.contains(&SemanticTokenModifier::DefaultLibrary)
    ).count();

    assert!(builtin_count >= 3, "Should recognize abs, cos, mkdir as builtins (found {})", builtin_count);
}

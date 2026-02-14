use perl_parser_core::{Parser, ast::NodeKind, pragma_tracker::{PragmaState, PragmaTracker}};
use perl_semantic_analyzer::scope_analyzer::{ScopeAnalyzer, IssueKind};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_scope_analysis() {
    // Generate large test code with many builtin calls
    // Note: The code itself has 'use strict', but we need to pass a PragmaMap that reflects it
    // or parse it. For benchmark stability, we'll construct a fixed PragmaMap.
    let mut code = String::from("use strict;\nuse warnings;\n");

    // Generate 5000 lines of builtin calls
    for i in 0..5000 {
        // Mix of builtins (both common and rare) and potential barewords
        code.push_str(&format!(
            "print \"hello\";\n\
             my $x_{} = abs(-1);\n\
             open(my $fh_{}, '<', 'file');\n\
             close $fh_{};\n\
             keys %hash;\n\
             values %hash;\n\
             defined $x_{};\n\
             scalar @array;\n\
             mkdir 'dir', 0755;\n\
             chmod 0644, 'file';\n\
             socket(my $sock_{}, 1, 2, 3);\n\
             sysopen(my $sysfh_{}, 'file', 0);\n\
             sysclose($sysfh_{});\n",
            i, i, i, i, i, i, i
        ));
    }

    println!("Code size: {} bytes", code.len());

    let mut parser = Parser::new(&code);
    let ast = parser.parse().expect("Failed to parse");

    // Construct a pragma map that enforces strict subs (which triggers is_known_function checks)
    let mut state = PragmaState::default();
    state.strict_subs = true;
    let pragma_map = vec![(0..code.len(), state)];

    // Warm up
    let analyzer = ScopeAnalyzer::new();
    analyzer.analyze(&ast, &code, &pragma_map);

    let iterations = 10;
    let start = Instant::now();

    for _ in 0..iterations {
        analyzer.analyze(&ast, &code, &pragma_map);
    }

    let duration = start.elapsed();
    let avg_time = duration / iterations;

    println!("Average scope analysis time: {:?}", avg_time);

    // Check for issues - specifically unquoted barewords which indicate missing builtins
    let issues = analyzer.analyze(&ast, &code, &pragma_map);
    let barewords: Vec<_> = issues.iter()
        .filter(|i| matches!(i.kind, IssueKind::UnquotedBareword))
        .map(|i| i.variable_name.as_str())
        .collect();

    println!("Found {} unquoted barewords", barewords.len());
    if !barewords.is_empty() {
        println!("First 10 barewords: {:?}", barewords.iter().take(10).collect::<Vec<_>>());
    }
}

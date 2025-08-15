use perl_parser::{Parser, PragmaTracker, scope_analyzer::ScopeAnalyzer};

fn main() {
    let source = r#"
use strict;

print FOO;  # Bareword not allowed
"#;

    let mut parser = Parser::new(source);
    let result = parser.parse();

    match result {
        Ok(ast) => {
            println!("AST parsed successfully");

            // Build pragma map
            let pragma_map = PragmaTracker::build(&ast);
            println!("Pragma map built with {} entries", pragma_map.len());

            // Run scope analyzer
            let analyzer = ScopeAnalyzer::new();
            let issues = analyzer.analyze(&ast, source, &pragma_map);

            println!("\nScope analyzer found {} issues:", issues.len());
            for issue in &issues {
                println!("  {:?}: {} ({})", issue.kind, issue.variable_name, issue.description);
            }
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
}

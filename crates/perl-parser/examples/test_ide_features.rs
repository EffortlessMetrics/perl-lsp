//! Test IDE features
//!
//! This example tests the new diagnostics and code actions features.

use perl_parser::{
    Parser,
    DiagnosticsProvider,
    CodeActionsProvider,
};

fn main() {
    println!("=== Test IDE Features ===\n");
    
    // Sample code with various issues
    let source = r#"
print $undefined;

my $unused = 42;

if ($x = 5) {
    print "x is 5\n";
}
"#;

    // Parse the code
    let mut parser = Parser::new(source);
    match parser.parse() {
        Ok(ast) => {
            println!("✓ Successfully parsed the code");
            
            // Get diagnostics
            let diagnostics_provider = DiagnosticsProvider::new(&ast, source.to_string());
            let diagnostics = diagnostics_provider.get_diagnostics(&ast, &[], source);
            
            println!("\nDiagnostics found: {}", diagnostics.len());
            for diag in &diagnostics {
                println!("- [{}] {}", diag.code.as_ref().unwrap_or(&"unknown".to_string()), diag.message);
                println!("  Range: {:?}", diag.range);
            }
            
            // Get code actions
            let code_actions_provider = CodeActionsProvider::new(source.to_string());
            let actions = code_actions_provider.get_code_actions(&ast, (0, source.len()), &diagnostics);
            
            println!("\nCode actions available: {}", actions.len());
            for action in &actions {
                println!("- {}", action.title);
                println!("  Fixes: {:?}", action.diagnostics);
            }
        }
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
        }
    }
}
use perl_parser::incremental_integration::{DocumentParser, IncrementalConfig};

fn main() {
    println!("Testing incremental parsing AST generation...\n");
    
    // Test with incremental disabled
    {
        println!("=== Testing with incremental DISABLED ===");
        let config = IncrementalConfig {
            enabled: false,
            target_parse_time_ms: 1.0,
            max_cache_size: 10000,
        };
        
        let code = "my $x = 42;";
        let doc = DocumentParser::new(code.to_string(), &config).unwrap();
        
        if let Some(ast) = doc.ast() {
            println!("AST Debug: {:?}", ast);
            println!("S-expr: {}", ast.to_sexp());
            println!("Contains ScalarVariable: {}", format!("{:?}", ast).contains("ScalarVariable"));
        } else {
            println!("No AST generated");
        }
    }
    
    println!("\n=== Testing with incremental ENABLED ===");
    
    // Test with incremental enabled
    unsafe { std::env::set_var("PERL_LSP_INCREMENTAL", "1") };
    let config = IncrementalConfig::default();
    
    let code = "my $x = 42;";
    let doc = DocumentParser::new(code.to_string(), &config).unwrap();
    
    if let Some(ast) = doc.ast() {
        println!("AST Debug: {:?}", ast);
        println!("S-expr: {}", ast.to_sexp());
        println!("Contains ScalarVariable: {}", format!("{:?}", ast).contains("ScalarVariable"));
    } else {
        println!("No AST generated");
    }
}
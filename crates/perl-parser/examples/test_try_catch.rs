//! Test try/catch blocks
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Basic try/catch
        "try {
            risky_operation();
        } catch ($e) {
            warn $e;
        }",
        
        // Try without catch
        "try {
            something();
        }",
        
        // Try/catch with finally
        "try {
            open_file();
        } catch ($e) {
            log_error($e);
        } finally {
            cleanup();
        }",
        
        // Nested try/catch
        "try {
            try {
                inner();
            } catch ($inner_e) {
                handle_inner($inner_e);
            }
        } catch ($outer_e) {
            handle_outer($outer_e);
        }",
        
        // Try in expression context
        "my $result = try { compute() } catch ($e) { default_value() }",
        
        // Multiple catch blocks (not standard Perl, but some modules support it)
        "try {
            operation();
        } catch ($e isa MyException) {
            handle_my_exception($e);
        } catch ($e) {
            handle_generic($e);
        }",
    ];
    
    for test in tests {
        println!("\nTesting: {}", test.lines().next().unwrap_or(""));
        let mut parser = Parser::new(test);
        match parser.parse() {
            Ok(ast) => {
                println!("✅ Success!");
                println!("   S-expr: {}", ast.to_sexp());
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }
}
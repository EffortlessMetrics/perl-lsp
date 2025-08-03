//! Demonstrate LSP server capabilities

use perl_parser::{Parser, NodeKind};

fn main() {
    println!("=== Perl LSP Server Capabilities Demo ===\n");
    
    println!("The Perl LSP server (perl-lsp) provides the following features:\n");
    
    println!("1. SYNTAX DIAGNOSTICS");
    println!("   - Real-time error detection");
    println!("   - Undefined variable warnings");
    println!("   - Syntax error highlighting");
    
    let error_code = r#"my $x = 42;
$undefined_var = 10;  # Error: undefined variable"#;
    
    println!("\n   Example code with error:");
    println!("   {}", error_code);
    
    let mut parser = Parser::new(error_code);
    match parser.parse() {
        Ok(_) => println!("   ✓ Parsed successfully (but would show warning in LSP)"),
        Err(e) => println!("   ✗ Parse error: {:?}", e),
    }
    
    println!("\n2. DOCUMENT SYMBOLS");
    println!("   - Outline view of subroutines");
    println!("   - Package declarations");
    println!("   - Variable declarations");
    
    let symbol_code = r#"package MyModule;

sub process_data {
    my ($input) = @_;
    return $input * 2;
}

my $result = process_data(21);"#;
    
    println!("\n   Example code:");
    println!("   {}", symbol_code);
    
    // Parse and show what symbols would be extracted
    let mut parser = Parser::new(symbol_code);
    if let Ok(ast) = parser.parse() {
        println!("\n   Symbols found:");
        extract_symbols(&ast, "   ");
    }
    
    println!("\n3. SIGNATURE HELP");
    println!("   - Parameter hints while typing");
    println!("   - Shows function signatures");
    println!("   - Highlights active parameter");
    
    println!("\n   Example: When typing 'substr($str, |)' the LSP shows:");
    println!("   substr(EXPR, OFFSET, LENGTH, REPLACEMENT)");
    println!("          ^^^^^^^^^^^^");
    println!("   (active parameter highlighted)");
    
    println!("\n4. SEMANTIC TOKENS");
    println!("   - Enhanced syntax highlighting");
    println!("   - Different colors for:");
    println!("     • Keywords (my, sub, return)");
    println!("     • Variables ($scalar, @array, %hash)");
    println!("     • Functions (print, substr)");
    println!("     • Operators (=, +, ->)");
    println!("     • Strings and numbers");
    
    println!("\n5. GO TO DEFINITION");
    println!("   - Navigate to where symbols are defined");
    println!("   - Jump to subroutine definitions");
    println!("   - Find variable declarations");
    
    println!("\n6. FIND REFERENCES");
    println!("   - Find all uses of a variable");
    println!("   - Find all calls to a function");
    println!("   - Helpful for refactoring");
    
    println!("\n7. INCREMENTAL PARSING");
    println!("   - Efficient updates as you type");
    println!("   - Only re-parses changed portions");
    println!("   - Fast response times");
    
    println!("\n{}", "=".repeat(50));
    println!("\nTo use the LSP server:");
    println!("1. Build: cargo build -p perl-parser --bin perl-lsp --release");
    println!("2. Configure your editor to use: perl-lsp --stdio");
    println!("3. Enjoy enhanced Perl development!");
}

fn extract_symbols(node: &perl_parser::ast::Node, indent: &str) {
    match &node.kind {
        NodeKind::Package { name, .. } => {
            println!("{}  - Package: {}", indent, name);
        }
        NodeKind::Subroutine { name, .. } => {
            if let Some(name) = name {
                println!("{}  - Subroutine: {}", indent, name);
            }
        }
        NodeKind::VariableDeclaration { variable, .. } => {
            if let NodeKind::Variable { sigil, name } = &variable.kind {
                println!("{}  - Variable: {}{}", indent, sigil, name);
            }
        }
        NodeKind::Program { statements } => {
            for stmt in statements {
                extract_symbols(stmt, indent);
            }
        }
        NodeKind::Block { statements } => {
            for stmt in statements {
                extract_symbols(stmt, indent);
            }
        }
        _ => {}
    }
}
//! Demonstration of default value substitution logging
//!
//! This example shows how the parser logs when default values are substituted.
//! Run with: cargo run --package perl-parser-core --example demonstrate_logging
//!
//! To see the logs, set RUST_LOG environment variable:
//! RUST_LOG=perl_parser_core=debug cargo run --package perl-parser-core --example demonstrate_logging

use perl_parser_core::Parser;

fn main() {
    // Initialize tracing subscriber to capture logs
    tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::DEBUG)
        .init();

    println!("=== Demonstration of Default Value Substitution Logging ===\n");

    // Test 1: Return without value (logs default end position)
    println!("Test 1: Return without value");
    let code = "sub foo { return; }";
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(_) => println!("✓ Parsed successfully\n"),
        Err(e) => println!("✗ Parse error: {:?}\n", e),
    }

    // Test 2: qw with unclosed delimiter (logs default by using rest of content)
    println!("Test 2: qw with unclosed delimiter");
    let code = "my @arr = qw(foo bar";
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(_) => println!("✓ Parsed successfully\n"),
        Err(e) => println!("✗ Parse error: {:?}\n", e),
    }

    // Test 3: Try block end position (logs when using body.location.end)
    println!("Test 3: Try block without catch/finally");
    let code = "try { my $x = 1; }";
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(_) => println!("✓ Parsed successfully\n"),
        Err(e) => println!("✗ Parse error: {:?}\n", e),
    }

    // Test 4: Position at end of input
    println!("Test 4: Simple variable declaration");
    let code = "my $x = 1;";
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(_) => println!("✓ Parsed successfully\n"),
        Err(e) => println!("✗ Parse error: {:?}\n", e),
    }

    println!("\n=== Check the log output above for debug messages ===");
    println!("Messages starting with 'Substituted default' indicate where defaults were used.");
}

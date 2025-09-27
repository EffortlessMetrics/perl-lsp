use perl_parser::Parser;

fn main() {
    println!("=== Testing Crash Reproducer ===");

    // This is the crash case found in perl-corpus/fuzz/
    let crash_input = "xqN<<\"";

    println!("Testing crash reproducer input: {}", crash_input);

    let result = std::panic::catch_unwind(|| {
        let mut parser = Parser::new(crash_input);
        parser.parse()
    });

    match result {
        Ok(_) => {
            println!("✅ SUCCESS: Crash reproducer no longer crashes!");
            println!("The boundary fix has resolved this specific vulnerability.");
        }
        Err(_) => {
            println!("❌ FAILURE: Crash reproducer still causes panic!");
            println!("The boundary fix may not be complete.");
        }
    }
}
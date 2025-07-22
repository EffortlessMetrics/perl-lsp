//! Test a single category to analyze failures
use perl_parser::Parser;
use std::env;

mod edge_cases {
    pub mod format_and_blocks;
    pub mod operator_overloading;
    pub mod indirect_and_methods;
    pub mod versions_and_vstrings;
    pub mod unicode_and_encoding;
    pub mod file_io_operations;
    pub mod regex_and_patterns;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let category = args.get(1).map(|s| s.as_str()).unwrap_or("format");
    
    let tests = match category {
        "format" => edge_cases::format_and_blocks::get_tests(),
        "operator" => edge_cases::operator_overloading::get_tests(),
        "indirect" => edge_cases::indirect_and_methods::get_tests(),
        "version" => edge_cases::versions_and_vstrings::get_tests(),
        "unicode" => edge_cases::unicode_and_encoding::get_tests(),
        "file" => edge_cases::file_io_operations::get_tests(),
        "regex" => edge_cases::regex_and_patterns::get_tests(),
        _ => {
            println!("Unknown category: {}", category);
            return;
        }
    };
    
    println!("Testing category: {}", category);
    println!("Total tests: {}", tests.len());
    println!();
    
    let mut passed = 0;
    let mut failed = 0;
    
    for (i, (code, desc)) in tests.iter().enumerate() {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => {
                passed += 1;
                if i < 5 || failed == 0 {
                    println!("✅ {}", desc);
                }
            }
            Err(e) => {
                failed += 1;
                println!("❌ {}", desc);
                println!("   Code: {}", code.lines().next().unwrap_or(code));
                println!("   Error: {:?}", e);
                println!();
            }
        }
    }
    
    println!("\nSummary: {} passed, {} failed ({:.1}%)", 
             passed, 
             failed,
             (passed as f64 / (passed + failed) as f64) * 100.0);
}
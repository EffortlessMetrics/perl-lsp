//! Quick summary of edge case test results
use perl_parser::Parser;

mod edge_cases {
    pub mod file_io_operations;
    pub mod format_and_blocks;
    pub mod indirect_and_methods;
    pub mod operator_overloading;
    pub mod regex_and_patterns;
    pub mod unicode_and_encoding;
    pub mod versions_and_vstrings;
}

fn test_category(name: &str, tests: Vec<(&str, &str)>) -> (usize, usize) {
    let mut passed = 0;
    let mut failed = 0;

    for (code, _desc) in tests {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => passed += 1,
            Err(_) => failed += 1,
        }
    }

    let total = passed + failed;
    let percentage = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        100.0
    };

    println!(
        "{:<25} {:>4}/{:<4} ({:>5.1}%)",
        name, passed, total, percentage
    );
    (passed, failed)
}

fn main() {
    println!("📊 Edge Case Test Summary\n");
    println!("{:<25} {:>10} {:>8}", "Category", "Passed", "Success");
    println!("{}", "-".repeat(50));

    let mut total_passed = 0;
    let mut total_failed = 0;

    // Test each category
    let (p, f) = test_category(
        "Format & Blocks",
        edge_cases::format_and_blocks::get_tests(),
    );
    total_passed += p;
    total_failed += f;

    let (p, f) = test_category(
        "Operator Overloading",
        edge_cases::operator_overloading::get_tests(),
    );
    total_passed += p;
    total_failed += f;

    let (p, f) = test_category(
        "Indirect & Methods",
        edge_cases::indirect_and_methods::get_tests(),
    );
    total_passed += p;
    total_failed += f;

    let (p, f) = test_category(
        "Versions & V-strings",
        edge_cases::versions_and_vstrings::get_tests(),
    );
    total_passed += p;
    total_failed += f;

    let (p, f) = test_category(
        "Unicode & Encoding",
        edge_cases::unicode_and_encoding::get_tests(),
    );
    total_passed += p;
    total_failed += f;

    let (p, f) = test_category(
        "File I/O Operations",
        edge_cases::file_io_operations::get_tests(),
    );
    total_passed += p;
    total_failed += f;

    let (p, f) = test_category(
        "Regex & Patterns",
        edge_cases::regex_and_patterns::get_tests(),
    );
    total_passed += p;
    total_failed += f;

    println!("{}", "-".repeat(50));

    let grand_total = total_passed + total_failed;
    let overall_percentage = if grand_total > 0 {
        (total_passed as f64 / grand_total as f64) * 100.0
    } else {
        100.0
    };

    println!(
        "{:<25} {:>4}/{:<4} ({:>5.1}%)",
        "TOTAL", total_passed, grand_total, overall_percentage
    );

    println!("\n✅ Tests passing: {}", total_passed);
    println!("❌ Tests failing: {}", total_failed);
    println!("\nNote: These are comprehensive edge case tests covering");
    println!("      many advanced and rarely-used Perl features.");
}

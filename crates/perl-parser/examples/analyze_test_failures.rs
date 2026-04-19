//! Analyze edge case test failures
use perl_parser::Parser;
use std::collections::HashMap;

mod edge_cases {
    pub mod file_io_operations;
    pub mod format_and_blocks;
    pub mod indirect_and_methods;
    pub mod operator_overloading;
    pub mod regex_and_patterns;
    pub mod unicode_and_encoding;
    pub mod versions_and_vstrings;
}

#[derive(Debug)]
struct TestFailure {
    category: String,
    description: String,
    code: String,
    error: String,
}

fn analyze_category(name: &str, tests: Vec<(&str, &str)>) -> (usize, usize, Vec<TestFailure>) {
    let mut passed = 0;
    let mut failed = 0;
    let mut failures = Vec::new();

    for (code, desc) in tests {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => passed += 1,
            Err(e) => {
                failed += 1;
                failures.push(TestFailure {
                    category: name.to_string(),
                    description: desc.to_string(),
                    code: code.to_string(),
                    error: format!("{:?}", e),
                });
            }
        }
    }

    (passed, failed, failures)
}

fn main() {
    println!("🔍 Perl Parser Edge Case Failure Analysis\n");

    let categories = vec![
        (
            "Format & Blocks",
            edge_cases::format_and_blocks::get_tests(),
        ),
        (
            "Operator Overloading",
            edge_cases::operator_overloading::get_tests(),
        ),
        (
            "Indirect & Methods",
            edge_cases::indirect_and_methods::get_tests(),
        ),
        (
            "Versions & V-strings",
            edge_cases::versions_and_vstrings::get_tests(),
        ),
        (
            "Unicode & Encoding",
            edge_cases::unicode_and_encoding::get_tests(),
        ),
        (
            "File I/O Operations",
            edge_cases::file_io_operations::get_tests(),
        ),
        (
            "Regex & Patterns",
            edge_cases::regex_and_patterns::get_tests(),
        ),
    ];

    let mut all_failures = Vec::new();
    let mut category_stats = HashMap::new();
    let mut total_passed = 0;
    let mut total_failed = 0;

    // Run tests and collect failures
    for (category_name, tests) in categories {
        let total_tests = tests.len();
        let (passed, failed, failures) = analyze_category(category_name, tests);

        total_passed += passed;
        total_failed += failed;
        all_failures.extend(failures);

        let percentage = if total_tests > 0 {
            (passed as f64 / total_tests as f64) * 100.0
        } else {
            100.0
        };

        category_stats.insert(category_name, (passed, failed, percentage));

        println!(
            "{:<25} {:>4}/{:<4} ({:>5.1}%)",
            category_name,
            passed,
            passed + failed,
            percentage
        );
    }

    println!("\n{}", "=".repeat(60));
    println!(
        "Total: {}/{} ({:.1}%)\n",
        total_passed,
        total_passed + total_failed,
        (total_passed as f64 / (total_passed + total_failed) as f64) * 100.0
    );

    // Analyze failure patterns
    println!("📊 Failure Analysis by Pattern\n");

    let mut failure_patterns: HashMap<String, Vec<&TestFailure>> = HashMap::new();

    for failure in &all_failures {
        // Categorize by error type
        let pattern = if failure.error.contains("Unexpected end of input") {
            "EOF/Incomplete Input"
        } else if failure.error.contains("Expected") {
            "Unexpected Token"
        } else if failure.error.contains("Unknown") {
            "Unknown Construct"
        } else if failure.error.contains("regex") || failure.error.contains("Regex") {
            "Regex Parsing"
        } else if failure.error.contains("quote") || failure.error.contains("Quote") {
            "Quote-like Operators"
        } else {
            "Other"
        };

        failure_patterns
            .entry(pattern.to_string())
            .or_default()
            .push(failure);
    }

    // Print pattern analysis
    for (pattern, failures) in &failure_patterns {
        println!("{}: {} failures", pattern, failures.len());

        // Show up to 3 examples for each pattern
        for (i, failure) in failures.iter().take(3).enumerate() {
            println!(
                "  {}. {} - {}",
                i + 1,
                failure.description,
                failure.code.lines().next().unwrap_or(&failure.code)
            );
        }

        if failures.len() > 3 {
            println!("  ... and {} more", failures.len() - 3);
        }
        println!();
    }

    // Detailed failure list by category
    println!("📋 Detailed Failures by Category\n");

    for (category, (_, failed, _)) in &category_stats {
        if *failed > 0 {
            println!("{} ({} failures):", category, failed);

            let category_failures: Vec<_> = all_failures
                .iter()
                .filter(|f| f.category == *category)
                .collect();

            for (i, failure) in category_failures.iter().take(10).enumerate() {
                println!("  {}. {}", i + 1, failure.description);
                println!(
                    "     Code: {}",
                    failure.code.lines().next().unwrap_or(&failure.code)
                );
                println!(
                    "     Error: {}",
                    failure.error.lines().next().unwrap_or(&failure.error)
                );
                println!();
            }

            if category_failures.len() > 10 {
                println!("  ... and {} more failures\n", category_failures.len() - 10);
            }
        }
    }

    // Summary of most problematic constructs
    println!("🎯 Most Problematic Perl Constructs:\n");

    let mut construct_failures: HashMap<String, usize> = HashMap::new();

    for failure in &all_failures {
        let construct = if failure.description.contains("format") {
            "Format declarations"
        } else if failure.description.contains("overload") {
            "Operator overloading"
        } else if failure.description.contains("indirect") {
            "Indirect object syntax"
        } else if failure.description.contains("v-string")
            || failure.description.contains("version")
        {
            "Version strings"
        } else if failure.description.contains("unicode")
            || failure.description.contains("encoding")
        {
            "Unicode/Encoding"
        } else if failure.description.contains("file") || failure.description.contains("handle") {
            "File operations"
        } else if failure.description.contains("regex") || failure.description.contains("match") {
            "Regular expressions"
        } else if failure.description.contains("pragma") {
            "Pragmas"
        } else if failure.description.contains("attribute") {
            "Attributes"
        } else if failure.description.contains("prototype") {
            "Prototypes"
        } else {
            "Other"
        };

        *construct_failures.entry(construct.to_string()).or_insert(0) += 1;
    }

    let mut sorted_constructs: Vec<_> = construct_failures.into_iter().collect();
    sorted_constructs.sort_by(|a, b| b.1.cmp(&a.1));

    for (construct, count) in sorted_constructs.iter().take(10) {
        println!("{:<30} {} failures", construct, count);
    }
}

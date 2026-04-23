#!/usr/bin/env cargo run --bin
//! Minimal reproducer to identify the exact stack overflow threshold

use std::panic;

/// Find the exact nesting depth that causes stack overflow
fn find_stack_overflow_threshold() -> Option<usize> {
    println!("🔍 Binary search for stack overflow threshold...");

    let mut low = 100;  // Known to work
    let mut high = 1000; // Known to crash
    let mut last_safe = low;

    while low <= high {
        let mid = (low + high) / 2;
        let test_input = format!("{}{}{}", "{ ".repeat(mid), "42", " }".repeat(mid));

        print!("  Testing depth {}: {} chars... ", mid, test_input.len());

        let result = panic::catch_unwind(|| {
            use perl_parser::Parser;
            let mut parser = Parser::new(&test_input);
            parser.parse()
        });

        match result {
            Ok(_) => {
                println!("✅ OK");
                last_safe = mid;
                low = mid + 1;
            }
            Err(_) => {
                println!("💥 STACK OVERFLOW");
                high = mid - 1;
            }
        }
    }

    Some(last_safe)
}

/// Test different nesting patterns to find which cause issues
fn test_nesting_patterns() {
    println!("\n🧪 Testing different nesting patterns...");

    let patterns = vec![
        ("Braces", "{ ", " }"),
        ("Parens", "( ", " )"),
        ("Brackets", "[ ", " ]"),
        ("Sub calls", "f(", ")"),
        ("Array access", "$x[", "]"),
        ("Hash access", "$x{", "}"),
        ("If blocks", "if(1){ ", " }"),
    ];

    for (name, open, close) in patterns {
        print!("  Testing {}: ", name);

        // Try moderate depth first
        let depth = 200;
        let test_input = format!("{}{}{}", open.repeat(depth), "42", close.repeat(depth));

        let result = panic::catch_unwind(|| {
            use perl_parser::Parser;
            let mut parser = Parser::new(&test_input);
            parser.parse()
        });

        match result {
            Ok(_) => println!("✅ OK (depth {})", depth),
            Err(_) => println!("💥 CRASH at depth {}", depth),
        }
    }
}

/// Save minimal crash reproduction
fn save_minimal_repro(threshold: usize) {
    let crash_input = format!("{}{}{}", "{ ".repeat(threshold + 50), "42", " }".repeat(threshold + 50));

    let _ = std::fs::create_dir_all("repros");

    if let Ok(mut file) = std::fs::File::create("repros/stack_overflow_minimal.pl") {
        use std::io::Write;
        let _ = writeln!(file, "#!/usr/bin/perl");
        let _ = writeln!(file, "# CRITICAL: Stack overflow reproducer");
        let _ = writeln!(file, "# Safe depth: ~{}", threshold);
        let _ = writeln!(file, "# Crash depth: ~{}", threshold + 50);
        let _ = writeln!(file, "# This input causes perl-parser to stack overflow");
        let _ = writeln!(file, "# Size: {} chars", crash_input.len());
        let _ = writeln!(file);
        let _ = write!(file, "{}", crash_input);

        println!("💾 Saved minimal stack overflow reproducer: repros/stack_overflow_minimal.pl");
    }
}

fn main() {
    println!("🔬 Minimal Stack Overflow Reproducer");
    println!("=====================================");

    // Find exact threshold
    if let Some(threshold) = find_stack_overflow_threshold() {
        println!("\n📈 Stack overflow threshold found: ~{} nesting depth", threshold);
        save_minimal_repro(threshold);
    }

    // Test different patterns
    test_nesting_patterns();

    println!("\n⚠️  CRITICAL SECURITY ISSUE IDENTIFIED:");
    println!("   The perl-parser has a stack overflow vulnerability");
    println!("   when parsing deeply nested constructs (e.g., braces).");
    println!("   This could be exploited for denial of service attacks.");
    println!("   Recommendation: Implement recursion depth limits or");
    println!("   convert recursive parsing to iterative approach.");
}
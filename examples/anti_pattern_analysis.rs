//! Example: Analyzing Perl code for anti-patterns and edge cases

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::{
    understanding_parser::UnderstandingParser,
    anti_pattern_detector::Severity,
};

fn main() {
    #[cfg(not(feature = "pure-rust"))]
    {
        eprintln!("This example requires the pure-rust feature");
        std::process::exit(1);
    }
    
    #[cfg(feature = "pure-rust")]
    {
        // Example 1: Clean code
        println!("=== Example 1: Clean Code ===");
        let clean_code = r#"
my $name = "Alice";
my $greeting = <<'END';
Hello, $name!
Welcome to our system.
END
print $greeting;
"#;
        
        analyze_code(clean_code);
        
        // Example 2: Format with heredoc
        println!("\n=== Example 2: Format with Heredoc ===");
        let format_code = r#"
format REPORT =
<<'HEADER'
Employee Report
===============
HEADER
@<<<<<<<<<<<<< @>>>>>>>>>>>
$name,         $salary
.

write REPORT;
"#;
        
        analyze_code(format_code);
        
        // Example 3: BEGIN block with heredoc
        println!("\n=== Example 3: BEGIN Block with Heredoc ===");
        let begin_code = r#"
BEGIN {
    our $CONFIG = <<'END';
    database = production
    server = 192.168.1.1
END
    
    # Side effect: modifying global state at compile time
    $ENV{DB_CONFIG} = $CONFIG;
}

print "Config loaded: $CONFIG\n";
"#;
        
        analyze_code(begin_code);
        
        // Example 4: Dynamic heredoc delimiter
        println!("\n=== Example 4: Dynamic Heredoc Delimiter ===");
        let dynamic_code = r#"
my $delimiter = "EOF";
my $content = <<$delimiter;
This uses a dynamic delimiter
which cannot be parsed statically
EOF

# Even worse: computed delimiter
my $computed = <<${\ get_delimiter() };
Content with computed delimiter
DYNAMIC_END
"#;
        
        analyze_code(dynamic_code);
        
        // Example 5: Multiple anti-patterns
        println!("\n=== Example 5: Multiple Anti-Patterns ===");
        let complex_code = r#"
BEGIN {
    # Dynamic delimiter in BEGIN block!
    my $delim = "DATA";
    $::config = <<$delim;
    setting = value
DATA
}

format REPORT =
<<'FMT'
Complex Report
FMT
@<<<<<<<<<<<
$data
.

# And a normal heredoc for comparison
my $normal = <<'END';
This part is fine
END
"#;
        
        analyze_code(complex_code);
    }
}

#[cfg(feature = "pure-rust")]
fn analyze_code(code: &str) {
    let parser = UnderstandingParser::new();
    
    match parser.parse_with_understanding(code) {
        Ok(result) => {
            println!("Parse Coverage: {:.1}%", result.parse_coverage);
            
            if result.diagnostics.is_empty() {
                println!("✓ No anti-patterns detected");
            } else {
                println!("⚠ Found {} issues:", result.diagnostics.len());
                
                for (i, diag) in result.diagnostics.iter().enumerate() {
                    let severity_icon = match diag.severity {
                        Severity::Error => "❌",
                        Severity::Warning => "⚠️",
                        Severity::Info => "ℹ️",
                    };
                    
                    println!("\n  {}. {} {}", i + 1, severity_icon, diag.message);
                    println!("     {}", diag.explanation);
                    
                    if let Some(fix) = &diag.suggested_fix {
                        println!("     💡 Suggestion: {}", fix.lines().next().unwrap_or(""));
                    }
                }
            }
            
            // Show AST structure summary
            println!("\nAST Structure:");
            let sexp = result.ast.to_sexp();
            if sexp.len() > 100 {
                println!("  {}...", &sexp[..100]);
            } else {
                println!("  {}", sexp);
            }
            
            // Show recovery points if any
            if !result.recovery_points.is_empty() {
                println!("\nRecovery points: {:?}", result.recovery_points);
            }
        }
        Err(e) => {
            eprintln!("Failed to parse: {}", e);
        }
    }
}
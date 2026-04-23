//! Comprehensive demonstration of Perl heredoc edge case handling

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::{
    dynamic_delimiter_recovery::RecoveryMode,
    edge_case_handler::{EdgeCaseConfig, EdgeCaseHandler},
};

fn main() {
    #[cfg(not(feature = "pure-rust"))]
    {
        eprintln!("This example requires the pure-rust feature");
        std::process::exit(1);
    }

    #[cfg(feature = "pure-rust")]
    {
        println!("=== Perl Heredoc Edge Case Demo ===\n");

        // Example 1: Dynamic Delimiters
        demo_dynamic_delimiters();

        // Example 2: Phase-Dependent Heredocs
        demo_phase_dependent();

        // Example 3: Multiple Edge Cases
        demo_complex_edge_cases();

        // Example 4: Recovery Modes
        demo_recovery_modes();
    }
}

#[cfg(feature = "pure-rust")]
fn demo_dynamic_delimiters() {
    println!("\n--- Example 1: Dynamic Delimiters ---");

    let code = r#"
# Simple dynamic delimiter
my $delimiter = "EOF";
my $content = <<$delimiter;
This heredoc uses a variable delimiter
EOF

# Complex dynamic delimiter
my $prefix = "END";
my $suffix = "_DATA";
my $text = <<$prefix$suffix;
This won't parse statically
END_DATA

# Runtime-computed delimiter
my $computed = get_delimiter();
my $data = <<$computed;
Delimiter unknown until runtime
UNKNOWN
"#;

    let config = EdgeCaseConfig { recovery_mode: RecoveryMode::BestGuess, ..Default::default() };

    let mut handler = EdgeCaseHandler::new(config);
    let analysis = handler.analyze(code);

    println!("Found {} dynamic delimiter issues", analysis.delimiter_resolutions.len());

    for resolution in &analysis.delimiter_resolutions {
        println!(
            "  - {}: {}",
            resolution.expression,
            if let Some(ref delim) = resolution.resolved_to {
                format!(
                    "resolved to '{}' ({}% confidence)",
                    delim,
                    (resolution.confidence * 100.0) as u32
                )
            } else {
                "could not resolve".to_string()
            }
        );
    }
}

#[cfg(feature = "pure-rust")]
fn demo_phase_dependent() {
    println!("\n--- Example 2: Phase-Dependent Heredocs ---");

    let code = r#"
# BEGIN block with heredoc
BEGIN {
    our $CONFIG = <<'END';
    database = production
    server = 10.0.0.1
    port = 5432
END
    
    # Side effect: modifying environment
    $ENV{DB_CONFIG} = $CONFIG;
}

# CHECK block heredoc
CHECK {
    my $validation = <<'RULES';
    - All modules loaded
    - Syntax verified
RULES
    print "Check phase: $validation\n";
}

# Normal runtime heredoc (for comparison)
my $runtime = <<'EOF';
This is fine - runtime heredoc
EOF
"#;

    let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
    let analysis = handler.analyze(code);

    println!("Phase warnings:");
    for warning in &analysis.phase_warnings {
        println!("  ⚠️  {}", warning);
    }

    println!("\nRecommended actions:");
    for action in &analysis.recommended_actions {
        println!("  • {:?}", action);
    }
}

#[cfg(feature = "pure-rust")]
fn demo_complex_edge_cases() {
    println!("\n--- Example 3: Multiple Edge Cases ---");

    let code = r#"
# Worst case: dynamic delimiter in BEGIN with format
BEGIN {
    my $delim = $ENV{HEREDOC_DELIMITER} || "DEFAULT";
    our $early_config = <<$delim;
    Compile-time dynamic heredoc!
DEFAULT
}

# Format with heredoc
format REPORT =
<<'HEADER'
====================
  System Report
====================
HEADER
@<<<<<<<<<< @>>>>>>>>>
$name,      $value
.

# Source filter simulation
use Filter::Simple;
FILTER {
    s/MAGIC_HEREDOC/<<'END'/g;
};

my $filtered = MAGIC_HEREDOC;
This might be transformed by filter
END

# Tied filehandle
tie *FH, 'MyPackage';
print FH <<'EOF';
This goes through custom I/O
EOF
"#;

    let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
    let analysis = handler.analyze(code);

    println!("Edge case summary:");
    println!("  Total issues: {}", analysis.diagnostics.len());

    // Group by severity
    let mut errors = 0;
    let mut warnings = 0;
    let mut info = 0;

    for diag in &analysis.diagnostics {
        match diag.severity {
            tree_sitter_perl::anti_pattern_detector::Severity::Error => errors += 1,
            tree_sitter_perl::anti_pattern_detector::Severity::Warning => warnings += 1,
            tree_sitter_perl::anti_pattern_detector::Severity::Info => info += 1,
        }
    }

    println!("  - {} errors", errors);
    println!("  - {} warnings", warnings);
    println!("  - {} info messages", info);

    // Show first few diagnostics
    println!("\nFirst 3 diagnostics:");
    for (i, diag) in analysis.diagnostics.iter().take(3).enumerate() {
        println!("  {}. {}", i + 1, diag.message);
        if let Some(ref fix) = diag.suggested_fix {
            println!("     Fix: {}", fix);
        }
    }
}

#[cfg(feature = "pure-rust")]
fn demo_recovery_modes() {
    println!("\n--- Example 4: Recovery Modes ---");

    let code = r#"
my $delimiter = "EOF";
my $text = <<$delimiter;
Test content
EOF
"#;

    // Test different recovery modes
    let modes = [
        ("Conservative", RecoveryMode::Conservative),
        ("BestGuess", RecoveryMode::BestGuess),
        ("Interactive", RecoveryMode::Interactive),
        ("Sandbox", RecoveryMode::Sandbox),
    ];

    for (name, mode) in modes {
        println!("\n  Recovery mode: {}", name);

        let config = EdgeCaseConfig { recovery_mode: mode, ..Default::default() };

        let mut handler = EdgeCaseHandler::new(config);
        let analysis = handler.analyze(code);

        if let Some(resolution) = analysis.delimiter_resolutions.first() {
            println!("    Strategy: {}", resolution.method);
            println!(
                "    Result: {}",
                if resolution.resolved_to.is_some() { "Success" } else { "Failed" }
            );
        }
    }

    println!("\n=== Summary ===");
    println!("The edge case handler can:");
    println!("  ✓ Detect dynamic delimiters and attempt recovery");
    println!("  ✓ Identify phase-dependent heredocs (BEGIN/CHECK/etc)");
    println!("  ✓ Warn about source filters and tied handles");
    println!("  ✓ Provide actionable recommendations");
    println!("  ✓ Support multiple recovery strategies");
}

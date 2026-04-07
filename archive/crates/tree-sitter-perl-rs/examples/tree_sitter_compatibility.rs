//! Example demonstrating tree-sitter compatible output for edge cases

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::{
    dynamic_delimiter_recovery::RecoveryMode, edge_case_handler::EdgeCaseConfig,
    tree_sitter_adapter::TreeSitterAdapter,
};

fn main() {
    #[cfg(not(feature = "pure-rust"))]
    {
        eprintln!("This example requires the pure-rust feature");
        std::process::exit(1);
    }

    #[cfg(feature = "pure-rust")]
    {
        println!("=== Tree-sitter Compatible Edge Case Output ===\n");

        // Example 1: Standard heredoc (baseline)
        demo_standard_heredoc();

        // Example 2: Dynamic delimiter
        demo_dynamic_delimiter();

        // Example 3: Phase-dependent heredoc
        demo_phase_dependent_heredoc();

        // Example 4: Multiple edge cases
        demo_complex_edge_cases();
    }
}

#[cfg(feature = "pure-rust")]
fn demo_standard_heredoc() {
    println!("--- Example 1: Standard Heredoc (Baseline) ---");

    let code = r#"
my $text = <<'EOF';
This is a standard heredoc
with multiple lines
EOF
"#;

    let output = analyze_and_convert(code);
    print_tree_sitter_output(&output);
}

#[cfg(feature = "pure-rust")]
fn demo_dynamic_delimiter() {
    println!("\n--- Example 2: Dynamic Delimiter ---");

    let code = r#"
my $delimiter = "END";
my $content = <<$delimiter;
Dynamic delimiter content
END
"#;

    let output = analyze_and_convert(code);
    print_tree_sitter_output(&output);
}

#[cfg(feature = "pure-rust")]
fn demo_phase_dependent_heredoc() {
    println!("\n--- Example 3: Phase-Dependent Heredoc ---");

    let code = r#"
BEGIN {
    our $CONFIG = <<'CFG';
    server = localhost
    port = 8080
CFG
}
"#;

    let output = analyze_and_convert(code);
    print_tree_sitter_output(&output);
}

#[cfg(feature = "pure-rust")]
fn demo_complex_edge_cases() {
    println!("\n--- Example 4: Multiple Edge Cases ---");

    let code = r#"
# Mix of edge cases
use encoding 'latin1';

BEGIN {
    my $delim = shift || "EOF";
    $::early = <<$delim;
    Complex case
EOF
}

format REPORT =
<<'HDR'
Report Header
HDR
.

tie *FH, 'CustomIO';
print FH <<'END';
Tied handle output
END
"#;

    let output = analyze_and_convert(code);
    print_tree_sitter_output(&output);
}

#[cfg(feature = "pure-rust")]
fn analyze_and_convert(code: &str) -> String {
    use tree_sitter_perl::edge_case_handler::EdgeCaseHandler;

    // Analyze with edge case handler
    let config = EdgeCaseConfig { recovery_mode: RecoveryMode::BestGuess, ..Default::default() };

    let mut handler = EdgeCaseHandler::new(config);
    let analysis = handler.analyze(code);

    // Convert to tree-sitter format
    let ts_output =
        TreeSitterAdapter::convert_to_tree_sitter(analysis.ast, analysis.diagnostics, code);

    // Format as JSON-like output
    format_output(&ts_output)
}

#[cfg(feature = "pure-rust")]
fn format_output(output: &tree_sitter_perl::tree_sitter_adapter::TreeSitterOutput) -> String {
    use std::fmt::Write;

    let mut result = String::new();

    // Tree structure
    let _ = writeln!(&mut result, "Tree:");
    format_node(&output.tree.root, &mut result, 0);

    // Diagnostics (separate from tree)
    if !output.diagnostics.is_empty() {
        let _ = writeln!(&mut result, "\nDiagnostics:");
        for (i, diag) in output.diagnostics.iter().enumerate() {
            let _ = writeln!(
                &mut result,
                "  {}. [{}] {} at {}:{}",
                i + 1,
                match diag.severity {
                    tree_sitter_perl::tree_sitter_adapter::DiagnosticSeverity::Error => "ERROR",
                    tree_sitter_perl::tree_sitter_adapter::DiagnosticSeverity::Warning => "WARN",
                    tree_sitter_perl::tree_sitter_adapter::DiagnosticSeverity::Info => "INFO",
                    tree_sitter_perl::tree_sitter_adapter::DiagnosticSeverity::Hint => "HINT",
                },
                diag.message,
                diag.start_point.0,
                diag.start_point.1
            );

            if let Some(ref code) = diag.code {
                let _ = writeln!(&mut result, "     Code: {}", code);
            }
        }
    }

    // Metadata
    let _ = writeln!(&mut result, "\nMetadata:");
    let _ = writeln!(&mut result, "  Parse coverage: {:.1}%", output.metadata.parse_coverage);
    let _ = writeln!(&mut result, "  Edge cases: {}", output.metadata.edge_case_count);

    result
}

#[cfg(feature = "pure-rust")]
fn format_node(
    node: &tree_sitter_perl::tree_sitter_adapter::TreeSitterNode,
    output: &mut String,
    indent: usize,
) {
    use std::fmt::Write;

    let indent_str = "  ".repeat(indent);

    // Node type with error/missing indicators
    let type_str = if node.is_error {
        format!("{} [ERROR]", node.node_type)
    } else if node.is_missing {
        format!("{} [MISSING]", node.node_type)
    } else {
        node.node_type.clone()
    };

    let _ = writeln!(output, "{}{}", indent_str, type_str);

    // Text content for leaf nodes
    if let Some(ref text) = node.text {
        let _ = writeln!(output, "{}  text: {:?}", indent_str, text);
    }

    // Children
    for child in &node.children {
        format_node(child, output, indent + 1);
    }
}

#[cfg(feature = "pure-rust")]
fn print_tree_sitter_output(output: &str) {
    println!("{}", output);

    // Show example JSON output
    println!("\nExample JSON representation:");
    println!(
        r#"{{
  "type": "source_file",
  "children": [
    {{
      "type": "statement",
      "children": [
        {{
          "type": "heredoc",
          "children": [
            {{ "type": "heredoc_opener", "text": "<<'EOF'" }},
            {{ "type": "heredoc_body", "text": "content\\n" }},
            {{ "type": "heredoc_delimiter", "text": "EOF" }}
          ]
        }}
      ]
    }}
  ]
}}"#
    );
}

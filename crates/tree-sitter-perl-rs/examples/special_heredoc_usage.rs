//! Example usage of special context heredoc handling

#[cfg(feature = "pure-rust")]
use tree_sitter_perl::{
    context_aware_parser::{ContextAwareFullParser, ContextAwareHeredocParser},
    runtime_heredoc_handler::{RuntimeHeredocContext, RuntimeHeredocHandler},
};

fn main() {
    #[cfg(not(feature = "pure-rust"))]
    {
        eprintln!("This example requires the pure-rust feature");
        std::process::exit(1);
    }

    #[cfg(feature = "pure-rust")]
    {
        println!("=== Special Context Heredoc Examples ===\n");

        example_eval_heredoc();
        example_substitution_heredoc();
        example_nested_contexts();
        example_runtime_handling();
        demonstrate_edge_cases();
    }
}

#[cfg(feature = "pure-rust")]
fn example_eval_heredoc() {
    println!("1. Eval with Heredoc:");
    println!("-----------------");

    let code = r#"
my $config = eval <<'CONFIG';
{
    host => 'localhost',
    port => 8080,
    message => <<'MSG',
Welcome to the server!
This is a multi-line message.
MSG
}
CONFIG

print "Config loaded\n";
"#;

    let parser = ContextAwareHeredocParser::new(code);
    let (_processed, declarations) = parser.parse();

    println!("Input code:{}", code);
    println!("\nFound {} heredoc declarations", declarations.len());
    for decl in &declarations {
        println!("  - Delimiter: '{}', Line: {}", decl.terminator, decl.declaration_line);
    }
    println!();
}

#[cfg(feature = "pure-rust")]
fn example_substitution_heredoc() {
    println!("2. Substitution with /e Flag:");
    println!("-------------------------");

    let code = r#"
my $template = "Hello FOO, welcome to BAR!";

$template =~ s/FOO/<<'NAME'/e;
John Doe
NAME

$template =~ s/BAR/<<END/e;
Perl Programming
END

print $template;
"#;

    let parser = ContextAwareHeredocParser::new(code);
    let (_processed, declarations) = parser.parse();

    println!("Input code:{}", code);
    println!("\nHeredocs in s///e context:");
    for decl in &declarations {
        println!("  - Delimiter: '{}', Interpolate: {}", decl.terminator, decl.interpolated);
    }
    println!();
}

#[cfg(feature = "pure-rust")]
fn example_nested_contexts() {
    println!("3. Nested Eval Contexts:");
    println!("--------------------");

    let code = r#"
eval <<'OUTER';
    print "Outer eval\n";
    
    my $inner_result = eval <<'INNER';
        my $data = <<'DATA';
        Nested data structure
        with multiple lines
DATA
        return "Processed: $data";
INNER
    
    print "Inner result: $inner_result\n";
OUTER
"#;

    let mut parser = ContextAwareFullParser::new();
    match parser.parse(code) {
        Ok(_ast) => {
            println!("Successfully parsed nested eval contexts");
            // In a real implementation, we'd inspect the AST
        }
        Err(e) => {
            println!("Parse error: {:?}", e);
        }
    }
    println!();
}

#[cfg(feature = "pure-rust")]
fn example_runtime_handling() {
    println!("4. Runtime Heredoc Handling:");
    println!("------------------------");

    // Simulate runtime evaluation
    let mut runtime = RuntimeHeredocHandler::new();

    // Example 1: Simple eval
    let eval_content = r#"print <<'EOF';
Runtime evaluated content
EOF"#;

    match runtime.eval_with_heredoc(eval_content) {
        Ok(_result) => println!("Eval result: processed successfully"),
        Err(e) => println!("Eval error: {:?}", e),
    }

    // Example 2: Variable interpolation
    let mut context = RuntimeHeredocContext::default();
    context.variables.insert("user".to_string(), "Alice".to_string());
    context.variables.insert("action".to_string(), "logged in".to_string());

    let heredoc_content = "User $user has $action";
    match runtime.evaluate_heredoc(heredoc_content, &context) {
        Ok(result) => println!("Interpolated: {}", result),
        Err(e) => println!("Interpolation error: {:?}", e),
    }

    // Example 3: Substitution with heredoc (placeholder - method not yet implemented)
    let _text = "Replace THIS with heredoc";
    let _pattern = "THIS";
    let _replacement = "<<'REPLACEMENT'\nNew Content\nREPLACEMENT";
    let _flags = "e";

    // TODO: Implement substitute_with_heredoc method
    // match runtime.substitute_with_heredoc(text, pattern, replacement, flags) {
    //     Ok(_result) => println!("Substitution: processed successfully"),
    //     Err(e) => println!("Substitution error: {:?}", e),
    // }
    println!("Substitution with heredoc: not yet implemented");

    println!();
}

#[cfg(feature = "pure-rust")]
fn demonstrate_edge_cases() {
    println!("5. Edge Cases:");
    println!("-----------");

    // Edge case 1: Heredoc in backticks
    let code1 = r#"my $result = `perl -e 'print <<EOF
Hello from subshell
EOF'`;"#;

    // Edge case 2: Multiple heredocs in eval
    let code2 = r#"eval <<'A', <<'B';
First heredoc
A
Second heredoc
B"#;

    // Edge case 3: Heredoc with special characters in delimiter
    let code3 = r#"eval <<'END_OF_DATA';
Special delimiter test
END_OF_DATA"#;

    for (i, code) in [code1, code2, code3].iter().enumerate() {
        println!("\nEdge case {}: ", i + 1);
        let parser = ContextAwareHeredocParser::new(code);
        let (_, declarations) = parser.parse();
        println!("Found {} heredocs", declarations.len());
    }
}

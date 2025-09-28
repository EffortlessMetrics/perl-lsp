//! Advanced IDE features demonstration
//!
//! This example showcases code completion, signature help, and rename refactoring.

use perl_parser::{
    apply_rename_edits, CompletionProvider, Parser, RenameOptions, RenameProvider,
    SignatureHelpProvider,
};

fn main() {
    println!("=== Advanced Perl IDE Features Demo ===\n");

    let code = r#"#!/usr/bin/perl
use strict;
use warnings;

# A simple calculator module
package Calculator;

sub add {
    my ($a, $b) = @_;
    return $a + $b;
}

sub multiply {
    my ($x, $y) = @_;
    return $x * $y;
}

sub divide {
    my ($dividend, $divisor) = @_;
    die "Division by zero!" if $divisor == 0;
    return $dividend / $divisor;
}

package main;

# Test the calculator
my $first = 10;
my $second = 5;

print "Addition: ", Calculator::add($first, $second), "\n";
print "Multiplication: ", Calculator::multiply($first, $second), "\n";

# More code here
my $result = Calculator::divide($first, $second);
print "Division: $result\n";

# Let's try some string operations
my $text = "Hello, World!";
my $upper = uc($text);
print $upper, "\n";

# Array operations
my @numbers = (1, 2, 3, 4, 5);
push(@numbers, 6);
my @squared = map { $_ * $_ } @numbers;

print "Squared: @squared\n";
"#;

    // Parse the code
    let mut parser = Parser::new(code);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return;
        }
    };

    println!("✓ Successfully parsed the code\n");

    // 1. Code Completion Demo
    println!("=== Code Completion Demo ===");

    let completion_provider = CompletionProvider::new(&ast);

    // Test 1: Variable completion after $fir
    let completion_pos = code.find("multiply($fir").unwrap() + "multiply($fir".len();
    println!("\n1. Completing variable after '$fir':");

    let completions = completion_provider.get_completions(code, completion_pos);
    println!("   Found {} completions:", completions.len());
    for (i, completion) in completions.iter().take(5).enumerate() {
        println!(
            "   {}: {} - {:?} ({})",
            i + 1,
            completion.label,
            completion.kind,
            completion.detail.as_deref().unwrap_or("")
        );
    }

    // Test 2: Function completion
    println!("\n2. Completing after 'Calculator::':");
    let calc_pos = code.find("Calculator::add").unwrap() + "Calculator::".len() - 3;
    let calc_completions = completion_provider.get_completions(code, calc_pos);

    println!("   Found {} completions:", calc_completions.len());
    for completion in calc_completions.iter().take(5) {
        println!("   - {} ({:?})", completion.label, completion.kind);
    }

    // Test 3: Built-in function completion
    println!("\n3. Completing built-in functions after 'pu':");
    let builtin_code = "my @arr = (); pu";
    let builtin_ast = Parser::new("").parse().unwrap();
    let builtin_provider = CompletionProvider::new(&builtin_ast);
    let builtin_completions = builtin_provider.get_completions(builtin_code, builtin_code.len());

    println!("   Found {} completions:", builtin_completions.len());
    for completion in builtin_completions.iter() {
        println!("   - {} : {}", completion.label, completion.detail.as_deref().unwrap_or(""));
    }

    // 2. Signature Help Demo
    println!("\n\n=== Signature Help Demo ===");

    let signature_provider = SignatureHelpProvider::new(&ast);

    // Test 1: Help for Calculator::add
    let add_call_pos = code.find("Calculator::add($first, ").unwrap() + "Calculator::add(".len();
    println!("\n1. Signature help for Calculator::add:");

    if let Some(sig_help) = signature_provider.get_signature_help(code, add_call_pos) {
        for sig in &sig_help.signatures {
            println!("   Signature: {}", sig.label);
            if let Some(doc) = &sig.documentation {
                println!("   Documentation: {}", doc);
            }
            println!("   Parameters:");
            for (i, param) in sig.parameters.iter().enumerate() {
                let active = sig_help.active_parameter == Some(i);
                println!(
                    "     {} {} {}",
                    if active { "→" } else { " " },
                    param.label,
                    param.documentation.as_deref().unwrap_or("")
                );
            }
        }
        println!("   Active parameter: {:?}", sig_help.active_parameter);
    }

    // Test 2: Help for built-in push
    println!("\n2. Signature help for push:");
    let push_pos = code.find("push(@numbers, ").unwrap() + "push(@numbers, ".len() - 1;

    if let Some(sig_help) = signature_provider.get_signature_help(code, push_pos) {
        for sig in &sig_help.signatures {
            println!("   Signature: {}", sig.label);
            println!("   Active parameter: {:?}", sig_help.active_parameter);
        }
    }

    // Test 3: Help for map
    println!("\n3. Signature help for map:");
    let map_pos = code.find("map { ").unwrap() + "map { ".len() - 1;

    if let Some(sig_help) = signature_provider.get_signature_help(code, map_pos) {
        for sig in &sig_help.signatures {
            println!("   Signature: {}", sig.label);
        }
    }

    // 3. Rename Refactoring Demo
    println!("\n\n=== Rename Refactoring Demo ===");

    let rename_provider = RenameProvider::new(&ast, code.to_string());

    // Test 1: Rename variable $first to $num1
    println!("\n1. Renaming variable '$first' to '$num1':");
    let first_pos = code.find("my $first").unwrap() + "my $".len();

    // Prepare rename (check if possible)
    if let Some((range, current_name)) = rename_provider.prepare_rename(first_pos) {
        println!("   Can rename '{}' at {:?}", current_name, range);

        // Perform rename
        let rename_result = rename_provider.rename(first_pos, "num1", &RenameOptions::default());

        if rename_result.is_valid {
            println!("   Found {} edits to apply:", rename_result.edits.len());
            for (i, edit) in rename_result.edits.iter().enumerate() {
                let preview_start = edit.location.start.saturating_sub(10);
                let preview_end = (edit.location.end + 10).min(code.len());
                let preview = &code[preview_start..preview_end];
                println!("     Edit {}: {:?} -> '{}'", i + 1, edit.location, edit.new_text);
                println!("       Context: ...{}...", preview.replace('\n', "\\n"));
            }

            // Apply the rename
            let renamed_code = apply_rename_edits(code, &rename_result.edits);

            // Show a snippet of the renamed code
            println!("\n   After rename (snippet):");
            for line in renamed_code.lines().skip(25).take(5) {
                println!("     {}", line);
            }
        } else if let Some(error) = rename_result.error {
            println!("   Rename failed: {}", error);
        }
    }

    // Test 2: Try to rename a built-in (should fail)
    println!("\n2. Attempting to rename built-in 'print':");
    let print_pos = code.find("print ").unwrap();

    if let Some((_, name)) = rename_provider.prepare_rename(print_pos) {
        println!("   Prepare returned: {}", name);
    } else {
        println!("   Cannot rename built-in functions (as expected)");
    }

    // Test 3: Rename with validation
    println!("\n3. Testing name validation:");
    let second_pos = code.find("my $second").unwrap() + "my $".len();

    let invalid_names = vec!["123invalid", "my-var", "if", ""];
    for invalid_name in invalid_names {
        let result = rename_provider.rename(second_pos, invalid_name, &RenameOptions::default());
        if let Some(error) = result.error {
            println!("   '{}' -> Error: {}", invalid_name, error);
        }
    }

    // 4. Combined Features Demo
    println!("\n\n=== Combined Features Demo ===");
    println!("Simulating an IDE workflow:");

    // Step 1: User types 'sub calc'
    println!("\n1. User starts typing a new function:");
    let partial_code = format!("{}\n\nsub calc", code);
    let partial_ast = Parser::new(&partial_code).parse().unwrap_or(ast.clone());
    let comp_provider = CompletionProvider::new(&partial_ast);

    let completions = comp_provider.get_completions(&partial_code, partial_code.len());
    println!("   Keyword completions for 'sub':");
    for comp in completions.iter().filter(|c| c.label.contains("sub")) {
        println!("     - {}", comp.insert_text.as_deref().unwrap_or(&comp.label));
    }

    // Step 2: User completes function and calls it
    println!("\n2. User writes function and starts calling it:");
    let with_function = format!(
        "{}\n\nsub calculate_average {{\n    my (@values) = @_;\n    # ...\n}}\n\nmy $avg = calculate_average(",
        code
    );

    let func_ast = Parser::new(&with_function).parse().unwrap_or(ast.clone());
    let sig_provider = SignatureHelpProvider::new(&func_ast);

    if let Some(help) = sig_provider.get_signature_help(&with_function, with_function.len() - 1) {
        println!("   Signature help for new function:");
        for sig in &help.signatures {
            println!("     {}", sig.label);
        }
    }

    println!("\n✓ Advanced IDE features demonstration complete!");
}

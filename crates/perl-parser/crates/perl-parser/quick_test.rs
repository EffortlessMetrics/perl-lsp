use std::io::Write;

fn main() {
    // Test cases that might still have issues
    let test_cases = vec![
        // Complex heredocs
        (r#"<<EOF
Line 1
Line 2
EOF
"#, "heredoc_simple"),
        
        // Nested data structures
        ("$hash->{key}->[0]->{sub}", "deep_dereference"),
        
        // Complex regex
        ("/$var.*\\Q$literal\\E/", "regex_with_quotemeta"),
        
        // Statement modifiers
        ("print $x if $y", "statement_modifier"),
        
        // Labels
        ("LABEL: for (@list) { next LABEL; }", "labeled_loop"),
        
        // Attributes
        ("sub foo : lvalue { }", "sub_attributes"),
        ("my $x :shared;", "var_attributes"),
        
        // Special blocks
        ("BEGIN { }", "begin_block"),
        ("END { }", "end_block"),
        
        // Smart match
        ("$x ~~ $y", "smart_match"),
        
        // ISA operator
        ("$obj ISA 'Class'", "isa_operator"),
        
        // File test operators
        ("-f $file", "file_test"),
        
        // Format declaration
        ("format STDOUT =\n.\n", "format_decl"),
    ];
    
    println!("Testing edge cases that might need attention:\n");
    
    for (code, desc) in test_cases {
        print!("{:<20} - ", desc);
        std::io::stdout().flush().unwrap();
        
        // In a real test, we'd parse and check
        // For now, just list what needs testing
        println!("Code: {}", code.replace('\n', "\\n"));
    }
    
    println!("\nNext steps:");
    println!("1. Test each of these cases with perl-parser");
    println!("2. Fix any parsing failures");
    println!("3. Ensure tree-sitter compatible output");
}
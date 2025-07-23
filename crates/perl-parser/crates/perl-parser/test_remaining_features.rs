use perl_parser::Parser;

fn test_feature(name: &str, code: &str) -> bool {
    print!("Testing {:<25} - ", name);
    
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            let sexp = ast.to_sexp();
            println!("✅ Parsed: {}", sexp.replace('\n', " "));
            true
        }
        Err(e) => {
            println!("❌ Failed: {:?}", e);
            false
        }
    }
}

fn main() {
    println!("=== Testing Remaining Perl Features ===\n");
    
    let mut passed = 0;
    let mut total = 0;
    
    // Test cases
    let tests = vec![
        // Statement modifiers
        ("statement_modifier_if", "print $x if $y"),
        ("statement_modifier_unless", "die 'error' unless $ok"),
        ("statement_modifier_while", "print $_ while <STDIN>"),
        ("statement_modifier_for", "print $_ for @list"),
        
        // ISA operator
        ("isa_operator", "$obj ISA 'MyClass'"),
        ("isa_with_var", "$x ISA $class"),
        
        // File test operators
        ("file_test_f", "-f $filename"),
        ("file_test_d", "-d '/tmp'"),
        ("file_test_e", "-e $path"),
        ("file_test_chain", "-f $file && -r $file"),
        
        // Smart match
        ("smart_match_simple", "$x ~~ $y"),
        ("smart_match_array", "$x ~~ @array"),
        ("smart_match_regex", "$x ~~ /pattern/"),
        
        // Labels
        ("labeled_statement", "LABEL: print 'hello'"),
        ("labeled_loop", "LOOP: for (@items) { }"),
        ("next_with_label", "for (@x) { next LABEL if $x }"),
        
        // Attributes
        ("sub_attribute", "sub foo : lvalue { }"),
        ("sub_multi_attr", "sub bar : lvalue : method { }"),
        ("var_attribute", "my $x :shared"),
        
        // Special blocks
        ("begin_block", "BEGIN { print 'compile time' }"),
        ("end_block", "END { print 'cleanup' }"),
        ("check_block", "CHECK { }"),
        ("init_block", "INIT { }"),
        
        // Complex expressions
        ("ternary_operator", "$x ? $y : $z"),
        ("chained_comparison", "$a < $b < $c"),
        ("range_operator", "1..10"),
        ("flip_flop", "if (/start/ .. /end/) { }"),
        
        // Modern features
        ("defined_or", "$x // 'default'"),
        ("state_var", "state $count = 0"),
        ("say_function", "say 'hello'"),
        
        // Edge cases we fixed
        ("regex_modifiers", "/pattern/imsx"),
        ("substitution", "$x =~ s/foo/bar/g"),
        ("transliteration", "$x =~ tr/a-z/A-Z/"),
        ("qw_construct", "qw(one two three)"),
        ("heredoc", "<<'EOF'\nHello\nEOF"),
    ];
    
    for (name, code) in tests {
        if test_feature(name, code) {
            passed += 1;
        }
        total += 1;
    }
    
    println!("\n=== Summary ===");
    println!("Passed: {}/{} ({:.1}%)", passed, total, (passed as f64 / total as f64) * 100.0);
    
    if passed < total {
        println!("\nFeatures that need implementation:");
        println!("1. Statement modifiers (if/unless/while/for after statements)");
        println!("2. ISA operator for type checking");
        println!("3. File test operators (-f, -d, -e, etc.)");
        println!("4. Proper label handling");
        println!("5. Variable attributes parsing");
    }
}
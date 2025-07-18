use tree_sitter_perl::full_parser::FullPerlParser;

fn main() {
    println!("Testing heredoc parsing...\n");
    
    let test_cases = vec![
        ("basic_heredoc", r#"my $text = <<'EOF';
Hello, World!
This is a heredoc.
EOF
print $text;"#),
        
        ("interpolated_heredoc", r#"my $name = "World";
my $greeting = <<EOF;
Hello, $name!
Welcome to Perl.
EOF
print $greeting;"#),
        
        ("multiple_heredocs", r#"print <<A, <<B, <<C;
First content
A
Second content
B
Third content
C"#),
        
        ("indented_heredoc", r#"if ($condition) {
    my $config = <<~'CONFIG';
        server: localhost
        port: 8080
        debug: true
        CONFIG
    print $config;
}"#),
        
        ("heredoc_in_expression", r#"my $result = process(<<'DATA') + calculate(42);
Input data for
processing function
DATA
print $result;"#),
        
        ("heredoc_with_empty_lines", r#"my $text = <<'EOF';
Line 1

Line 3 (with empty line above)
EOF
print $text;"#),
    ];
    
    for (name, input) in test_cases {
        print!("{}: ", name);
        
        let mut parser = FullPerlParser::new();
        match parser.parse(input) {
            Ok(_) => println!("✓ PASSED"),
            Err(e) => println!("✗ FAILED - {:?}", e),
        }
    }
}
use tree_sitter_perl::EnhancedFullParser;

fn main() {
    // Test enhanced heredoc parsing
    let heredoc_code = r#"
my $cmd = <<`CMD`;
echo "Hello from shell"
date
CMD

my $text = <<\EOF;
This has $no interpolation
EOF

my $indented = <<~'END';
    This is indented
    content
END

print $cmd, $text, $indented;
"#;

    let mut parser = EnhancedFullParser::new();
    match parser.parse(heredoc_code) {
        Ok(ast) => {
            println!("✓ Enhanced heredoc parsing successful!");
            println!("AST: {:#?}", ast);
        }
        Err(e) => println!("✗ Enhanced heredoc parsing failed: {:?}", e),
    }

    // Test DATA section
    let data_code = r#"
#!/usr/bin/perl
print "Hello World\n";

__DATA__
This is data content
that can be read with <DATA>
"#;

    match parser.parse(data_code) {
        Ok(_ast) => println!("✓ DATA section parsing successful!"),
        Err(e) => println!("✗ DATA section parsing failed: {:?}", e),
    }

    // Test POD extraction
    let pod_code = r#"
print "Before POD\n";

=head1 NAME

TestModule - A test module

=head2 SYNOPSIS

    use TestModule;

=cut

print "After POD\n";
"#;

    match parser.parse(pod_code) {
        Ok(_ast) => println!("✓ POD parsing successful!"),
        Err(e) => println!("✗ POD parsing failed: {:?}", e),
    }

    // Test complex heredoc in hash
    let complex_code = r#"
my %config = (
    name => "Test",
    description => <<'DESC',
This is a long description
that spans multiple lines
DESC
    version => "1.0",
);
"#;

    match parser.parse(complex_code) {
        Ok(_ast) => println!("✓ Complex heredoc parsing successful!"),
        Err(e) => println!("✗ Complex heredoc parsing failed: {:?}", e),
    }
}

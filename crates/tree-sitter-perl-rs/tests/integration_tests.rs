//! Integration tests combining multiple new features

#[cfg(feature = "pure-rust")]
mod tests {
    use tree_sitter_perl::pure_rust_parser::PureRustPerlParser;
    use tree_sitter_perl::stateful_parser::StatefulPerlParser;

    #[test]
    fn test_complex_perl_code() {
        let mut parser = PureRustPerlParser::new();
        
        let code = r#"
package MyModule 1.0 {
    use strict;
    use warnings;
    
    # Lexical subroutine with signature
    my sub _helper ($x, $y = 10) {
        state $counter = 0;
        $counter++;
        return $x + $y + $counter;
    }
    
    # Method using typeglob
    sub import {
        my $class = shift;
        my $caller = caller;
        
        # Install into caller's symbol table
        *{"${caller}::helper"} = \&_helper;
        
        # Tie a variable
        tie ${"${caller}::magic"}, 'MyModule::Magic';
    }
    
    # Given/when with various operators
    sub process ($input) {
        given ($input) {
            when ($_ isa 'MyClass') { return $input->method() }
            when ($_ ~~ [1..10]) { return $input * 2 }
            when (defined $_ && $_ ne '') { return $input // 'default' }
            default { return undef }
        }
    }
}

# Format declaration
format REPORT =
@<<<<<<< @||||| @>>>>>
$name,   $status, $score
.

# Complex expression with precedence
my $result = $a + $b * $c ** 2 // $default;

# Nested delimiters
my $json = qq({
    "name": "test",
    "data": {
        "nested": "value"
    }
});

# Postfix dereference
my @items = $ref->@*;
my %hash = $ref->%*;

print "Done\n";
"#;
        
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        
        // Verify key features are parsed
        assert!(sexp.contains("package_declaration"));
        assert!(sexp.contains("subroutine"));
        assert!(sexp.contains("given_statement"));
        assert!(sexp.contains("format_declaration"));
        assert!(sexp.contains("typeglob_variable"));
        assert!(sexp.contains("tie_statement"));
        assert!(sexp.contains("postfix_deref"));
    }

    #[test]
    fn test_stateful_parsing_heredoc_and_format() {
        let mut parser = StatefulPerlParser::new();
        
        let code = r#"
# Heredoc followed by format
my $message = <<'END_MESSAGE';
This is a heredoc
with multiple lines
END_MESSAGE

format STDOUT =
Name: @<<<<<<<<<<<<<<
      $name
Age:  @###
      $age
.

# Another heredoc
print <<~EOF;
    This is indented
    heredoc content
    EOF

print "Done\n";
"#;
        
        let ast = parser.parse(code).unwrap();
        // The stateful parser should properly handle both heredocs and format
    }

    #[test]
    fn test_operator_precedence_complex() {
        let mut parser = PureRustPerlParser::new();
        
        // Complex expression testing all precedence levels
        let code = r#"
my $x = !$a && $b || $c // $d ? $e + $f * $g ** 2 : $h <=> $i;
my $y = $obj isa Class && $val ~~ @list or $flag;
my $z = $bits &. $mask |. $other ^. $toggle;
"#;
        
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        
        // Verify operators are parsed
        assert!(sexp.contains("&&"));
        assert!(sexp.contains("||"));
        assert!(sexp.contains("//"));
        assert!(sexp.contains("isa"));
        assert!(sexp.contains("~~"));
        assert!(sexp.contains("&."));
    }

    #[test]
    fn test_real_world_perl_patterns() {
        let mut parser = PureRustPerlParser::new();
        
        // Common Perl idioms using new features
        let code = r#"
# Modern Perl OO with signatures
package Point {
    sub new ($class, $x = 0, $y = 0) {
        bless { x => $x, y => $y }, $class;
    }
    
    sub distance ($self, $other) {
        return sqrt(($self->{x} - $other->{x})**2 + 
                   ($self->{y} - $other->{y})**2);
    }
}

# Using tie for magical behavior
tie my %cache, 'Tie::Cache', 100;
$cache{key} //= expensive_operation();

# Type checking with isa
sub process_data ($obj) {
    if ($obj isa 'Point') {
        return $obj->distance(Point->new(0, 0));
    }
    elsif ($obj isa 'ARRAY') {
        return scalar $obj->@*;
    }
    else {
        die "Unknown object type";
    }
}

# Smart matching
my $status = do {
    given ($response_code) {
        when ([200..299]) { 'success' }
        when ([300..399]) { 'redirect' }
        when ([400..499]) { 'client_error' }
        when ([500..599]) { 'server_error' }
        default { 'unknown' }
    }
};
"#;
        
        let ast = parser.parse(code).unwrap();
        let sexp = parser.to_sexp(&ast);
        
        // Verify parsing succeeded with complex real-world patterns
        assert!(sexp.contains("source_file"));
        assert!(!sexp.contains("ERROR"));
    }
}
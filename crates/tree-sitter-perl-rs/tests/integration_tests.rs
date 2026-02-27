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

        let _ast = parser.parse(code).unwrap();
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

    #[test]
    fn test_edge_case_integration() {
        use tree_sitter_perl::{
            edge_case_handler::{EdgeCaseConfig, EdgeCaseHandler},
            tree_sitter_adapter::TreeSitterAdapter,
        };

        let code = r#"
# Mix of normal and edge case code
use strict;
use warnings;

# Normal heredoc
my $normal = <<'EOF';
This is a standard heredoc
EOF

# Dynamic delimiter
my $delimiter = "END";
my $dynamic = <<$delimiter;
Dynamic delimiter content
END

# Phase dependent
BEGIN {
    our $CONFIG = <<'CFG';
    compile-time config
CFG
}

# Format with heredoc
format REPORT =
<<'HEADER'
Report Header
HEADER
@<<<<<<<<<< @>>>>>>>>>
$name,      $value
.

# Final normal code
print "Done\n";
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Should detect multiple edge cases
        assert!(!analysis.diagnostics.is_empty());
        assert!(!analysis.delimiter_resolutions.is_empty());

        // Convert to tree-sitter format
        let ts_output =
            TreeSitterAdapter::convert_to_tree_sitter(analysis.ast, analysis.diagnostics, code);

        // Verify tree-sitter compatibility
        assert_eq!(ts_output.tree.root.node_type, "source_file");
        assert!(ts_output.metadata.edge_case_count > 0);

        // Should have both clean and problematic nodes
        assert!(ts_output.metadata.parse_coverage > 50.0);
    }

    #[test]
    fn test_recovery_mode_effectiveness() {
        use tree_sitter_perl::{
            dynamic_delimiter_recovery::RecoveryMode,
            edge_case_handler::{EdgeCaseConfig, EdgeCaseHandler},
        };

        let code = r#"
my $delim = "EOF";
my $text = <<$delim;
This should be recoverable
EOF
"#;

        // Test BestGuess mode
        let config =
            EdgeCaseConfig { recovery_mode: RecoveryMode::BestGuess, ..Default::default() };

        let mut handler = EdgeCaseHandler::new(config);
        let analysis = handler.analyze(code);

        // Should successfully recover the delimiter
        assert_eq!(analysis.delimiter_resolutions.len(), 1);
        assert!(analysis.delimiter_resolutions[0].resolved_to.is_some());
        assert_eq!(analysis.delimiter_resolutions[0].resolved_to.as_ref().unwrap(), "EOF");
    }

    #[test]
    fn test_encoding_aware_heredocs() {
        use tree_sitter_perl::edge_case_handler::{EdgeCaseConfig, EdgeCaseHandler};

        let code = r#"
use encoding 'latin1';
my $latin = <<'END';
Latin-1 content
END

use utf8;
my $unicode = <<'終了';
Unicode content
終了

no utf8;
my $bytes = <<'BYTES';
Back to bytes
BYTES
"#;

        let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
        let analysis = handler.analyze(code);

        // Should have encoding-related diagnostics
        let encoding_diagnostics = analysis
            .diagnostics
            .iter()
            .filter(|d| d.message.contains("encoding") || d.message.contains("utf8"))
            .count();

        assert!(encoding_diagnostics > 0);
    }
}

//! Edge case testing for tree-sitter-perl parser
//!
//! This tests various edge cases and tricky Perl constructs

use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Tree-sitter Perl Edge Case Tests ===\n");

    let edge_cases = [
        // Heredoc edge cases
        (
            "Simple heredoc",
            r#"
my $text = <<EOF;
This is a heredoc
with multiple lines
EOF
print $text;
"#,
        ),
        (
            "Multiple heredocs on one line",
            r#"
print <<EOF, <<'END';
First heredoc
EOF
Second heredoc
END
"#,
        ),
        (
            "Indented heredoc with ~",
            r#"
my $indented = <<~"END";
    This is indented
    heredoc content
    END
"#,
        ),
        (
            "Heredoc with interpolation",
            r#"
my $name = "World";
my $greeting = <<"GREETING";
Hello, $name!
Welcome to Perl
GREETING
"#,
        ),
        (
            "Heredoc in expression",
            r#"
my $result = join("\n", <<'HEADER', @data, <<'FOOTER');
=== Header ===
HEADER
=== Footer ===
FOOTER
"#,
        ),
        // Tricky syntax edge cases
        (
            "Bareword filehandles",
            r#"
open FH, '<', 'file.txt';
print FH "data";
close FH;
"#,
        ),
        (
            "Indirect object syntax",
            r#"
my $obj = new Class::Name;
my $result = method $obj @args;
"#,
        ),
        (
            "Statement modifiers",
            r#"
print "Hello" if $condition;
die "Error" unless defined $value;
next while $iterator->has_next;
"#,
        ),
        (
            "Complex dereferencing",
            r#"
my $value = $hash->{key}->[0]->{nested}->[1];
my @array = @{$ref->{data}};
my %hash = %{$obj->get_hash_ref()};
"#,
        ),
        (
            "Glob assignment",
            r#"
*foo = *bar;
*{$package . '::function'} = sub { print "Dynamic sub" };
local *STDOUT;
"#,
        ),
        // Regex edge cases
        (
            "Complex regex with modifiers",
            r#"
$text =~ s{
    (\w+)       # word
    \s*         # optional whitespace  
    =           # equals
    \s*         # optional whitespace
    (\S+)       # value
}{$1 => $2}gex;
"#,
        ),
        (
            "Regex with unusual delimiters",
            r#"
$text =~ m!pattern!;
$text =~ s|old|new|g;
$text =~ tr/a-z/A-Z/;
"#,
        ),
        // Quote-like operator edge cases
        (
            "qw with various delimiters",
            r#"
my @words1 = qw(one two three);
my @words2 = qw{four five six};
my @words3 = qw[seven eight nine];
my @words4 = qw!ten eleven twelve!;
"#,
        ),
        (
            "qq and q operators",
            r#"
my $interpolated = qq{Hello $name};
my $literal = q{Hello $name};
my $custom = qq|Path: $ENV{PATH}|;
"#,
        ),
        // Special variable edge cases
        (
            "Special variables",
            r#"
local $" = ', ';
print "@array";
$\ = "\n";
$, = "\t";
print @data;
"#,
        ),
        (
            "Typeglobs and symbol table",
            r#"
*alias = \$scalar;
*{$package . '::var'} = \$value;
my $ref = \*STDOUT;
"#,
        ),
        // Package and namespace edge cases
        (
            "Package with version",
            r#"
package Foo::Bar 1.23;
use parent qw(Base::Class);
our $VERSION = '1.23';
"#,
        ),
        (
            "Nested packages",
            r#"
package Outer {
    package Inner {
        sub method { }
    }
}
"#,
        ),
        // Format edge cases
        (
            "Format declaration",
            r#"
format STDOUT =
@<<<<<<< @||||| @>>>>>>
$name,   $id,   $score
.
write;
"#,
        ),
        // Attribute edge cases
        (
            "Subroutine attributes",
            r#"
sub method : lvalue : method {
    $self->{value};
}

my $shared : shared;
"#,
        ),
        // Modern Perl edge cases
        (
            "Signatures and prototypes",
            r#"
sub add ($x, $y) { $x + $y }
sub old_style ($$) { $_[0] + $_[1] }
sub optional ($x, $y = 0) { $x + $y }
"#,
        ),
        (
            "Given/when (deprecated but valid)",
            r#"
use feature 'switch';
given ($value) {
    when (1) { say "one" }
    when ([2,3,4]) { say "two to four" }
    default { say "other" }
}
"#,
        ),
        // Unicode edge cases
        (
            "Unicode in various contexts",
            r#"
my $καλημέρα = "good morning";
sub Σ { sum(@_) }
my %χάρτης = (κλειδί => 'τιμή');
"#,
        ),
        (
            "Mixed Unicode scripts",
            r#"
my $مرحبا = "hello";
my $你好 = "hello";
my $🐪 = "camel";
"#,
        ),
        // Edge cases with special parsing
        (
            "Slash ambiguity",
            r#"
my $result = $x / $y / $z;  # division
$text =~ /pattern/;         # regex
print $q/$r/$s;            # division in print
"#,
        ),
        (
            "Contextual keywords",
            r#"
my $sub = "not a keyword";
my %if = (then => 1, else => 2);
sub for { print "for sub" }
"#,
        ),
        // Operator edge cases
        (
            "Unusual operators",
            r#"
my $match = $a ~~ $b;
my $isa = $obj isa Some::Class;
my $range = 1..10;
my $flipflop = $start ... $end;
"#,
        ),
        (
            "Compound assignments",
            r#"
$x //= 0;
$y &&= 1;
$z ||= [];
$a .= "suffix";
"#,
        ),
        // Error recovery edge cases
        (
            "Missing semicolons",
            r#"
my $x = 1
my $y = 2
print $x + $y
"#,
        ),
        (
            "Unclosed constructs",
            r#"
if ($x) {
    print "unclosed"
# missing closing brace
"#,
        ),
        // Perl-specific weirdness
        (
            "Perl golf constructs",
            r#"
@{[do{local$"=')(';qq(@{[sort@_]})}]}
$_=reverse;print;
"#,
        ),
        (
            "Bareword before =>",
            r#"
my %hash = (
    bareword => 'value',
    -option => 'dash',
    Class::Name => 'qualified',
);
"#,
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;

    for (name, code) in edge_cases {
        print!("Testing {}: ", name);

        let mut parser = EnhancedFullParser::new();
        match parser.parse(code) {
            Ok(ast) => {
                // Basic validation - check if we got an AST
                if validate_ast(&ast) {
                    println!("✓ PASSED");
                    passed += 1;

                    // For debugging, show structure of interesting cases
                    if name.contains("heredoc") || name.contains("Unicode") {
                        print_ast_summary(&ast, 2);
                    }
                } else {
                    println!("✗ FAILED (invalid AST structure)");
                    failed += 1;
                }
            }
            Err(e) => {
                println!("✗ FAILED (parse error: {})", e);
                failed += 1;

                // Show the error location if available
                if let Some(pos) = extract_error_position(&e) {
                    println!("  Error at line {}, column {}", pos.0, pos.1);
                }
            }
        }
        println!();
    }

    println!("\n=== Summary ===");
    println!("Total tests: {}", passed + failed);
    println!("Passed: {} ({}%)", passed, (passed * 100) / (passed + failed).max(1));
    println!("Failed: {} ({}%)", failed, (failed * 100) / (passed + failed).max(1));

    if failed > 0 {
        println!("\nNote: Some edge cases may fail due to parser limitations.");
        println!("This is expected for experimental or deprecated Perl features.");
    }
}

fn validate_ast(ast: &AstNode) -> bool {
    // Basic validation - ensure we have a non-empty AST
    match ast {
        AstNode::Program(items) => !items.is_empty(),
        _ => false,
    }
}

fn print_ast_summary(ast: &AstNode, max_depth: usize) {
    fn print_node(node: &AstNode, depth: usize, max_depth: usize) {
        if depth > max_depth {
            return;
        }

        let indent = "  ".repeat(depth);
        match node {
            AstNode::Program(items) => {
                println!("{}Program ({} items)", indent, items.len());
                for item in items.iter().take(3) {
                    print_node(item, depth + 1, max_depth);
                }
                if items.len() > 3 {
                    println!("{}...", "  ".repeat(depth + 1));
                }
            }
            AstNode::Statement(content) => {
                println!("{}Statement", indent);
                print_node(content, depth + 1, max_depth);
            }
            AstNode::Heredoc { marker, content, .. } => {
                println!("{}Heredoc [{}]: {} chars", indent, marker, content.len());
            }
            AstNode::Identifier(name) => {
                println!("{}Identifier: {}", indent, name);
            }
            AstNode::String(s) => {
                let preview = if s.len() > 20 { format!("{}...", &s[..20]) } else { s.to_string() };
                println!("{}String: \"{}\"", indent, preview);
            }
            _ => {
                println!("{}[{:?}]", indent, node_type_name(node));
            }
        }
    }

    print_node(ast, 1, max_depth);
}

fn node_type_name(node: &AstNode) -> &'static str {
    match node {
        AstNode::Program(_) => "Program",
        AstNode::Statement(_) => "Statement",
        AstNode::SubDeclaration { .. } => "SubDeclaration",
        AstNode::VariableDeclaration { .. } => "VariableDeclaration",
        AstNode::UseStatement { .. } => "UseStatement",
        AstNode::PackageDeclaration { .. } => "PackageDeclaration",
        AstNode::IfStatement { .. } => "IfStatement",
        AstNode::ForStatement { .. } => "ForStatement",
        AstNode::WhileStatement { .. } => "WhileStatement",
        AstNode::BinaryOp { .. } => "BinaryOp",
        AstNode::UnaryOp { .. } => "UnaryOp",
        AstNode::FunctionCall { .. } => "FunctionCall",
        AstNode::Regex { .. } => "Regex",
        AstNode::Substitution { .. } => "Substitution",
        AstNode::Heredoc { .. } => "Heredoc",
        AstNode::Identifier(_) => "Identifier",
        AstNode::String(_) => "String",
        AstNode::Number(_) => "Number",
        AstNode::List(_) => "List",
        AstNode::HashRef(_) => "HashRef",
        AstNode::ArrayRef(_) => "ArrayRef",
        _ => "Other",
    }
}

fn extract_error_position(error: &perl_parser_pest::error::ParseError) -> Option<(usize, usize)> {
    // Try to extract line/column from error message
    let error_str = format!("{}", error);
    if error_str.contains("line") {
        // Simple extraction - would need proper implementation
        None
    } else {
        None
    }
}

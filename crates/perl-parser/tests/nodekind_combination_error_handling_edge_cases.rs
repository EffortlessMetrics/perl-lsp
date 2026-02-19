//! Comprehensive tests for error handling and edge cases
//!
//! These tests validate complex interactions between error conditions
//! including multiple error conditions, parser recovery, malformed inputs,
//! ambiguous syntax, and resource limits.

use perl_parser::Parser;

mod nodekind_helpers;
use nodekind_helpers::has_node_kind;

/// Test multiple error conditions in single constructs
#[test]
fn test_multiple_error_conditions_single_constructs() {
    let code = r#"
# Multiple errors in subroutine declaration
sub problematic_sub {
    my ($param1, $param2,;  # Syntax error: extra comma
    my $local_var =;           # Syntax error: missing initializer
    my @array = (1, 2,      # Syntax error: missing closing paren
    my %hash = (              # Syntax error: incomplete hash
    
    # Multiple runtime errors
    die "Error 1" if $param1;      # Potential die
    die "Error 2" unless $param2;     # Potential die
    warn "Warning 1" if !defined $local_var;  # Potential warn
    warn "Warning 2" unless @array;         # Potential warn
    
    # Complex error condition
    if ($param1 && $param2 || !defined $local_var && @array && %hash) {
        die "Complex error condition";
    }
    
    # Error in return
    return;  # Missing return value in context expecting value
}

# Multiple errors in eval
eval {
    my $x = 1 +;          # Syntax error in eval
    my $y = /unclosed_regex;  # Syntax error in eval
    die "Eval error";      # Die in eval
};

if ($@) {
    print "Eval failed: $@\n";
}

# Multiple errors in file operations
open my $fh, '<', 'nonexistent.txt' or die "Cannot open: $!";
my $line = <$fh> or die "Cannot read: $!";
close $fh or die "Cannot close: $!";

# Multiple errors in regex operations
my $text = "test";
if ($text =~ /unclosed_regex) {  # Unclosed regex
    print "Match\n";
}

if ($text =~ s/unclosed_sub/) {  # Unclosed substitution
    print "Substituted\n";
}

# Multiple errors in data structures
my @bad_array = (1, 2, ;     # Syntax error in array
my %bad_hash = (              # Syntax error in hash
    key1 => 1,
    key2 =>,                # Missing value
);

# Multiple errors in control flow
if ($x == $y) {             # Undefined variables
    print "Undefined comparison\n";
}

while (my $item = @bad_array) {  # Wrong syntax for array iteration
    print "$item\n";
}

# Multiple errors in package declaration
package Bad::Package {
    our $VERSION = '1.0;
    # Missing closing quote
    
    sub problematic_method {
        my ($self) = shift;
        return $self->{nonexistent};  # Potential runtime error
    }
}

# Multiple errors in use statements
use Non::Existent::Module;    # Non-existent module
use warnings 'all';           # Valid
use strict 'refs';            # Valid

# Multiple errors in typeglob operations
*OLD_HANDLE = *NEW_HANDLE;  # Typeglob assignment
*ANOTHER_GLOB = \&subroutine;  # Typeglob with code ref

# Multiple errors in complex expressions
my $complex = $x + $y * $z / 0;  # Division by zero
my $nested = $hash->{key}[0]{subkey};  # Potential undefined dereference

# Multiple errors in exception handling
eval {
    die "Inner error";
};

if ($@) {
    eval {
        die "Outer error";
    };
    
    if ($@) {
        die "Nested error: $@";
    }
}

# Multiple errors in subroutine calls
&nonexistent_sub();  # Call to non-existent sub
&undefined_sub(@undefined_args);  # Call with undefined args

# Multiple errors in method calls
my $obj = bless {}, 'SomeClass';
$obj->nonexistent_method();  # Call to non-existent method
$obj->method_with_undefined_args($undef1, $undef2);  # Undefined args

# Multiple errors in regex with complex patterns
my $complex_text = "abc123def";
if ($complex_text =~ /(?P<group>\w+)(?P<duplicate>\g{group})/) {
    print "Backreference error\n";
}

# Multiple errors in file tests
if (-f 'nonexistent.txt' && -d 'nonexistent_dir' && -r 'nonexistent_readable') {
    print "All file tests failed\n";
}

# Multiple errors in string operations
my $undefined_str = undef;
my $length = length $undefined_str;  # Length of undefined
my $substr = substr($undefined_str, 0, 5);  # Substr of undefined
my $index = index($undefined_str, 'test');  # Index of undefined

# Multiple errors in array operations
my @undefined_array;
my $array_size = scalar @undefined_array;  # Size of undefined array
my $first_element = $undefined_array[0];  # Access undefined array
my $array_slice = @undefined_array[0..2];  # Slice undefined array

# Multiple errors in hash operations
my %undefined_hash;
my $hash_keys = keys %undefined_hash;  # Keys of undefined hash
my $hash_values = values %undefined_hash;  # Values of undefined hash
my $hash_exists = exists $undefined_hash{key};  # Exists in undefined hash

# Multiple errors in reference operations
my $undefined_ref = undef;
my $deref = $$undefined_ref;  # Dereference undefined
my $method_call = $undefined_ref->();  # Method call on undefined
my $array_deref = @$undefined_ref;  # Array dereference of undefined
my $hash_deref = %$undefined_ref;  # Hash dereference of undefined
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify error nodes
    assert!(has_node_kind(&ast, "Error"), "Should have error nodes");

    // Verify missing nodes
    assert!(
        has_node_kind(&ast, "MissingExpression")
            || has_node_kind(&ast, "MissingStatement")
            || has_node_kind(&ast, "MissingIdentifier")
            || has_node_kind(&ast, "MissingBlock"),
        "Should have missing nodes"
    );

    // Verify eval blocks for error handling
    assert!(has_node_kind(&ast, "Eval"), "Should have eval blocks");

    // Verify die/warn operations
    assert!(has_node_kind(&ast, "FunctionCall"), "Should have function calls for die/warn");

    // Verify conditional statements with error conditions
    assert!(has_node_kind(&ast, "If"), "Should have conditional statements");
}

/// Test parser recovery with various malformed inputs
#[test]
fn test_parser_recovery_malformed_inputs() {
    let code = r#"
# Incomplete subroutine declarations
sub incomplete_sub1 {
sub incomplete_sub2 {
    my ($param1,
sub incomplete_sub3 {
    my ($param1, $param2

# Incomplete control structures
if ($condition) {
while ($condition) {
for ($i = 0; $i < 10; $i++) {
foreach my $item (@array) {

# Incomplete data structures
my @incomplete_array = (1, 2, 3
my %incomplete_hash = (
    key1 => 1,
    key2 => 2,
my $incomplete_ref = [
    {a => 1, b => 2},
    {c => 3, d => 4

# Incomplete regex patterns
my $text = "test";
if ($text =~ /unclosed_pattern
if ($text =~ s/incomplete_sub
if ($text =~ tr/incomplete_trans

# Incomplete strings
my $incomplete_string1 = "unterminated string
my $incomplete_string2 = 'unterminated string
my $incomplete_heredoc = <<EOF;
This is an unterminated heredoc

# Incomplete package declarations
package Incomplete::Package {
package Another::Incomplete

# Incomplete use statements
use Incomplete::Module
use Another::Incomplete

# Malformed subroutine signatures
sub malformed_sig1 ($param1, $param2,) {
sub malformed_sig2 ($param1, $param2 $param3) {
sub malformed_sig3 ($param1, $param2, $param3

# Malformed regex with special characters
if ($text =~ /[\w+/) {  # Unclosed character class
if ($text =~ /(?P<unclosed/) {  # Unclosed named group
if ($text =~ /(?<=unclosed/) {  # Unclosed lookbehind

# Malformed operators
my $result = $x +* $y;  # Invalid operator
my $another = $a === $b;  # Invalid comparison
my $invalid = $x &&& $y;  # Invalid logical operator

# Malformed typeglob operations
*GLOB1 = *GLOB2
*GLOB3 = \&subroutine
*GLOB4 = $scalar_ref

# Malformed file operations
open my $fh, '<', 'file.txt' or die "Error";
my $line = <$fh  # Missing closing angle bracket
close $fh

# Malformed eval blocks
eval {
    my $x = 1 +;
    die "Error";
}

# Malformed try-catch
try {
    die "Error";
} catch ($e) {
    print "Caught: $e\n";
}  # Missing semicolon

# Malformed given-when
given ($value) {
    when (/pattern/) {
        print "Match\n";
    when (/another/) {  # Missing closing brace
        print "Another match\n";

# Malformed loops
for ($i = 0; $i < 10; $i++  {  # Missing closing paren
while ($condition  {  # Missing closing paren
foreach my $item (@array  {  # Missing closing paren

# Mixed syntax errors
sub mixed_errors {
    my ($param1, $param2,);  # Extra semicolon
    my $var = 1 +;           # Incomplete expression
    my @arr = (1, 2, 3,     # Missing closing paren
    my %hash = (               # Incomplete hash
        
        # Valid code mixed with errors
        if ($param1) {
            print "Valid\n";
        }
        
        die "Error";  # Valid die
    }
    
    return 1;  # Valid return
}

# Recovery after syntax errors
print "This should still execute\n";
my $recovery_var = 42;
print "Recovery value: $recovery_var\n";

# More complex recovery scenarios
sub complex_recovery {
    my ($data) = @_;
    
    # Syntax error but parser should recover
    my $bad_expr = $data->{key}[0]{subkey} +;
    
    # Continue parsing despite error
    my $good_expr = $data->{other_key} || "default";
    
    return $good_expr;
}

my $recovered = complex_recovery({key => [1, 2, 3]});
print "Recovered: $recovered\n";
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify error nodes for malformed inputs
    assert!(has_node_kind(&ast, "Error"), "Should have error nodes");

    // Verify missing nodes for incomplete structures
    assert!(
        has_node_kind(&ast, "MissingExpression")
            || has_node_kind(&ast, "MissingStatement")
            || has_node_kind(&ast, "MissingIdentifier")
            || has_node_kind(&ast, "MissingBlock"),
        "Should have missing nodes"
    );

    // Verify that some valid structures still parse
    assert!(has_node_kind(&ast, "Subroutine"), "Should have some valid subroutine nodes");

    // Verify variable declarations
    assert!(has_node_kind(&ast, "VariableDeclaration"), "Should have variable declarations");

    // Verify function calls
    assert!(has_node_kind(&ast, "FunctionCall"), "Should have function calls");
}

// /// Test behavior with incomplete or ambiguous syntax
// Temporarily commented out due to compiler panic
// #[test]
// #[ignore] // Temporarily ignored due to compiler panic
// fn test_incomplete_ambiguous_syntax() {
//     let code = r#"
// # Ambiguous bareword vs function call
// my $result = ambiguous_function;  # Could be bareword or function call
// my $another = another_ambiguous($param);  # Could be method or function
//
// # Ambiguous operator precedence
// my $precedence1 = 1 + 2 * 3;  # Should be 1 + (2 * 3)
// my $precedence2 = 4 * 5 + 6;  # Should be (4 * 5) + 6
//
// # Ambiguous dereferencing
// my $deref1 = $$ref;  # Could be scalar deref or code deref
// my $deref2 = $hash->{key}[0];  # Could be hash then array or array then hash
//
// # Ambiguous regex delimiters
// if ($text =~ m#pattern#) {  # Using # as delimiter
// if ($text =~ m!pattern!) {  # Using ! as delimiter
// if ($text =~ m|pattern|) {  # Using | as delimiter
//
// # Ambiguous string delimiters
// my $str1 = q{test string};  # Using braces as delimiter
// my $str2 = q[test string];  # Using brackets as delimiter
// my $str3 = q|test string|;  # Using pipe as delimiter
//
// # Ambiguous heredoc delimiters
// my $heredoc1 = <<'EOF';
// Single quoted heredoc
// EOF
//
// my $heredoc2 = <<"EOF";
// Double quoted heredoc with $variable interpolation
// EOF
//
// my $heredoc3 = <<`EOF`;
// Command heredoc
// EOF
//
// # Ambiguous block syntax
// my $block1 = { 1, 2, 3 };  # Could be block or hash
// my $block2 = { 1 => 2, 3 => 4 };  # Could be block or hash
//
// # Ambiguous subroutine vs method
// sub ambiguous {
//     return "subroutine";
// }
//
// my $obj = bless {};
// sub method {
//     return "method";
// }
//
// my $result1 = ambiguous();  # Subroutine call
// my $result2 = $obj->ambiguous();  # Method call
//
// # Ambiguous package vs bareword
// package My::Package;
// my $bareword = Some::Class;  # Could be package name or bareword
//
// # Ambiguous array vs hash context
// my @array_context = (%hash);  # Hash in array context
// my %hash_context = (@array);  # Array in hash context
//
// # Ambiguous filehandle vs bareword
// open FH, '<', 'file.txt';  # Could be filehandle or bareword
// my $line = <FH>;  # Could be readline or bareword
//
// # Ambiguous reference vs string
// my $ref_or_string = "test";
// my $result = $ref_or_string->{key};  # Could be string method or dereference
//
// # Ambiguous operator vs method
// my $method_or_op = $obj->method;  # Could be method call or indirect object
// my $result2 = $method_or_op + 1;  # Could be method call then addition
//
// # Ambiguous regex match vs assignment
// my $match_or_assign = $text =~ /pattern/;  # Could be match or assignment in regex context
//
// # Ambiguous eval vs block
// eval {  # Could be eval block or hash key
//     my $x = 1;
//     $x;
// };
//
// # Ambiguous do vs block
// do {  # Could be do block or hash key
//     my $y = 2;
//     $y;
// };
//
// # Ambiguous grep vs map
// my @grep_or_map = grep { /pattern/ } @array;  # Could be grep or map with regex
// my @map_or_grep = map { s/old/new/ } @array;  # Could be map or grep with substitution
//
// # Ambiguous sort vs subroutine
// my @sorted = sort special_sub @array;  # Could be sort with sub or sort with string
//
// # Ambiguous print vs function
// print special_sub @args;  # Could be print with filehandle or function call
//
// # Ambiguous return vs list
// my @list_or_return = (return 1, 2, 3);  # Could be return statement or list
//
// # Ambiguous die vs string
// die "Error message";  # Could be die with string or string literal
//
// # Ambiguous warn vs function
// warn "Warning message";  # Could be warn with string or function call
//
// # Ambiguous use vs bareword
// use strict;  # Could be use pragma or bareword
// use warnings 'all';  # Could be use with options or bareword
//
// # Ambiguous package vs block
// package Test::Package {  # Could be package with block or package declaration
//     my $var = 1;
// }
//
// # Ambiguous sub vs prototype
// sub ($;$) {  # Could be sub with prototype or sub with optional parameter
//     return $_[0] || 'default';
// }
//
// # Ambiguous bless vs function
// my $obj = bless {}, 'Class';  # Could be bless or function call
// my $ref = bless \$scalar;  # Could be bless or function call
//
// # Ambiguous tie vs function
// tie $scalar, 'Class', @args;  # Could be tie or function call
//
// # Ambiguous undef vs function
// undef $variable;  # Could be undef function or bareword
//
// # Ambiguous defined vs function
// defined $variable;  # Could be defined function or bareword
//
// # Ambiguous ref vs function
// ref $variable;  # Could be ref function or bareword
//
// # Ambiguous scalar vs function
// scalar @array;  # Could be scalar function or bareword
// "#;
//
//     let mut parser = Parser::new(code);
//     let ast = parser.parse().expect("Should parse successfully");
//
//     // Verify that ambiguous syntax is parsed (may produce errors or warnings)
//     let error_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Error { .. }));
//     // May or may not have errors depending on parser's ambiguity resolution
//
//     // Verify function calls (could be ambiguous)
//     let function_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::FunctionCall { .. }));
//     assert!(!function_calls.is_empty(), "Should have function calls");
//
//     // Verify method calls (could be ambiguous)
//     let method_calls = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::MethodCall { .. }));
//     assert!(!method_calls.is_empty(), "Should have method calls");
//
//     // Verify package declarations
//     let package_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Package { .. }));
//     assert!(!package_nodes.is_empty(), "Should have package declarations");
//
//     // Verify subroutine declarations
//     let sub_nodes = find_nodes_of_kind(&ast, |k| matches!(k, NodeKind::Subroutine { .. }));
//     assert!(!sub_nodes.is_empty(), "Should have subroutine declarations");
// }

/// Test resource limits and boundary conditions
#[test]
fn test_resource_limits_boundary_conditions() {
    let code = r#"
# Very long identifiers
my $very_long_variable_name_that_exceeds_reasonable_limits_and_should_test_parser_boundaries = 1;
my @very_long_array_name_that_tests_parser_limits_and_boundary_conditions = (1, 2, 3);
my %very_long_hash_name_that_tests_parser_resource_limits_and_boundary_handling = (key => 'value');

sub very_long_subroutine_name_that_tests_parser_limits_and_boundary_conditions_and_error_handling {
    my ($very_long_parameter_name_that_tests_parser_limits) = @_;
    return $very_long_parameter_name_that_tests_parser_limits * 2;
}

# Deep nesting
my $deeply_nested = {
    level1 => {
        level2 => {
            level3 => {
                level4 => {
                    level5 => {
                        level6 => {
                            level7 => {
                                level8 => {
                                    level9 => {
                                        level10 => {
                                            deep_value => "found"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
};

# Deep array nesting
my @deep_array = (
    [
        [
            [
                [
                    [
                        [
                            [
                                [
                                    [
                                        [
                                            [
                                                "deeply nested"
                                        ]
                                    ]
                                ]
                            ]
                        ]
                    ]
                ]
            ]
        ]
    ]
);

# Complex regex with many groups
my $complex_regex = qr/
    ^(?P<protocol>https?):\/\/
    (?P<host>[^\/:]+)
    (?::(?P<port>\d+))?
    (?P<path>\/.*)$
    /x;

# Large data structures
my @large_array = (1..10000);
my %large_hash = map { $_ => $_ * 2 } 1..5000;

# Complex expressions with many operators
my $complex_expr = 
    $var1 + $var2 * $var3 / $var4 % $var5 ** $var6 + 
    $var7 && $var8 || $var9 & $var10 | $var11 ^ 
    $var12 << $var13 >> $var14 . $var15 x $var16;

# Many parameters
sub many_params {
    my (
        $param1, $param2, $param3, $param4, $param5,
        $param6, $param7, $param8, $param9, $param10,
        $param11, $param12, $param13, $param14, $param15,
        $param16, $param17, $param18, $param19, $param20
    ) = @_;
    
    return scalar @_;
}

# Complex conditional chains
if ($condition1 && $condition2 && $condition3 && $condition4 && $condition5) {
    print "All conditions true\n";
} elsif ($condition6 || $condition7 || $condition8 || $condition9 || $condition10) {
    print "Some elsif condition true\n";
} elsif ($condition11 && $condition12 || $condition13 && $condition14) {
    print "Complex elsif condition true\n";
} else {
    print "No conditions true\n";
}

# Many catch blocks
try {
    die "Error 1";
} catch ($e1) {
    print "Caught 1: $e1\n";
} catch ($e2) {
    print "Caught 2: $e2\n";
} catch ($e3) {
    print "Caught 3: $e3\n";
} catch ($e4) {
    print "Caught 4: $e4\n";
} catch ($e5) {
    print "Caught 5: $e5\n";
}

# Complex string operations
my $very_long_string = "This is a very long string that tests parser handling of large string literals and ensures that the parser can handle extended string content without issues including various escape sequences and embedded variables like $variable and @array and %hash references";
my $interpolated_string = "Value: $very_long_variable_name_that_exceeds_reasonable_limits_and_should_test_parser_boundaries, Array: @very_long_array_name_that_tests_parser_limits_and_boundary_conditions, Hash: %very_long_hash_name_that_tests_parser_resource_limits_and_boundary_handling";

# Complex regex with many alternatives
my $many_alternatives = qr/
    ^(
        option1|
        option2|
        option3|
        option4|
        option5|
        option6|
        option7|
        option8|
        option9|
        option10|
        option11|
        option12|
        option13|
        option14|
        option15|
        option16|
        option17|
        option18|
        option19|
        option20
    )$
    /x;

# Resource exhaustion simulation
my @exhaustive_array;
for my $i (1..100000) {
    push @exhaustive_array, {
        id => $i,
        data => "Item $i",
        nested => {
            level1 => { value => $i * 2 },
            level2 => { value => $i * 3 },
            level3 => { value => $i * 4 },
            level4 => { value => $i * 5 },
        }
    };
}

# Memory pressure simulation
my %memory_intensive;
for my $key (1..1000) {
    $memory_intensive{$key} = {
        data => "x" x 1000,  # 1KB per key
        array => [(1) x 100],   # 100 elements
        hash => { map { $_ => $_ } 1..100 },  # 100 key-value pairs
    };
}

# Boundary condition testing
sub test_boundaries {
    my ($input) = @_;
    
    # Test very large numbers
    my $large_num = 999999999999999999999;
    my $small_num = -999999999999999999999;
    
    # Test string boundaries
    my $empty_string = "";
    my $single_char = "x";
    my $max_length_string = "x" x 1000000;
    
    # Test array boundaries
    my @empty_array = ();
    my $single_element = (1);
    my $large_array = (1) x 100000;
    
    # Test hash boundaries
    my %empty_hash = ();
    my %single_key = (key => 'value');
    my %large_hash = map { $_ => $_ } 1..10000;
    
    return {
        large_num => $large_num,
        small_num => $small_num,
        empty_string => $empty_string,
        single_char => $single_char,
        max_length => $max_length_string,
        empty_array => \@empty_array,
        single_element => \@single_element,
        large_array => \@large_array,
        empty_hash => \%empty_hash,
        single_key => \%single_key,
        large_hash => \%large_hash,
    };
}

# Test the boundary conditions
my $boundary_results = test_boundaries("test_input");
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify that large structures are parsed
    assert!(has_node_kind(&ast, "HashLiteral"), "Should have hash literals");

    assert!(has_node_kind(&ast, "ArrayLiteral"), "Should have array literals");

    // Verify complex expressions
    assert!(has_node_kind(&ast, "Binary"), "Should have binary operations");

    // Verify subroutine declarations with many parameters
    assert!(has_node_kind(&ast, "Subroutine"), "Should have subroutine declarations");

    // Verify try-catch blocks
    assert!(has_node_kind(&ast, "Try"), "Should have try-catch blocks");

    // Verify regex operations
    assert!(has_node_kind(&ast, "Regex"), "Should have regex operations");

    // Verify string literals
    assert!(has_node_kind(&ast, "String"), "Should have string literals");
}

/// Test parser edge cases with unusual constructs
#[test]
fn test_parser_edge_cases_unusual_constructs() {
    let code = r#"
# Edge case: Subroutine prototype with complex characters
sub complex_proto ($\@%);\@) {  # Complex prototype
    return @_;
}

# Edge case: Subroutine with attributes
sub attr_sub :lvalue :method {
    my $self = shift;
    return $self->{value};
}

# Edge case: Format with complex format
format STDOUT =
@<<<<<<<<<<<<<<<<<< @|||||||||| @>>>>>>>>>>>>>>>
@<<<<<<<<<<<<<<<<<< @|||||||||| @>>>>>>>>>>>>>>>
@<<<<<<<<<<<<<<<<<< @|||||||||| @>>>>>>>>>>>>>>>
.

# Edge case: Heredoc with various delimiters and options
my $simple_heredoc = <<EOF;
Simple heredoc
EOF

my $quoted_heredoc = <<'EOF';
Single quoted heredoc $variable
EOF

my $double_quoted_heredoc = <<"EOF";
Double quoted heredoc $variable
EOF

my $indented_heredoc = <<~EOF;
Indented heredoc
    Content with indentation
EOF

my $command_heredoc = <<`CMD`;
echo "Command heredoc"
CMD

# Edge case: Regex with various delimiters and modifiers
my $slash_regex = /pattern/modifiers;
my $bracket_regex = [pattern]modifiers;
my $brace_regex = {pattern}modifiers;
my $angle_regex = <pattern>modifiers;
my $pipe_regex = |pattern|modifiers;
my $exclamation_regex = !pattern!modifiers;

# Edge case: Substitution with various delimiters
my $slash_sub = s/pattern/replacement/modifiers;
my $bracket_sub = s[pattern][replacement]modifiers;
my $brace_sub = s{pattern}{replacement}modifiers;
my $angle_sub = s<pattern><replacement>modifiers;
my $pipe_sub = s|pattern|replacement|modifiers;
my $exclamation_sub = s!pattern!replacement!modifiers;

# Edge case: Transliteration with various delimiters
my $slash_trans = tr/search/replace/modifiers;
my $bracket_trans = tr[search][replace]modifiers;
my $brace_trans = tr{search}{replace}modifiers;
my $angle_trans = tr<search><replace>modifiers;
my $pipe_trans = tr|search|replace|modifiers;

# Edge case: Complex quote-like operators
my $single_q = q'Single quoted string $variable';
my $double_q = qq"Double quoted string $variable with \n newline";
my $backtick_q = qx`command with $variable`;
my $qw_list = qw(item1 item2 item3 $variable item4);

# Edge case: Complex file test operations
my $file_test_result = (-f $file && -r $file && -w $file && -x $file && -o $file && -s $file && -t $file && -T $file && -B $file && -M $file && -A $file && -C $file);

# Edge case: Complex dereferencing chains
my $complex_deref = ${${${$hash_ref}{key1}}{key2}}[0]{key3};

# Edge case: Ambiguous operator combinations
my $ambiguous1 = $x . $y . $z;  # Could be concatenation or string method calls
my $ambiguous2 = $x x $y x $z;  # Could be repetition or string method calls
my $ambiguous3 = $x && $y || $z;  # Operator precedence ambiguity

# Edge case: Package with complex name
package Complex::Package::Name::With::Many::Parts;
our $VERSION = '1.0.0';

# Edge case: Use with complex arguments
use Complex::Module qw(:DEFAULT :special @export_list);
use Another::Module 'param1', 'param2', {key => 'value'};

# Edge case: No statement with complex expression
no strict 'refs';
no warnings 'uninitialized';

# Edge case: Typeglob with complex assignment
*Complex::Glob = *Simple::Glob;
*Another::Glob = \&subroutine_name;
*Third::Glob = \$scalar_variable;

# Edge case: Eval with complex content
eval qq{
    my \$var = "interpolated";
    print "Value: \$var\\n";
    die "Eval error";
};

# Edge case: Do with different syntax forms
do subroutine_name();  # Do with subroutine call
do $filename;        # Do with file
do {                # Do with block
    print "In do block\n";
};

# Edge case: Complex return statements
return;                    # Return without value
return $value;             # Return with value
return ($value1, $value2); # Return list
return {key => $value};   # Return hash
return [$value1, $value2]; # Return array

# Edge case: Complex goto statements (if supported)
LABEL1: for my $i (1..10) {
    if ($i == 5) {
        goto LABEL2;  # Complex goto
    }
    last LABEL1 if $i > 8;
}

LABEL2: print "Reached label 2\n";

# Edge case: Complex local declarations
local $SIG{__DIE__} = sub { print "Die handler\n"; };
local $SIG{__WARN__} = sub { print "Warn handler\n"; };
local $| = 1;  # Autoflush
local $\ = "\n";  # Output record separator

# Edge case: Complex tie operations
tie %Complex::Hash, 'Complex::TieClass', 
    option1 => $value1,
    option2 => $value2,
    {
        nested_option1 => $nested_value1,
        nested_option2 => $nested_value2,
    };

# Edge case: Complex binary operations with precedence
my $precedence_test = $a + $b * $c ** $d % $e << $f >> $g . $h x $y && $z || $w & $v ^ $u;

# Edge case: Complex conditional expressions
my $complex_conditional = $condition1 ? ($condition2 ? $value1 : $value2) : ($condition3 ? $value3 : $value4);

# Edge case: Complex map/grep operations
my @complex_map = map { 
    $_->{key} + $_->{value} * $_->{factor} 
} @complex_data;

my @complex_grep = grep { 
    $_->{status} eq 'active' && $_->{value} > 100 
} @complex_data;

# Edge case: Complex sort operations
my @complex_sort = sort { 
    $a->{priority} <=> $b->{priority} || 
    $a->{name} cmp $b->{name} 
} @unsorted_data;

# Edge case: Complex splice operations
splice(@large_array, $offset, $length, @replacement_items);

# Edge case: Complex push/pop operations
push @$array_ref, {complex => 'structure', with => ['nested', 'elements']};
my $complex_pop = pop @$array_ref;

# Edge case: Complex shift/unshift operations
unshift @$array_ref, {complex => 'structure', at => 'front'};
my $complex_shift = shift @$array_ref;
"#;

    let mut parser = Parser::new(code);
    use perl_tdd_support::must;
    let ast = must(parser.parse());

    // Verify that edge case constructs are parsed
    assert!(has_node_kind(&ast, "Subroutine"), "Should have subroutine nodes");

    // Verify format declarations
    assert!(has_node_kind(&ast, "Format"), "Should have format nodes");

    // Verify heredoc nodes
    assert!(has_node_kind(&ast, "Heredoc"), "Should have heredoc nodes");

    // Verify regex operations
    assert!(has_node_kind(&ast, "Regex"), "Should have regex nodes");

    // Verify substitution operations
    assert!(has_node_kind(&ast, "Substitution"), "Should have substitution nodes");

    // Verify transliteration operations
    assert!(has_node_kind(&ast, "Transliteration"), "Should have transliteration nodes");

    // Verify package declarations
    assert!(has_node_kind(&ast, "Package"), "Should have package nodes");

    // Verify use/no statements
    assert!(has_node_kind(&ast, "Use"), "Should have use nodes");
    assert!(has_node_kind(&ast, "No"), "Should have no nodes");
}

//! Resource Exhaustion Tests for Perl Parser
//!
//! This test suite validates parser behavior under resource exhaustion conditions:
//! - Parser stack space exhaustion
//! - Circular references causing infinite loops
//! - Pathological regex patterns
//! - Massive data structures

use perl_parser::Parser;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Maximum recursion depth before stack overflow
const MAX_RECURSION_DEPTH: usize = 128;

/// Maximum heredoc depth before resource exhaustion
const MAX_HEREDOC_DEPTH: usize = 100;

/// Test parser with stack space exhaustion scenarios
#[test]
fn test_stack_space_exhaustion() {
    println!("Testing stack space exhaustion scenarios...");
    
    let stack_exhaustion_cases = vec![
        ("Deeply nested parentheses", generate_deep_parentheses(200)),
        ("Deeply nested brackets", generate_deep_brackets(200)),
        ("Deeply nested braces", generate_deep_braces(200)),
        ("Deeply nested subroutines", generate_deep_subroutines(200)),
        ("Deeply nested conditionals", generate_deep_conditionals(200)),
    ];
    
    for (name, code) in stack_exhaustion_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Should either parse successfully or fail gracefully with recursion limit error
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
                // Verify the AST is reasonable
                let depth = calculate_ast_depth(&ast);
                assert!(
                    depth <= MAX_RECURSION_DEPTH + 10,
                    "AST depth {} exceeds reasonable bounds",
                    depth
                );
            }
            Err(e) => {
                println!("  ✓ Failed gracefully with recursion limit: {:?}", e);
                // Should fail with recursion limit error, not crash
                assert!(e.to_string().contains("recursion") || 
                       e.to_string().contains("depth") ||
                       e.to_string().contains("nesting"),
                       "Should fail with recursion-related error, got: {}", e);
            }
        }
        
        // Should complete within reasonable time even on failure
        assert!(
            parse_time < Duration::from_secs(5),
            "Stack exhaustion test took too long: {:?}",
            parse_time
        );
    }
}

/// Test parser with circular reference scenarios
#[test]
fn test_circular_reference_scenarios() {
    println!("Testing circular reference scenarios...");
    
    let circular_cases = vec![
        ("Self-referencing package", generate_self_referencing_package()),
        ("Circular package references", generate_circular_package_references()),
        ("Recursive function calls", generate_recursive_function_calls()),
        ("Circular data structures", generate_circular_data_structures()),
        ("Infinite loop constructs", generate_infinite_loop_constructs()),
    ];
    
    for (name, code) in circular_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Should parse without infinite loops
        assert!(result.is_ok(), "Should parse {} without infinite loops", name);
        
        // Should complete within reasonable time
        assert!(
            parse_time < Duration::from_secs(3),
            "Circular reference test took too long: {:?}",
            parse_time
        );
        
        println!("  ✓ {} completed in {:?}", name, parse_time);
    }
}

/// Test parser with pathological regex patterns
#[test]
fn test_pathological_regex_patterns() {
    println!("Testing pathological regex patterns...");
    
    let regex_cases = vec![
        ("Catastrophic backtracking", generate_catastrophic_backtracking_regex()),
        ("Nested quantifiers", generate_nested_quantifiers_regex()),
        ("Excessive alternation", generate_excessive_alternation_regex()),
        ("Complex lookarounds", generate_complex_lookaround_regex()),
        ("Recursive patterns", generate_recursive_patterns()),
    ];
    
    for (name, code) in regex_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Should parse without regex engine DoS
        assert!(result.is_ok(), "Should parse {} without regex DoS", name);
        
        // Should complete within reasonable time
        assert!(
            parse_time < Duration::from_secs(2),
            "Pathological regex test took too long: {:?}",
            parse_time
        );
        
        println!("  ✓ {} completed in {:?}", name, parse_time);
    }
}

/// Test parser with massive data structures
#[test]
fn test_massive_data_structures() {
    println!("Testing massive data structures...");
    
    let structure_cases = vec![
        ("Huge array literal", generate_huge_array_literal(10000)),
        ("Massive hash literal", generate_massive_hash_literal(5000)),
        ("Deep nested structures", generate_deep_nested_structures(50)),
        ("Large string concatenation", generate_large_string_concatenation(1000)),
        ("Massive function calls", generate_massive_function_calls(1000)),
    ];
    
    for (name, code) in structure_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Should parse without memory exhaustion
        assert!(result.is_ok(), "Should parse {} without memory exhaustion", name);
        
        // Should complete within reasonable time
        assert!(
            parse_time < Duration::from_secs(5),
            "Massive structure test took too long: {:?}",
            parse_time
        );
        
        println!("  ✓ {} completed in {:?}", name, parse_time);
    }
}

/// Test parser with heredoc exhaustion scenarios
#[test]
fn test_heredoc_exhaustion_scenarios() {
    println!("Testing heredoc exhaustion scenarios...");
    
    let heredoc_cases = vec![
        ("Maximum heredoc depth", generate_maximum_heredoc_depth()),
        ("Nested heredocs", generate_nested_heredocs()),
        ("Large heredoc content", generate_large_heredoc_content()),
        ("Complex heredoc delimiters", generate_complex_heredoc_delimiters()),
        ("Heredoc with interpolation", generate_heredoc_with_interpolation()),
    ];
    
    for (name, code) in heredoc_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Should either parse successfully or fail gracefully
        match result {
            Ok(ast) => {
                println!("  ✓ Parsed successfully in {:?}", parse_time);
            }
            Err(e) => {
                println!("  ✓ Failed gracefully: {:?}", e);
                // Should fail with heredoc-related error, not crash
                assert!(e.to_string().contains("heredoc") ||
                       e.to_string().contains("depth") ||
                       e.to_string().contains("timeout"),
                       "Should fail with heredoc-related error, got: {}", e);
            }
        }
        
        // Should complete within reasonable time
        assert!(
            parse_time < Duration::from_secs(5),
            "Heredoc exhaustion test took too long: {:?}",
            parse_time
        );
    }
}

/// Test parser with memory exhaustion scenarios
#[test]
fn test_memory_exhaustion_scenarios() {
    println!("Testing memory exhaustion scenarios...");
    
    let memory_cases = vec![
        ("Massive variable declarations", generate_massive_variable_declarations(10000)),
        ("Huge symbol table", generate_huge_symbol_table(5000)),
        ("Excessive string literals", generate_excessive_string_literals(1000)),
        ("Large comment blocks", generate_large_comment_blocks(100)),
        ("Massive import statements", generate_massive_import_statements(1000)),
    ];
    
    for (name, code) in memory_cases {
        println!("Testing: {}", name);
        
        let start_time = Instant::now();
        let mut parser = Parser::new(&code);
        let result = parser.parse();
        let parse_time = start_time.elapsed();
        
        // Should parse without memory exhaustion
        assert!(result.is_ok(), "Should parse {} without memory exhaustion", name);
        
        // Should complete within reasonable time
        assert!(
            parse_time < Duration::from_secs(10),
            "Memory exhaustion test took too long: {:?}",
            parse_time
        );
        
        println!("  ✓ {} completed in {:?}", name, parse_time);
    }
}

/// Test parser with concurrent resource exhaustion
#[test]
fn test_concurrent_resource_exhaustion() {
    println!("Testing concurrent resource exhaustion...");
    
    let thread_count = 8;
    let iterations_per_thread = 10;
    
    let results = Arc::new(Mutex::new(Vec::new()));
    
    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let results_clone = Arc::clone(&results);
            
            thread::spawn(move || {
                for iteration in 0..iterations_per_thread {
                    // Generate different resource exhaustion scenarios
                    let scenarios = vec![
                        generate_deep_parentheses(100),
                        generate_massive_hash_literal(1000),
                        generate_catastrophic_backtracking_regex(),
                        generate_huge_array_literal(1000),
                    ];
                    
                    for (scenario_id, code) in scenarios.into_iter().enumerate() {
                        let start_time = Instant::now();
                        let mut parser = Parser::new(&code);
                        let result = parser.parse();
                        let parse_time = start_time.elapsed();
                        
                        results_clone.lock().unwrap().push((
                            thread_id,
                            iteration,
                            scenario_id,
                            result.is_ok(),
                            parse_time,
                        ));
                    }
                }
            })
        })
        .collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete successfully");
    }
    
    let results = results.lock().unwrap();
    
    // Verify all scenarios completed
    assert_eq!(
        results.len(),
        thread_count * iterations_per_thread * 4,
        "All concurrent scenarios should complete"
    );
    
    // Verify reasonable performance
    let mut total_time = Duration::new(0, 0);
    let mut success_count = 0;
    
    for (thread_id, iteration, scenario_id, success, parse_time) in results.iter() {
        total_time += *parse_time;
        if *success {
            success_count += 1;
        }
        
        // Individual scenarios should complete quickly
        assert!(
            *parse_time < Duration::from_secs(3),
            "Thread {} iteration {} scenario {} took too long: {:?}",
            thread_id, iteration, scenario_id, parse_time
        );
    }
    
    let avg_time = total_time / results.len() as u32;
    let success_rate = success_count as f64 / results.len() as f64;
    
    println!("  ✓ Concurrent exhaustion: {} scenarios, avg time: {:?}, success rate: {:.1}%",
             results.len(), avg_time, success_rate * 100.0);
    
    // Should have reasonable success rate
    assert!(
        success_rate > 0.5,
        "Success rate {:.1}% should be > 50%",
        success_rate * 100.0
    );
}

/// Test parser recovery from resource exhaustion
#[test]
fn test_resource_exhaustion_recovery() {
    println!("Testing resource exhaustion recovery...");
    
    // Test that parser can recover after hitting limits
    let exhaustion_code = generate_deep_parentheses(200); // Should hit recursion limit
    
    let mut parser = Parser::new(&exhaustion_code);
    let result1 = parser.parse();
    
    // First parse might fail due to recursion limit
    match result1 {
        Ok(_) => println!("  First parse succeeded"),
        Err(e) => println!("  First parse failed as expected: {:?}", e),
    }
    
    // Test that parser can handle normal code after exhaustion
    let normal_code = r#"
use strict;
use warnings;
my $x = 42;
print "Hello, world!\n";
"#;
    
    let mut parser2 = Parser::new(normal_code);
    let result2 = parser2.parse();
    
    assert!(result2.is_ok(), "Parser should recover and handle normal code");
    println!("  ✓ Parser recovered and handled normal code");
    
    // Test multiple cycles of exhaustion and recovery
    for cycle in 0..5 {
        let mut parser_exhaust = Parser::new(&exhaustion_code);
        let _ = parser_exhaust.parse(); // May fail
        
        let mut parser_normal = Parser::new(normal_code);
        let result = parser_normal.parse();
        
        assert!(result.is_ok(), "Parser should recover in cycle {}", cycle);
    }
    
    println!("  ✓ Parser recovered through {} exhaustion/recovery cycles", 5);
}

// Helper functions for generating test code

fn generate_deep_parentheses(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my $result = ");
    
    for _ in 0..depth {
        code.push('(');
    }
    
    code.push_str("42");
    
    for _ in 0..depth {
        code.push(')');
    }
    
    code.push_str(";\n");
    code
}

fn generate_deep_brackets(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my @result = ");
    
    for _ in 0..depth {
        code.push('[');
    }
    
    code.push_str("42");
    
    for _ in 0..depth {
        code.push(']');
    }
    
    code.push_str(";\n");
    code
}

fn generate_deep_braces(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my %result = ");
    
    for _ in 0..depth {
        code.push('{');
    }
    
    code.push_str("key => 42");
    
    for _ in 0..depth {
        code.push('}');
    }
    
    code.push_str(";\n");
    code
}

fn generate_deep_subroutines(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..depth {
        code.push_str(&format!(
            r#"sub sub_{i} {{
    return sub_{next}($param) if $param > 0;
    return 0;
}}
"#,
            i = i,
            next = i + 1
        ));
    }
    
    code.push_str(&format!("sub sub_{} {{ return 42; }}\n", depth));
    code.push_str("my $result = sub_0(10);\n");
    
    code
}

fn generate_deep_conditionals(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my $result = ");
    
    for i in 0..depth {
        code.push_str(&format!("$var{} ? ", i));
    }
    
    code.push_str("42");
    
    for i in 0..depth {
        code.push_str(&format!(" : $var{}", depth - i - 1));
    }
    
    code.push_str(";\n");
    code
}

fn generate_self_referencing_package() -> String {
    r#"
use strict;
use warnings;

package SelfRef;

sub new {
    my $class = shift;
    my $self = {
        value => 42,
        ref => undef,
    };
    $self->{ref} = $self;  # Self-reference
    return bless $self, $class;
}

package main;

my $obj = SelfRef->new();
print $obj->{value};
"#
    .to_string()
}

fn generate_circular_package_references() -> String {
    r#"
use strict;
use warnings;

package PackageA;

use PackageB;

sub method_a {
    return PackageB::method_b();
}

package PackageB;

use PackageA;

sub method_b {
    return PackageA::method_a();
}

package main;

my $result = PackageA::method_a();
"#
    .to_string()
}

fn generate_recursive_function_calls() -> String {
    r#"
use strict;
use warnings;

sub factorial {
    my ($n) = @_;
    return 1 if $n <= 1;
    return $n * factorial($n - 1);
}

sub fibonacci {
    my ($n) = @_;
    return 0 if $n == 0;
    return 1 if $n == 1;
    return fibonacci($n - 1) + fibonacci($n - 2);
}

my $fact = factorial(10);
my $fib = fibonacci(10);
"#
    .to_string()
}

fn generate_circular_data_structures() -> String {
    r#"
use strict;
use warnings;

my $ref1 = { value => 1 };
my $ref2 = { value => 2 };
my $ref3 = { value => 3 };

$ref1->{next} = $ref2;
$ref2->{next} = $ref3;
$ref3->{next} = $ref1;  # Circular reference

my @array = (\$ref1, \$ref2, \$ref3);
push @array, \@array;  # Self-reference
"#
    .to_string()
}

fn generate_infinite_loop_constructs() -> String {
    r#"
use strict;
use warnings;

while (1) {
    last if $condition;
}

for (;;) {
    last if $done;
}

until (0) {
    last if $finished;
}

do {
    # Something
} until (0);
"#
    .to_string()
}

fn generate_catastrophic_backtracking_regex() -> String {
    r#"
use strict;
use warnings;

# This pattern can cause catastrophic backtracking
my $pattern = qr/^(a+)+b$/;

# Test with string that causes backtracking
my $test = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaac";
if ($test =~ /$pattern/) {
    print "Match\n";
}

# Another problematic pattern
my $pattern2 = qr/^(a*)*$/;
my $test2 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
if ($test2 =~ /$pattern2/) {
    print "Match 2\n";
}
"#
    .to_string()
}

fn generate_nested_quantifiers_regex() -> String {
    r#"
use strict;
use warnings;

# Nested quantifiers can be problematic
my $pattern = qr/(a+*)+/;
my $pattern2 = qr/(a*)*+/;
my $pattern3 = qr/(a+?)*+/;

my $test = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
if ($test =~ /$pattern/) {
    print "Match 1\n";
}
"#
    .to_string()
}

fn generate_excessive_alternation_regex() -> String {
    r#"
use strict;
use warnings;

# Many alternatives can cause performance issues
my $pattern = qr/aaa|bbb|ccc|ddd|eee|fff|ggg|hhh|iii|jjj|kkk|lll|mmm|nnn|ooo|ppp|qqq|rrr|sss|ttt|uuu|vvv|www|xxx|yyy|zzz/;

my $test = "not_in_list";
if ($test =~ /$pattern/) {
    print "Match\n";
}
"#
    .to_string()
}

fn generate_complex_lookaround_regex() -> String {
    r#"
use strict;
use warnings;

# Complex lookarounds can be expensive
my $pattern = qr/^(?=.*a)(?=.*b)(?=.*c)(?=.*d)(?=.*e)(?=.*f)(?=.*g)(?=.*h)(?=.*i)(?=.*j)/;

my $test = "abcdefghijk";
if ($test =~ /$pattern/) {
    print "Match\n";
}
"#
    .to_string()
}

fn generate_recursive_patterns() -> String {
    r#"
use strict;
use warnings;

# Recursive patterns (if supported)
my $pattern = qr/(?<paren>\((?:[^()]|(?&paren))*\))/;

my $test = "(((nested)))";
if ($test =~ /$pattern/) {
    print "Match: $&\n";
}
"#
    .to_string()
}

fn generate_huge_array_literal(size: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my @huge_array = (\n");
    
    for i in 0..size {
        code.push_str(&format!("    {},", i));
        if i % 10 == 9 {
            code.push('\n');
        }
    }
    
    code.push_str(");\n");
    code
}

fn generate_massive_hash_literal(size: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my %massive_hash = (\n");
    
    for i in 0..size {
        code.push_str(&format!("    'key{}' => 'value{}',", i, i));
        if i % 10 == 9 {
            code.push('\n');
        }
    }
    
    code.push_str(");\n");
    code
}

fn generate_deep_nested_structures(depth: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my $deep = {\n");
    
    for i in 0..depth {
        code.push_str(&format!("    level{} => {{\n", i));
    }
    
    code.push_str("        value => 42\n");
    
    for i in (0..depth).rev() {
        code.push_str("    }");
        if i > 0 {
            code.push(',');
        }
        code.push('\n');
    }
    
    code.push_str("};\n");
    code
}

fn generate_large_string_concatenation(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my $large_string = ");
    
    for i in 0..count {
        code.push_str(&format!("'part{}'", i));
        if i < count - 1 {
            code.push_str(" . ");
        }
    }
    
    code.push_str(";\n");
    code
}

fn generate_massive_function_calls(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..count {
        code.push_str(&format!(
            r#"sub test_{} {{
    my ($arg) = @_;
    return $arg * 2;
}}
"#, i));
    }
    
    code.push_str("my $result = ");
    for i in 0..count {
        code.push_str(&format!("test_{}(", i));
        if i > 0 {
            code.push_str("test_{}(", i - 1);
        } else {
            code.push_str("42");
        }
        code.push_str(")");
        if i < count - 1 {
            code.push_str(" + ");
        }
    }
    code.push_str(";\n");
    
    code
}

fn generate_maximum_heredoc_depth() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    // Generate heredocs at maximum depth
    for i in 0..MAX_HEREDOC_DEPTH {
        code.push_str(&format!(
            r#"my $heredoc{} = <<END{};
Heredoc content {}
END
"#, i, i, i));
    }
    
    code
}

fn generate_nested_heredocs() -> String {
    r#"
use strict;
use warnings;

my $outer = <<OUTER;
Outer content
my $inner = <<INNER;
Inner content
INNER
More outer content
OUTER

my $nested = <<NEST1;
Level 1
my $nested2 = <<NEST2;
Level 2
my $nested3 = <<NEST3;
Level 3
NEST3
Level 2 continued
NEST2
Level 1 continued
NEST1
"#
    .to_string()
}

fn generate_large_heredoc_content() -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    code.push_str("my $large = <<LARGE;\n");
    
    for i in 0..10000 {
        code.push_str(&format!("Line {} with some content here\n", i));
    }
    
    code.push_str("LARGE\n");
    code
}

fn generate_complex_heredoc_delimiters() -> String {
    r#"
use strict;
use warnings;

my $complex1 = <<'COMPLEX_DELIMITER_WITH_UNDERSCORES_123';
Content with complex delimiter
COMPLEX_DELIMITER_WITH_UNDERSCORES_123

my $complex2 = <<"DELIMITER_$with$variables";
Content with $interpolation
DELIMITER_$with$variables

my $complex3 = <<'DELIMITER-WITH-DASHES';
Content with dashes
DELIMITER-WITH-DASHES
"#
    .to_string()
}

fn generate_heredoc_with_interpolation() -> String {
    r#"
use strict;
use warnings;

my $var1 = "hello";
my $var2 = "world";
my @array = (1, 2, 3);
my %hash = (key => "value");

my $interpolated = <<INTERPOLATED;
This is a heredoc with $var1 and $var2
Array elements: @array
Hash value: $hash{key}
Complex expression: @{[map { $_ * 2 } @array]}
INTERPOLATED

print $interpolated;
"#
    .to_string()
}

fn generate_massive_variable_declarations(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..count {
        code.push_str(&format!("my $var{} = {};\n", i, i));
        code.push_str(&format!("my @arr{} = ({});\n", i, (0..10).map(|j| j.to_string()).collect::<Vec<_>>().join(", ")));
        code.push_str(&format!("my %hash{} = ('key' => 'value{}');\n", i, i));
    }
    
    code
}

fn generate_huge_symbol_table(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    // Generate many subroutines
    for i in 0..count {
        code.push_str(&format!(
            r#"sub symbol_{} {{
    my ($param) = @_;
    return $param * {};
}}
"#, i, i));
    }
    
    // Generate many package declarations
    for i in 0..count {
        code.push_str(&format!(
            r#"package Symbol_{};

sub import {{
    my ($class) = @_;
    # Export something
}

package main;
"#, i));
    }
    
    code
}

fn generate_excessive_string_literals(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..count {
        code.push_str(&format!("my $str{} = 'This is string number {} with some content';\n", i, i));
        code.push_str(&format!("my $quoted{} = \"This is quoted string number {}\";\n", i, i));
        code.push_str(&format!("my $backtick{} = `echo backtick string {}`;\n", i, i));
    }
    
    code
}

fn generate_large_comment_blocks(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..count {
        code.push_str(&format!(
            r#"# This is a large comment block number {}
# It contains multiple lines of comments
# that should be parsed and stored
# in the AST without causing issues
# Comment line 1 for block {}
# Comment line 2 for block {}
# Comment line 3 for block {}
# Comment line 4 for block {}
# Comment line 5 for block {}

"#, i, i, i, i, i, i));
    }
    
    code.push_str("my $var = 42; # Code after comments\n");
    
    code
}

fn generate_massive_import_statements(count: usize) -> String {
    let mut code = String::new();
    code.push_str("use strict; use warnings;\n");
    
    for i in 0..count {
        code.push_str(&format!("use Module{};\n", i));
        code.push_str(&format!("use Module{} qw(function1 function2);\n", i));
        code.push_str(&format!("require Module{};\n", i));
        code.push_str(&format!("use base 'Module{}';\n", i));
        code.push_str(&format!("use parent 'Module{}';\n", i));
    }
    
    code
}

// Helper function for AST analysis
fn calculate_ast_depth(ast: &perl_parser::ast::Node) -> usize {
    use perl_parser::ast::NodeKind;
    
    match &ast.kind {
        NodeKind::SOURCE { children } => {
            1 + children.iter().map(calculate_ast_depth).max().unwrap_or(0)
        }
        NodeKind::STATEMENT { children } => {
            1 + children.iter().map(calculate_ast_depth).max().unwrap_or(0)
        }
        // Add more cases as needed based on actual NodeKind variants
        _ => 1,
    }
}
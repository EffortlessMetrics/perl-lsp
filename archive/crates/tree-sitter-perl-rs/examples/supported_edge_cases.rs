//! Test supported edge cases to verify parser capabilities
//!
//! This focuses on edge cases that the parser should handle correctly

use tree_sitter_perl::{EnhancedFullParser, pure_rust_parser::AstNode};

fn main() {
    println!("=== Testing Supported Edge Cases ===\n");

    let test_cases = [
        // Basic heredocs that should work
        (
            "Basic heredoc",
            r#"
my $text = <<'EOF';
Line 1
Line 2
EOF
"#,
        ),
        (
            "Quoted heredoc",
            r#"
my $text = <<"END";
Hello World
END
"#,
        ),
        // Variable edge cases
        (
            "Special variables",
            r#"
$_ = "default var";
@_ = (1, 2, 3);
$/ = "\n";
$\ = "\n";
"#,
        ),
        (
            "Package variables",
            r#"
our $VERSION = '1.0';
our @EXPORT = qw(func1 func2);
local $| = 1;
"#,
        ),
        // String edge cases
        (
            "Various string formats",
            r#"
my $single = 'single quotes';
my $double = "double quotes";
my $qq = qq(parentheses);
my $q = q{braces};
"#,
        ),
        (
            "String interpolation",
            r#"
my $name = "World";
my $greeting = "Hello, $name!";
my $complex = "Array: @{[$x, $y, $z]}";
"#,
        ),
        // Reference operations
        (
            "Basic references",
            r#"
my $scalar_ref = \$scalar;
my $array_ref = \@array;
my $hash_ref = \%hash;
my $code_ref = \&function;
"#,
        ),
        (
            "Dereferencing",
            r#"
my $value = ${$scalar_ref};
my @arr = @{$array_ref};
my %h = %{$hash_ref};
$code_ref->();
"#,
        ),
        // Anonymous structures
        (
            "Anonymous refs",
            r#"
my $aref = [1, 2, 3];
my $href = {a => 1, b => 2};
my $cref = sub { print "anon" };
"#,
        ),
        // Method calls
        (
            "Method calls",
            r#"
$obj->method();
$obj->method($arg);
$obj->method(@args);
Class->new();
"#,
        ),
        // List operations
        (
            "List contexts",
            r#"
my @sorted = sort @list;
my @mapped = map { $_ * 2 } @numbers;
my @filtered = grep { $_ > 0 } @values;
"#,
        ),
        // Hash operations
        (
            "Hash operations",
            r#"
my %hash = (
    key1 => 'value1',
    key2 => 'value2',
);
my @keys = keys %hash;
my @values = values %hash;
"#,
        ),
        // Control flow
        (
            "Basic control flow",
            r#"
if ($x) {
    print "true";
} elsif ($y) {
    print "elsif";
} else {
    print "false";
}
"#,
        ),
        (
            "Loops",
            r#"
for my $i (0..10) {
    print $i;
}

while ($condition) {
    last if $done;
    next if $skip;
}
"#,
        ),
        // Pattern matching
        (
            "Basic regex",
            r#"
if ($text =~ /pattern/) {
    print "match";
}
$text =~ s/old/new/g;
"#,
        ),
        // Subroutines
        (
            "Basic subroutines",
            r#"
sub simple {
    return 42;
}

sub with_params {
    my ($x, $y) = @_;
    return $x + $y;
}
"#,
        ),
        // Operators
        (
            "Various operators",
            r#"
my $sum = $a + $b;
my $concat = $x . $y;
my $logical = $p && $q || $r;
my $ternary = $test ? $true : $false;
"#,
        ),
        (
            "Comparison operators",
            r#"
if ($a == $b) { }
if ($x eq $y) { }
if ($m < $n) { }
if ($p gt $q) { }
"#,
        ),
        // Special constructs
        (
            "BEGIN and END blocks",
            r#"
BEGIN {
    print "compile time";
}

END {
    print "exit time";
}
"#,
        ),
        (
            "eval blocks",
            r#"
eval {
    risky_operation();
};
if ($@) {
    print "Error: $@";
}
"#,
        ),
    ];

    let mut passed = 0;
    let mut failed = 0;
    let mut details = Vec::new();

    for (name, code) in test_cases {
        print!("Testing {}: ", name);

        let mut parser = EnhancedFullParser::new();
        match parser.parse(code) {
            Ok(ast) => {
                if validate_ast(&ast) {
                    println!("✓ PASSED");
                    passed += 1;

                    // Collect AST info for summary
                    let node_count = count_nodes(&ast);
                    details.push((name, true, format!("{} nodes", node_count)));
                } else {
                    println!("✗ FAILED (empty AST)");
                    failed += 1;
                    details.push((name, false, "Empty AST".to_string()));
                }
            }
            Err(e) => {
                println!("✗ FAILED");
                failed += 1;
                let error_msg = format!("{}", e);
                let short_error = if error_msg.len() > 50 {
                    format!("{}...", &error_msg[..50])
                } else {
                    error_msg
                };
                details.push((name, false, short_error));
            }
        }
    }

    println!("\n=== Detailed Results ===");
    println!("{:<25} {:<8} Details", "Test Case", "Result");
    println!("{}", "-".repeat(70));

    for (name, passed, detail) in details {
        let result = if passed { "PASS" } else { "FAIL" };
        let symbol = if passed { "✓" } else { "✗" };
        println!("{:<25} {} {:<6} {}", name, symbol, result, detail);
    }

    println!("\n=== Summary ===");
    println!("Total tests: {}", passed + failed);
    println!("Passed: {} ({}%)", passed, (passed * 100) / (passed + failed));
    println!("Failed: {} ({}%)", failed, (failed * 100) / (passed + failed));

    if passed == passed + failed {
        println!("\n🎉 All supported edge cases passed!");
    } else if passed > (passed + failed) * 3 / 4 {
        println!("\n✓ Most edge cases passed. Parser is working well!");
    } else {
        println!("\n⚠ Several edge cases failed. Parser may need improvements.");
    }
}

fn validate_ast(ast: &AstNode) -> bool {
    match ast {
        AstNode::Program(items) => !items.is_empty(),
        _ => false,
    }
}

fn count_nodes(ast: &AstNode) -> usize {
    let mut count = 1;

    match ast {
        AstNode::Program(items) => {
            for item in items {
                count += count_nodes(item);
            }
        }
        AstNode::Statement(content) => {
            count += count_nodes(content);
        }
        AstNode::Block(statements) => {
            for stmt in statements {
                count += count_nodes(stmt);
            }
        }
        AstNode::IfStatement { condition, then_block, elsif_clauses, else_block } => {
            count += count_nodes(condition);
            count += count_nodes(then_block);
            for (cond, block) in elsif_clauses {
                count += count_nodes(cond);
                count += count_nodes(block);
            }
            if let Some(block) = else_block {
                count += count_nodes(block);
            }
        }
        AstNode::BinaryOp { left, right, .. } => {
            count += count_nodes(left);
            count += count_nodes(right);
        }
        AstNode::FunctionCall { function, args } => {
            count += count_nodes(function);
            for arg in args {
                count += count_nodes(arg);
            }
        }
        AstNode::List(items) => {
            for item in items {
                count += count_nodes(item);
            }
        }
        _ => {}
    }

    count
}

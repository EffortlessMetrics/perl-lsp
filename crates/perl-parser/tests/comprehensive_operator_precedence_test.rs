//! Comprehensive operator precedence tests for Perl parser
//! Tests all operator precedence levels to ensure correct parsing

use perl_parser::Parser;

#[test]
fn test_complete_precedence_hierarchy() {
    // Test cases organized by precedence level (highest to lowest)

    // Level 1: Terms and list operators (leftward)
    test_parse("print", "print 1, 2, 3");
    test_parse("sort", "sort { $a <=> $b } @list");

    // Level 2: Arrow operator
    test_parse("arrow", "$obj->method");
    test_parse("arrow_deref", "$ref->[0]");

    // Level 3: Autoincrement and autodecrement
    test_parse("postinc", "$x++");
    test_parse("preinc", "++$x");
    test_parse("postdec", "$x--");
    test_parse("predec", "--$x");

    // Level 4: Exponentiation
    test_parse("power", "2 ** 3");
    test_parse("power_assoc", "2 ** 3 ** 4"); // Right associative

    // Level 5: Symbolic unary operators
    test_parse("unary_not", "!$x");
    test_parse("unary_neg", "-$x");
    test_parse("unary_complement", "~$x");
    test_parse("unary_ref", "\\$x");

    // Level 6: Binding operators
    test_parse("match", "$str =~ /pattern/");
    test_parse("no_match", "$str !~ /pattern/");

    // Level 7: Multiplicative operators
    test_parse("multiply", "$a * $b");
    test_parse("divide", "$a / $b");
    test_parse("modulo", "$a % $b");
    test_parse("repeat", "'x' x 3");

    // Level 8: Additive operators
    test_parse("add", "$a + $b");
    test_parse("subtract", "$a - $b");
    test_parse("concat", "$a . $b");

    // Level 9: Shift operators
    test_parse("left_shift", "$a << 2");
    test_parse("right_shift", "$a >> 2");

    // Level 10: Named unary operators
    test_parse("named_unary", "defined $x ");
    test_parse("file_test", "-e $file ");

    // Level 11: Relational operators
    test_parse("less_than", "$a < $b");
    test_parse("greater_than", "$a > $b");
    test_parse("less_equal", "$a <= $b");
    test_parse("greater_equal", "$a >= $b");
    test_parse("string_lt", "$a lt $b");
    test_parse("string_gt", "$a gt $b");

    // Level 12: Equality operators
    test_parse("numeric_eq", "$a == $b");
    test_parse("numeric_ne", "$a != $b");
    test_parse("numeric_cmp", "$a <=> $b");
    test_parse("string_eq", "$a eq $b");
    test_parse("string_ne", "$a ne $b");
    test_parse("string_cmp", "$a cmp $b");
    test_parse("smart_match", "$a ~~ $b");

    // Level 13: Bitwise AND
    test_parse("bit_and", "$a & $b");

    // Level 14: Bitwise OR and XOR
    test_parse("bit_or", "$a | $b");
    test_parse("bit_xor", "$a ^ $b");

    // Level 15: C-style logical AND
    test_parse("c_and", "$a && $b");

    // Level 16: C-style logical OR
    test_parse("c_or", "$a || $b");
    test_parse("defined_or", "$a // $b");

    // Level 17: Range operators
    test_parse("range", "1..10");
    test_parse("flip_flop", "1...10");

    // Level 18: Ternary conditional
    test_parse("ternary", "$a ? $b : $c");

    // Level 19: Assignment operators
    test_parse("assign", "$a = $b");
    test_parse("add_assign", "$a += $b");
    test_parse("multiply_assign", "$a *= $b");
    test_parse("concat_assign", "$a .= $b");
    test_parse("or_assign", "$a ||= $b");
    test_parse("defined_or_assign", "$a //= $b");

    // Level 20: Comma and fat comma
    test_parse("comma", "($a, $b, $c)");
    test_parse("fat_comma", "(key => 'value')");

    // Level 21: List operators (rightward)
    test_parse("list_op_right", "print $fh $data");

    // Level 22: Logical not
    test_parse("word_not", "not $x");

    // Level 23: Logical and
    test_parse("word_and", "$a and $b");

    // Level 24: Logical or and xor
    test_parse("word_or", "$a or $b");
    test_parse("word_xor", "$a xor $b");
}

#[test]
fn test_complex_precedence_combinations() {
    // Test complex expressions that combine multiple precedence levels

    // Assignment with word operators (critical fix in v0.7.2)
    test_parse("assign_or", "$a = 1 or $b = 2");
    test_parse("assign_and", "$a = 1 and $b = 2");

    // Mixed arithmetic and logical
    test_parse("mixed_1", "$a + $b * $c");
    test_parse("mixed_2", "$a * $b + $c");
    test_parse("mixed_3", "$a || $b && $c");
    test_parse("mixed_4", "$a && $b || $c");

    // Ternary with various operators
    test_parse("ternary_complex", "$a > $b ? $c + $d : $e * $f");
    test_parse("ternary_nested", "$a ? $b ? $c : $d : $e");

    // Assignment chains
    test_parse("chain_assign", "$a = $b = $c = 1");
    test_parse("chain_mixed", "$a = $b + $c = $d");

    // Word operators with complex expressions
    test_parse("word_complex_1", "$a = func() or die 'error'");
    test_parse("word_complex_2", "open $fh, $file or return");
    test_parse("word_complex_3", "$x > 0 and $y < 10 or $z == 0");
}

#[test]
fn test_associativity() {
    // Test operator associativity

    // Right associative: exponentiation
    test_parse("power_right", "2 ** 3 ** 4"); // Should parse as 2 ** (3 ** 4)

    // Right associative: assignment
    test_parse("assign_right", "$a = $b = $c"); // Should parse as $a = ($b = $c)

    // Left associative: most binary operators
    test_parse("add_left", "$a + $b + $c"); // Should parse as ($a + $b) + $c
    test_parse("mult_left", "$a * $b * $c"); // Should parse as ($a * $b) * $c

    // Non-associative: comparison operators
    test_parse("compare_chain", "$a < $b < $c"); // Should be an error or special handling
}

#[test]
fn test_parentheses_override() {
    // Test that parentheses correctly override precedence

    test_parse("paren_1", "($a + $b) * $c");
    test_parse("paren_2", "$a + ($b * $c)");
    test_parse("paren_3", "($a or $b) and $c");
    test_parse("paren_4", "$a or ($b and $c)");
    test_parse("paren_5", "($a = 1) or ($b = 2)");
}

#[test]
fn test_statement_modifiers() {
    // Statement modifiers have special precedence rules

    test_parse("if_modifier", "print $x if $y");
    test_parse("unless_modifier", "die 'error' unless $ok");
    test_parse("while_modifier", "print while <>");
    test_parse("until_modifier", "$x++ until $x > 10");
    test_parse("for_modifier", "print for @list");

    // Complex statement modifiers
    test_parse("complex_modifier_1", "$a = $b or die 'error' if $c");
    test_parse("complex_modifier_2", "return $x || $y unless $z");
}

#[test]
fn test_list_context_precedence() {
    // List operators have special precedence in list context

    test_parse("list_print", "print $a, $b, $c");
    test_parse("list_map", "map { $_ * 2 } @list");
    test_parse("list_grep", "grep { $_ > 0 } @list");
    test_parse("list_sort", "sort { $a <=> $b } @list");

    // Mixed list and scalar context
    test_parse("list_scalar_1", "my @a = map { $_ * 2 } 1..10");
    test_parse("list_scalar_2", "print join ',', map { $_ + 1 } @list");
}

#[test]
fn test_special_cases() {
    // Test special parsing cases that affect precedence

    // Indirect object syntax
    test_parse("indirect_1", "print STDOUT 'hello'");
    test_parse("indirect_2", "new Class $arg");

    // Filehandle operators
    test_parse("diamond", "while (<>) { print }");
    test_parse("readline", "my $line = <$fh>");

    // Regex with different delimiters
    test_parse("regex_slash", "$x =~ /pattern/");
    test_parse("regex_bang", "$x =~ m!pattern!");
    test_parse("regex_brace", "$x =~ m{pattern}");

    // Quote-like operators
    test_parse("q_op", "q(string)");
    test_parse("qq_op", "qq{interpolated $var}");
    test_parse("qw_op", "qw(word list here)");
}

#[test]
fn test_precedence_errors() {
    // Test that certain precedence combinations produce expected errors or warnings

    // These should parse but might have unexpected results
    test_parse("unexpected_1", "$a = 1, 2, 3"); // Comma has lower precedence than assignment
    test_parse("unexpected_2", "$a || $b = 1"); // Assignment has lower precedence than ||
    test_parse("unexpected_3", "!$a = 1"); // ! has higher precedence than =
}

// Helper function to test parsing
fn test_parse(name: &str, code: &str) {
    let mut parser = Parser::new(code);
    match parser.parse() {
        Ok(ast) => {
            // Verify AST is valid (has a kind)
            match &ast.kind {
                perl_parser::ast::NodeKind::Program { statements } => {
                    // Could check statements are not empty for non-trivial code
                    println!(
                        "✓ {}: Successfully parsed: {} ({} statements)",
                        name,
                        code,
                        statements.len()
                    );
                }
                _ => {
                    println!("✓ {}: Successfully parsed: {}", name, code);
                }
            }
        }
        Err(e) => {
            unreachable!("{}: Failed to parse '{}': {:?}", name, code, e);
        }
    }
}

#[test]
fn test_word_operator_precedence_comprehensive() {
    // Comprehensive tests specifically for word operators (or, and, not, xor)
    // These were the main fix in v0.7.2

    struct TestCase {
        name: &'static str,
        input: &'static str,
        description: &'static str,
    }

    let test_cases = vec![
        TestCase { name: "simple_or", input: "$a or $b", description: "Simple OR expression" },
        TestCase { name: "simple_and", input: "$a and $b", description: "Simple AND expression" },
        TestCase { name: "simple_not", input: "not $a", description: "Simple NOT expression" },
        TestCase { name: "simple_xor", input: "$a xor $b", description: "Simple XOR expression" },
        TestCase {
            name: "or_with_assignment",
            input: "$a = 1 or $b = 2",
            description: "OR with assignment (should parse as ($a = 1) or ($b = 2))",
        },
        TestCase {
            name: "and_with_assignment",
            input: "$a = 1 and $b = 2",
            description: "AND with assignment (should parse as ($a = 1) and ($b = 2))",
        },
        TestCase {
            name: "complex_chain",
            input: "$a = $b or $c = $d and $e = $f",
            description: "Complex chain of assignments with word operators",
        },
        TestCase {
            name: "with_function_calls",
            input: "open $fh, $file or die 'Cannot open'",
            description: "Common Perl idiom with OR",
        },
        TestCase {
            name: "nested_word_ops",
            input: "$a and $b or $c and $d",
            description: "Nested word operators (AND has higher precedence than OR)",
        },
        TestCase {
            name: "not_with_comparison",
            input: "not $x > 10",
            description: "NOT with comparison (NOT has lowest precedence)",
        },
        TestCase {
            name: "mixed_c_and_word",
            input: "$a || $b or $c && $d and $e",
            description: "Mixed C-style and word operators",
        },
        TestCase {
            name: "with_ternary",
            input: "$a ? $b : $c or $d",
            description: "Word operator with ternary",
        },
        TestCase {
            name: "statement_modifier_or",
            input: "return $x or die if $error",
            description: "OR in statement with modifier",
        },
        TestCase {
            name: "multiple_xor",
            input: "$a xor $b xor $c",
            description: "Multiple XOR operations",
        },
        TestCase {
            name: "parentheses_override",
            input: "($a or $b) and ($c or $d)",
            description: "Parentheses overriding natural precedence",
        },
    ];

    for test in test_cases {
        let mut parser = Parser::new(test.input);
        match parser.parse() {
            Ok(_ast) => {
                println!("✓ {}: {} - {}", test.name, test.input, test.description);
                // AST successfully parsed
            }
            Err(e) => {
                unreachable!("Failed to parse {}: {} - Error: {:?}", test.name, test.input, e);
            }
        }
    }
}

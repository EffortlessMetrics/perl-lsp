mod cpan_test_helpers;
use cpan_test_helpers::*;

// --- Regression tests for expected_variable bucket growth after PR #1860 ---
// The `field` keyword (Perl 5.38+) was unconditionally treated as a variable
// declarator, causing parse failures when `field` is used as a bareword
// identifier in pre-5.38 code (hash keys, function calls, method calls, etc.).

#[test]
fn test_field_keyword_valid_class_declaration() -> Result<(), Box<dyn std::error::Error>> {
    // `field` inside a class body IS a valid field declaration
    let source = r#"
        use v5.38;
        class Point {
            field $x;
            field $y :param;
            field $z = 0;
        }
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_as_hash_key_bareword() -> Result<(), Box<dyn std::error::Error>> {
    // `field` used as a bareword hash key (very common in CPAN modules)
    let source = r#"
        my %config = (field => 'name', type => 'text');
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_as_hash_access() -> Result<(), Box<dyn std::error::Error>> {
    // `field` as a hash key in brace access
    let source = r#"
        my $val = $hash{field};
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_as_function_call() -> Result<(), Box<dyn std::error::Error>> {
    // `field` used as a function name
    let source = r#"
        field('name', type => 'text');
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_as_method_call() -> Result<(), Box<dyn std::error::Error>> {
    // `field` used as a method name (common in ORMs/form builders)
    let source = r#"
        $form->field('username');
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_followed_by_arrow() -> Result<(), Box<dyn std::error::Error>> {
    // Calling field as a function then dereferencing
    let source = r#"
        my $val = field()->name;
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_as_subroutine_name() -> Result<(), Box<dyn std::error::Error>> {
    // Defining a sub called `field`
    let source = r#"
        sub field {
            return $_[0]->{field};
        }
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_in_print() -> Result<(), Box<dyn std::error::Error>> {
    // Using `field` as a function arg to print
    let source = r#"
        print "Field: " . field() . "\n";
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_with_variable_is_declaration() -> Result<(), Box<dyn std::error::Error>> {
    // When `field` IS followed by a sigil, it's a field declaration
    let source = r#"
        field $name;
        field @items;
        field %lookup;
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_spaced_variable_args_are_function_call() -> Result<(), Box<dyn std::error::Error>> {
    // A spaced call with variable args must stay a function call, not a field declaration.
    let source = r#"
        field ($x, $y);
    "#;
    let ast = parse(source);
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "Expected clean parse for spaced field call, got: {}", sexp);
    assert_eq!(top_level_kinds(&ast), vec!["ExpressionStatement"]);
    Ok(())
}

#[test]
fn test_field_not_followed_by_sigil_semicolon() -> Result<(), Box<dyn std::error::Error>> {
    // `field;` alone - like a bareword or function call
    let source = r#"
        field;
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_string_concat() -> Result<(), Box<dyn std::error::Error>> {
    // `field` in a string concatenation context
    let source = r#"
        my $sql = "SELECT " . field . " FROM table";
    "#;
    assert_clean_parse(source);
    Ok(())
}

#[test]
fn test_field_in_conditional() -> Result<(), Box<dyn std::error::Error>> {
    // `field` used as a function in a conditional
    let source = r#"
        if (field()) {
            print "has field\n";
        }
    "#;
    assert_clean_parse(source);
    Ok(())
}

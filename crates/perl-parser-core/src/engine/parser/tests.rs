#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn test_simple_variable() {
    let mut parser = Parser::new("my $x = 42;");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    println!("AST: {}", ast.to_sexp());
}

#[test]
fn test_if_statement() {
    let mut parser = Parser::new("if ($x > 10) { print $x; }");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    println!("AST: {}", ast.to_sexp());
}

#[test]
fn test_function_definition() {
    let mut parser = Parser::new("sub greet { print \"Hello\"; }");
    let result = parser.parse();
    assert!(result.is_ok());

    let ast = result.unwrap();
    println!("AST: {}", ast.to_sexp());
}

#[test]
fn test_list_declarations() {
    // Test simple list declaration
    let mut parser = Parser::new("my ($x, $y);");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    println!("List declaration AST: {}", ast.to_sexp());

    // Test list declaration with initialization
    let mut parser = Parser::new("state ($a, $b) = (1, 2);");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    println!("List declaration with init AST: {}", ast.to_sexp());

    // Test mixed sigils
    let mut parser = Parser::new("our ($scalar, @array, %hash);");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    println!("Mixed sigils AST: {}", ast.to_sexp());

    // Test empty list
    let mut parser = Parser::new("my ();");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    println!("Empty list AST: {}", ast.to_sexp());
}

#[test]
fn test_qw_delimiters() {
    // Test qw with parentheses
    let mut parser = Parser::new("qw(one two three)");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    assert_eq!(
        ast.to_sexp(),
        r#"(source_file (array (string "one") (string "two") (string "three")))"#
    );

    // Test qw with brackets
    let mut parser = Parser::new("qw[foo bar]");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    assert_eq!(ast.to_sexp(), r#"(source_file (array (string "foo") (string "bar")))"#);

    // Test qw with non-paired delimiters
    let mut parser = Parser::new("qw/alpha beta/");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    assert_eq!(ast.to_sexp(), r#"(source_file (array (string "alpha") (string "beta")))"#);

    // Test qw with exclamation marks
    let mut parser = Parser::new("qw!hello world!");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    assert_eq!(ast.to_sexp(), r#"(source_file (array (string "hello") (string "world")))"#);
}

#[test]
fn test_block_vs_hash_context() {
    // Statement context: block containing hash
    let mut parser = Parser::new("{ key => 'value' }");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    // Statement context: block with hash inside
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(block (expression_statement (hash"),
        "Statement context should have block containing hash, got: {}",
        sexp
    );

    // Expression context: direct hash literal in assignment
    let mut parser = Parser::new("my $x = { key => 'value' }");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    // In expression context, should have hash
    let sexp = ast.to_sexp();
    assert!(sexp.contains("(hash"), "Expression context should have hash, got: {}", sexp);
    assert!(sexp.contains("my"), "Should have my declaration, got: {}", sexp);

    // Hash reference with parentheses
    let mut parser = Parser::new("$ref = ( a => 1, b => 2 )");
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    // Parentheses with fat arrow should create hash
    let sexp = ast.to_sexp();
    assert!(
        sexp.contains("(hash") || sexp.contains("(array"),
        "Should have hash or array, got: {}",
        sexp
    );
}

#[test]
fn test_qualified_function_call() {
    let mut parser = Parser::new("return Data::Dumper::Dumper($param);");
    let result = parser.parse();
    match result {
        Ok(ast) => {
            println!("✅ Successfully parsed qualified function call: {}", ast.to_sexp());
        }
        Err(e) => {
            println!("❌ Failed to parse qualified function call: {}", e);
            panic!("Parsing failed: {}", e);
        }
    }
}

#[test]
fn test_issue_461_variable_length_lookbehind() {
    // Variable-length lookbehind
    let code = r#"my $pattern = qr/(?<=\d{1,1000})\w+/;"#;
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok(), "Failed to parse variable-length lookbehind");
    
    // Deeply nested lookbehind
    let code_nested = r#"my $nested = qr/(?<=(?<=(?<=\d)\w+)\s+)\w+/;"#;
    let mut parser_nested = Parser::new(code_nested);
    let result_nested = parser_nested.parse();
    assert!(result_nested.is_ok(), "Failed to parse nested lookbehind");
    
    // Check if the AST contains the regex pattern
    let ast = result_nested.unwrap();
    println!("Nested Lookbehind AST: {}", ast.to_sexp());
}

#[test]
fn test_regex_complexity_failure() {
    // 11 levels of nesting (max is 10)
    let code = r#"qr/(?<=(?<=(?<=(?<=(?<=(?<=(?<=(?<=(?<=(?<=(?<=\d)))))))))))\w+/"#;
    let mut parser = Parser::new(code);
    let result = parser.parse();
    
    // Parser might recover, so check either result is Err or errors list has the error
    if let Err(e) = result {
        assert!(e.to_string().contains("Regex lookbehind nesting too deep"), "Error was: {}", e);
    } else {
        let errors = parser.errors();
        assert!(!errors.is_empty(), "Should have recorded errors for excessive nesting");
        let found = errors.iter().any(|e| e.to_string().contains("Regex lookbehind nesting too deep"));
        assert!(found, "Should have found specific error in: {:?}", errors);
    }
}

#[test]
fn test_unicode_property_valid() {
    // 50 properties (limit is 50, so this should pass)
    let mut pattern = String::from("qr/");
    for i in 0..50 {
        pattern.push_str(&format!("\\p{{Prop{}}}", i));
    }
    pattern.push('/');
    
    let mut parser = Parser::new(&pattern);
    let result = parser.parse();
    assert!(result.is_ok(), "Should accept 50 Unicode properties");
}

#[test]
fn test_unicode_property_complexity() {
    // 51 properties (max is 50)
    let mut pattern = String::from("qr/");
    for i in 0..51 {
        pattern.push_str(&format!("\\p{{Prop{}}}", i));
    }
    pattern.push('/');
    
    let mut parser = Parser::new(&pattern);
    let result = parser.parse();
    
    // Parser might recover, so check either result is Err or errors list has the error
    if let Err(e) = result {
        assert!(e.to_string().contains("Too many Unicode properties"), "Error was: {}", e);
    } else {
        let errors = parser.errors();
        assert!(!errors.is_empty(), "Should have recorded errors for excessive properties");
        let found = errors.iter().any(|e| e.to_string().contains("Too many Unicode properties"));
        assert!(found, "Should have found specific error in: {:?}", errors);
    }
}

#[test]
fn test_source_filter_detection() {
    // Known filter
    let code = "use Filter::Util::Call;";
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("(risk:filter)"), "Should detect filter usage in: {}", sexp);

    // Safe module
    let code_safe = "use strict;";
    let mut parser_safe = Parser::new(code_safe);
    let result_safe = parser_safe.parse();
    assert!(result_safe.is_ok());
    let ast_safe = result_safe.unwrap();
    let sexp_safe = ast_safe.to_sexp();
    assert!(!sexp_safe.contains("(risk:filter)"), "Should not flag strict as filter in: {}", sexp_safe);
}

#[test]
fn test_regex_code_execution_detection() {
    // Regex with code execution
    let code = r#"my $re = qr/(?{ print "hi" })/;"#;
    let mut parser = Parser::new(code);
    let result = parser.parse();
    assert!(result.is_ok());
    let ast = result.unwrap();
    let sexp = ast.to_sexp();
    assert!(sexp.contains("(risk:code)"), "Should detect regex code execution in: {}", sexp);

    // Safe regex
    let code_safe = r#"my $re = qr/hello/;"#;
    let mut parser_safe = Parser::new(code_safe);
    let result_safe = parser_safe.parse();
    assert!(result_safe.is_ok());
    let ast_safe = result_safe.unwrap();
    let sexp_safe = ast_safe.to_sexp();
    assert!(!sexp_safe.contains("(risk:code)"), "Should not flag safe regex in: {}", sexp_safe);
}

#[test]
fn test_branch_reset_complexity() {
    // 51 branches (max is 50)
    let mut pattern = String::from("qr/(?|");
    for i in 0..51 {
        pattern.push_str(&format!("(a{})|", i));
    }
    // Remove last pipe
    pattern.pop(); 
    pattern.push_str(")/");
    
    let mut parser = Parser::new(&pattern);
    let result = parser.parse();
    
    // Parser might recover, so check either result is Err or errors list has the error
    if let Err(e) = result {
        assert!(e.to_string().contains("Too many branches"), "Error was: {}", e);
    } else {
        let errors = parser.errors();
        assert!(!errors.is_empty(), "Should have recorded errors for excessive branches");
        let found = errors.iter().any(|e| e.to_string().contains("Too many branches"));
        assert!(found, "Should have found specific error in: {:?}", errors);
    }
}

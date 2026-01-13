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

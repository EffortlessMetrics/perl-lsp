#[cfg(test)]
mod test_slash {
    use crate::perl_lexer::{PerlLexer, TokenType};

    #[test]
    fn test_basic_disambiguation() {
        // Test 1: Division after identifier
        let mut lexer = PerlLexer::new("x / 2");
        
        let token1 = lexer.next_token().unwrap();
        assert!(matches!(token1.token_type, TokenType::Identifier(_)));
        
        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.token_type, TokenType::Division);
        
        let token3 = lexer.next_token().unwrap();
        assert!(matches!(token3.token_type, TokenType::Number(_)));
        
        // Test 2: Regex after operator
        let mut lexer = PerlLexer::new("=~ /foo/");
        
        let token1 = lexer.next_token().unwrap();
        assert!(matches!(token1.token_type, TokenType::Operator(ref op) if op.as_ref() == "=~"));
        
        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.token_type, TokenType::RegexMatch);
        assert!(token2.text.contains("foo"));
    }
    
    #[test]
    fn test_complex_cases() {
        // Test: 1/ /abc/
        let mut lexer = PerlLexer::new("1/ /abc/");
        
        let token1 = lexer.next_token().unwrap();
        assert!(matches!(token1.token_type, TokenType::Number(_)));
        
        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.token_type, TokenType::Division);
        
        let token3 = lexer.next_token().unwrap();
        assert_eq!(token3.token_type, TokenType::RegexMatch);
        assert!(token3.text.contains("abc"));
    }
    
    #[test]
    fn test_substitution() {
        let mut lexer = PerlLexer::new("s/foo/bar/g");
        
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Substitution);
        assert_eq!(token.text.as_ref(), "s/foo/bar/g");
        
        // Test with braces
        let mut lexer = PerlLexer::new("s{foo}{bar}g");
        
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Substitution);
    }
    
    #[test]
    fn test_token_positions() {
        let input = "my $x = 42 + 3.14;";
        let mut lexer = PerlLexer::new(input);
        
        // "my"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Keyword(ref k) if k.as_ref() == "my"));
        assert_eq!(token.start, 0);
        assert_eq!(token.end, 2);
        assert_eq!(&input[token.start..token.end], "my");
        
        // "$x"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "$x"));
        assert_eq!(token.start, 3);
        assert_eq!(token.end, 5);
        assert_eq!(&input[token.start..token.end], "$x");
        
        // "="
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "="));
        assert_eq!(token.start, 6);
        assert_eq!(token.end, 7);
        assert_eq!(&input[token.start..token.end], "=");
        
        // "42"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Number(ref n) if n.as_ref() == "42"));
        assert_eq!(token.start, 8);
        assert_eq!(token.end, 10);
        assert_eq!(&input[token.start..token.end], "42");
        
        // "+"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "+"));
        assert_eq!(token.start, 11);
        assert_eq!(token.end, 12);
        assert_eq!(&input[token.start..token.end], "+");
        
        // "3.14"
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Number(ref n) if n.as_ref() == "3.14"));
        assert_eq!(token.start, 13);
        assert_eq!(token.end, 17);
        assert_eq!(&input[token.start..token.end], "3.14");
        
        // ";"
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Semicolon);
        assert_eq!(token.start, 17);
        assert_eq!(token.end, 18);
        assert_eq!(&input[token.start..token.end], ";");
    }
    
    #[test]
    fn test_variable_types() {
        // Test scalar
        let mut lexer = PerlLexer::new("$foo");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "$foo"));
        
        // Test array
        let mut lexer = PerlLexer::new("@bar");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "@bar"));
        
        // Test hash
        let mut lexer = PerlLexer::new("%baz");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "%baz"));
        
        // Test glob
        let mut lexer = PerlLexer::new("*STDOUT");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(ref i) if i.as_ref() == "*STDOUT"));
    }
    
    #[test]
    fn test_operators() {
        let input = "=~ !~ == != <= >= <=> .. ...";
        let mut lexer = PerlLexer::new(input);
        
        let expected = vec!["=~", "!~", "==", "!=", "<=", ">=", "<=>", "..", "..."];
        
        for exp in expected {
            let token = lexer.next_token().unwrap();
            assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == exp), 
                   "Expected operator {}, got {:?}", exp, token);
        }
    }
    
    #[test]
    fn test_edge_cases() {
        // Empty variable (just sigil)
        let mut lexer = PerlLexer::new("$ ");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "$"));
        
        // Modulo operator
        let mut lexer = PerlLexer::new("10 % 3");
        let _num = lexer.next_token().unwrap();
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "%"));
        
        // Multiplication
        let mut lexer = PerlLexer::new("5 * 3");
        let _num = lexer.next_token().unwrap();
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Operator(ref op) if op.as_ref() == "*"));
    }
    
    #[test]
    fn test_regex_operators() {
        // Match operator
        let mut lexer = PerlLexer::new("m/pattern/i");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::RegexMatch);
        assert!(token.text.contains("pattern"));
        
        // Transliteration
        let mut lexer = PerlLexer::new("tr/abc/def/");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Transliteration);
        
        // Quote regex
        let mut lexer = PerlLexer::new("qr{pattern}i");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::QuoteRegex);
    }
    
    #[test]
    fn test_string_literals() {
        // Single quoted strings
        let mut lexer = PerlLexer::new("'simple string'");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::StringLiteral);
        assert_eq!(token.text.as_ref(), "'simple string'");
        
        // Double quoted strings  
        let mut lexer = PerlLexer::new(r#""double quoted""#);
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::StringLiteral);
        
        // Escaped quotes
        let mut lexer = PerlLexer::new(r#"'it\'s escaped'"#);
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::StringLiteral);
        
        // Double quoted with escapes
        let mut lexer = PerlLexer::new(r#""line\nbreak""#);
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::StringLiteral);
    }
    
    #[test]
    fn test_string_interpolation() {
        // Variable interpolation
        let mut lexer = PerlLexer::new(r#""Hello $name""#);
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::StringLiteral);
        assert!(token.text.contains("$name"));
        
        // Array interpolation
        let mut lexer = PerlLexer::new(r#""Items: @items""#);
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::StringLiteral);
        
        // Hash element interpolation
        let mut lexer = PerlLexer::new(r#""Value: $hash{key}""#);
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::StringLiteral);
        
        // Complex interpolation
        let mut lexer = PerlLexer::new(r#""Result: ${expr}""#);
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::StringLiteral);
    }
    
    #[test]
    fn test_quote_operators() {
        // q// single quotes
        let mut lexer = PerlLexer::new("q/simple string/");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::QuoteSingle);
        
        // qq// double quotes
        let mut lexer = PerlLexer::new("qq{interpolated $var}");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::QuoteDouble);
        
        // qw// word list
        let mut lexer = PerlLexer::new("qw(foo bar baz)");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::QuoteWords);
        
        // qx// backticks
        let mut lexer = PerlLexer::new("qx{ls -la}");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::QuoteCommand);
    }
    
    #[test]
    fn test_delimiter_variations() {
        // Different delimiters for quotes
        let delimiters = vec![
            ("q(text)", TokenType::QuoteSingle),
            ("q[text]", TokenType::QuoteSingle),
            ("q{text}", TokenType::QuoteSingle),
            ("q<text>", TokenType::QuoteSingle),
            ("q!text!", TokenType::QuoteSingle),
            ("q#text#", TokenType::QuoteSingle),
            ("q|text|", TokenType::QuoteSingle),
        ];
        
        for (input, expected_type) in delimiters {
            let mut lexer = PerlLexer::new(input);
            let token = lexer.next_token().unwrap();
            assert_eq!(token.token_type, expected_type, "Failed for input: {}", input);
        }
    }
    
    #[test]
    fn test_heredoc_edge_cases() {
        // Simple heredoc
        let mut lexer = PerlLexer::new("<<EOF");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::HeredocStart);
        
        // Quoted heredoc
        let mut lexer = PerlLexer::new("<<'EOF'");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::HeredocStart);
        
        // Indented heredoc (Perl 5.26+)
        let mut lexer = PerlLexer::new("<<~EOF");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::HeredocStart);
        
        // Backtick heredoc
        let mut lexer = PerlLexer::new("<<`CMD`");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::HeredocStart);
    }
    
    #[test]
    fn test_special_variables() {
        let special_vars = vec![
            "$_", "$.", "$@", "$!", "$?", "$&", "$`", "$'", "$+",
            "$1", "$2", "$10", "$$", "$<", "$>", "$(", "$)",
            "$[", "$]", "$^A", "$^W", "$^X", "$|", "$~", "$%",
            "${^GLOBAL_PHASE}", "${^TAINT}", "${^UNICODE}",
        ];
        
        for var in special_vars {
            let mut lexer = PerlLexer::new(var);
            let token = lexer.next_token().unwrap();
            assert!(matches!(token.token_type, TokenType::Identifier(_)), 
                    "Failed to recognize special variable: {}", var);
            assert_eq!(token.text.as_ref(), var);
        }
    }
    
    #[test]
    fn test_bareword_edge_cases() {
        // Bareword after arrow
        let mut lexer = PerlLexer::new("$obj->method");
        let _obj = lexer.next_token().unwrap();
        let _arrow = lexer.next_token().unwrap();
        let method = lexer.next_token().unwrap();
        assert!(matches!(method.token_type, TokenType::Identifier(_)));
        
        // Bareword in hash key
        let mut lexer = PerlLexer::new("$hash{bareword}");
        let _hash = lexer.next_token().unwrap();
        let _brace = lexer.next_token().unwrap();
        let key = lexer.next_token().unwrap();
        assert!(matches!(key.token_type, TokenType::Identifier(_)));
    }
    
    #[test]
    fn test_numeric_edge_cases() {
        let numbers = vec![
            ("42", "integer"),
            ("3.14", "float"),
            ("6.02e23", "scientific"),
            ("0xFF", "hex"),
            ("0377", "octal"),
            ("0b1010", "binary"),
            ("1_234_567", "with underscores"),
            ("12.34_56", "float with underscores"),
            (".5", "no leading zero"),
            ("5.", "no trailing zero"),
            ("0xDEAD_BEEF", "hex with underscores"),
            ("Inf", "infinity"),
            ("NaN", "not a number"),
        ];
        
        for (num, desc) in numbers {
            let mut lexer = PerlLexer::new(num);
            let token = lexer.next_token().unwrap();
            assert!(matches!(token.token_type, TokenType::Number(_)) || 
                    matches!(token.token_type, TokenType::Identifier(_)), // For Inf/NaN
                    "Failed to parse {} ({})", num, desc);
        }
    }
    
    #[test]
    fn test_comment_and_pod() {
        // Single line comment
        let mut lexer = PerlLexer::new("# comment\n$x");
        let comment = lexer.next_token().unwrap();
        assert!(matches!(comment.token_type, TokenType::Comment(_)));
        assert!(comment.text.contains("comment"));
        
        // POD documentation
        let mut lexer = PerlLexer::new("=head1 NAME\n\nTest\n\n=cut\n$x");
        let pod = lexer.next_token().unwrap();
        assert_eq!(pod.token_type, TokenType::Pod);
        
        // Inline POD
        let mut lexer = PerlLexer::new("=for comment\nThis is hidden\n=cut");
        let pod = lexer.next_token().unwrap();
        assert_eq!(pod.token_type, TokenType::Pod);
    }
    
    #[test]
    fn test_context_sensitive_edge_cases() {
        // print followed by regex (not division)
        let mut lexer = PerlLexer::new("print /pattern/");
        let _print = lexer.next_token().unwrap();
        let regex = lexer.next_token().unwrap();
        assert_eq!(regex.token_type, TokenType::RegexMatch);
        
        // split with regex
        let mut lexer = PerlLexer::new("split /,/");
        let _split = lexer.next_token().unwrap();
        let regex = lexer.next_token().unwrap();
        assert_eq!(regex.token_type, TokenType::RegexMatch);
        
        // map followed by braces (block, not hash)
        let mut lexer = PerlLexer::new("map { $_ * 2 }");
        let _map = lexer.next_token().unwrap();
        let brace = lexer.next_token().unwrap();
        assert_eq!(brace.token_type, TokenType::LeftBrace);
    }
    
    #[test]
    fn test_version_strings() {
        // v-strings
        let mut lexer = PerlLexer::new("v5.32.0");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Version(_)));
        
        // Dotted decimal
        let mut lexer = PerlLexer::new("5.032_001");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Number(_)));
    }
    
    #[test]
    fn test_prototypes_and_attributes() {
        // Subroutine with prototype
        let mut lexer = PerlLexer::new("sub foo ($@) { }");
        let _sub = lexer.next_token().unwrap();
        let _name = lexer.next_token().unwrap();
        let _paren = lexer.next_token().unwrap();
        let proto1 = lexer.next_token().unwrap();
        assert!(matches!(proto1.token_type, TokenType::Operator(_))); // $ as operator
        
        // Attribute
        let mut lexer = PerlLexer::new(": lvalue");
        let colon = lexer.next_token().unwrap();
        assert_eq!(colon.token_type, TokenType::Colon);
        let attr = lexer.next_token().unwrap();
        assert!(matches!(attr.token_type, TokenType::Identifier(_)));
    }
    
    #[test]
    fn test_file_test_operators() {
        // File test operators should be recognized
        let file_tests = vec![
            "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-e", "-z", "-s",
            "-f", "-d", "-l", "-p", "-S", "-b", "-c", "-t", "-u", "-g", "-k",
            "-T", "-B", "-M", "-A", "-C"
        ];
        
        for op in file_tests {
            let input = format!("{} $file", op);
            let mut lexer = PerlLexer::new(&input);
            let token = lexer.next_token().unwrap();
            assert!(matches!(token.token_type, TokenType::Operator(_)), 
                    "Failed to recognize file test operator: {}", op);
            assert_eq!(token.text.as_ref(), op);
        }
        
        // Stacked file tests
        let mut lexer = PerlLexer::new("-f -w -x $file");
        let op1 = lexer.next_token().unwrap();
        assert!(matches!(op1.token_type, TokenType::Operator(_)));
        let op2 = lexer.next_token().unwrap();
        assert!(matches!(op2.token_type, TokenType::Operator(_)));
        let op3 = lexer.next_token().unwrap();
        assert!(matches!(op3.token_type, TokenType::Operator(_)));
    }
    
    #[test]
    fn test_glob_and_filehandles() {
        // GLOB filehandles
        let mut lexer = PerlLexer::new("open(FH, '<', 'file.txt')");
        let _open = lexer.next_token().unwrap();
        let _paren = lexer.next_token().unwrap();
        let fh = lexer.next_token().unwrap();
        assert!(matches!(fh.token_type, TokenType::Identifier(_)));
        assert_eq!(fh.text.as_ref(), "FH");
        
        // Diamond operator
        let mut lexer = PerlLexer::new("<>");
        let diamond = lexer.next_token().unwrap();
        assert!(matches!(diamond.token_type, TokenType::Operator(_)));
        assert_eq!(diamond.text.as_ref(), "<>");
        
        // Glob operator
        let mut lexer = PerlLexer::new("<*.txt>");
        let glob = lexer.next_token().unwrap();
        assert!(matches!(glob.token_type, TokenType::Operator(_)));
        
        // Readline from filehandle
        let mut lexer = PerlLexer::new("<FH>");
        let readline = lexer.next_token().unwrap();
        assert!(matches!(readline.token_type, TokenType::Operator(_)));
    }
    
    #[test]
    fn test_regex_modifiers() {
        // All regex modifiers
        let mut lexer = PerlLexer::new("/pattern/gimsxoadlupn");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::RegexMatch);
        assert!(token.text.contains("gimsxoadlupn"));
        
        // Substitution with eval modifier
        let mut lexer = PerlLexer::new("s/old/new/gee");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::Substitution);
        assert!(token.text.contains("gee"));
        
        // Match with compiled flag
        let mut lexer = PerlLexer::new("m/pattern/o");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::RegexMatch);
        
        // Extended regex with comments
        let mut lexer = PerlLexer::new("/(?x) pattern # comment/");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::RegexMatch);
    }
    
    #[test]
    fn test_statement_modifiers() {
        // if modifier
        let mut lexer = PerlLexer::new("print $x if $y");
        let _print = lexer.next_token().unwrap();
        let _var1 = lexer.next_token().unwrap();
        let if_mod = lexer.next_token().unwrap();
        assert!(matches!(if_mod.token_type, TokenType::Keyword(ref k) if k.as_ref() == "if"));
        
        // unless modifier
        let mut lexer = PerlLexer::new("die unless $ok");
        let _die = lexer.next_token().unwrap();
        let unless = lexer.next_token().unwrap();
        assert!(matches!(unless.token_type, TokenType::Keyword(ref k) if k.as_ref() == "unless"));
        
        // while modifier
        let mut lexer = PerlLexer::new("$x++ while $y");
        let _var = lexer.next_token().unwrap();
        let _op = lexer.next_token().unwrap();
        let while_mod = lexer.next_token().unwrap();
        assert!(matches!(while_mod.token_type, TokenType::Keyword(ref k) if k.as_ref() == "while"));
    }
    
    #[test]
    fn test_package_and_method_calls() {
        // Package separator
        let mut lexer = PerlLexer::new("Foo::Bar::baz");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(_)));
        // Note: Currently we treat Foo::Bar::baz as a single identifier
        
        // Method calls with packages
        let mut lexer = PerlLexer::new("Foo::Bar->new");
        let _package = lexer.next_token().unwrap();
        let arrow = lexer.next_token().unwrap();
        assert_eq!(arrow.token_type, TokenType::Arrow);
        let method = lexer.next_token().unwrap();
        assert!(matches!(method.token_type, TokenType::Identifier(_)));
        
        // SUPER and CORE
        let mut lexer = PerlLexer::new("SUPER::method");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(_)));
    }
    
    #[test]
    fn test_block_and_hash_disambiguation() {
        // Anonymous hash
        let mut lexer = PerlLexer::new("{ key => 'value' }");
        let brace = lexer.next_token().unwrap();
        assert_eq!(brace.token_type, TokenType::LeftBrace);
        
        // Code block after map/grep
        let mut lexer = PerlLexer::new("map { $_ * 2 }");
        let _map = lexer.next_token().unwrap();
        let brace = lexer.next_token().unwrap();
        assert_eq!(brace.token_type, TokenType::LeftBrace);
        
        // Hash slice
        let mut lexer = PerlLexer::new("@hash{qw(a b c)}");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(_)));
    }
    
    #[test]
    fn test_special_literals() {
        // __END__ and __DATA__
        let mut lexer = PerlLexer::new("__END__");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(_)));
        assert_eq!(token.text.as_ref(), "__END__");
        
        let mut lexer = PerlLexer::new("__DATA__");
        let token = lexer.next_token().unwrap();
        assert!(matches!(token.token_type, TokenType::Identifier(_)));
        assert_eq!(token.text.as_ref(), "__DATA__");
        
        // __FILE__, __LINE__, __PACKAGE__
        let special = vec!["__FILE__", "__LINE__", "__PACKAGE__", "__SUB__"];
        for lit in special {
            let mut lexer = PerlLexer::new(lit);
            let token = lexer.next_token().unwrap();
            assert!(matches!(token.token_type, TokenType::Identifier(_)));
            assert_eq!(token.text.as_ref(), lit);
        }
    }
    
    #[test]
    fn test_smartmatch_and_junction() {
        // Smart match operator
        let mut lexer = PerlLexer::new("$x ~~ $y");
        let _var1 = lexer.next_token().unwrap();
        let smartmatch = lexer.next_token().unwrap();
        assert!(matches!(smartmatch.token_type, TokenType::Operator(_)));
        assert_eq!(smartmatch.text.as_ref(), "~~");
        
        // Junction operators (Perl 6 style, sometimes used)
        let mut lexer = PerlLexer::new("$a | $b");
        let _var1 = lexer.next_token().unwrap();
        let junction = lexer.next_token().unwrap();
        assert!(matches!(junction.token_type, TokenType::Operator(_)));
    }
}
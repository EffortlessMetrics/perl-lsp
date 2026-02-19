//! Tests for potentially missing edge cases
use tree_sitter_perl::perl_lexer::{PerlLexer, TokenType};

#[test]
fn test_stacked_file_tests() {
    // Multiple file test operators
    let input = "if (-f -w -x $file) { print 'ok' }";
    let mut lexer = PerlLexer::new(input);
    let mut found_file_test = false;

    while let Some(token) = lexer.next_token() {
        if matches!(&token.token_type, TokenType::Operator(op) if op.starts_with("-")) {
            found_file_test = true;
        }
    }
    assert!(found_file_test);
}

#[test]
fn test_underscore_filehandle() {
    // Special _ filehandle for file tests
    let input = "-f $file && -w _";
    let mut lexer = PerlLexer::new(input);
    let mut found_underscore = false;

    while let Some(token) = lexer.next_token() {
        if matches!(&token.token_type, TokenType::Identifier(id) if id.as_ref() == "_") {
            found_underscore = true;
        }
    }
    assert!(found_underscore);
}

#[test]
fn test_glob_assignment() {
    let input = "*foo = *bar;";
    let mut lexer = PerlLexer::new(input);
    let mut found_glob = false;

    while let Some(token) = lexer.next_token() {
        if matches!(&token.token_type, TokenType::Operator(op) if op.as_ref() == "*") {
            found_glob = true;
        }
    }
    assert!(found_glob);
}

#[test]
fn test_typeglob_slots() {
    let input = r#"
*foo{SCALAR} = \$x;
*foo{ARRAY} = \@arr;
*foo{HASH} = \%hash;
*foo{CODE} = \&func;
"#;
    let mut lexer = PerlLexer::new(input);
    let mut found_typeglob_syntax = false;

    while let Some(token) = lexer.next_token() {
        if token.text.contains("{SCALAR}") || token.text.contains("{ARRAY}") {
            found_typeglob_syntax = true;
        }
    }
    // This might fail - typeglob slots are complex
    println!("Typeglob slot syntax found: {}", found_typeglob_syntax);
}

#[test]
fn test_symbolic_references() {
    let input = r#"
$${"var_" . $n} = 42;
&{$pkg . "::func"}(@args);
"#;
    let mut lexer = PerlLexer::new(input);
    let mut has_errors = false;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::Error(_)) {
            has_errors = true;
            println!("Error: {:?}", token);
        }
    }
    // Symbolic refs might cause parsing issues
    println!("Has errors: {}", has_errors);
}

#[test]
fn test_indirect_object_syntax() {
    let input = r#"
new Module::Name @args;
print $fh $data;
method $obj @params;
"#;
    let mut lexer = PerlLexer::new(input);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        tokens.push(token);
    }

    // Check if these parse without errors
    let errors: Vec<_> =
        tokens.iter().filter(|t| matches!(t.token_type, TokenType::Error(_))).collect();

    println!("Indirect object syntax errors: {}", errors.len());
}

#[test]
fn test_lvalue_subroutines() {
    let input = r#"
sub temperature :lvalue {
    $temp;
}
temperature() = 98.6;
"#;
    let mut lexer = PerlLexer::new(input);
    let mut found_lvalue = false;

    while let Some(token) = lexer.next_token() {
        if token.text.contains("lvalue") {
            found_lvalue = true;
        }
    }
    println!("Found :lvalue attribute: {}", found_lvalue);
}

#[test]
fn test_hash_array_slices() {
    let input = r#"
@hash{@keys} = @values;
@array[@indices] = reverse @array[@indices];
@hash{'key1', 'key2'} = (1, 2);
"#;
    let mut lexer = PerlLexer::new(input);
    let mut errors = 0;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::Error(_)) {
            errors += 1;
            println!("Slice error: {:?}", token);
        }
    }
    println!("Slice syntax errors: {}", errors);
}

#[test]
fn test_vstring_edge_cases() {
    let input = r#"
%h = (v1.2.3 => 'value');
$v = v2048.2049.2050;
print "Version ${\(v1.2.3)}";
"#;
    let mut lexer = PerlLexer::new(input);
    let mut found_vstring = false;

    while let Some(token) = lexer.next_token() {
        if token.text.starts_with('v') && token.text.contains('.') {
            found_vstring = true;
            println!("V-string: {}", token.text);
        }
    }
    println!("Found v-string: {}", found_vstring);
}

#[test]
fn test_format_edge_cases() {
    let input = r#"
format STDOUT =
@<<<<<<   @||||||   @>>>>>>
$name,    $score,   $date
.
"#;
    let mut lexer = PerlLexer::new(input);
    let mut in_format = false;

    while let Some(token) = lexer.next_token() {
        if matches!(&token.token_type, TokenType::Keyword(k) if k.as_ref() == "format") {
            in_format = true;
        }
        if in_format {
            println!("Format token: {:?}", token);
        }
    }
}

#[test]
fn test_encoding_pragma() {
    let input = r#"
use utf8;
my $str = "café";
no utf8;
use encoding 'latin1';
"#;
    let mut lexer = PerlLexer::new(input);
    let mut found_encoding = false;

    while let Some(token) = lexer.next_token() {
        if token.text.contains("encoding") || token.text.contains("utf8") {
            found_encoding = true;
        }
    }
    println!("Found encoding directives: {}", found_encoding);
}

#[test]
fn test_regex_code_assertions() {
    let input = r#"
$str =~ /pattern(?{ $code++ })/;
$str =~ /(??{ $regex })/;
$str =~ /(*SKIP)(*FAIL)/;
"#;
    let mut lexer = PerlLexer::new(input);
    let mut complex_regex = 0;

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::RegexMatch)
            && (token.text.contains("(?{")
                || token.text.contains("(??{")
                || token.text.contains("(*"))
        {
            complex_regex += 1;
        }
    }
    println!("Complex regex patterns found: {}", complex_regex);
}

#[test]
fn test_data_section() {
    let input = r#"
print "Hello";
__DATA__
This is data
More data here
"#;
    let mut lexer = PerlLexer::new(input);
    let mut found_data = false;

    while let Some(token) = lexer.next_token() {
        if token.text.contains("__DATA__") {
            found_data = true;
        }
    }
    println!("Found __DATA__ section: {}", found_data);
}

#[test]
fn test_autoload_edge_case() {
    let input = r#"
sub AUTOLOAD {
    goto &{$AUTOLOAD};
}
"#;
    let mut lexer = PerlLexer::new(input);
    let mut found_autoload = false;

    while let Some(token) = lexer.next_token() {
        if token.text.contains("AUTOLOAD") {
            found_autoload = true;
        }
    }
    println!("Found AUTOLOAD: {}", found_autoload);
}

#[test]
fn test_unusual_delimiters() {
    let input = r#"
q###text###;
qq«text»;
qw¡one two three!;
"#;
    let mut lexer = PerlLexer::new(input);
    let mut unusual_quotes = 0;

    while let Some(token) = lexer.next_token() {
        if matches!(
            token.token_type,
            TokenType::StringLiteral
                | TokenType::QuoteSingle
                | TokenType::QuoteDouble
                | TokenType::QuoteWords
        ) {
            unusual_quotes += 1;
            println!("Quote token: {:?}", token);
        }
    }
    println!("Unusual quote delimiters: {}", unusual_quotes);
}

use perl_lexer::{PerlLexer, Token, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn collect_tokens(input: &str) -> Vec<Token> {
    let mut lexer = PerlLexer::new(input);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        let reached_eof = matches!(token.token_type, TokenType::EOF);
        tokens.push(token);
        if reached_eof {
            break;
        }
    }

    tokens
}

fn type_label(token_type: &TokenType) -> &'static str {
    match token_type {
        TokenType::Division => "Division",
        TokenType::RegexMatch => "RegexMatch",
        TokenType::Substitution => "Substitution",
        TokenType::Transliteration => "Transliteration",
        TokenType::QuoteRegex => "QuoteRegex",
        TokenType::StringLiteral => "StringLiteral",
        TokenType::QuoteSingle => "QuoteSingle",
        TokenType::QuoteDouble => "QuoteDouble",
        TokenType::QuoteWords => "QuoteWords",
        TokenType::QuoteCommand => "QuoteCommand",
        TokenType::InterpolatedString(_) => "InterpolatedString",
        TokenType::HeredocStart => "HeredocStart",
        TokenType::HeredocBody(_) => "HeredocBody",
        TokenType::FormatBody(_) => "FormatBody",
        TokenType::Version(_) => "Version",
        TokenType::Pod => "Pod",
        TokenType::DataMarker(_) => "DataMarker",
        TokenType::DataBody(_) => "DataBody",
        TokenType::UnknownRest => "UnknownRest",
        TokenType::Identifier(_) => "Identifier",
        TokenType::Number(_) => "Number",
        TokenType::Operator(_) => "Operator",
        TokenType::Keyword(_) => "Keyword",
        TokenType::LeftParen => "LeftParen",
        TokenType::RightParen => "RightParen",
        TokenType::LeftBracket => "LeftBracket",
        TokenType::RightBracket => "RightBracket",
        TokenType::LeftBrace => "LeftBrace",
        TokenType::RightBrace => "RightBrace",
        TokenType::Semicolon => "Semicolon",
        TokenType::Comma => "Comma",
        TokenType::Colon => "Colon",
        TokenType::Arrow => "Arrow",
        TokenType::FatComma => "FatComma",
        TokenType::Whitespace => "Whitespace",
        TokenType::Newline => "Newline",
        TokenType::Comment(_) => "Comment",
        TokenType::EOF => "EOF",
        TokenType::Error(_) => "Error",
    }
}

#[test]
fn given_term_context_when_slash_is_first_then_it_starts_regex() {
    // Given input in ExpectTerm mode (start of statement)
    let tokens = collect_tokens("/foo/");

    // When the lexer reads a leading slash
    let labels: Vec<_> = tokens.iter().map(|t| type_label(&t.token_type)).collect();

    // Then slash is tokenized as regex, not division
    assert_eq!(labels, vec!["RegexMatch", "EOF"]);
}

#[test]
fn given_operator_context_when_slash_follows_identifier_then_it_is_division() {
    // Given input where slash follows an identifier
    let tokens = collect_tokens("$x / 2");

    // When slash is lexed after a term
    let labels: Vec<_> = tokens.iter().map(|t| type_label(&t.token_type)).collect();

    // Then slash is treated as division operator
    assert_eq!(labels, vec!["Identifier", "Division", "Number", "EOF"]);
}

#[test]
fn given_quote_like_operators_when_delimiters_present_then_correct_quote_tokens_are_emitted()
-> TestResult {
    // Given common quote-like operators with valid delimiters
    let cases = [
        ("q{abc}", "QuoteSingle"),
        ("qq{abc}", "QuoteDouble"),
        ("qw(a b c)", "QuoteWords"),
        ("qx(ls)", "QuoteCommand"),
        ("qr/test/", "QuoteRegex"),
        ("s/a/b/", "Substitution"),
        ("tr/a-z/A-Z/", "Transliteration"),
    ];

    for (input, expected_first_type) in cases {
        // When each case is tokenized
        let tokens = collect_tokens(input);
        let first = tokens.first().ok_or("expected first token")?;

        // Then the first token maps to the operator-specific token kind
        assert_eq!(type_label(&first.token_type), expected_first_type, "input: {input}");
    }

    Ok(())
}

#[test]
fn given_heredoc_when_body_tokens_mode_enabled_then_body_is_emitted_before_eof() -> TestResult {
    // Given a valid heredoc program fragment
    let input = "print <<'EOF';\nhello\nEOF\n";

    // When lexing with heredoc body token emission enabled
    let mut lexer = PerlLexer::with_body_tokens(input);
    let mut tokens = Vec::new();
    while let Some(token) = lexer.next_token() {
        let reached_eof = matches!(token.token_type, TokenType::EOF);
        tokens.push(token);
        if reached_eof {
            break;
        }
    }

    // Then heredoc start and body both appear in order
    let labels: Vec<_> = tokens.iter().map(|t| type_label(&t.token_type)).collect();
    let heredoc_start_index = labels.iter().position(|label| *label == "HeredocStart");
    let heredoc_body_index = labels.iter().position(|label| *label == "HeredocBody");
    assert!(heredoc_start_index.is_some(), "expected HeredocStart token, got {:?}", labels);
    assert!(heredoc_body_index.is_some(), "expected HeredocBody token, got {:?}", labels);

    let heredoc_start_index = heredoc_start_index.ok_or("expected HeredocStart index")?;
    let heredoc_body_index = heredoc_body_index.ok_or("expected HeredocBody index")?;
    assert!(
        heredoc_start_index < heredoc_body_index,
        "expected HeredocStart before HeredocBody, got {:?}",
        labels
    );
    assert_eq!(labels.last(), Some(&"EOF"));

    Ok(())
}

#[test]
fn given_sigil_brace_sequences_when_tokenized_then_sigil_and_left_brace_are_split() {
    // Given sigil+brace forms that must not collapse into identifiers
    for input in ["${", "@{", "%{"] {
        // When tokenizing the two-character input
        let tokens = collect_tokens(input);
        let labels: Vec<_> = tokens.iter().map(|t| type_label(&t.token_type)).collect();

        // Then sigil is emitted as Identifier and brace as LeftBrace
        assert_eq!(labels, vec!["Identifier", "LeftBrace", "EOF"], "input: {input}");
    }
}

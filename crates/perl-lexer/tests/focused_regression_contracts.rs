use perl_lexer::{PerlLexer, Token, TokenType};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn next_non_trivia(lexer: &mut PerlLexer<'_>) -> Option<Token> {
    loop {
        let token = lexer.next_token()?;
        if !matches!(token.token_type, TokenType::Whitespace | TokenType::Newline) {
            return Some(token);
        }
    }
}

#[test]
fn slash_after_block_is_division_with_expected_text_and_span() -> TestResult {
    let input = "{1} / 2";
    let mut lexer = PerlLexer::new(input);

    // { 1 }
    let _ = next_non_trivia(&mut lexer).ok_or("missing {")?;
    let _ = next_non_trivia(&mut lexer).ok_or("missing 1")?;
    let _ = next_non_trivia(&mut lexer).ok_or("missing }")?;

    let slash = next_non_trivia(&mut lexer).ok_or("missing slash token")?;
    assert_eq!(slash.token_type, TokenType::Division);
    assert_eq!(slash.text.as_ref(), "/");
    assert_eq!(&input[slash.start..slash.end], "/");
    Ok(())
}

#[test]
fn slash_budget_guard_emits_unknown_rest_with_valid_span() -> TestResult {
    let huge = "a".repeat(70_000);
    let input = format!("/{huge}/");

    let mut lexer = PerlLexer::new(&input);
    let first = lexer.next_token().ok_or("expected a token")?;

    assert_eq!(first.token_type, TokenType::UnknownRest);
    assert!(first.start < first.end, "UnknownRest should consume remaining input");
    assert!(first.end <= input.len(), "UnknownRest span must stay in bounds");
    Ok(())
}

#[test]
fn quote_like_q_brace_preserves_token_text() -> TestResult {
    let input = "q{abc def}";
    let mut lexer = PerlLexer::new(input);

    let token = next_non_trivia(&mut lexer).ok_or("expected q token")?;
    assert_eq!(token.token_type, TokenType::QuoteSingle);
    assert_eq!(token.text.as_ref(), input);
    assert_eq!(token.start, 0);
    assert_eq!(token.end, input.len());
    Ok(())
}

#[test]
fn quote_like_qr_bracket_is_quote_regex() -> TestResult {
    let input = "qr[abc]+";
    let mut lexer = PerlLexer::new(input);

    let token = next_non_trivia(&mut lexer).ok_or("expected qr token")?;
    assert_eq!(token.token_type, TokenType::QuoteRegex);
    assert_eq!(token.text.as_ref(), "qr[abc]");
    Ok(())
}

#[test]
fn quote_like_words_after_sub_are_identifiers() -> TestResult {
    let cases = [
        ("sub m { # source comment keeps the block open\n}\n", "m"),
        ("sub s { 1 }\n", "s"),
        ("sub tr { 1 }\n", "tr"),
        ("sub q { 1 }\n", "q"),
    ];

    for (input, expected_name) in cases {
        let mut lexer = PerlLexer::new(input);

        let sub = next_non_trivia(&mut lexer).ok_or("missing sub token")?;
        assert!(
            matches!(&sub.token_type, TokenType::Keyword(keyword) if keyword.as_ref() == "sub")
        );

        let name = next_non_trivia(&mut lexer).ok_or("missing sub name token")?;
        assert!(matches!(
            &name.token_type,
            TokenType::Identifier(identifier) | TokenType::Keyword(identifier)
                if identifier.as_ref() == expected_name
        ));
        assert_eq!(name.text.as_ref(), expected_name);

        let brace = next_non_trivia(&mut lexer).ok_or("missing sub block brace")?;
        assert_eq!(brace.token_type, TokenType::LeftBrace);
    }

    Ok(())
}

#[test]
fn transliteration_tr_with_modifiers_is_single_token() -> TestResult {
    let input = "tr/a-z/A-Z/cdr";
    let mut lexer = PerlLexer::new(input);

    let token = next_non_trivia(&mut lexer).ok_or("expected transliteration")?;
    assert_eq!(token.token_type, TokenType::Transliteration);
    assert_eq!(token.text.as_ref(), input);
    assert_eq!(token.start, 0);
    assert_eq!(token.end, input.len());
    Ok(())
}

#[test]
fn heredoc_with_cr_terminates_without_looping() {
    let mut lexer = PerlLexer::new("``<<TAG\r");

    for i in 0..32 {
        match lexer.next_token() {
            Some(token) if matches!(token.token_type, TokenType::EOF) => return,
            Some(_) => {}
            None => return,
        }
        assert!(i < 31, "lexer did not terminate within bounded token budget");
    }
}

#[test]
fn utf8_bom_then_vstring_keeps_version_span_valid() -> TestResult {
    let input = "\u{FEFF}use v5.38;";
    let mut lexer = PerlLexer::new(input);

    let first = next_non_trivia(&mut lexer).ok_or("missing first token")?;
    assert!(matches!(first.token_type, TokenType::Keyword(_)));

    let version = next_non_trivia(&mut lexer).ok_or("missing version token")?;
    assert!(matches!(version.token_type, TokenType::Version(_)));
    assert_eq!(&input[version.start..version.end], "v5.38");
    Ok(())
}

#[test]
fn unicode_heredoc_regression_input_does_not_panic() {
    let input = "¡<<'";
    let result = std::panic::catch_unwind(|| {
        let mut lexer = PerlLexer::new(input);
        let token = lexer.next_token();
        if let Some(tok) = token {
            assert!(tok.end <= input.len(), "token span must remain in bounds");
            assert!(tok.start <= tok.end, "token span must be well-formed");
        }
    });

    assert!(result.is_ok(), "lexer should not panic for unicode heredoc edge input");
}

#[test]
fn utf8_heredoc_body_data_marker_does_not_split_code() -> TestResult {
    let input = "my $text = <<\"TXT\";\nαβ __DATA__\nTXT\nsay \"ok\";\n__DATA__\npayload\n";
    let real_marker = input.rfind("__DATA__").ok_or("missing real data marker")?;

    assert_eq!(perl_lexer::find_data_marker_byte_lexed(input), Some(real_marker));
    assert_eq!(perl_lexer::code_slice(input), &input[..real_marker]);
    Ok(())
}

#[test]
fn data_marker_after_utf8_prefix_reports_byte_offset() -> TestResult {
    let input = "say \"é\";\n__END__\ntrailer\n";
    let marker = input.find("__END__").ok_or("missing end marker")?;

    assert_eq!(perl_lexer::find_data_marker_byte_lexed(input), Some(marker));
    assert_eq!(&input[marker..marker + "__END__".len()], "__END__");
    Ok(())
}

#[test]
fn unterminated_quote_command_degrades_gracefully() -> TestResult {
    let mut lexer = PerlLexer::new("qx{unterminated");
    let token = next_non_trivia(&mut lexer).ok_or("expected token")?;

    assert!(matches!(token.token_type, TokenType::Error(_) | TokenType::QuoteCommand));
    assert!(token.end <= "qx{unterminated".len());
    Ok(())
}

#[test]
fn y_alias_for_tr_is_transliteration_token() -> TestResult {
    // `y///` is the historical alias for `tr///`; both must lex as Transliteration.
    let input = "y/a-z/A-Z/";
    let mut lexer = PerlLexer::new(input);
    let token = next_non_trivia(&mut lexer).ok_or("expected transliteration")?;
    assert_eq!(token.token_type, TokenType::Transliteration);
    assert_eq!(token.text.as_ref(), input);
    Ok(())
}

#[test]
fn percent_y_hash_name_is_not_transliteration() -> TestResult {
    let input = "my %y = %$y;";
    let mut lexer = PerlLexer::new(input);

    while let Some(token) = next_non_trivia(&mut lexer) {
        if token.token_type == TokenType::EOF {
            break;
        }
        assert!(
            !matches!(token.token_type, TokenType::Transliteration | TokenType::Error(_)),
            "`%y` hash variable must not trigger y/// transliteration; got {:?} from {:?}",
            token.token_type,
            token.text
        );
    }

    Ok(())
}

#[test]
fn qr_with_modifier_flag_is_quote_regex_token() -> TestResult {
    // `qr//i` — regex with ignore-case flag; modifier must be part of the token.
    let input = "qr/hello world/i";
    let mut lexer = PerlLexer::new(input);
    let token = next_non_trivia(&mut lexer).ok_or("expected qr token")?;
    assert_eq!(token.token_type, TokenType::QuoteRegex);
    assert_eq!(token.text.as_ref(), input, "modifier 'i' must be included in token text");
    Ok(())
}

// ---------------------------------------------------------------------------
// Braced-variable `::`-folding boundary (issue #3939 /
// `qualified_name_closes_brace_from_here`, crates/perl-lexer/src/lib.rs).
//
// The braced-variable scan only folds a `::`-delimited segment into the
// identifier token when it sees TWO colons (`ch == ':' && peek_char(1) ==
// Some(':')`). A single colon must NOT trigger that branch — it must stop
// the scan exactly at the colon and let the colon become a separate token,
// same as any other non-identifier character. This pins the `ch == ':'`
// boundary directly at the lexer/token level (call-observation on
// `PerlLexer::next_token`), independent of the parser-level
// `${Foo::}`-error and `${Foo::bar->{baz}}`-partial-deref regression tests
// in `perl-parser-core`.
// ---------------------------------------------------------------------------

#[test]
fn braced_variable_single_colon_does_not_fold_qualified_name() -> TestResult {
    // `${Foo:bar}` — a single colon (not `::`) right after the identifier
    // inside the braces. `qualified_name_closes_brace_from_here` must never
    // even be reached: the compound guard's first colon check passes but
    // `peek_char(1) == Some(':')` is false (next char is `b`), so the scan
    // must break immediately at the colon rather than folding `Foo:bar`
    // into one token.
    let input = "${Foo:bar}";
    let mut lexer = PerlLexer::new(input);

    let ident = next_non_trivia(&mut lexer).ok_or("expected identifier token")?;
    assert_eq!(
        ident.token_type,
        TokenType::Identifier(std::sync::Arc::from("${Foo")),
        "braced scan must stop at the single colon, got text {:?}",
        ident.text
    );
    assert_eq!(
        ident.text.as_ref(),
        "${Foo",
        "identifier token must not swallow past the single colon"
    );
    let colon_pos = input.find(':').ok_or("test input must contain a colon")?;
    assert_eq!(
        ident.end, colon_pos,
        "identifier token must end exactly at the single colon boundary"
    );

    let colon = next_non_trivia(&mut lexer).ok_or("expected a separate colon token")?;
    assert_eq!(
        colon.token_type,
        TokenType::Operator(std::sync::Arc::from(":")),
        "the un-folded single colon must surface as its own token (not be swallowed into the \
         identifier), got {:?}",
        colon.token_type
    );
    assert_eq!(colon.text.as_ref(), ":");

    Ok(())
}

#[test]
fn braced_variable_double_colon_folds_qualified_name_into_one_token() -> TestResult {
    // Contrast case: `${Foo::bar}` — a real `::` immediately followed by an
    // identifier segment and then `}` — DOES hit
    // `qualified_name_closes_brace_from_here` and folds the whole braced
    // qualified name into a single Identifier token, confirming the
    // single-colon test above is discriminating the intended boundary and
    // not just an unrelated lexer quirk.
    let input = "${Foo::bar}";
    let mut lexer = PerlLexer::new(input);

    let ident = next_non_trivia(&mut lexer).ok_or("expected identifier token")?;
    assert_eq!(
        ident.token_type,
        TokenType::Identifier(std::sync::Arc::from(input)),
        "double-colon qualified name must fold into a single braced-variable token, got text {:?}",
        ident.text
    );
    assert_eq!(ident.text.as_ref(), input);
    assert_eq!(ident.end, input.len(), "folded token must consume the whole `${{Foo::bar}}` span");

    Ok(())
}

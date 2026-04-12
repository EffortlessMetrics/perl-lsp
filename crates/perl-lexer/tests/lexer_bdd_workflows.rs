//! BDD-style workflow tests for perl-lexer.
//!
//! These scenarios model user-facing Perl snippets and assert tokenization
//! outcomes using Given/When/Then structure.

use perl_lexer::{PerlLexer, Token, TokenType};

struct BddScenario {
    name: &'static str,
}

impl BddScenario {
    fn new(name: &'static str) -> Self {
        eprintln!("Scenario: {name}");
        Self { name }
    }

    fn given(&self, message: &str) {
        eprintln!("[{}] Given {message}", self.name);
    }

    fn when(&self, message: &str) {
        eprintln!("[{}] When {message}", self.name);
    }

    fn then(&self, message: &str) {
        eprintln!("[{}] Then {message}", self.name);
    }
}

fn collect_tokens(input: &str) -> Vec<Token> {
    PerlLexer::new(input).collect_tokens()
}

fn collect_tokens_with_heredoc_bodies(input: &str) -> Vec<Token> {
    PerlLexer::with_body_tokens(input).collect_tokens()
}

#[test]
fn scenario_division_and_regex_are_disambiguated_by_mode() {
    let scenario = BddScenario::new("division_vs_regex_disambiguation");

    scenario.given("a term appears before '/' so lexer expects an operator");
    let division_input = "my $x = 10 / 2;";

    scenario.when("the line is tokenized");
    let division_tokens = collect_tokens(division_input);

    scenario.then("'/' is emitted as Division and not as RegexMatch");
    assert!(division_tokens.iter().any(|t| matches!(t.token_type, TokenType::Division)));
    assert!(!division_tokens.iter().any(|t| matches!(t.token_type, TokenType::RegexMatch)));

    scenario.given("an expression starts with '/' so lexer expects a term");
    let regex_input = "/answer/";

    scenario.when("the regex literal is tokenized");
    let regex_tokens = collect_tokens(regex_input);

    scenario.then("the token stream contains RegexMatch");
    assert!(regex_tokens.iter().any(|t| matches!(t.token_type, TokenType::RegexMatch)));
}

#[test]
fn scenario_quote_operators_require_delimiters() {
    let scenario = BddScenario::new("quote_operator_delimiter_requirement");

    scenario.given("word-like quote operators without delimiters");
    let bare_inputs = ["q", "qq", "qw", "qr", "qx", "m", "s", "tr", "y"];

    scenario.when("each bare operator is tokenized");
    for input in bare_inputs {
        let tokens = collect_tokens(input);
        let first = &tokens[0];
        assert!(
            matches!(first.token_type, TokenType::Identifier(_)),
            "input {input:?} should start as Identifier, got {:?}",
            first.token_type
        );
    }

    scenario.then("operators with delimiters are recognized as quote-like tokens");
    let quoted = [
        ("q{a}", "QuoteSingle"),
        ("qq{a}", "QuoteDouble"),
        ("qw{a b}", "QuoteWords"),
        ("qr{a+}", "QuoteRegex"),
        ("qx{echo hi}", "QuoteCommand"),
        ("s/a/b/", "Substitution"),
        ("tr/a-z/A-Z/", "Transliteration"),
    ];

    for (input, kind_name) in quoted {
        let tokens = collect_tokens(input);
        let first = &tokens[0];
        let is_expected = match kind_name {
            "QuoteSingle" => matches!(first.token_type, TokenType::QuoteSingle),
            "QuoteDouble" => matches!(first.token_type, TokenType::QuoteDouble),
            "QuoteWords" => matches!(first.token_type, TokenType::QuoteWords),
            "QuoteRegex" => matches!(first.token_type, TokenType::QuoteRegex),
            "QuoteCommand" => matches!(first.token_type, TokenType::QuoteCommand),
            "Substitution" => matches!(first.token_type, TokenType::Substitution),
            "Transliteration" => matches!(first.token_type, TokenType::Transliteration),
            _ => false,
        };
        assert!(is_expected, "input {input:?} should be {kind_name}, got {:?}", first.token_type);
    }
}

#[test]
fn scenario_heredoc_is_emitted_as_start_then_body() {
    let scenario = BddScenario::new("heredoc_tokens_flow");
    let input = "print <<EOF;\nhello world\nEOF\n";

    scenario.given("a heredoc declaration followed by body and terminator");
    scenario.when("the document is tokenized");
    let tokens = collect_tokens_with_heredoc_bodies(input);

    scenario.then("the stream contains HeredocStart and HeredocBody in order");
    let start_idx = tokens.iter().position(|t| matches!(t.token_type, TokenType::HeredocStart));
    let body_idx = tokens.iter().position(|t| matches!(t.token_type, TokenType::HeredocBody(_)));

    assert!(start_idx.is_some(), "expected HeredocStart token");
    assert!(body_idx.is_some(), "expected HeredocBody token");
    assert!(start_idx < body_idx, "HeredocStart should appear before HeredocBody");
}

#[test]
fn scenario_hash_subscript_context_suppresses_quote_op_detection() {
    let scenario = BddScenario::new("hash_subscript_quote_op_suppression");
    let input = "$h{m} + $h{s} + $h{tr}";

    scenario.given("quote-op looking words are used as hash keys inside braces");
    scenario.when("the expression is tokenized");
    let tokens = collect_tokens(input);

    scenario.then("no quote-like operator tokens are emitted for hash keys");
    for token in &tokens {
        assert!(
            !matches!(
                token.token_type,
                TokenType::QuoteSingle
                    | TokenType::QuoteDouble
                    | TokenType::QuoteWords
                    | TokenType::QuoteRegex
                    | TokenType::QuoteCommand
                    | TokenType::Substitution
                    | TokenType::Transliteration
                    | TokenType::RegexMatch
            ),
            "did not expect quote-like token in hash-key scenario: {:?}",
            token.token_type
        );
    }

    assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::LeftBrace)));
    assert!(tokens.iter().any(|t| matches!(t.token_type, TokenType::RightBrace)));
}

#[test]
fn scenario_sigil_brace_sequences_split_into_sigil_and_left_brace() {
    let scenario = BddScenario::new("sigil_brace_boundary");

    scenario
        .given("sigil-plus-brace sequences that begin scalar, array, and hash dereference forms");

    for input in ["${", "@{", "%{"] {
        scenario.when(&format!("{input:?} is tokenized"));
        let tokens = collect_tokens(input);

        scenario.then("the sigil remains separate from the following left brace");
        assert!(
            matches!(tokens.first().map(|token| &token.token_type), Some(TokenType::Identifier(_))),
            "expected leading sigil token for {input:?}, got {:?}",
            tokens.first().map(|token| &token.token_type)
        );
        assert!(
            matches!(tokens.get(1).map(|token| &token.token_type), Some(TokenType::LeftBrace)),
            "expected LeftBrace after sigil for {input:?}, got {:?}",
            tokens.get(1).map(|token| &token.token_type)
        );
    }
}

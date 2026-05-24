//! Perl token definitions shared across the parser ecosystem.
//!
//! This crate defines [`Token`] and [`TokenKind`], the fundamental types that
//! flow from the lexer (`perl-lexer`) into the parser (`perl-parser-core`).
//! Downstream crates re-export these types so consumers rarely need to depend
//! on `perl-token` directly.
//!
//! # Examples
//!
//! Create and inspect tokens:
//!
//! ```rust
//! use perl_token::{Token, TokenKind};
//!
//! // Create a keyword token for `my`
//! let token = Token::new(TokenKind::My, "my", 0, 2);
//! assert_eq!(token.kind, TokenKind::My);
//! assert_eq!(&*token.text, "my");
//! assert_eq!(token.start, 0);
//! assert_eq!(token.end, 2);
//!
//! // Create a numeric literal token
//! let num = Token::new(TokenKind::Number, "42", 7, 9);
//! assert_eq!(num.kind, TokenKind::Number);
//! assert_eq!(&*num.text, "42");
//! ```
//!
//! Use [`TokenKind::display_name`] for user-facing error messages:
//!
//! ```rust
//! use perl_token::TokenKind;
//!
//! assert_eq!(TokenKind::LeftBrace.display_name(), "'{'");
//! assert_eq!(TokenKind::Identifier.display_name(), "identifier");
//! assert_eq!(TokenKind::Eof.display_name(), "end of input");
//! ```

#![warn(missing_docs)]

mod kind;
mod token;

pub use kind::{
    DELIMITER_SPELLINGS, KEYWORD_SPELLINGS, OPERATOR_SPELLINGS, SIGIL_SPELLINGS, TokenCategory,
    TokenKind, TokenKindMetadata,
};
pub use token::{Token, TokenRef, TokenSpan, TokenSpanError};

#[cfg(test)]
mod tests {
    use super::*;

    // --- TokenSpan ---

    #[test]
    fn token_span_new_and_accessors() {
        let span = TokenSpan::new(5, 10);
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 10);
        assert_eq!(span.len(), 5);
        assert!(!span.is_empty());
        assert_eq!(span.range(), 5..10);
    }

    #[test]
    fn token_span_is_empty_when_zero_length() {
        let span = TokenSpan::new(3, 3);
        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn token_span_try_new_ok() -> Result<(), TokenSpanError> {
        let span = TokenSpan::try_new(0, 5)?;
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
        Ok(())
    }

    #[test]
    fn token_span_try_new_end_before_start_errors() {
        assert_eq!(
            TokenSpan::try_new(10, 5),
            Err(TokenSpanError::EndBeforeStart { start: 10, end: 5 })
        );
    }

    #[test]
    fn token_span_error_display_end_before_start() {
        let err = TokenSpanError::EndBeforeStart { start: 10, end: 5 };
        let msg = err.to_string();
        assert!(msg.contains("10"));
        assert!(msg.contains("5"));
    }

    #[test]
    fn token_span_error_display_empty_span_not_allowed() {
        let err = TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, at: 7 };
        let msg = err.to_string();
        assert!(msg.contains("Identifier"));
        assert!(msg.contains("7"));
    }

    // --- Token ---

    #[test]
    fn token_new_stores_fields() {
        let tok = Token::new(TokenKind::My, "my", 0, 2);
        assert_eq!(tok.kind, TokenKind::My);
        assert_eq!(&*tok.text, "my");
        assert_eq!(tok.start, 0);
        assert_eq!(tok.end, 2);
    }

    #[test]
    fn token_len_saturates_for_inverted_span() {
        let tok = Token::new(TokenKind::Identifier, "x", 9, 4);
        assert_eq!(tok.len(), 0);
        assert!(tok.is_empty());
    }

    #[test]
    fn token_len_and_is_empty() {
        let tok = Token::new(TokenKind::Identifier, "foo", 10, 13);
        assert_eq!(tok.len(), 3);
        assert!(!tok.is_empty());

        let eof = Token::eof_at(8);
        assert_eq!(eof.len(), 0);
        assert!(eof.is_empty());
    }

    #[test]
    fn token_span_and_range() {
        let tok = Token::new(TokenKind::Number, "42", 5, 7);
        assert_eq!(tok.span(), TokenSpan::new(5, 7));
        assert_eq!(tok.range(), 5..7);
    }

    #[test]
    fn token_try_new_allows_ordered_spans() -> Result<(), TokenSpanError> {
        let tok = Token::try_new(TokenKind::Identifier, "name", 4, 8)?;
        assert_eq!(tok.kind, TokenKind::Identifier);
        assert_eq!(&*tok.text, "name");
        assert_eq!(tok.span(), TokenSpan::new(4, 8));
        Ok(())
    }

    #[test]
    fn token_try_new_rejects_end_before_start() {
        assert_eq!(
            Token::try_new(TokenKind::Identifier, "x", 10, 5),
            Err(TokenSpanError::EndBeforeStart { start: 10, end: 5 })
        );
    }

    #[test]
    fn token_new_checked_rejects_empty_non_eof() {
        assert_eq!(
            Token::new_checked(TokenKind::Identifier, "", 5, 5),
            Err(TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, at: 5 })
        );
    }

    #[test]
    fn token_new_checked_allows_empty_eof() -> Result<(), TokenSpanError> {
        let tok = Token::new_checked(TokenKind::Eof, "", 5, 5)?;
        assert_eq!(tok.kind, TokenKind::Eof);
        assert_eq!(tok.start, 5);
        Ok(())
    }

    #[test]
    fn token_new_checked_allows_empty_unknown() -> Result<(), TokenSpanError> {
        let tok = Token::new_checked(TokenKind::Unknown, "", 6, 6)?;
        assert_eq!(tok.kind, TokenKind::Unknown);
        assert_eq!(tok.start, 6);
        assert!(tok.is_empty());
        Ok(())
    }

    #[test]
    fn token_eof_at() {
        let eof = Token::eof_at(42);
        assert_eq!(eof.kind, TokenKind::Eof);
        assert_eq!(eof.start, 42);
        assert_eq!(eof.end, 42);
        assert!(eof.is_empty());
    }

    #[test]
    fn token_unknown_at_normalises_inverted_span() {
        let tok = Token::unknown_at("?", 5, 3); // end < start
        assert_eq!(tok.kind, TokenKind::Unknown);
        assert_eq!(tok.start, 5);
        assert_eq!(tok.end, 5); // bounded to start
    }

    #[test]
    fn token_with_kind() {
        let tok = Token::new(TokenKind::Identifier, "sub", 0, 3);
        let retyped = tok.with_kind(TokenKind::Sub);
        assert_eq!(retyped.kind, TokenKind::Sub);
        assert_eq!(&*retyped.text, "sub");
        assert_eq!(retyped.start, 0);
        assert_eq!(retyped.end, 3);
    }

    #[test]
    fn token_with_span_ok() -> Result<(), TokenSpanError> {
        let tok = Token::new(TokenKind::String, "hello", 0, 5);
        let moved = tok.with_span(10, 15)?;
        assert_eq!(moved.start, 10);
        assert_eq!(moved.end, 15);
        Ok(())
    }

    #[test]
    fn token_with_span_rejects_empty_non_eof() {
        let tok = Token::new(TokenKind::String, "hello", 0, 5);
        assert_eq!(
            tok.with_span(10, 10),
            Err(TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::String, at: 10 })
        );
    }

    #[test]
    fn token_display_name_delegates_to_kind() {
        let tok = Token::new(TokenKind::LeftBrace, "{", 0, 1);
        assert_eq!(tok.display_name(), "'{'");
    }

    #[test]
    fn token_as_ref_token_round_trip() {
        let tok = Token::new(TokenKind::Sub, "sub", 0, 3);
        let tok_ref = tok.as_ref_token();
        assert_eq!(tok_ref.kind, TokenKind::Sub);
        assert_eq!(tok_ref.text, "sub");
        assert_eq!(tok_ref.start, 0);
        assert_eq!(tok_ref.end, 3);

        let owned: Token = tok_ref.into();
        assert_eq!(owned.kind, TokenKind::Sub);
        assert_eq!(&*owned.text, "sub");
    }

    // --- TokenRef ---

    #[test]
    fn token_ref_len_saturates_for_inverted_span() {
        let r = TokenRef::new(TokenKind::Identifier, "x", 9, 4);
        assert_eq!(r.len(), 0);
        assert!(r.is_empty());
    }

    #[test]
    fn token_ref_accessors() {
        let r = TokenRef::new(TokenKind::Number, "99", 4, 6);
        assert_eq!(r.len(), 2);
        assert!(!r.is_empty());
        assert_eq!(r.span(), (4, 6));
        assert_eq!(r.display_name(), "number");
    }

    #[test]
    fn token_ref_try_new_allows_ordered_spans() -> Result<(), TokenSpanError> {
        let r = TokenRef::try_new(TokenKind::Number, "99", 4, 6)?;
        assert_eq!(r.kind, TokenKind::Number);
        assert_eq!(r.text, "99");
        assert_eq!(r.span(), (4, 6));
        Ok(())
    }

    #[test]
    fn token_ref_to_owned_token() {
        let r = TokenRef::new(TokenKind::Identifier, "foo", 1, 4);
        let owned = r.to_owned_token();
        assert_eq!(owned.kind, TokenKind::Identifier);
        assert_eq!(&*owned.text, "foo");
    }

    // --- TokenKind::from_keyword ---

    #[test]
    fn from_keyword_recognises_perl_keywords() {
        assert_eq!(TokenKind::from_keyword("my"), Some(TokenKind::My));
        assert_eq!(TokenKind::from_keyword("sub"), Some(TokenKind::Sub));
        assert_eq!(TokenKind::from_keyword("if"), Some(TokenKind::If));
        assert_eq!(TokenKind::from_keyword("elsif"), Some(TokenKind::Elsif));
        assert_eq!(TokenKind::from_keyword("else"), Some(TokenKind::Else));
        assert_eq!(TokenKind::from_keyword("while"), Some(TokenKind::While));
        assert_eq!(TokenKind::from_keyword("for"), Some(TokenKind::For));
        assert_eq!(TokenKind::from_keyword("foreach"), Some(TokenKind::Foreach));
        assert_eq!(TokenKind::from_keyword("return"), Some(TokenKind::Return));
        assert_eq!(TokenKind::from_keyword("package"), Some(TokenKind::Package));
        assert_eq!(TokenKind::from_keyword("use"), Some(TokenKind::Use));
        assert_eq!(TokenKind::from_keyword("BEGIN"), Some(TokenKind::Begin));
        assert_eq!(TokenKind::from_keyword("END"), Some(TokenKind::End));
        assert_eq!(TokenKind::from_keyword("eval"), Some(TokenKind::Eval));
        assert_eq!(TokenKind::from_keyword("class"), Some(TokenKind::Class));
        assert_eq!(TokenKind::from_keyword("defer"), Some(TokenKind::Defer));
        assert_eq!(TokenKind::from_keyword("and"), Some(TokenKind::WordAnd));
        assert_eq!(TokenKind::from_keyword("or"), Some(TokenKind::WordOr));
        assert_eq!(TokenKind::from_keyword("not"), Some(TokenKind::WordNot));
        assert_eq!(TokenKind::from_keyword("xor"), Some(TokenKind::WordXor));
        assert_eq!(TokenKind::from_keyword("cmp"), Some(TokenKind::StringCompare));
    }

    #[test]
    fn from_keyword_unknown_returns_none() {
        assert_eq!(TokenKind::from_keyword("MY"), None);
        assert_eq!(TokenKind::from_keyword("Sub"), None);
        assert_eq!(TokenKind::from_keyword("unknown"), None);
        assert_eq!(TokenKind::from_keyword(""), None);
    }

    // --- TokenKind::from_operator ---

    #[test]
    fn from_operator_recognises_operators() {
        assert_eq!(TokenKind::from_operator("="), Some(TokenKind::Assign));
        assert_eq!(TokenKind::from_operator("+"), Some(TokenKind::Plus));
        assert_eq!(TokenKind::from_operator("**"), Some(TokenKind::Power));
        assert_eq!(TokenKind::from_operator("->"), Some(TokenKind::Arrow));
        assert_eq!(TokenKind::from_operator("=>"), Some(TokenKind::FatArrow));
        assert_eq!(TokenKind::from_operator("<=>"), Some(TokenKind::Spaceship));
        assert_eq!(TokenKind::from_operator("//="), Some(TokenKind::DefinedOrAssign));
        assert_eq!(TokenKind::from_operator("..."), Some(TokenKind::Ellipsis));
        assert_eq!(TokenKind::from_operator("~~"), Some(TokenKind::SmartMatch));
    }

    #[test]
    fn from_operator_unknown_returns_none() {
        assert_eq!(TokenKind::from_operator(""), None);
        assert_eq!(TokenKind::from_operator("xyz"), None);
    }

    // --- TokenKind::from_delimiter ---

    #[test]
    fn from_delimiter_recognises_all() {
        assert_eq!(TokenKind::from_delimiter("("), Some(TokenKind::LeftParen));
        assert_eq!(TokenKind::from_delimiter(")"), Some(TokenKind::RightParen));
        assert_eq!(TokenKind::from_delimiter("{"), Some(TokenKind::LeftBrace));
        assert_eq!(TokenKind::from_delimiter("}"), Some(TokenKind::RightBrace));
        assert_eq!(TokenKind::from_delimiter("["), Some(TokenKind::LeftBracket));
        assert_eq!(TokenKind::from_delimiter("]"), Some(TokenKind::RightBracket));
        assert_eq!(TokenKind::from_delimiter(";"), Some(TokenKind::Semicolon));
        assert_eq!(TokenKind::from_delimiter(","), Some(TokenKind::Comma));
        assert_eq!(TokenKind::from_delimiter("x"), None);
    }

    // --- TokenKind::from_sigil ---

    #[test]
    fn from_sigil_recognises_all() {
        assert_eq!(TokenKind::from_sigil("$"), Some(TokenKind::ScalarSigil));
        assert_eq!(TokenKind::from_sigil("@"), Some(TokenKind::ArraySigil));
        assert_eq!(TokenKind::from_sigil("%"), Some(TokenKind::HashSigil));
        assert_eq!(TokenKind::from_sigil("&"), Some(TokenKind::SubSigil));
        assert_eq!(TokenKind::from_sigil("*"), Some(TokenKind::GlobSigil));
        assert_eq!(TokenKind::from_sigil("!"), None);
    }

    // --- TokenKind::category ---

    #[test]
    fn category_keyword_variants() {
        assert_eq!(TokenKind::My.category(), TokenCategory::Keyword);
        assert_eq!(TokenKind::Sub.category(), TokenCategory::Keyword);
        assert_eq!(TokenKind::Defer.category(), TokenCategory::Keyword);
    }

    #[test]
    fn category_operator_variants() {
        assert_eq!(TokenKind::Plus.category(), TokenCategory::Operator);
        assert_eq!(TokenKind::Spaceship.category(), TokenCategory::Operator);
        assert_eq!(TokenKind::WordAnd.category(), TokenCategory::Operator);
    }

    #[test]
    fn category_delimiter_variants() {
        assert_eq!(TokenKind::LeftParen.category(), TokenCategory::Delimiter);
        assert_eq!(TokenKind::Comma.category(), TokenCategory::Delimiter);
    }

    #[test]
    fn category_literal_variants() {
        assert_eq!(TokenKind::Number.category(), TokenCategory::Literal);
        assert_eq!(TokenKind::HeredocStart.category(), TokenCategory::Literal);
        assert_eq!(TokenKind::DataMarker.category(), TokenCategory::Literal);
    }

    #[test]
    fn category_identifier_variants() {
        assert_eq!(TokenKind::Identifier.category(), TokenCategory::Identifier);
        assert_eq!(TokenKind::ScalarSigil.category(), TokenCategory::Identifier);
        assert_eq!(TokenKind::GlobSigil.category(), TokenCategory::Identifier);
    }

    #[test]
    fn category_special_variants() {
        assert_eq!(TokenKind::Eof.category(), TokenCategory::Special);
        assert_eq!(TokenKind::Unknown.category(), TokenCategory::Special);
    }

    // --- TokenKind::display_name ---

    #[test]
    fn display_name_selected_variants() {
        assert_eq!(TokenKind::LeftBrace.display_name(), "'{'");
        assert_eq!(TokenKind::RightBrace.display_name(), "'}'");
        assert_eq!(TokenKind::Identifier.display_name(), "identifier");
        assert_eq!(TokenKind::Eof.display_name(), "end of input");
        assert_eq!(TokenKind::Number.display_name(), "number");
        assert_eq!(TokenKind::Sub.display_name(), "'sub'");
        assert_eq!(TokenKind::Semicolon.display_name(), "';'");
        assert_eq!(TokenKind::HeredocStart.display_name(), "heredoc (<<)");
        assert_eq!(TokenKind::DataMarker.display_name(), "data marker (__DATA__ or __END__)");
    }

    // --- TokenKind::all / metadata_count ---

    #[test]
    fn all_returns_132_variants() {
        assert_eq!(TokenKind::all().len(), 132);
        assert_eq!(TokenKind::metadata_count(), 132);
    }

    #[test]
    fn metadata_round_trips_through_kind() {
        let m = TokenKind::Sub.metadata();
        assert_eq!(m.category, TokenCategory::Keyword);
        assert_eq!(m.display_name, "'sub'");
    }

    // --- TokenKind role predicates ---

    #[test]
    fn is_assignment_operator_returns_true_for_assign_variants() {
        assert!(TokenKind::Assign.is_assignment_operator());
        assert!(TokenKind::PlusAssign.is_assignment_operator());
        assert!(TokenKind::MinusAssign.is_assignment_operator());
        assert!(TokenKind::StarAssign.is_assignment_operator());
        assert!(TokenKind::SlashAssign.is_assignment_operator());
        assert!(TokenKind::PercentAssign.is_assignment_operator());
        assert!(TokenKind::DotAssign.is_assignment_operator());
        assert!(TokenKind::AndAssign.is_assignment_operator());
        assert!(TokenKind::OrAssign.is_assignment_operator());
        assert!(TokenKind::XorAssign.is_assignment_operator());
        assert!(TokenKind::PowerAssign.is_assignment_operator());
        assert!(TokenKind::LeftShiftAssign.is_assignment_operator());
        assert!(TokenKind::RightShiftAssign.is_assignment_operator());
        assert!(TokenKind::LogicalAndAssign.is_assignment_operator());
        assert!(TokenKind::LogicalOrAssign.is_assignment_operator());
        assert!(TokenKind::DefinedOrAssign.is_assignment_operator());
    }

    #[test]
    fn is_assignment_operator_returns_false_for_non_assign() {
        assert!(!TokenKind::Plus.is_assignment_operator());
        assert!(!TokenKind::Equal.is_assignment_operator());
        assert!(!TokenKind::Identifier.is_assignment_operator());
    }

    #[test]
    fn is_logical_operator_returns_true_for_logical_variants() {
        assert!(TokenKind::And.is_logical_operator());
        assert!(TokenKind::Or.is_logical_operator());
        assert!(TokenKind::Not.is_logical_operator());
        assert!(TokenKind::DefinedOr.is_logical_operator());
        assert!(TokenKind::WordAnd.is_logical_operator());
        assert!(TokenKind::WordOr.is_logical_operator());
        assert!(TokenKind::WordNot.is_logical_operator());
        assert!(TokenKind::WordXor.is_logical_operator());
    }

    #[test]
    fn is_logical_operator_returns_false_for_non_logical() {
        assert!(!TokenKind::Plus.is_logical_operator());
        assert!(!TokenKind::Assign.is_logical_operator());
        assert!(!TokenKind::Identifier.is_logical_operator());
    }

    #[test]
    fn is_open_delimiter_returns_true_for_open_delimiters() {
        assert!(TokenKind::LeftParen.is_open_delimiter());
        assert!(TokenKind::LeftBrace.is_open_delimiter());
        assert!(TokenKind::LeftBracket.is_open_delimiter());
    }

    #[test]
    fn is_open_delimiter_returns_false_for_non_open() {
        assert!(!TokenKind::RightParen.is_open_delimiter());
        assert!(!TokenKind::Semicolon.is_open_delimiter());
        assert!(!TokenKind::Plus.is_open_delimiter());
    }

    #[test]
    fn is_quote_like_returns_true_for_quote_variants() {
        assert!(TokenKind::Regex.is_quote_like());
        assert!(TokenKind::Substitution.is_quote_like());
        assert!(TokenKind::Transliteration.is_quote_like());
        assert!(TokenKind::QuoteSingle.is_quote_like());
        assert!(TokenKind::QuoteDouble.is_quote_like());
        assert!(TokenKind::QuoteWords.is_quote_like());
        assert!(TokenKind::QuoteCommand.is_quote_like());
        assert!(TokenKind::HeredocStart.is_quote_like());
    }

    #[test]
    fn is_quote_like_returns_false_for_non_quote() {
        assert!(!TokenKind::String.is_quote_like());
        assert!(!TokenKind::Identifier.is_quote_like());
        assert!(!TokenKind::LeftParen.is_quote_like());
    }

    #[test]
    fn is_recovery_boundary_returns_true_for_boundaries() {
        assert!(TokenKind::Semicolon.is_recovery_boundary());
        assert!(TokenKind::RightParen.is_recovery_boundary());
        assert!(TokenKind::RightBrace.is_recovery_boundary());
        assert!(TokenKind::RightBracket.is_recovery_boundary());
        assert!(TokenKind::Eof.is_recovery_boundary());
    }

    #[test]
    fn is_recovery_boundary_returns_false_for_non_boundary() {
        assert!(!TokenKind::Plus.is_recovery_boundary());
        assert!(!TokenKind::Identifier.is_recovery_boundary());
        assert!(!TokenKind::LeftParen.is_recovery_boundary());
    }

    // --- TokenRef::new_checked branches ---

    #[test]
    fn token_ref_new_checked_rejects_end_before_start() {
        assert_eq!(
            TokenRef::new_checked(TokenKind::Identifier, "x", 10, 3),
            Err(TokenSpanError::EndBeforeStart { start: 10, end: 3 })
        );
    }

    #[test]
    fn token_ref_new_checked_allows_empty_eof() -> Result<(), Box<dyn std::error::Error>> {
        let tok = TokenRef::new_checked(TokenKind::Eof, "", 7, 7)?;
        assert_eq!(tok.kind, TokenKind::Eof);
        assert_eq!(tok.start, 7);
        assert!(tok.is_empty());
        Ok(())
    }

    #[test]
    fn token_ref_new_checked_allows_empty_unknown() -> Result<(), Box<dyn std::error::Error>> {
        let tok = TokenRef::new_checked(TokenKind::Unknown, "", 3, 3)?;
        assert_eq!(tok.kind, TokenKind::Unknown);
        assert_eq!(tok.start, 3);
        assert!(tok.is_empty());
        Ok(())
    }

    #[test]
    fn token_ref_new_checked_rejects_empty_non_eof() {
        assert_eq!(
            TokenRef::new_checked(TokenKind::Identifier, "", 5, 5),
            Err(TokenSpanError::EmptySpanNotAllowed { kind: TokenKind::Identifier, at: 5 })
        );
    }
}

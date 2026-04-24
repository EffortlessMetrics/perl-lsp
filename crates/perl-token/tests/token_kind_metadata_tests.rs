use perl_token::TokenKind;

fn assert_exhaustive(kind: TokenKind) {
    match kind {
        TokenKind::My
        | TokenKind::Our
        | TokenKind::Local
        | TokenKind::State
        | TokenKind::Sub
        | TokenKind::If
        | TokenKind::Elsif
        | TokenKind::Else
        | TokenKind::Unless
        | TokenKind::While
        | TokenKind::Until
        | TokenKind::For
        | TokenKind::Foreach
        | TokenKind::Return
        | TokenKind::Package
        | TokenKind::Use
        | TokenKind::No
        | TokenKind::Begin
        | TokenKind::End
        | TokenKind::Check
        | TokenKind::Init
        | TokenKind::Unitcheck
        | TokenKind::Eval
        | TokenKind::Do
        | TokenKind::Given
        | TokenKind::When
        | TokenKind::Default
        | TokenKind::Try
        | TokenKind::Catch
        | TokenKind::Finally
        | TokenKind::Continue
        | TokenKind::Next
        | TokenKind::Last
        | TokenKind::Redo
        | TokenKind::Goto
        | TokenKind::Class
        | TokenKind::Method
        | TokenKind::Field
        | TokenKind::Format
        | TokenKind::Undef
        | TokenKind::Defer
        | TokenKind::Assign
        | TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::Power
        | TokenKind::LeftShift
        | TokenKind::RightShift
        | TokenKind::BitwiseAnd
        | TokenKind::BitwiseOr
        | TokenKind::BitwiseXor
        | TokenKind::BitwiseNot
        | TokenKind::PlusAssign
        | TokenKind::MinusAssign
        | TokenKind::StarAssign
        | TokenKind::SlashAssign
        | TokenKind::PercentAssign
        | TokenKind::DotAssign
        | TokenKind::AndAssign
        | TokenKind::OrAssign
        | TokenKind::XorAssign
        | TokenKind::PowerAssign
        | TokenKind::LeftShiftAssign
        | TokenKind::RightShiftAssign
        | TokenKind::LogicalAndAssign
        | TokenKind::LogicalOrAssign
        | TokenKind::DefinedOrAssign
        | TokenKind::Equal
        | TokenKind::NotEqual
        | TokenKind::Match
        | TokenKind::NotMatch
        | TokenKind::SmartMatch
        | TokenKind::Less
        | TokenKind::Greater
        | TokenKind::LessEqual
        | TokenKind::GreaterEqual
        | TokenKind::Spaceship
        | TokenKind::StringCompare
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Not
        | TokenKind::DefinedOr
        | TokenKind::WordAnd
        | TokenKind::WordOr
        | TokenKind::WordNot
        | TokenKind::WordXor
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::Dot
        | TokenKind::Range
        | TokenKind::Ellipsis
        | TokenKind::Increment
        | TokenKind::Decrement
        | TokenKind::DoubleColon
        | TokenKind::Question
        | TokenKind::Colon
        | TokenKind::Backslash
        | TokenKind::LeftParen
        | TokenKind::RightParen
        | TokenKind::LeftBrace
        | TokenKind::RightBrace
        | TokenKind::LeftBracket
        | TokenKind::RightBracket
        | TokenKind::Semicolon
        | TokenKind::Comma
        | TokenKind::Number
        | TokenKind::String
        | TokenKind::Regex
        | TokenKind::Substitution
        | TokenKind::Transliteration
        | TokenKind::QuoteSingle
        | TokenKind::QuoteDouble
        | TokenKind::QuoteWords
        | TokenKind::QuoteCommand
        | TokenKind::HeredocStart
        | TokenKind::HeredocBody
        | TokenKind::FormatBody
        | TokenKind::DataMarker
        | TokenKind::DataBody
        | TokenKind::VString
        | TokenKind::UnknownRest
        | TokenKind::HeredocDepthLimit
        | TokenKind::Identifier
        | TokenKind::ScalarSigil
        | TokenKind::ArraySigil
        | TokenKind::HashSigil
        | TokenKind::SubSigil
        | TokenKind::GlobSigil
        | TokenKind::Eof
        | TokenKind::Unknown => {}
    }
}

#[test]
fn metadata_all_kinds_is_exhaustive() {
    let all_kinds = TokenKind::all_kinds();

    for kind in all_kinds {
        assert_exhaustive(*kind);
    }

    let mut unique = Vec::new();
    for kind in all_kinds {
        if !unique.contains(kind) {
            unique.push(*kind);
        }
    }
    assert_eq!(unique.len(), all_kinds.len(), "TokenKind::all_kinds() contains duplicates");
}

#[test]
fn spelling_tables_have_unique_kind_entries() {
    let mut seen = Vec::new();

    for (kind, _) in TokenKind::keyword_spellings()
        .iter()
        .chain(TokenKind::operator_spellings().iter())
        .chain(TokenKind::delimiter_spellings().iter())
        .chain(TokenKind::sigil_spellings().iter())
    {
        assert!(!seen.contains(kind), "duplicate spelling entry for {kind:?}");
        seen.push(*kind);
    }
}

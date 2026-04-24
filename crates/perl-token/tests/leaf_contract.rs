use perl_token::{Token, TokenKind};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenCategory {
    Keyword,
    Operator,
    Delimiter,
    Literal,
    IdentifierOrSigil,
    Special,
}

fn token_kind_metadata(kind: TokenKind) -> (&'static str, TokenCategory) {
    use TokenCategory::{Delimiter, IdentifierOrSigil, Keyword, Literal, Operator, Special};

    match kind {
        TokenKind::My => ("My", Keyword),
        TokenKind::Our => ("Our", Keyword),
        TokenKind::Local => ("Local", Keyword),
        TokenKind::State => ("State", Keyword),
        TokenKind::Sub => ("Sub", Keyword),
        TokenKind::If => ("If", Keyword),
        TokenKind::Elsif => ("Elsif", Keyword),
        TokenKind::Else => ("Else", Keyword),
        TokenKind::Unless => ("Unless", Keyword),
        TokenKind::While => ("While", Keyword),
        TokenKind::Until => ("Until", Keyword),
        TokenKind::For => ("For", Keyword),
        TokenKind::Foreach => ("Foreach", Keyword),
        TokenKind::Return => ("Return", Keyword),
        TokenKind::Package => ("Package", Keyword),
        TokenKind::Use => ("Use", Keyword),
        TokenKind::No => ("No", Keyword),
        TokenKind::Begin => ("Begin", Keyword),
        TokenKind::End => ("End", Keyword),
        TokenKind::Check => ("Check", Keyword),
        TokenKind::Init => ("Init", Keyword),
        TokenKind::Unitcheck => ("Unitcheck", Keyword),
        TokenKind::Eval => ("Eval", Keyword),
        TokenKind::Do => ("Do", Keyword),
        TokenKind::Given => ("Given", Keyword),
        TokenKind::When => ("When", Keyword),
        TokenKind::Default => ("Default", Keyword),
        TokenKind::Try => ("Try", Keyword),
        TokenKind::Catch => ("Catch", Keyword),
        TokenKind::Finally => ("Finally", Keyword),
        TokenKind::Continue => ("Continue", Keyword),
        TokenKind::Next => ("Next", Keyword),
        TokenKind::Last => ("Last", Keyword),
        TokenKind::Redo => ("Redo", Keyword),
        TokenKind::Goto => ("Goto", Keyword),
        TokenKind::Class => ("Class", Keyword),
        TokenKind::Method => ("Method", Keyword),
        TokenKind::Field => ("Field", Keyword),
        TokenKind::Format => ("Format", Keyword),
        TokenKind::Undef => ("Undef", Keyword),
        TokenKind::Defer => ("Defer", Keyword),
        TokenKind::Assign => ("Assign", Operator),
        TokenKind::Plus => ("Plus", Operator),
        TokenKind::Minus => ("Minus", Operator),
        TokenKind::Star => ("Star", Operator),
        TokenKind::Slash => ("Slash", Operator),
        TokenKind::Percent => ("Percent", Operator),
        TokenKind::Power => ("Power", Operator),
        TokenKind::LeftShift => ("LeftShift", Operator),
        TokenKind::RightShift => ("RightShift", Operator),
        TokenKind::BitwiseAnd => ("BitwiseAnd", Operator),
        TokenKind::BitwiseOr => ("BitwiseOr", Operator),
        TokenKind::BitwiseXor => ("BitwiseXor", Operator),
        TokenKind::BitwiseNot => ("BitwiseNot", Operator),
        TokenKind::PlusAssign => ("PlusAssign", Operator),
        TokenKind::MinusAssign => ("MinusAssign", Operator),
        TokenKind::StarAssign => ("StarAssign", Operator),
        TokenKind::SlashAssign => ("SlashAssign", Operator),
        TokenKind::PercentAssign => ("PercentAssign", Operator),
        TokenKind::DotAssign => ("DotAssign", Operator),
        TokenKind::AndAssign => ("AndAssign", Operator),
        TokenKind::OrAssign => ("OrAssign", Operator),
        TokenKind::XorAssign => ("XorAssign", Operator),
        TokenKind::PowerAssign => ("PowerAssign", Operator),
        TokenKind::LeftShiftAssign => ("LeftShiftAssign", Operator),
        TokenKind::RightShiftAssign => ("RightShiftAssign", Operator),
        TokenKind::LogicalAndAssign => ("LogicalAndAssign", Operator),
        TokenKind::LogicalOrAssign => ("LogicalOrAssign", Operator),
        TokenKind::DefinedOrAssign => ("DefinedOrAssign", Operator),
        TokenKind::Equal => ("Equal", Operator),
        TokenKind::NotEqual => ("NotEqual", Operator),
        TokenKind::Match => ("Match", Operator),
        TokenKind::NotMatch => ("NotMatch", Operator),
        TokenKind::SmartMatch => ("SmartMatch", Operator),
        TokenKind::Less => ("Less", Operator),
        TokenKind::Greater => ("Greater", Operator),
        TokenKind::LessEqual => ("LessEqual", Operator),
        TokenKind::GreaterEqual => ("GreaterEqual", Operator),
        TokenKind::Spaceship => ("Spaceship", Operator),
        TokenKind::StringCompare => ("StringCompare", Operator),
        TokenKind::And => ("And", Operator),
        TokenKind::Or => ("Or", Operator),
        TokenKind::Not => ("Not", Operator),
        TokenKind::DefinedOr => ("DefinedOr", Operator),
        TokenKind::WordAnd => ("WordAnd", Operator),
        TokenKind::WordOr => ("WordOr", Operator),
        TokenKind::WordNot => ("WordNot", Operator),
        TokenKind::WordXor => ("WordXor", Operator),
        TokenKind::Arrow => ("Arrow", Operator),
        TokenKind::FatArrow => ("FatArrow", Operator),
        TokenKind::Dot => ("Dot", Operator),
        TokenKind::Range => ("Range", Operator),
        TokenKind::Ellipsis => ("Ellipsis", Operator),
        TokenKind::Increment => ("Increment", Operator),
        TokenKind::Decrement => ("Decrement", Operator),
        TokenKind::DoubleColon => ("DoubleColon", Operator),
        TokenKind::Question => ("Question", Operator),
        TokenKind::Colon => ("Colon", Operator),
        TokenKind::Backslash => ("Backslash", Operator),
        TokenKind::LeftParen => ("LeftParen", Delimiter),
        TokenKind::RightParen => ("RightParen", Delimiter),
        TokenKind::LeftBrace => ("LeftBrace", Delimiter),
        TokenKind::RightBrace => ("RightBrace", Delimiter),
        TokenKind::LeftBracket => ("LeftBracket", Delimiter),
        TokenKind::RightBracket => ("RightBracket", Delimiter),
        TokenKind::Semicolon => ("Semicolon", Delimiter),
        TokenKind::Comma => ("Comma", Delimiter),
        TokenKind::Number => ("Number", Literal),
        TokenKind::String => ("String", Literal),
        TokenKind::Regex => ("Regex", Literal),
        TokenKind::Substitution => ("Substitution", Literal),
        TokenKind::Transliteration => ("Transliteration", Literal),
        TokenKind::QuoteSingle => ("QuoteSingle", Literal),
        TokenKind::QuoteDouble => ("QuoteDouble", Literal),
        TokenKind::QuoteWords => ("QuoteWords", Literal),
        TokenKind::QuoteCommand => ("QuoteCommand", Literal),
        TokenKind::HeredocStart => ("HeredocStart", Literal),
        TokenKind::HeredocBody => ("HeredocBody", Literal),
        TokenKind::FormatBody => ("FormatBody", Literal),
        TokenKind::DataMarker => ("DataMarker", Literal),
        TokenKind::DataBody => ("DataBody", Literal),
        TokenKind::VString => ("VString", Literal),
        TokenKind::UnknownRest => ("UnknownRest", Literal),
        TokenKind::HeredocDepthLimit => ("HeredocDepthLimit", Literal),
        TokenKind::Identifier => ("Identifier", IdentifierOrSigil),
        TokenKind::ScalarSigil => ("ScalarSigil", IdentifierOrSigil),
        TokenKind::ArraySigil => ("ArraySigil", IdentifierOrSigil),
        TokenKind::HashSigil => ("HashSigil", IdentifierOrSigil),
        TokenKind::SubSigil => ("SubSigil", IdentifierOrSigil),
        TokenKind::GlobSigil => ("GlobSigil", IdentifierOrSigil),
        TokenKind::Eof => ("Eof", Special),
        TokenKind::Unknown => ("Unknown", Special),
    }
}

#[test]
fn token_kind_all_matches_metadata_count_and_coverage() -> TestResult {
    let all = TokenKind::all();
    let mut metadata_count = 0usize;

    for kind in all {
        let (name, _) = token_kind_metadata(*kind);
        assert_eq!(name, format!("{kind:?}"));
        metadata_count += 1;
    }

    assert_eq!(all.len(), metadata_count, "TokenKind::all() count must match metadata coverage");
    Ok(())
}

#[test]
fn token_kind_api_conformance_snapshot() -> TestResult {
    let variants =
        TokenKind::all().iter().map(|kind| format!("{kind:?}")).collect::<Vec<_>>().join(",");
    let expected = include_str!("fixtures/tokenkind_api_snapshot.txt").trim();
    assert_eq!(
        variants, expected,
        "TokenKind variant set changed; update metadata/docs/conformance intentionally"
    );
    Ok(())
}

#[test]
fn token_struct_api_conformance_snapshot() -> TestResult {
    let token = Token::new(TokenKind::Identifier, "x", 2, 3);
    let contract = format!(
        "kind={:?};text={};start={};end={};len={};is_empty={}",
        token.kind,
        &*token.text,
        token.start,
        token.end,
        token.len(),
        token.is_empty()
    );
    let expected = "kind=Identifier;text=x;start=2;end=3;len=1;is_empty=false";
    assert_eq!(contract, expected, "Token API contract changed");
    Ok(())
}

#[test]
fn docs_variant_count_note_is_kept_in_sync() -> TestResult {
    let expected = format!("TokenKind variant count: {}", TokenKind::all().len());
    let readme = include_str!("../README.md");
    let roadmap = include_str!("../ROADMAP.md");

    assert!(readme.contains(&expected), "README must include updated variant count note");
    assert!(roadmap.contains(&expected), "ROADMAP must include updated variant count note");
    Ok(())
}

use perl_token::TokenKind;
use std::error::Error;

const SNAPSHOT_PATH: &str = "tests/snapshots/token_kind_display_names.md";

#[derive(Clone, Copy)]
enum Category {
    Keyword,
    Operator,
    Delimiter,
    Literal,
    IdentifierOrSigil,
    Special,
}

impl Category {
    fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Operator => "operator",
            Self::Delimiter => "delimiter",
            Self::Literal => "literal",
            Self::IdentifierOrSigil => "identifier/sigil",
            Self::Special => "special",
        }
    }
}

fn all_token_kinds() -> Vec<TokenKind> {
    vec![
        TokenKind::My,
        TokenKind::Our,
        TokenKind::Local,
        TokenKind::State,
        TokenKind::Sub,
        TokenKind::If,
        TokenKind::Elsif,
        TokenKind::Else,
        TokenKind::Unless,
        TokenKind::While,
        TokenKind::Until,
        TokenKind::For,
        TokenKind::Foreach,
        TokenKind::Return,
        TokenKind::Package,
        TokenKind::Use,
        TokenKind::No,
        TokenKind::Begin,
        TokenKind::End,
        TokenKind::Check,
        TokenKind::Init,
        TokenKind::Unitcheck,
        TokenKind::Eval,
        TokenKind::Do,
        TokenKind::Given,
        TokenKind::When,
        TokenKind::Default,
        TokenKind::Try,
        TokenKind::Catch,
        TokenKind::Finally,
        TokenKind::Continue,
        TokenKind::Next,
        TokenKind::Last,
        TokenKind::Redo,
        TokenKind::Goto,
        TokenKind::Class,
        TokenKind::Method,
        TokenKind::Field,
        TokenKind::Format,
        TokenKind::Undef,
        TokenKind::Defer,
        TokenKind::Assign,
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::Power,
        TokenKind::LeftShift,
        TokenKind::RightShift,
        TokenKind::BitwiseAnd,
        TokenKind::BitwiseOr,
        TokenKind::BitwiseXor,
        TokenKind::BitwiseNot,
        TokenKind::PlusAssign,
        TokenKind::MinusAssign,
        TokenKind::StarAssign,
        TokenKind::SlashAssign,
        TokenKind::PercentAssign,
        TokenKind::DotAssign,
        TokenKind::AndAssign,
        TokenKind::OrAssign,
        TokenKind::XorAssign,
        TokenKind::PowerAssign,
        TokenKind::LeftShiftAssign,
        TokenKind::RightShiftAssign,
        TokenKind::LogicalAndAssign,
        TokenKind::LogicalOrAssign,
        TokenKind::DefinedOrAssign,
        TokenKind::Equal,
        TokenKind::NotEqual,
        TokenKind::Match,
        TokenKind::NotMatch,
        TokenKind::SmartMatch,
        TokenKind::Less,
        TokenKind::Greater,
        TokenKind::LessEqual,
        TokenKind::GreaterEqual,
        TokenKind::Spaceship,
        TokenKind::StringCompare,
        TokenKind::And,
        TokenKind::Or,
        TokenKind::Not,
        TokenKind::DefinedOr,
        TokenKind::WordAnd,
        TokenKind::WordOr,
        TokenKind::WordNot,
        TokenKind::WordXor,
        TokenKind::Arrow,
        TokenKind::FatArrow,
        TokenKind::Dot,
        TokenKind::Range,
        TokenKind::Ellipsis,
        TokenKind::Increment,
        TokenKind::Decrement,
        TokenKind::DoubleColon,
        TokenKind::Question,
        TokenKind::Colon,
        TokenKind::Backslash,
        TokenKind::LeftParen,
        TokenKind::RightParen,
        TokenKind::LeftBrace,
        TokenKind::RightBrace,
        TokenKind::LeftBracket,
        TokenKind::RightBracket,
        TokenKind::Semicolon,
        TokenKind::Comma,
        TokenKind::Number,
        TokenKind::String,
        TokenKind::Regex,
        TokenKind::Substitution,
        TokenKind::Transliteration,
        TokenKind::QuoteSingle,
        TokenKind::QuoteDouble,
        TokenKind::QuoteWords,
        TokenKind::QuoteCommand,
        TokenKind::HeredocStart,
        TokenKind::HeredocBody,
        TokenKind::FormatBody,
        TokenKind::DataMarker,
        TokenKind::DataBody,
        TokenKind::VString,
        TokenKind::UnknownRest,
        TokenKind::HeredocDepthLimit,
        TokenKind::Identifier,
        TokenKind::ScalarSigil,
        TokenKind::ArraySigil,
        TokenKind::HashSigil,
        TokenKind::SubSigil,
        TokenKind::GlobSigil,
        TokenKind::Eof,
        TokenKind::Unknown,
    ]
}

fn category(kind: TokenKind) -> Category {
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
        | TokenKind::Defer => Category::Keyword,
        TokenKind::Assign
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
        | TokenKind::Backslash => Category::Operator,
        TokenKind::LeftParen
        | TokenKind::RightParen
        | TokenKind::LeftBrace
        | TokenKind::RightBrace
        | TokenKind::LeftBracket
        | TokenKind::RightBracket
        | TokenKind::Semicolon
        | TokenKind::Comma => Category::Delimiter,
        TokenKind::Number
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
        | TokenKind::HeredocDepthLimit => Category::Literal,
        TokenKind::Identifier
        | TokenKind::ScalarSigil
        | TokenKind::ArraySigil
        | TokenKind::HashSigil
        | TokenKind::SubSigil
        | TokenKind::GlobSigil => Category::IdentifierOrSigil,
        TokenKind::Eof | TokenKind::Unknown => Category::Special,
    }
}

fn canonical_lexeme(kind: TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::DataMarker => Some("__DATA__|__END__"),
        TokenKind::Eof => Some("<EOF>"),
        TokenKind::Unknown => None,
        other => {
            let display = other.display_name();
            if display.starts_with('\'') && display.ends_with('\'') {
                Some(&display[1..display.len() - 1])
            } else {
                None
            }
        }
    }
}

fn render_table() -> String {
    let mut out = String::from(
        "| TokenKind | display_name | category | canonical lexeme |\n|---|---|---|---|\n",
    );
    for kind in all_token_kinds() {
        let canonical = canonical_lexeme(kind).unwrap_or("-");
        out.push_str(&format!(
            "| `{:?}` | `{}` | {} | `{}` |\n",
            kind,
            kind.display_name(),
            category(kind).as_str(),
            canonical
        ));
    }
    out
}

#[test]
fn display_name_table_snapshot() -> Result<(), Box<dyn Error>> {
    let rendered = render_table();
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(SNAPSHOT_PATH, rendered)?;
        return Ok(());
    }

    let expected = std::fs::read_to_string(SNAPSHOT_PATH)?;
    assert_eq!(rendered, expected);
    Ok(())
}

#[test]
fn display_names_are_non_empty_for_all_token_kinds() -> Result<(), Box<dyn Error>> {
    for kind in all_token_kinds() {
        assert!(!kind.display_name().trim().is_empty(), "display name must not be empty: {kind:?}");
    }
    Ok(())
}

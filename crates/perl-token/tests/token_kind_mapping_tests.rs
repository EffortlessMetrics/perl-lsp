use perl_token::TokenKind;

#[test]
fn from_keyword_maps_all_canonical_keywords() {
    let keywords = [
        ("my", TokenKind::My),
        ("our", TokenKind::Our),
        ("local", TokenKind::Local),
        ("state", TokenKind::State),
        ("sub", TokenKind::Sub),
        ("if", TokenKind::If),
        ("elsif", TokenKind::Elsif),
        ("else", TokenKind::Else),
        ("unless", TokenKind::Unless),
        ("while", TokenKind::While),
        ("until", TokenKind::Until),
        ("for", TokenKind::For),
        ("foreach", TokenKind::Foreach),
        ("return", TokenKind::Return),
        ("package", TokenKind::Package),
        ("use", TokenKind::Use),
        ("no", TokenKind::No),
        ("BEGIN", TokenKind::Begin),
        ("END", TokenKind::End),
        ("CHECK", TokenKind::Check),
        ("INIT", TokenKind::Init),
        ("UNITCHECK", TokenKind::Unitcheck),
        ("eval", TokenKind::Eval),
        ("do", TokenKind::Do),
        ("given", TokenKind::Given),
        ("when", TokenKind::When),
        ("default", TokenKind::Default),
        ("try", TokenKind::Try),
        ("catch", TokenKind::Catch),
        ("finally", TokenKind::Finally),
        ("continue", TokenKind::Continue),
        ("next", TokenKind::Next),
        ("last", TokenKind::Last),
        ("redo", TokenKind::Redo),
        ("goto", TokenKind::Goto),
        ("class", TokenKind::Class),
        ("method", TokenKind::Method),
        ("field", TokenKind::Field),
        ("format", TokenKind::Format),
        ("undef", TokenKind::Undef),
        ("defer", TokenKind::Defer),
    ];

    for (spelling, expected) in keywords {
        assert_eq!(TokenKind::from_keyword(spelling), Some(expected));
    }
}

#[test]
fn from_operator_maps_all_canonical_operator_spellings() {
    let operators = [
        ("=", TokenKind::Assign),
        ("+", TokenKind::Plus),
        ("-", TokenKind::Minus),
        ("*", TokenKind::Star),
        ("/", TokenKind::Slash),
        ("%", TokenKind::Percent),
        ("**", TokenKind::Power),
        ("<<", TokenKind::LeftShift),
        (">>", TokenKind::RightShift),
        ("&", TokenKind::BitwiseAnd),
        ("|", TokenKind::BitwiseOr),
        ("^", TokenKind::BitwiseXor),
        ("~", TokenKind::BitwiseNot),
        ("+=", TokenKind::PlusAssign),
        ("-=", TokenKind::MinusAssign),
        ("*=", TokenKind::StarAssign),
        ("/=", TokenKind::SlashAssign),
        ("%=", TokenKind::PercentAssign),
        (".=", TokenKind::DotAssign),
        ("&=", TokenKind::AndAssign),
        ("|=", TokenKind::OrAssign),
        ("^=", TokenKind::XorAssign),
        ("**=", TokenKind::PowerAssign),
        ("<<=", TokenKind::LeftShiftAssign),
        (">>=", TokenKind::RightShiftAssign),
        ("&&=", TokenKind::LogicalAndAssign),
        ("||=", TokenKind::LogicalOrAssign),
        ("//=", TokenKind::DefinedOrAssign),
        ("==", TokenKind::Equal),
        ("!=", TokenKind::NotEqual),
        ("=~", TokenKind::Match),
        ("!~", TokenKind::NotMatch),
        ("~~", TokenKind::SmartMatch),
        ("<", TokenKind::Less),
        (">", TokenKind::Greater),
        ("<=", TokenKind::LessEqual),
        (">=", TokenKind::GreaterEqual),
        ("<=>", TokenKind::Spaceship),
        ("cmp", TokenKind::StringCompare),
        ("&&", TokenKind::And),
        ("||", TokenKind::Or),
        ("!", TokenKind::Not),
        ("//", TokenKind::DefinedOr),
        ("and", TokenKind::WordAnd),
        ("or", TokenKind::WordOr),
        ("not", TokenKind::WordNot),
        ("xor", TokenKind::WordXor),
        ("->", TokenKind::Arrow),
        ("=>", TokenKind::FatArrow),
        (".", TokenKind::Dot),
        ("..", TokenKind::Range),
        ("...", TokenKind::Ellipsis),
        ("++", TokenKind::Increment),
        ("--", TokenKind::Decrement),
        ("::", TokenKind::DoubleColon),
        ("?", TokenKind::Question),
        (":", TokenKind::Colon),
        ("\\", TokenKind::Backslash),
    ];

    for (spelling, expected) in operators {
        assert_eq!(TokenKind::from_operator(spelling), Some(expected));
    }
}

#[test]
fn from_delimiter_maps_all_canonical_delimiters() {
    let delimiters = [
        ("(", TokenKind::LeftParen),
        (")", TokenKind::RightParen),
        ("{", TokenKind::LeftBrace),
        ("}", TokenKind::RightBrace),
        ("[", TokenKind::LeftBracket),
        ("]", TokenKind::RightBracket),
        (";", TokenKind::Semicolon),
        (",", TokenKind::Comma),
    ];

    for (spelling, expected) in delimiters {
        assert_eq!(TokenKind::from_delimiter(spelling), Some(expected));
    }
}

#[test]
fn from_sigil_maps_all_canonical_sigils() {
    let sigils = [
        ("$", TokenKind::ScalarSigil),
        ("@", TokenKind::ArraySigil),
        ("%", TokenKind::HashSigil),
        ("&", TokenKind::SubSigil),
        ("*", TokenKind::GlobSigil),
    ];

    for (spelling, expected) in sigils {
        assert_eq!(TokenKind::from_sigil(spelling), Some(expected));
    }
}

#[test]
fn mapping_helpers_preserve_contextual_behavior() {
    assert_eq!(TokenKind::from_keyword("begin"), None);
    assert_eq!(TokenKind::from_keyword("BEGIN"), Some(TokenKind::Begin));

    // Quote-like operators remain contextual and are intentionally not plain keywords.
    assert_eq!(TokenKind::from_keyword("qw"), None);

    assert_eq!(TokenKind::from_operator("my"), None);
    assert_eq!(TokenKind::from_delimiter("<>"), None);
    assert_eq!(TokenKind::from_sigil("!"), None);
}

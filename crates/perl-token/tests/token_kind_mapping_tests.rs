use perl_token::TokenKind;

#[test]
fn keyword_mappings_are_exhaustive_and_case_sensitive() {
    let expected = [
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
        ("and", TokenKind::WordAnd),
        ("or", TokenKind::WordOr),
        ("not", TokenKind::WordNot),
        ("xor", TokenKind::WordXor),
        ("cmp", TokenKind::StringCompare),
    ];

    for (spelling, kind) in expected {
        assert_eq!(TokenKind::from_keyword(spelling), Some(kind));
    }

    assert_eq!(TokenKind::from_keyword("begin"), None);
    assert_eq!(TokenKind::from_keyword("BEGIN "), None);
    assert_eq!(TokenKind::from_keyword("qw"), None);
}

#[test]
fn operator_mappings_are_exhaustive() {
    let expected = [
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
        ("&&", TokenKind::And),
        ("||", TokenKind::Or),
        ("!", TokenKind::Not),
        ("//", TokenKind::DefinedOr),
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

    for (spelling, kind) in expected {
        assert_eq!(TokenKind::from_operator(spelling), Some(kind));
    }

    assert_eq!(TokenKind::from_operator("and"), None);
    assert_eq!(TokenKind::from_operator("#"), None);
}

#[test]
fn delimiter_and_sigil_mappings_are_exhaustive() {
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

    for (spelling, kind) in delimiters {
        assert_eq!(TokenKind::from_delimiter(spelling), Some(kind));
    }

    let sigils = [
        ("$", TokenKind::ScalarSigil),
        ("@", TokenKind::ArraySigil),
        ("%", TokenKind::HashSigil),
        ("&", TokenKind::SubSigil),
        ("*", TokenKind::GlobSigil),
    ];

    for (spelling, kind) in sigils {
        assert_eq!(TokenKind::from_sigil(spelling), Some(kind));
    }

    assert_eq!(TokenKind::from_delimiter(":"), None);
    assert_eq!(TokenKind::from_sigil("+"), None);
}

use perl_token::TokenKind;

#[test]
fn keyword_mapping_covers_canonical_spellings() {
    let cases = [
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

    for (spelling, expected) in cases {
        assert_eq!(TokenKind::from_keyword(spelling), Some(expected), "keyword: {spelling}");
    }
}

#[test]
fn operator_mapping_covers_canonical_spellings() {
    let cases = [
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

    for (spelling, expected) in cases {
        assert_eq!(TokenKind::from_operator(spelling), Some(expected), "operator: {spelling}");
    }
}

#[test]
fn delimiter_mapping_covers_canonical_spellings() {
    let cases = [
        ("(", TokenKind::LeftParen),
        (")", TokenKind::RightParen),
        ("{", TokenKind::LeftBrace),
        ("}", TokenKind::RightBrace),
        ("[", TokenKind::LeftBracket),
        ("]", TokenKind::RightBracket),
        (";", TokenKind::Semicolon),
        (",", TokenKind::Comma),
    ];

    for (spelling, expected) in cases {
        assert_eq!(TokenKind::from_delimiter(spelling), Some(expected), "delimiter: {spelling}");
    }
}

#[test]
fn sigil_mapping_covers_canonical_spellings() {
    let cases = [
        ("$", TokenKind::ScalarSigil),
        ("@", TokenKind::ArraySigil),
        ("%", TokenKind::HashSigil),
        ("&", TokenKind::SubSigil),
        ("*", TokenKind::GlobSigil),
    ];

    for (spelling, expected) in cases {
        assert_eq!(TokenKind::from_sigil(spelling), Some(expected), "sigil: {spelling}");
    }
}

#[test]
fn mappings_are_case_sensitive_and_contextual() {
    assert_eq!(TokenKind::from_keyword("My"), None);
    assert_eq!(TokenKind::from_keyword("begin"), None);
    assert_eq!(TokenKind::from_keyword("qw"), None);

    assert_eq!(TokenKind::from_operator("AND"), None);
    assert_eq!(TokenKind::from_delimiter("<"), None);
    assert_eq!(TokenKind::from_sigil("+"), None);
}

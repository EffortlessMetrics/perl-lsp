use perl_parser_core::TokenStream;
use perl_tdd_support::must;
use perl_token::TokenKind;

fn first_kind(input: &str) -> TokenKind {
    let mut stream = TokenStream::new(input);
    must(stream.next()).kind
}

fn kinds(input: &str) -> Vec<TokenKind> {
    let mut stream = TokenStream::new(input);
    let mut out = Vec::new();
    loop {
        let kind = must(stream.next()).kind;
        out.push(kind);
        if kind == TokenKind::Eof {
            break;
        }
        assert!(out.len() < 32, "unexpectedly long token stream for {input:?}");
    }
    out
}

fn second_kind(input: &str) -> TokenKind {
    kinds(input)[1]
}

#[test]
fn canonical_keywords_map_to_parser_token_kinds() {
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
    ];

    for (lexeme, expected) in cases {
        assert_eq!(first_kind(lexeme), expected, "keyword mapping failed for {lexeme}");
    }
}

#[test]
fn canonical_operators_map_to_parser_token_kinds() {
    let cases = [
        ("$x = 1", TokenKind::Assign),
        ("1 + 2", TokenKind::Plus),
        ("1 - 2", TokenKind::Minus),
        ("~1", TokenKind::BitwiseNot),
        ("$x += 1", TokenKind::PlusAssign),
        ("$x -= 1", TokenKind::MinusAssign),
        ("$x *= 1", TokenKind::StarAssign),
        ("$x /= 1", TokenKind::SlashAssign),
        ("$x %= 1", TokenKind::PercentAssign),
        ("$x .= 1", TokenKind::DotAssign),
        ("$x &= 1", TokenKind::AndAssign),
        ("$x |= 1", TokenKind::OrAssign),
        ("$x ^= 1", TokenKind::XorAssign),
        ("$x **= 1", TokenKind::PowerAssign),
        ("$x <<= 1", TokenKind::LeftShiftAssign),
        ("$x >>= 1", TokenKind::RightShiftAssign),
        ("$x &&= 1", TokenKind::LogicalAndAssign),
        ("$x ||= 1", TokenKind::LogicalOrAssign),
        ("$x //= 1", TokenKind::DefinedOrAssign),
        ("1 == 2", TokenKind::Equal),
        ("1 != 2", TokenKind::NotEqual),
        ("$x =~ /a/", TokenKind::Match),
        ("$x !~ /a/", TokenKind::NotMatch),
        ("$x ~~ @y", TokenKind::SmartMatch),
        ("1 < 2", TokenKind::Less),
        ("2 > 1", TokenKind::Greater),
        ("1 <= 2", TokenKind::LessEqual),
        ("2 >= 1", TokenKind::GreaterEqual),
        ("1 <=> 2", TokenKind::Spaceship),
        ("1 && 2", TokenKind::And),
        ("1 || 2", TokenKind::Or),
        ("!$x", TokenKind::Not),
        ("$x // $y", TokenKind::DefinedOr),
        ("$x -> method", TokenKind::Arrow),
        ("foo => 1", TokenKind::FatArrow),
        ("$x . $y", TokenKind::Dot),
        ("1 .. 2", TokenKind::Range),
        ("1 ... 2", TokenKind::Ellipsis),
        ("$x++", TokenKind::Increment),
        ("$x--", TokenKind::Decrement),
        ("$x ? 1 : 2", TokenKind::Question),
        ("$x ? 1 : 2", TokenKind::Colon),
        ("\\$x", TokenKind::Backslash),
    ];

    for (input, expected) in cases {
        assert!(
            kinds(input).contains(&expected),
            "operator mapping failed for {input} -> {expected:?}"
        );
    }

    assert_eq!(second_kind("1 * 2"), TokenKind::Star);
    assert_eq!(second_kind("1 / 2"), TokenKind::Slash);
    assert_eq!(second_kind("1 % 2"), TokenKind::Percent);
    assert_eq!(second_kind("1 ** 2"), TokenKind::Power);
    assert_eq!(second_kind("1 << 2"), TokenKind::LeftShift);
    assert_eq!(second_kind("1 >> 2"), TokenKind::RightShift);
    assert_eq!(second_kind("1 & 2"), TokenKind::BitwiseAnd);
    assert_eq!(second_kind("1 | 2"), TokenKind::BitwiseOr);
    assert_eq!(second_kind("1 ^ 2"), TokenKind::BitwiseXor);
    assert_eq!(second_kind("1 cmp 2"), TokenKind::StringCompare);
    assert_eq!(second_kind("1 and 2"), TokenKind::WordAnd);
    assert_eq!(second_kind("1 or 2"), TokenKind::WordOr);
    assert_eq!(second_kind("1 not 2"), TokenKind::WordNot);
    assert_eq!(second_kind("1 xor 2"), TokenKind::WordXor);
}

#[test]
fn token_kind_delimiters_quote_like_tokens_and_markers_map_consistently() {
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
    for (lexeme, expected) in delimiters {
        assert_eq!(first_kind(lexeme), expected, "delimiter mapping failed for {lexeme}");
    }

    assert_eq!(first_kind("q/a/"), TokenKind::QuoteSingle);
    assert_eq!(first_kind("qq/a/"), TokenKind::QuoteDouble);
    assert_eq!(first_kind("qw(a b)"), TokenKind::QuoteWords);
    assert_eq!(first_kind("qx/echo/"), TokenKind::QuoteCommand);
    assert_eq!(first_kind("qr/a/"), TokenKind::Regex);
    assert_eq!(first_kind("m/a/"), TokenKind::Regex);
    assert_eq!(first_kind("s/a/b/"), TokenKind::Substitution);
    assert_eq!(first_kind("tr/a/b/"), TokenKind::Transliteration);
    assert_eq!(first_kind("y/a/b/"), TokenKind::Transliteration);

    let heredoc = kinds("my $x = <<EOF;\nbody\nEOF\n");
    assert!(heredoc.contains(&TokenKind::HeredocStart));

    let data = kinds("__DATA__\nbody\n");
    assert_eq!(data[0], TokenKind::DataMarker);
    assert!(data.contains(&TokenKind::DataBody));
    assert_eq!(*data.last().unwrap_or(&TokenKind::Unknown), TokenKind::Eof);
}

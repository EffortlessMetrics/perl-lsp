//! Edge case tests for Unicode and encoding edge cases

pub fn get_tests() -> Vec<(&'static str, &'static str)> {
    vec![
        // Unicode identifiers
        ("my $café = 'coffee'", "unicode identifier café"),
        ("my $π = 3.14159", "unicode identifier pi"),
        ("my $Σ = 0", "unicode identifier sigma"),
        ("my $αβγ = 'greek'", "unicode identifier greek"),
        ("my $привет = 'hello'", "unicode identifier cyrillic"),
        ("my $你好 = 'hello'", "unicode identifier chinese"),
        ("my $مرحبا = 'hello'", "unicode identifier arabic"),
        ("my $שלום = 'hello'", "unicode identifier hebrew"),
        ("my $日本語 = 'japanese'", "unicode identifier japanese"),
        ("my $한글 = 'korean'", "unicode identifier korean"),
        // Unicode in different contexts
        ("sub café { }", "unicode sub name"),
        ("package Café", "unicode package name"),
        ("Café->new", "unicode class name"),
        ("$obj->café", "unicode method name"),
        ("use Café", "unicode module name"),
        // Unicode operators and delimiters
        ("my $x = $y ∘ $z", "unicode compose operator"),
        ("my $x = $y × $z", "unicode times"),
        ("my $x = $y ÷ $z", "unicode divide"),
        ("my $x = $y ≤ $z", "unicode less equal"),
        ("my $x = $y ≥ $z", "unicode greater equal"),
        ("my $x = $y ≠ $z", "unicode not equal"),
        // Unicode in strings
        (r#""Hello 世界""#, "unicode in double quotes"),
        (r#"'Hello 世界'"#, "unicode in single quotes"),
        (r#"q{Hello 世界}"#, "unicode in q{}"),
        (r#"qq{Hello 世界}"#, "unicode in qq{}"),
        (r#""café \x{E9}""#, "unicode escape in string"),
        (r#""\N{LATIN SMALL LETTER E WITH ACUTE}""#, "unicode name in string"),
        (r#""\N{U+00E9}""#, "unicode code point in string"),
        // Unicode in regex
        ("/café/", "unicode in regex"),
        ("m/世界/", "unicode in match"),
        ("s/café/coffee/", "unicode in substitution"),
        ("/\\p{Letter}/", "unicode property"),
        ("/\\p{L}/", "unicode property short"),
        ("/\\P{Letter}/", "unicode property negated"),
        ("/\\p{Script=Greek}/", "unicode script property"),
        ("/\\p{Block=Latin-1}/", "unicode block property"),
        ("/\\X/", "unicode extended grapheme"),
        ("/\\N{SNOWMAN}/", "unicode name in regex"),
        // Unicode categories
        ("/\\p{Uppercase}/", "unicode uppercase"),
        ("/\\p{Lowercase}/", "unicode lowercase"),
        ("/\\p{Digit}/", "unicode digit"),
        ("/\\p{Space}/", "unicode space"),
        ("/\\p{Punctuation}/", "unicode punctuation"),
        ("/\\p{Symbol}/", "unicode symbol"),
        ("/\\p{Mark}/", "unicode mark"),
        // Encoding pragmas
        ("use utf8", "utf8 pragma"),
        ("no utf8", "no utf8 pragma"),
        ("use encoding 'utf8'", "encoding utf8"),
        ("use encoding 'latin1'", "encoding latin1"),
        ("use encoding 'cp1252'", "encoding cp1252"),
        ("use encoding 'shift_jis'", "encoding shift_jis"),
        ("no encoding", "no encoding"),
        // Byte order marks
        (r#"\x{FEFF}use utf8"#, "BOM before code"),
        (r#"use utf8;\x{FEFF}"#, "BOM after pragma"),
        // Unicode filehandles
        ("open my $fh, '<:utf8', 'file.txt'", "utf8 input layer"),
        ("open my $fh, '>:utf8', 'file.txt'", "utf8 output layer"),
        ("open my $fh, '<:encoding(UTF-8)', 'file.txt'", "encoding layer"),
        ("binmode STDOUT, ':utf8'", "binmode utf8"),
        ("binmode $fh, ':encoding(UTF-8)'", "binmode encoding"),
        // Unicode and bytes
        ("use bytes; length($str)", "bytes pragma length"),
        ("no bytes; length($str)", "no bytes length"),
        ("utf8::encode($str)", "utf8 encode"),
        ("utf8::decode($str)", "utf8 decode"),
        ("utf8::is_utf8($str)", "utf8 check"),
        ("utf8::valid($str)", "utf8 valid"),
        ("utf8::upgrade($str)", "utf8 upgrade"),
        ("utf8::downgrade($str)", "utf8 downgrade"),
        // Unicode normalization
        ("use Unicode::Normalize", "normalization module"),
        ("NFD($str)", "NFD normalization"),
        ("NFC($str)", "NFC normalization"),
        ("NFKD($str)", "NFKD normalization"),
        ("NFKC($str)", "NFKC normalization"),
        // Wide character issues
        ("print '\\x{1F600}'", "emoji in print"),
        ("warn '\\x{1F600}'", "emoji in warn"),
        // Charnames
        ("use charnames ':full'", "charnames full"),
        ("use charnames ':short'", "charnames short"),
        ("use charnames qw(:full :alias)", "charnames with alias"),
        ("charnames::viacode(0x1F600)", "charnames viacode"),
        ("charnames::vianame('SNOWMAN')", "charnames vianame"),
        // Unicode in heredocs
        (
            r#"<<'世界'
Hello World
世界"#,
            "unicode heredoc delimiter",
        ),
        // Unicode in formats
        (
            r#"format UNICODE =
@<<<<< @>>>>> 
$英語, $日本語
.
"#,
            "unicode in format",
        ),
        // Mixed encodings
        (r#"my $mixed = "ASCII " . "\x{1F600}" . " UTF-8""#, "mixed encoding concat"),
        // Unicode constants
        ("use constant π => 3.14159", "unicode constant name"),
        ("use constant CAFÉ => 'coffee'", "unicode constant uppercase"),
        // Unicode in attributes
        ("my $x :café", "unicode attribute"),
        ("sub foo :café { }", "unicode sub attribute"),
        // Unicode in globs
        ("*café", "unicode glob"),
        ("*{café}", "unicode in glob"),
        // Source filters with encoding
        ("use Filter::Util::Call", "filter module"),
        ("filter_add(sub { s/café/coffee/g; $_ })", "unicode in filter"),
        // Unicode in special variables
        ("local $café = 1", "local unicode var"),
        ("our $café = 1", "our unicode var"),
        ("state $café = 1", "state unicode var"),
        // Unicode method lookup
        ("->can('café')", "unicode in can"),
        ("->isa('Café')", "unicode in isa"),
        ("UNIVERSAL::can($obj, 'café')", "unicode in UNIVERSAL"),
        // Unicode in tie
        ("tie $café, 'Class'", "tie unicode var"),
        ("tied $café", "tied unicode var"),
        // Unicode in BEGIN blocks
        ("BEGIN { my $café = 1 }", "unicode in BEGIN"),
        ("CHECK { my $café = 1 }", "unicode in CHECK"),
        // Emoji edge cases
        ("my $🐪 = 'camel'", "emoji identifier"),
        ("sub 🐪 { }", "emoji sub name"),
        ("$obj->🐪", "emoji method"),
        // Grapheme clusters
        (r#"my $e\x{301} = 'e-acute'"#, "combining character"),
        (r#"/e\x{301}/"#, "combining in regex"),
        (r#"length('e\x{301}')"#, "length of combining"),
        // Right-to-left
        ("my $עברית = 'hebrew'", "RTL identifier"),
        ("my $العربية = 'arabic'", "RTL arabic identifier"),
        // Surrogate pairs
        (r#"\x{D800}\x{DC00}"#, "surrogate pair"),
        // Zero-width characters
        (r#"my $a\x{200B}b = 1"#, "zero-width space in identifier"),
        (r#"my $a\x{200C}b = 1"#, "zero-width non-joiner"),
        (r#"my $a\x{200D}b = 1"#, "zero-width joiner"),
        // Control characters in strings
        (r#""\x{0000}""#, "null in string"),
        (r#""\x{0001}""#, "control char in string"),
        // Full-width characters
        ("my $ｆｕｌｌｗｉｄｔｈ = 1", "fullwidth identifier"),
        // Case folding
        ("fc('ß')", "case fold German sharp s"),
        ("'ß' =~ /SS/i", "case insensitive unicode"),
    ]
}

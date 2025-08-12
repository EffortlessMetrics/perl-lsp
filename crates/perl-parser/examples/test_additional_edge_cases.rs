//! Additional edge case tests for Perl parser - going beyond the original 128
use perl_parser::Parser;

fn main() {
    let tests = vec![
        // Complex heredoc edge cases
        ("my @x = (<<EOF, 'other');\ntext\nEOF", "heredoc in list"),
        (
            "print <<'EOF' . 'suffix';\ntext\nEOF",
            "heredoc with concatenation",
        ),
        (
            "func(<<EOF, <<'END');\nfirst\nEOF\nsecond\nEND",
            "multiple heredocs in call",
        ),
        ("$hash{<<EOF};\nkey\nEOF", "heredoc as hash key"),
        // Complex interpolation edge cases
        ("\"${\\$x}\"", "reference in interpolation"),
        ("\"@{[1, 2, 3]}\"", "anonymous array in interpolation"),
        ("\"$hash->{key}->[0]\"", "deep deref in interpolation"),
        ("qq{$obj->method()}", "method call in qq string"),
        ("\"$$ref\"", "double sigil in string"),
        // Special variable edge cases
        ("$#{$array_ref}", "last index of array ref"),
        ("@{^CAPTURE}", "special array with caret"),
        ("${^MATCH}", "special scalar with caret"),
        ("$::{foo}", "stash access"),
        ("*{$glob}{HASH}", "glob slot access"),
        // Complex package/namespace cases
        ("package Foo::Bar::Baz 1.23;", "package with version"),
        ("package Foo { }", "package block syntax"),
        ("::Foo::bar()", "absolute package path"),
        ("'Foo'->bar", "quoted class name"),
        ("\\&Foo::bar", "reference to qualified sub"),
        // Exotic operator combinations
        ("$x =~ s/a/b/r", "non-destructive substitution"),
        ("$x =~ tr/a-z/A-Z/r", "non-destructive transliteration"),
        ("$a <=> $b || $c cmp $d", "spaceship and cmp"),
        ("~~@array", "smartmatch with precedence"),
        ("$x ... $y", "range operator (not flip-flop)"),
        // Complex regex patterns
        ("m{(?<name>\\w+)}g", "named capture group"),
        ("/\\p{Letter}/", "unicode property"),
        ("/\\N{LATIN SMALL LETTER A}/", "named unicode char"),
        ("s/\\K\\w+//", "\\K in substitution"),
        ("/(?{code})/", "embedded code in regex"),
        // Signature edge cases
        ("sub foo ($x = 1, $y = 2) { }", "defaults in signature"),
        ("sub bar ($x, $y, @rest) { }", "slurpy in signature"),
        ("sub baz ($x, $y, %opts) { }", "hash slurpy in signature"),
        ("sub qux :prototype($) ($x) { }", "prototype with signature"),
        // Complex filehandle operations
        ("open my $fh, '<:utf8', 'file'", "open with encoding"),
        (
            "binmode STDOUT, ':encoding(UTF-8)'",
            "binmode with encoding",
        ),
        ("<STDIN>", "readline from STDIN"),
        ("print {$fh} 'text'", "print with block filehandle"),
        // Loop edge cases
        ("for ($i = 0; $i < 10; $i++) { }", "C-style for loop"),
        ("for (; $x; ) { }", "for with missing parts"),
        ("foreach $_ (@list) { }", "explicit $_ in foreach"),
        ("while (my $line = <>) { }", "assignment in while"),
        // Typeglob operations
        ("*foo = \\&bar", "assign coderef to glob"),
        ("*{$pkg . '::foo'} = \\&bar", "dynamic glob assignment"),
        ("local *STDOUT", "local typeglob"),
        ("*foo{HASH}", "access hash slot of glob"),
        // Eval edge cases
        ("eval { die }", "eval block"),
        ("eval 'code'", "eval string"),
        ("do 'file.pl'", "do file"),
        ("require 5.010", "require version"),
        // Special blocks in unusual places
        ("sub foo { BEGIN { } }", "BEGIN in sub"),
        ("if (1) { END { } }", "END in if block"),
        ("{ INIT { } }", "INIT in bare block"),
        // Tied variable operations
        ("tie my @array, 'Class'", "tie array"),
        ("tied(%hash)->method", "tied hash method call"),
        ("untie $scalar", "untie variable"),
        // Complex list operations
        ("@list[0..$#list]", "full array slice"),
        ("@array[@indices]", "slice with array"),
        ("(LIST)[0, 2, 4]", "list slice"),
        (
            "keys %{{ map { $_ => 1 } @list }}",
            "complex keys operation",
        ),
        // More postfix dereference cases
        ("$ref->@[0..2]", "postfix slice"),
        ("$ref->%{qw(a b)}", "postfix hash slice"),
        ("$ref->**", "postfix glob deref"),
        // Nested quoting constructs
        ("qq{foo {bar} baz}", "nested braces in qq"),
        ("qr(a(b)c)", "nested parens in qr"),
        ("s{a{b}c}{x{y}z}g", "nested braces in s///"),
        // Special assignment forms
        ("($x, $y) = ($y, $x)", "list assignment swap"),
        ("($a, undef, $b) = @list", "undef in list assignment"),
        ("my ($x, $y) = @_;", "my with list assignment"),
        // Exotic function calls
        ("print", "print without args"),
        ("sort { $b cmp $a } @list", "sort with block"),
        ("map { $_ * 2 } grep { $_ > 0 } @nums", "chained map/grep"),
        ("do { local $/; <$fh> }", "slurp mode"),
    ];

    let mut passed = 0;
    let mut failed = 0;

    println!("Running {} additional edge case tests...\n", tests.len());

    for (code, description) in tests {
        let mut parser = Parser::new(code);
        match parser.parse() {
            Ok(_) => {
                println!("✅ {:<50} {}", description, code);
                passed += 1;
            }
            Err(e) => {
                println!("❌ {:<50} {}", description, code);
                println!("   Error: {:?}", e);
                failed += 1;
            }
        }
    }

    println!(
        "\nAdditional Tests Summary: {} passed, {} failed",
        passed, failed
    );

    if failed > 0 {
        std::process::exit(1);
    }
}

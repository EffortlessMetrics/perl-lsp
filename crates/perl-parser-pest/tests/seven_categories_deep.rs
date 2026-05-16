//! Verify the v2 (Pest) parse depth by inspecting the produced S-expression
//! and testing against deliberate garbage. If garbage also "parses", the
//! all-pass result is recovery, not real coverage.

use perl_parser_pest::PureRustPerlParser;

fn show(label: &str, source: &str) {
    let mut parser = PureRustPerlParser::new();
    match parser.parse(source) {
        Ok(ast) => {
            let sexp = parser.to_sexp(&ast);
            let one_line: String = sexp.chars().take(280).collect();
            println!("  OK   [{}]\n         sexp: {}", label, one_line);
        }
        Err(e) => {
            let msg = format!("{}", e);
            let short = msg.lines().next().unwrap_or("").chars().take(160).collect::<String>();
            println!("  ERR  [{}] :: {}", label, short);
        }
    }
}

#[test]
fn deep_inspect_known_hard_cases() {
    println!("\n=== Deep inspection of hard cases ===");
    show("multi_heredoc",
        "print <<A, <<B;\naaa\nA\nbbb\nB\n");
    show("map_hashlike",
        "my @x = map { a => $_ } @list;");
    show("format_block",
        "format STDOUT =\n@<<<<<<<\n$name\n.\n");
    show("indirect_new",
        "my $obj = new Foo('arg');");
    show("dollar_caret_match",
        "print ${^MATCH};");
    show("s_mixed_delim",
        "my $s = 'abc'; $s =~ s{foo}/bar/g;");
}

#[test]
fn deliberate_garbage() {
    println!("\n=== Deliberate garbage (these SHOULD fail) ===");
    show("pure_garbage",
        "@@@ this is not perl at all $$$ <<<");
    show("unclosed_sub",
        "sub broken { my $x = ");
    show("random_punctuation",
        "} ) ] ; => => => ;;");
    show("not_perl_keywords",
        "function foo() { return; }");  // JS-style
    show("invalid_sigil",
        "my @@x = 5;");
    show("emoji_program",
        "💀 my $x = 🦀 ;");
}

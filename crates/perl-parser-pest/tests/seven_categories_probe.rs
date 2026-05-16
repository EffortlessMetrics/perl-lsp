//! Probe the v2 (Pest) parser against the seven categories that defeat
//! the v1 (C tree-sitter) parser. Each test reports parse success/failure
//! rather than asserting; we want to see the actual capability surface.

use perl_parser_pest::PureRustPerlParser;

fn probe(label: &str, source: &str) -> bool {
    let mut parser = PureRustPerlParser::new();
    let result = parser.parse(source);
    match &result {
        Ok(_) => {
            println!("  PASS  [{}]", label);
            true
        }
        Err(e) => {
            let msg = format!("{}", e);
            let short = msg.lines().next().unwrap_or("").chars().take(120).collect::<String>();
            println!("  FAIL  [{}] :: {}", label, short);
            false
        }
    }
}

#[test]
fn category_0_control() {
    println!("\n=== Category 0: Control (should all pass) ===");
    probe("simple_assign", "my $x = 5;");
    probe("sub_decl",      "sub f { return 1; }");
    probe("if_stmt",       "if ($x) { print $x; }");
}

#[test]
fn category_1_slash_disambiguation() {
    println!("\n=== Category 1: / regex vs division ===");
    probe("division",        "my $avg = $sum / $count;");
    probe("regex_after_if",  "if (/error/) { die; }");
    probe("div_assign",      "my $x = 10; $x /= 2;");
    probe("regex_match",     "my @m = /pattern/;");
    probe("regex_in_print",  "print /pat/;");
    probe("nested_div",      "my $r = ($a / $b) / ($c / $d);");
}

#[test]
fn category_2_heredoc_deferral() {
    println!("\n=== Category 2: heredoc body deferral ===");
    probe("basic_heredoc",
        "my $x = <<EOF;\nhello\nEOF\nmy $y = 5;\n");
    probe("multiple_per_line",
        "print <<A, <<B;\naaa\nA\nbbb\nB\n");
    probe("indented_heredoc",
        "my $x = <<~EOF;\n  indented\n  EOF\n");
    probe("noninterp_heredoc",
        "my $x = <<'EOF';\n$not_a_var\nEOF\n");
    probe("heredoc_in_expr",
        "my $x = (<<END) . \"suffix\";\nbody\nEND\n");
}

#[test]
fn category_3_brace_ambiguity() {
    println!("\n=== Category 3: {{}} hash vs block vs map block ===");
    probe("hashref_construct", "my $h = { a => 1, b => 2 };");
    probe("map_block",         "my @x = map { $_ * 2 } @list;");
    probe("map_hashlike_block",
        "my @x = map { a => $_ } @list;");
    probe("eval_block",        "eval { die 'x'; };");
    probe("grep_block",        "my @x = grep { $_ > 0 } @list;");
    probe("sort_block",        "my @x = sort { $a <=> $b } @list;");
    probe("nested_braces",
        "my $f = sub { { inner => sub { 42 } } };");
}

#[test]
fn category_4_quote_like_operators() {
    println!("\n=== Category 4: quote-like operators ===");
    probe("q_braces",          "my $x = q{hello world};");
    probe("qq_parens",         "my $x = qq(hello $name);");
    probe("qw_brackets",       "my @a = qw[one two three];");
    probe("qr_slashes",        "my $re = qr/pattern/i;");
    probe("s_paired_braces",
        "my $s = 'abc'; $s =~ s{a}{X};");
    probe("s_pipe_delim",
        "my $s = 'abc'; $s =~ s|a|X|g;");
    probe("tr_brackets",
        "my $s = 'abc'; $s =~ tr[a-c][A-C];");
    probe("s_mixed_delim",
        "my $s = 'abc'; $s =~ s{foo}/bar/g;");
}

#[test]
fn category_5_special_variables() {
    println!("\n=== Category 5: punctuation special variables ===");
    probe("dollar_slash",     "local $/ = undef;");
    probe("dollar_dollar",    "print $$;");
    probe("dollar_bang",      "die $!;");
    probe("dollar_at",        "warn $@;");
    probe("dollar_caret_W",   "$^W = 1;");
    probe("dollar_caret_brace", "print ${^MATCH};");
    probe("dollar_underscore", "for (@x) { print $_; }");
    probe("dollar_amp",       "print $&;");
    probe("numbered_capture", "if ('abc' =~ /(.)/) { print $1; }");
}

#[test]
fn category_6_indirect_object() {
    println!("\n=== Category 6: indirect object syntax ===");
    probe("indirect_new",     "my $obj = new Foo();");
    probe("indirect_new_arg", "my $obj = new Foo('arg');");
    probe("print_filehandle", "print STDERR \"oops\\n\";");
    probe("printf_filehandle","printf STDERR \"%s\\n\", $msg;");
    probe("method_arrow",     "my $obj = Foo->new();");
}

#[test]
fn category_7_format_blocks() {
    println!("\n=== Category 7: format blocks ===");
    probe("simple_format",
        "format STDOUT =\n@<<<<<<<\n$name\n.\n");
    probe("multiline_format",
        "format STDOUT =\n@<<< @>>>\n$a, $b\n.\nmy $x = 1;\n");
    probe("format_top",
        "format STDOUT_TOP =\nName     Value\n.\n");
}

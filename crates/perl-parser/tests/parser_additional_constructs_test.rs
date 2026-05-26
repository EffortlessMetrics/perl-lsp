use perl_parser::Parser;

fn assert_clean_parse(code: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(code);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "expected clean parse without ERROR nodes for `{code}`, got: {sexp}"
    );
    Ok(())
}

#[test]
fn parses_phase_blocks_and_pragmas() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"BEGIN { $ENV{APP_MODE} = 'test'; }
INIT { setup_runtime(); }
CHECK { validate_config(); }
END { cleanup_runtime(); }
use strict;
no warnings 'experimental';"#,
    )
}

#[test]
fn parses_do_block_and_do_file_expressions() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"my $value = do { my $tmp = compute(); $tmp + 1 };
my $config = do './config.pl';"#,
    )
}

#[test]
fn parses_labeled_loop_control() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"OUTER: for my $row (@rows) {
    next OUTER if $row->{skip};
    last OUTER if $row->{done};
    redo OUTER if retry($row);
}"#,
    )
}

#[test]
fn parses_continue_block_after_while() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"while (my $line = <$fh>) {
    next if $line =~ /^#/;
    process($line);
} continue {
    $line_count++;
}"#,
    )
}

#[test]
fn parses_typeglob_aliasing_and_localization() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"local *STDOUT;
*alias = \&target;
my $glob = *main::handler;"#,
    )
}

#[test]
fn parses_continue_block_after_foreach() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"for my $item (@items) {
    process_item($item);
} continue {
    $seen{$item}++;
}"#,
    )
}

#[test]
fn parses_eval_block_and_eval_string() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"my $ok = eval {
    risky_operation();
    1;
};
my $result = eval '$x + 41';
die $@ if $@;"#,
    )
}

#[test]
fn parses_map_and_grep_with_block_bodies() -> Result<(), Box<dyn std::error::Error>> {
    assert_clean_parse(
        r#"my @normalized = map {
    lc $_;
} grep {
    defined $_ && $_ ne '';
} @raw_values;"#,
    )
}

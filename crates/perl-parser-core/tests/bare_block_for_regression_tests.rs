use perl_parser_core::Parser;

fn parse_clean(src: &str) -> Result<(), String> {
    let mut parser = Parser::new(src);
    let ast = parser.parse().map_err(|e| format!("parse error: {e:?}"))?;
    let sexp = ast.to_sexp();
    if sexp.contains("ERROR") {
        return Err(format!(
            "expected clean parse, got ERROR nodes in: {sexp}\nsource: {src}"
        ));
    }
    Ok(())
}

#[test]
fn bare_block_then_for_my_var() -> Result<(), String> {
    parse_clean("{ my $x = 1; }\nfor my $i (1..3) { print $i; }\n")
}

#[test]
fn bare_block_then_foreach_my_var() -> Result<(), String> {
    parse_clean("{ my $y = 2; }\nforeach my $item (@arr) { print $item; }\n")
}

#[test]
fn bare_block_then_for_my_alias() -> Result<(), String> {
    // Real-world pattern from List::SomeUtils / Module::Implementation
    parse_clean(
        r#"
{
    my $loader = build_loader_sub(
        implementations => [ 'XS', 'PP' ],
        symbols         => \@subs,
    );
    $loader->();
}

for my $alias ( keys %aliases ) {
    no strict 'refs';
    *{$alias} = __PACKAGE__->can( $aliases{$alias} );
}
"#,
    )
}

#[test]
fn if_block_then_for_still_works() -> Result<(), String> {
    // if/while/etc. blocks were already compound — should be unaffected
    parse_clean("if (1) { my $x = 1; }\nfor my $i (1..3) { print $i; }\n")
}

#[test]
fn nested_bare_blocks_then_for() -> Result<(), String> {
    parse_clean("{ { my $x = 1; } }\nfor my $k (keys %h) { print $k; }\n")
}

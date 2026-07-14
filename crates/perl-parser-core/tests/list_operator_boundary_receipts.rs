mod cpan_test_helpers;

use cpan_test_helpers::{assert_clean_parse, parse};
use perl_parser_core::{Node, NodeKind};

fn program_statement(source: &str, context: &str) -> Result<Node, String> {
    let ast = parse(source);
    let NodeKind::Program { statements } = ast.kind else {
        return Err(format!("expected Program node for {context}, got {:?}", ast.kind));
    };
    statements.into_iter().next().ok_or_else(|| format!("expected first statement for {context}"))
}

fn declaration_initializer<'a>(statement: &'a Node, context: &str) -> Result<&'a Node, String> {
    let NodeKind::VariableDeclaration { initializer, .. } = &statement.kind else {
        return Err(format!(
            "expected VariableDeclaration for {context}, got {:?}",
            statement.kind
        ));
    };
    initializer.as_deref().ok_or_else(|| format!("expected initializer for {context}"))
}

fn list_declaration_initializer<'a>(
    statement: &'a Node,
    context: &str,
) -> Result<&'a Node, String> {
    let NodeKind::VariableListDeclaration { initializer, .. } = &statement.kind else {
        return Err(format!(
            "expected VariableListDeclaration for {context}, got {:?}",
            statement.kind
        ));
    };
    initializer.as_deref().ok_or_else(|| format!("expected initializer for {context}"))
}

fn function_call<'a>(
    node: &'a Node,
    expected_name: &str,
    context: &str,
) -> Result<&'a [Node], String> {
    let NodeKind::FunctionCall { name, args } = &node.kind else {
        return Err(format!(
            "expected FunctionCall({expected_name}) for {context}, got {:?}",
            node.kind
        ));
    };
    if name != expected_name {
        return Err(format!("expected FunctionCall({expected_name}) for {context}, got {name}"));
    }
    Ok(args)
}

fn arg<'a>(args: &'a [Node], index: usize, context: &str) -> Result<&'a Node, String> {
    args.get(index).ok_or_else(|| format!("expected arg {index} for {context}"))
}

#[test]
fn map_block_pipeline_keeps_grep_and_keys_inside_map() -> Result<(), String> {
    let source = r#"my %dbd_class_registry = map { $dbd_prefix_registry->{$_}->{class} => { prefix => $_ } } grep { exists $dbd_prefix_registry->{$_}->{class} } keys %{$dbd_prefix_registry};"#;
    assert_clean_parse(source);

    let statement = program_statement(source, "DBI map/grep/keys boundary")?;
    let initializer = declaration_initializer(&statement, "DBI map/grep/keys boundary")?;

    let map_args = function_call(initializer, "map", "DBI map boundary")?;
    assert_eq!(map_args.len(), 2, "map should contain block and grep source");
    assert!(matches!(arg(map_args, 0, "map block")?.kind, NodeKind::Block { .. }));

    let grep_args = function_call(arg(map_args, 1, "map source")?, "grep", "grep source boundary")?;
    assert_eq!(grep_args.len(), 2, "grep should contain block and keys source");
    assert!(matches!(arg(grep_args, 0, "grep block")?.kind, NodeKind::Block { .. }));
    function_call(arg(grep_args, 1, "grep source")?, "keys", "keys source boundary")?;

    Ok(())
}

#[test]
fn map_quote_like_expression_keeps_sort_source_inside_map() -> Result<(), String> {
    let source = r#"my $attrs = join " ", map { qq[$_="$attrs{$_}"] } sort keys %attrs;"#;
    assert_clean_parse(source);

    let statement = program_statement(source, "ExtUtils attrs map/sort boundary")?;
    let initializer = declaration_initializer(&statement, "ExtUtils attrs map/sort boundary")?;

    let join_args = function_call(initializer, "join", "join boundary")?;
    assert_eq!(join_args.len(), 2, "join should contain separator and map result");

    let map_args = function_call(arg(join_args, 1, "join map arg")?, "map", "map boundary")?;
    assert_eq!(map_args.len(), 2, "map should contain qq block and sort source");
    assert!(matches!(arg(map_args, 0, "map qq block")?.kind, NodeKind::Block { .. }));

    let sort_args = function_call(arg(map_args, 1, "map source")?, "sort", "sort source boundary")?;
    assert_eq!(sort_args.len(), 1, "sort should contain keys source");
    function_call(arg(sort_args, 0, "sort source")?, "keys", "keys source boundary")?;

    Ok(())
}

#[test]
fn map_sort_source_stops_before_ternary_colon() -> Result<(), String> {
    let source = r#"my @params = ref $data eq 'HASH' ? map { ($_ => $data->{$_}) } sort keys %$data : @$data;"#;
    assert_clean_parse(source);

    let statement = program_statement(source, "HTTP::Tiny map/sort ternary boundary")?;
    let initializer = declaration_initializer(&statement, "HTTP::Tiny map/sort ternary boundary")?;

    let NodeKind::Ternary { then_expr, .. } = &initializer.kind else {
        return Err(format!(
            "expected ternary initializer for HTTP::Tiny map/sort boundary, got {:?}",
            initializer.kind
        ));
    };

    let map_args = function_call(then_expr, "map", "HTTP::Tiny ternary then map boundary")?;
    assert_eq!(map_args.len(), 2, "map should contain block and sort source");

    let sort_args = function_call(arg(map_args, 1, "map source")?, "sort", "sort source boundary")?;
    assert_eq!(sort_args.len(), 1, "sort should contain keys source before ternary colon");
    function_call(arg(sort_args, 0, "sort source")?, "keys", "keys source boundary")?;

    Ok(())
}

#[test]
fn map_block_source_stays_inside_list_declaration_initializer() -> Result<(), String> {
    let source = r#"my ($fh, $pos) = map { $stash->{$_}{$name} } qw/capture pos/;"#;
    assert_clean_parse(source);

    let statement = program_statement(source, "Capture::Tiny map list declaration boundary")?;
    let initializer =
        list_declaration_initializer(&statement, "Capture::Tiny map list declaration boundary")?;

    let map_args = function_call(initializer, "map", "Capture::Tiny map boundary")?;
    assert_eq!(map_args.len(), 2, "map should contain block and qw source");
    assert!(matches!(arg(map_args, 0, "map block")?.kind, NodeKind::Block { .. }));
    assert!(matches!(arg(map_args, 1, "map source")?.kind, NodeKind::ArrayLiteral { .. }));

    Ok(())
}

#[test]
fn map_expression_keeps_split_source_inside_map() -> Result<(), String> {
    let source = r#"my $curHST = join '', map getHST($_, $vers), split /;/, $jcps;"#;
    assert_clean_parse(source);

    let statement = program_statement(source, "Unicode Collate map/split boundary")?;
    let initializer = declaration_initializer(&statement, "Unicode Collate map/split boundary")?;

    let join_args = function_call(initializer, "join", "join boundary")?;
    assert_eq!(join_args.len(), 2, "join should contain separator and map result");

    let map_args = function_call(arg(join_args, 1, "join map arg")?, "map", "map boundary")?;
    assert_eq!(map_args.len(), 2, "map should contain getHST expression and split source");
    function_call(arg(map_args, 0, "map expression")?, "getHST", "map expression boundary")?;

    let split_args =
        function_call(arg(map_args, 1, "map source")?, "split", "split source boundary")?;
    assert_eq!(split_args.len(), 2, "split should contain regex and source scalar");

    Ok(())
}

#[test]
fn parenthesized_bare_call_accepts_numeric_arg_expression() {
    assert_clean_parse(r#"print "not " unless (unilist 0 || 5) == 6;"#);
}

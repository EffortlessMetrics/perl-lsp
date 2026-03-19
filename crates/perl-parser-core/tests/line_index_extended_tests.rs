use perl_parser_core::{Parser, line_index::LineIndex};

#[test]
fn line_index_crlf_input() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\r\nline2\r\nline3".to_string();
    let index = LineIndex::new(source);
    let (line, _col) = index.offset_to_position(7); // start of "line2"
    assert_eq!(line, 1);
    Ok(())
}

#[test]
fn line_index_unicode() -> Result<(), Box<dyn std::error::Error>> {
    let source = "héllo\nwörld".to_string();
    let index = LineIndex::new(source);
    let (line, _col) = index.offset_to_position(0);
    assert_eq!(line, 0);
    // 'h' + 'é'(2 bytes) + 'l' + 'l' + 'o' + '\n' = 7 bytes for first line
    let (line2, _col2) = index.offset_to_position(7);
    assert_eq!(line2, 1);
    Ok(())
}

#[test]
fn line_index_trailing_newline() -> Result<(), Box<dyn std::error::Error>> {
    let source = "line1\nline2\n".to_string();
    let index = LineIndex::new(source);
    // Past the last newline
    let (line, _col) = index.offset_to_position(12);
    assert_eq!(line, 2);
    Ok(())
}

// ---- Wave 2B: Fat arrow as general separator ----

#[test]
fn wave2b_push_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("push @array => $value;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "push @array => $value should parse cleanly, got: {sexp}");
    assert!(sexp.contains("call push"), "should be a function call");
    Ok(())
}

#[test]
fn wave2b_bless_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("bless \\%opts => $class;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "bless \\%opts => $class should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2b_push_fat_arrow_nested() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("push @attrs => (key => $val);");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "push @attrs => (key => $val) should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_push_comma_regression() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("push @array, $value;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "push @array, $value should still work, got: {sexp}");
    assert!(sexp.contains("call push"), "should be a function call");
    Ok(())
}

#[test]
fn wave2b_indirect_call_regression() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("print $fh \"data\";");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "print $fh \"data\" should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2b_hash_fat_arrow_regression() -> Result<(), Box<dyn std::error::Error>> {
    // Hash construction should still work
    let mut parser = Parser::new("my %h = (key => 'value');");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "hash construction should still work, got: {sexp}");
    Ok(())
}

#[test]
fn wave2b_unshift_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("unshift @arr => $val;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "unshift @arr => $val should parse cleanly, got: {sexp}");
    assert!(sexp.contains("call unshift"), "should be a function call");
    Ok(())
}

#[test]
fn wave2b_splice_mixed_comma_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    // `splice @a, 0, 1 => @replacement` — the `=>` after `1` is consumed by
    // the builtin argument loop as a separator, exactly like a comma.
    let mut parser = Parser::new("splice @a, 0, 1 => @replacement;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "splice @a, 0, 1 => @replacement should parse cleanly, got: {sexp}"
    );
    // splice uses the generic builtin path; the sexp uses ambiguous_function_call_expression
    assert!(
        sexp.contains("function_call_expression") || sexp.contains("call splice"),
        "should be a function call, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_tie_fat_arrow_in_args() -> Result<(), Box<dyn std::error::Error>> {
    // tie uses a dedicated AST handler; fat arrow must work in trailing args
    let mut parser = Parser::new("tie %hash, 'MyModule' => @args;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "tie %hash, 'MyModule' => @args should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_map_fat_arrow_separator() -> Result<(), Box<dyn std::error::Error>> {
    // map with block then fat arrow before list
    let mut parser = Parser::new("map { $_ * 2 } => @list;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "map {{ $_ * 2 }} => @list should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2b_grep_fat_arrow_separator() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("grep { defined } => @list;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "grep {{ defined }} => @list should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_sort_fat_arrow_separator() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("sort { $a <=> $b } => @list;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "sort {{ $a <=> $b }} => @list should parse cleanly, got: {sexp}"
    );
    Ok(())
}

// ---- Wave 2B-ext: Fat arrow in postfix builtin paths ----

#[test]
fn wave2b_bless_hash_literal_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    // bless {} => $class  —  exercises the bless-with-LeftBrace path in postfix.rs
    let mut parser = Parser::new("bless {} => $class;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "bless {{}} => $class should parse cleanly, got: {sexp}");
    assert!(sexp.contains("call bless"), "should be a bless call, got: {sexp}");
    Ok(())
}

#[test]
fn wave2b_bless_hash_with_entries_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    // bless { key => 1 } => $class
    let mut parser = Parser::new("bless { key => 1 } => $class;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "bless {{ key => 1 }} => $class should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_split_regex_fat_arrow() -> Result<(), Box<dyn std::error::Error>> {
    // split /,/ => @parts  —  exercises the split-with-Slash path in postfix.rs
    let mut parser = Parser::new("split /,/ => @parts;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "split /,/ => @parts should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2b_unshift_fat_arrow_multiple() -> Result<(), Box<dyn std::error::Error>> {
    // unshift @arr => 1, 2, 3  —  fat arrow then commas
    let mut parser = Parser::new("unshift @arr => 1, 2, 3;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "unshift @arr => 1, 2, 3 should parse cleanly, got: {sexp}");
    assert!(sexp.contains("call unshift"), "should be an unshift call, got: {sexp}");
    Ok(())
}

#[test]
fn wave2b_grep_block_fat_arrow_list() -> Result<(), Box<dyn std::error::Error>> {
    // grep { defined } => @list  —  exercises the sort/map/grep block path in postfix.rs
    // (when reached via postfix, e.g. inside an expression context)
    let mut parser = Parser::new("my @r = grep { defined } => @list;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "grep {{ defined }} => @list in assignment should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_bless_hash_fat_arrow_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
    // my $obj = bless {} => $class  —  in assignment, goes through postfix.rs bless path
    let mut parser = Parser::new("my $obj = bless {} => $class;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "bless {{}} => $class in assignment should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_split_regex_fat_arrow_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
    // my @parts = split /,/ => $str  —  in assignment, goes through postfix.rs split path
    let mut parser = Parser::new("my @parts = split /,/ => $str;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "split /,/ => $str in assignment should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_sort_block_fat_arrow_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
    // my @s = sort { $a <=> $b } => @list  —  in assignment, goes through postfix.rs
    let mut parser = Parser::new("my @s = sort { $a <=> $b } => @list;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "sort {{ $a <=> $b }} => @list in assignment should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2b_map_block_fat_arrow_in_assignment() -> Result<(), Box<dyn std::error::Error>> {
    // my @r = map { $_ * 2 } => @list
    let mut parser = Parser::new("my @r = map { $_ * 2 } => @list;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "map {{ $_ * 2 }} => @list in assignment should parse cleanly, got: {sexp}"
    );
    Ok(())
}

// ---- Wave 2C: split /regex/ ----

#[test]
fn wave2c_split_regex() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("split /\\./, $string;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "split /\\./, $string should parse cleanly, got: {sexp}");
    assert!(sexp.contains("regex"), "should contain a regex node");
    Ok(())
}

#[test]
fn wave2c_split_regex_whitespace() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("split /\\s+/, $cmd;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "split /\\s+/, $cmd should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("my @parts = split /::/, $module;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "my @parts = split /::/, $module should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2c_split_parens_regression() -> Result<(), Box<dyn std::error::Error>> {
    // Parenthesized form should still work
    let mut parser = Parser::new("split(/\\./, $x);");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "split(/\\./, $x) should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_string_regression() -> Result<(), Box<dyn std::error::Error>> {
    // split with string pattern should still work
    let mut parser = Parser::new("split ',', $csv;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "split with string should still work, got: {sexp}");
    Ok(())
}

// ---- Wave 2C+: split /regex/ in expression contexts (not just statement start) ----

#[test]
fn wave2c_split_regex_in_assignment_comma_pattern() -> Result<(), Box<dyn std::error::Error>> {
    // split with a single-char regex pattern containing comma
    let mut parser = Parser::new("my @p = split /,/, $s;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "my @p = split /,/, $s should parse cleanly, got: {sexp}");
    assert!(sexp.contains("regex"), "should contain a regex node, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_after_return() -> Result<(), Box<dyn std::error::Error>> {
    // return split /regex/, $var — split in expression context after return
    let mut parser = Parser::new("return split /\\s+/, $line;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "return split /\\s+/, $line should parse cleanly, got: {sexp}"
    );
    assert!(sexp.contains("regex"), "should contain a regex node, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_inside_push_args() -> Result<(), Box<dyn std::error::Error>> {
    // push @r, split /;/, $v — split as argument to another builtin
    let mut parser = Parser::new("push @r, split /;/, $v;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "push @r, split /;/, $v should parse cleanly, got: {sexp}");
    assert!(sexp.contains("regex"), "should contain a regex node, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_in_for_list() -> Result<(), Box<dyn std::error::Error>> {
    // split in for loop list context
    let mut parser = Parser::new("for my $x (split /,/, $s) { }");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "for my $x (split /,/, $s) should parse cleanly, got: {sexp}");
    assert!(sexp.contains("regex"), "should contain a regex node, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_in_ternary() -> Result<(), Box<dyn std::error::Error>> {
    // split in ternary expression
    let mut parser = Parser::new("my @r = $flag ? split(/,/, $a) : split(/;/, $b);");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "ternary with split should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_chained() -> Result<(), Box<dyn std::error::Error>> {
    // join of split — split as argument inside another function call
    let mut parser = Parser::new("my $x = join('-', split /\\s+/, $input);");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "join('-', split /\\s+/, $input) should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2c_split_regex_in_array_ref() -> Result<(), Box<dyn std::error::Error>> {
    // split result stored in an anonymous array ref
    let mut parser = Parser::new("my $r = [split /,/, $s];");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "[split /,/, $s] should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_conditional_or() -> Result<(), Box<dyn std::error::Error>> {
    // split in || expression
    let mut parser = Parser::new("my @r = split(/,/, $s) || die;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "split(/,/, $s) || die should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_three_args() -> Result<(), Box<dyn std::error::Error>> {
    // split with limit argument
    let mut parser = Parser::new("my @p = split /,/, $s, 3;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "split /,/, $s, 3 should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2c_split_regex_no_parens_method_chain() -> Result<(), Box<dyn std::error::Error>> {
    // using scalar result of split
    let mut parser = Parser::new("my $count = scalar(split /,/, $s);");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "scalar(split /,/, $s) should parse cleanly, got: {sexp}");
    Ok(())
}

// ---- Wave 2D: Postfix modifiers after complex expressions ----

#[test]
fn wave2d_push_deref_with_if() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("push @{$hash{key}}, $val if $cond;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "push @{{$hash{{key}}}}, $val if $cond should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2d_push_deref_simple() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("push @{$arr}, 1;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "push @{{$arr}}, 1 should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2d_or_assign_for() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("$hash{$_} ||= '' for @list;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "$hash{{$_}} ||= '' for @list should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2d_simple_modifier_regression() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("print $msg unless $quiet;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "print $msg unless $quiet should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2d_do_thing_for_list() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("do_thing() for @list;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "do_thing() for @list should parse cleanly, got: {sexp}");
    Ok(())
}

#[test]
fn wave2d_deref_hash_push() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("push @{$self->{items}}, $item;");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(
        !sexp.contains("ERROR"),
        "push @{{$self->{{items}}}}, $item should parse cleanly, got: {sexp}"
    );
    Ok(())
}

#[test]
fn wave2d_complex_lvalue_while() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new("chomp($line) while ($line = <STDIN>);");
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    assert!(!sexp.contains("ERROR"), "should parse cleanly, got: {sexp}");
    Ok(())
}

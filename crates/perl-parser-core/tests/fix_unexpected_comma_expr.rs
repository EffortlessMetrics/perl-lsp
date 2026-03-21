mod cpan_test_helpers;
use cpan_test_helpers::*;

// === Core comma patterns (regression tests) ===

#[test]
fn test_trailing_comma_in_list_assignment() {
    let source = r#"my @list = (1, 2, 3,);"#;
    assert_clean_parse(source);
}

#[test]
fn test_trailing_comma_in_hash_constructor() {
    let source = r#"my %hash = (a => 1, b => 2,);"#;
    assert_clean_parse(source);
}

#[test]
fn test_trailing_comma_in_function_call() {
    let source = r#"foo(1, 2, 3,);"#;
    assert_clean_parse(source);
}

#[test]
fn test_trailing_comma_in_anonymous_hash() {
    let source = r#"my $ref = { a => 1, b => 2, };"#;
    assert_clean_parse(source);
}

#[test]
fn test_trailing_comma_in_anonymous_array() {
    let source = r#"my $ref = [1, 2, 3,];"#;
    assert_clean_parse(source);
}

// === grep EXPR, LIST patterns (the #1 cause of comma errors) ===

#[test]
fn test_grep_defined_comma_list() {
    // This is the most common CPAN pattern causing comma errors
    // grep EXPR, LIST where EXPR is a named-unary builtin
    let source = r#"my @result = grep defined, @list;"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_not_defined_comma_list() {
    let source = r#"return if grep !defined, $fu, $fv;"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_ref_comma_list() {
    let source = r#"my @refs = grep ref, @items;"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_length_comma_list() {
    let source = r#"my @nonempty = grep length, @strings;"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_negated_regex_comma_list() {
    let source = r#"return grep !/gcc/ && -d, split /:/, $lib_line;"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_file_test_comma_list() {
    // grep -e, @INC  (file test -e on each element)
    let source = r#"my @existing = grep -e, @INC;"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_expression_form() {
    let source = r#"my @upper = map uc, @words;"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_complex_expr_form() {
    let source = r#"local $ENV{PERL5LIB} = join $sep, map abs_path($_), grep -e, @INC;"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_defined_in_function_call() {
    // join '', grep defined, @$body
    let source = r#"$res->content(join '', grep defined, @$body);"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_defined_with_hash_slice() {
    let source = r#"return if grep !defined, my ($i, $j) = @$m1{ $u, $v };"#;
    assert_clean_parse(source);
}

#[test]
fn test_for_grep_combo() {
    // for/foreach over a grep result
    let source = r#"$stab = $stab->{$_.'::'} for grep length, split /::/, $package;"#;
    assert_clean_parse(source);
}

#[test]
fn test_max_with_map_grep() {
    let source = r#"max($sofar, map { compute($_) } grep ref, @$item);"#;
    assert_clean_parse(source);
}

// === Double commas (found in real CPAN code) ===

#[test]
fn test_double_comma_in_function_args() {
    // Real CPAN: Imager::Font::FT2 has double commas
    let source = r#"func($a, $b, , $c, $d);"#;
    // Should parse without crashing (may have error nodes but no panic)
    parse(source);
}

#[test]
fn test_double_comma_trailing() {
    // Real CPAN: Throwable::Error has }),, pattern
    let source = r#"foo(sub { 1 },, );"#;
    parse(source);
}

#[test]
fn test_double_comma_in_hash() {
    let source = r#"my %h = (a => 1,, b => 2);"#;
    parse(source);
}

// === Comma operator (sequence) ===

#[test]
fn test_comma_as_sequence_operator() {
    // seek(...), return 'ascii'  — comma operator for side effects
    let source = r#"seek($fh, $loc, 0), return 'ascii';"#;
    assert_clean_parse(source);
}

// === no warnings with multiple args ===

#[test]
fn test_no_warnings_multiple() {
    let source = r#"no warnings 'once', 'redefine';"#;
    assert_clean_parse(source);
}

#[test]
fn test_no_warnings_numeric_uninitialized() {
    let source = r#"no warnings 'numeric', 'uninitialized';"#;
    assert_clean_parse(source);
}

// === CORE:: prefixed builtins ===

#[test]
fn test_core_select() {
    let source = r#"CORE::select undef, undef, undef, $wait if $wait;"#;
    assert_clean_parse(source);
}

#[test]
fn test_core_grep_with_block() {
    // CORE::grep with block argument should get block-list-func handling
    let source = r#"my @result = CORE::grep { defined $_ } @list;"#;
    assert_clean_parse(source);
}

#[test]
fn test_core_grep_with_regex() {
    // CORE::grep /regex/, @list should re-lex / as regex delimiter
    let source = r#"my @matches = CORE::grep /foo/, @list;"#;
    assert_clean_parse(source);
}

#[test]
fn test_core_sort_with_block() {
    let source = r#"my @sorted = CORE::sort { $a <=> $b } @list;"#;
    assert_clean_parse(source);
}

#[test]
fn test_core_map_with_block() {
    let source = r#"my @doubled = CORE::map { $_ * 2 } @list;"#;
    assert_clean_parse(source);
}

#[test]
fn test_core_print_with_filehandle() {
    let source = r#"CORE::print $fh "hello\n";"#;
    assert_clean_parse(source);
}

// === Complex real-world patterns ===

#[test]
fn test_return_list() {
    let source = r#"sub foo { return (1, 2, 3); }"#;
    assert_clean_parse(source);
}

#[test]
fn test_return_bare_list() {
    let source = r#"sub foo { return 1, 2, 3; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_hash_slice_at_sigil() {
    let source = r#"my @vals = @hash{$a, $b};"#;
    assert_clean_parse(source);
}

#[test]
fn test_array_slice() {
    let source = r#"my @selected = @array[0, 2, 4];"#;
    assert_clean_parse(source);
}

#[test]
fn test_print_comma_list() {
    let source = r#"print "foo", "bar", "baz";"#;
    assert_clean_parse(source);
}

#[test]
fn test_die_with_comma() {
    let source = r#"die "Error: ", $msg, "\n";"#;
    assert_clean_parse(source);
}

#[test]
fn test_splice_with_commas() {
    let source = r#"splice(@array, 0, 2, @replacement);"#;
    assert_clean_parse(source);
}

#[test]
fn test_complex_data_structure() {
    let source = r#"my $data = {
        users => [
            { name => "Alice", age => 30, },
            { name => "Bob", age => 25, },
        ],
        count => 2,
    };"#;
    assert_clean_parse(source);
}

#[test]
fn test_bless_with_complex_hash() {
    let source = r#"my $self = bless {
        name => $name,
        data => [],
        opts => { verbose => 0 },
    }, $class;"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_with_comma_expr() {
    let source = r#"my @pairs = map { ($_, $_) } @list;"#;
    assert_clean_parse(source);
}

#[test]
fn test_sort_with_custom_comparison() {
    let source = r#"my @sorted = sort { $a->{name} cmp $b->{name} } @items;"#;
    assert_clean_parse(source);
}

#[test]
fn test_for_c_style() {
    let source = r#"for (my $i = 0; $i < 10; $i++) { print $i; }"#;
    assert_clean_parse(source);
}

#[test]
fn test_comma_operator_in_for_init() {
    let source = r#"for ($i = 0, $j = 10; $i < $j; $i++, $j--) { }"#;
    assert_clean_parse(source);
}

#[test]
fn test_anonymous_sub_in_hash() {
    let source = r#"my %dispatch = (
        add => sub { $_[0] + $_[1] },
        sub => sub { $_[0] - $_[1] },
    );"#;
    assert_clean_parse(source);
}

#[test]
fn test_complex_method_chain_with_hash_args() {
    let source = r#"$self->method({
        key1 => $val1,
        key2 => $val2,
    });"#;
    assert_clean_parse(source);
}

// === Patterns from specific CPAN modules that hit comma errors ===

#[test]
fn test_text_trim_pattern() {
    // From Text::Trim
    let source = r#"sub trim {
    @_ = @_ ? @_ : $_ if defined wantarray;
    for (@_ ? @_ : $_) { next unless defined; s/\A\s+//; s/\s+\z// }
    return @_ if wantarray || !defined wantarray;
    if (my @def = grep defined, @_) { return "@def" } else { return }
}"#;
    assert_clean_parse(source);
}

#[test]
fn test_graph_union_find_pattern() {
    // From Graph::UnionFind
    let source = r#"sub same {
    my ($uf, $u, $v) = @_;
    my ($fu, $fv) = $uf->find($u, $v);
    return undef if grep !defined, $fu, $fv;
    $fu eq $fv;
}"#;
    assert_clean_parse(source);
}

#[test]
fn test_pdl_autoloader_pattern() {
    // From PDL::AutoLoader
    let source = r#"@PDLLIB = grep length, @PDLLIB;"#;
    assert_clean_parse(source);
}

#[test]
fn test_http_message_psgi_pattern() {
    // From HTTP::Message::PSGI
    let source = r#"$res->content(join '', grep defined, @$body);"#;
    assert_clean_parse(source);
}

#[test]
fn test_graph_bitmatrix_pattern() {
    // From Graph::BitMatrix
    let source = r#"return if grep !defined, my ($i, $j) = @$m1{ $u, $v };"#;
    assert_clean_parse(source);
}

#[test]
fn test_pdl_dbg_pattern() {
    // From PDL::Dbg
    let source = r#"$stab = $stab->{$_.'::'} for grep length, split /::/, $package;"#;
    assert_clean_parse(source);
}

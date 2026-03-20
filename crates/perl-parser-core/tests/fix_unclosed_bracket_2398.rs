mod cpan_test_helpers;
use cpan_test_helpers::*;

// Regression tests for issue #2398: unclosed_bracket — array slice and
// complex subscripts with range operator inside [...].

// ==========================================================================
// Arrow subscript with range operator: $ref->[$a..$b]
// ==========================================================================

#[test]
fn test_arrow_subscript_range() {
    assert_clean_parse("my @slice = @{$self}[$a..$b];");
}

#[test]
fn test_arrow_subscript_range_complex() {
    assert_clean_parse("my @x = $ref->[$start..$end];");
}

#[test]
fn test_arrow_subscript_range_expr() {
    assert_clean_parse("my @x = @$aref[$i..$j];");
}

// ==========================================================================
// Array slice with range operator: @arr[$start..$end]
// ==========================================================================

#[test]
fn test_array_slice_range() {
    assert_clean_parse("my @slice = @array[$start..$end];");
}

#[test]
fn test_array_slice_range_literals() {
    assert_clean_parse("my @slice = @array[0..2];");
}

#[test]
fn test_array_slice_range_in_return() {
    assert_clean_parse("return @array[0..$#array];");
}

// ==========================================================================
// Multi-line anonymous arrayref constructor
// ==========================================================================

#[test]
fn test_multiline_arrayref() {
    assert_clean_parse(
        r#"my $aref = [
    $a,
    $b,
    $c,
];"#,
    );
}

#[test]
fn test_arrayref_with_range() {
    assert_clean_parse("my $aref = [0..9];");
}

#[test]
fn test_arrayref_range_var() {
    assert_clean_parse("my $aref = [$start..$end];");
}

// ==========================================================================
// Complex arithmetic expressions in range subscripts
// ==========================================================================

#[test]
fn test_array_slice_arithmetic_range() {
    // @array[$offset..$offset+$limit-1] — arithmetic in range bounds
    assert_clean_parse("my @chunk = @array[$offset..$offset+$limit-1];");
}

#[test]
fn test_array_slice_count_minus_one() {
    assert_clean_parse("my @first = @array[0..$count-1];");
}

#[test]
fn test_block_deref_slice_range() {
    // @{$self->get_all_roles}[$start..$end]
    assert_clean_parse("my @roles = @{$self->get_all_roles}[$start..$end];");
}

#[test]
fn test_arrow_chain_bracket_range() {
    assert_clean_parse("my $val = $obj->method->[$a + $b .. $c * $d];");
}

#[test]
fn test_block_deref_slice_offset_limit() {
    // @{$self->params}[$offset..$offset+$limit-1]
    assert_clean_parse("my @params = @{$self->params}[$offset..$offset+$limit-1];");
}

// ==========================================================================
// Ternary inside bracket subscript
// ==========================================================================

#[test]
fn test_arrow_bracket_ternary() {
    assert_clean_parse("my $x = $ref->[$cond ? $a : $b];");
}

// ==========================================================================
// Patterns from Moose::Meta::Class — $aref->[$idx] in list context / map
// ==========================================================================

#[test]
fn test_moose_aref_in_list_context() {
    // @{$aref}[$idx] — block-deref subscript in list context
    assert_clean_parse("my @result = @{$aref}[$idx];");
}

#[test]
fn test_moose_method_slice_in_map() {
    assert_clean_parse(r#"my @classes = map { $_->[$idx] } @{$self->all_metaclasses};"#);
}

#[test]
fn test_moose_arrow_bracket_in_return() {
    assert_clean_parse(r#"return $metaclass->[$offset];"#);
}

#[test]
fn test_moose_arrow_bracket_complex_index() {
    assert_clean_parse(r#"my $x = $self->metaclasses->[$offset + $count - 1];"#);
}

// ==========================================================================
// Patterns from Catalyst::Request — @array[$start..$end] in method chains
// ==========================================================================

#[test]
fn test_catalyst_deref_slice_method() {
    assert_clean_parse(r#"my @args = @{$self->args}[0..$#_];"#);
}

#[test]
fn test_catalyst_query_params_slice() {
    assert_clean_parse(r#"my @top = @{$self->query_parameters->{$key}}[0..9];"#);
}

#[test]
fn test_catalyst_body_data_slice() {
    assert_clean_parse(r#"my @body_data = @{$c->request->body_data}[$offset..$limit];"#);
}

// ==========================================================================
// Patterns from DBIx::Class — anonymous arrayref with method calls
// ==========================================================================

#[test]
fn test_dbix_anonymous_arrayref_multiline() {
    assert_clean_parse(
        r#"my $x = [
    $a,
    $b,
    $c
];"#,
    );
}

#[test]
fn test_dbix_anonymous_arrayref_with_methods() {
    assert_clean_parse(
        r#"my $result = [
    $schema->resultset('Artist')->find($id),
    $schema->resultset('CD')->find($other_id),
];"#,
    );
}

#[test]
fn test_dbix_complex_arrayref_constructor() {
    assert_clean_parse(
        r#"push @results, [
    $row->id,
    $row->name,
    $row->created_at->strftime('%Y-%m-%d'),
];"#,
    );
}

// ==========================================================================
// $aref->[$idx] in various list contexts
// ==========================================================================

#[test]
fn test_aref_bracket_in_list_context() {
    assert_clean_parse(r#"my ($a, $b) = ($aref->[$idx], $other->[$idx2]);"#);
}

#[test]
fn test_aref_bracket_as_hash_value() {
    assert_clean_parse(r#"my %h = (key => $aref->[$idx]);"#);
}

#[test]
fn test_aref_bracket_in_push() {
    assert_clean_parse(r#"push @list, $aref->[$start..$end];"#);
}

#[test]
fn test_array_slice_in_hash_value() {
    assert_clean_parse(r#"my %h = (items => [@array[$start..$end]]);"#);
}

#[test]
fn test_array_ref_slice_in_list() {
    assert_clean_parse(r#"my @x = ($a, @array[$start..$end], $b);"#);
}

// ==========================================================================
// Nested arrow chains with range subscript
// ==========================================================================

#[test]
fn test_nested_arrow_with_range() {
    assert_clean_parse(r#"my @x = $obj->list->[$a..$b];"#);
}

#[test]
fn test_deref_subscript_in_conditional() {
    assert_clean_parse(
        r#"if (defined $aref->[$start..$end]) {
    do_something();
}"#,
    );
}

#[test]
fn test_anonymous_arrayref_spread_assign() {
    assert_clean_parse(
        r#"my ($first, @rest) = @{[
    $schema->resultset('A'),
    $schema->resultset('B'),
    $schema->resultset('C'),
]};"#,
    );
}

// ==========================================================================
// Subscript range on parenthesized expression: (expr)[0..n]
// ==========================================================================

#[test]
fn test_paren_expr_range_subscript() {
    assert_clean_parse(r#"my @top = (sort { $freq{$b} <=> $freq{$a} } keys %freq)[0..9];"#);
}

#[test]
fn test_paren_list_range_subscript() {
    assert_clean_parse(r#"my @x = (1, 2, 3, 4, 5)[1..3];"#);
}

#[test]
fn test_paren_sort_range_subscript() {
    assert_clean_parse(r#"my @first3 = (sort @array)[0..2];"#);
}

// ==========================================================================
// From Expect.pm: @{$pat}[ N .. $#{$pat} ] — block deref with last-index range
// ==========================================================================

#[test]
fn test_block_deref_range_to_last_index() {
    // @{$pattern}[ 4 .. $#{$pattern} ]
    assert_clean_parse(r#"my @args = @{$pattern}[ 4 .. $#{$pattern} ];"#);
}

#[test]
fn test_block_deref_range_from_one() {
    // @{$pat}[ 1 .. $#{$pat} ]
    assert_clean_parse(r#"my @rest = @{$pat}[ 1 .. $#{$pat} ];"#);
}

#[test]
fn test_call_with_block_deref_slice() {
    // &{ $pattern->[3] }( $exp, @{$pattern}[ 4 .. $#{$pattern} ] )
    assert_clean_parse(
        r#"my @result = &{ $pattern->[3] }( $exp, @{$pattern}[ 4 .. $#{$pattern} ] );"#,
    );
}

#[test]
fn test_grep_with_block_deref_slice() {
    // grep { ... } @{$pat}[ 1 .. $#{$pat} ]
    assert_clean_parse(
        r#"foreach my $eof_pat ( grep { $_->[1] eq '-eof' } @{$pat}[ 1 .. $#{$pat} ] ) { 1; }"#,
    );
}

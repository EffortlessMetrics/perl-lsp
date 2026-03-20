mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for nested ternary operator parsing — issue #2393
// These cover patterns from MOOSE, CATALYST, and DBIx::Class corpus files.

// === Core nested ternary patterns ===

#[test]
fn test_nested_ternary_in_true_branch() {
    // $x ? $y ? 'a' : 'b' : 'c'
    // Inner ternary is the true-branch of the outer
    assert_clean_parse(r#"my $x = $cond ? $inner ? 'a' : 'b' : 'c';"#);
}

#[test]
fn test_ternary_condition_is_ternary() {
    // ($a ? $b : $c) ? $d : $e
    // The condition itself is a ternary expression
    assert_clean_parse(r#"my $x = ($a ? $b : $c) ? $d : $e;"#);
}

#[test]
fn test_deeply_nested_ternary_three_levels() {
    // $a ? $b ? $c ? 1 : 2 : 3 : 4
    assert_clean_parse(r#"my $x = $a ? $b ? $c ? 1 : 2 : 3 : 4;"#);
}

// === MOOSE/Meta/Attribute.pm patterns ===

#[test]
fn test_nested_ternary_in_grep_block_hash_subscripts() {
    // Pattern from MOOSE/Meta/Attribute.pm line 207
    // grep block with && and ternary involving hash subscripts
    assert_clean_parse(
        r#"my @found = grep { exists $options{$_} && exists $self->{$_} ? $_ : undef } @list;"#,
    );
}

#[test]
fn test_ternary_push_into_arrayref() {
    // Pattern from MOOSE/Meta/Attribute.pm line 244
    // push into an arrayref chosen by ternary
    assert_clean_parse(r#"push @{ $attr->has_init_arg ? \@init : \@non_init }, $attr;"#);
}

#[test]
fn test_multiline_ternary_returning_parens_list() {
    // Pattern from MOOSE/Meta/Attribute.pm — ternary returns a paren list or empty
    assert_clean_parse(
        r#"my @result = (
    defined $self->init_arg
    ? ( attribute_init_arg => $self->init_arg )
    : ()
);"#,
    );
}

#[test]
fn test_ternary_with_scalar_traits_in_constructor() {
    // Pattern from MOOSE: scalar(@traits) ? (traits => \@traits) : ()
    assert_clean_parse(
        r#"$new_class->new($name, %args, ( scalar(@traits) ? ( traits => \@traits ) : () ) );"#,
    );
}

#[test]
fn test_wantarray_ternary_in_parens() {
    // Pattern from MOOSE/Meta/Attribute.pm line 169
    // return ( wantarray ? (...) : $scalar )
    assert_clean_parse(r#"return ( wantarray ? ( $class, @traits ) : $class );"#);
}

#[test]
fn test_wantarray_ternary_array_or_hashref() {
    // Pattern from MOOSE: wantarray ? @{$rv} : $rv
    assert_clean_parse(r#"return wantarray ? @{ $rv } : $rv;"#);
}

#[test]
fn test_wantarray_ternary_hash_or_hashref() {
    // Pattern from MOOSE: wantarray ? %{$rv} : $rv
    assert_clean_parse(r#"return wantarray ? %{ $rv } : $rv;"#);
}

// === Chained ternary in hash value (CATALYST pattern) ===

#[test]
fn test_ternary_in_hash_value_fat_arrow() {
    // key => $x ? 'a' : 'b'
    assert_clean_parse(r#"my %h = (key => $x ? 'a' : 'b');"#);
}

#[test]
fn test_chained_ternary_in_hash_constructor() {
    // Multi-key hash with ternary values
    assert_clean_parse(
        r#"my $obj = Foo->new(
    name  => $x ? 'alpha' : 'beta',
    value => $y ? 1 : 0,
);"#,
    );
}

#[test]
fn test_nested_ternary_in_hash_value() {
    // Nested ternary as hash value
    assert_clean_parse(r#"my %h = (key => $a ? $b ? 'x' : 'y' : 'z');"#);
}

// === DBIx::Class chained ternary patterns ===

#[test]
fn test_chained_ternary_multiline() {
    // $a ? 1 : $b ? 2 : $c ? 3 : 4  multiline variant
    assert_clean_parse(
        r#"my $x = $type eq 'a' ? 'one'
         : $type eq 'b' ? 'two'
         : $type eq 'c' ? 'three'
         : 'other';"#,
    );
}

#[test]
fn test_chained_ternary_with_method_calls() {
    // Chained ternary where each branch is a method call
    assert_clean_parse(
        r#"my $result = $cond->is_a ? $obj->first_method($arg)
                   : $cond->is_b ? $obj->second_method($arg)
                   : $obj->default_method;"#,
    );
}

#[test]
fn test_chained_ternary_returns_complex_values() {
    // Chained ternary where branches return complex expressions
    assert_clean_parse(
        r#"my $val = defined $x
    ? $x > 0 ? "positive" : "negative or zero"
    : "undefined";"#,
    );
}

// === Additional patterns to cover all 8 failing files ===

#[test]
fn test_ternary_in_complex_method_argument() {
    // Ternary as part of complex method call argument
    assert_clean_parse(r#"$self->process($x ? $self->do_a($y) : $self->do_b($z));"#);
}

#[test]
fn test_nested_ternary_with_exists() {
    // exists check with nested ternary
    assert_clean_parse(r#"my $v = exists $h{$k} ? $h{$k} ? 'truthy' : 'falsy' : 'missing';"#);
}

#[test]
fn test_ternary_with_sprintf_multiarg() {
    // ternary that selects format string for sprintf
    assert_clean_parse(
        r#"my $s = $n == 1 ? sprintf('%d item', $n) : $n > 0 ? sprintf('%d items', $n) : 'none';"#,
    );
}

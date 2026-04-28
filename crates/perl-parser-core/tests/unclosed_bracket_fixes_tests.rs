mod cpan_test_helpers;
use cpan_test_helpers::*;

// ==========================================================================
// Fix 1: $#$ref -- last-index on scalar deref
// Perl: $#$arrayref gives the last index of the array pointed to by $arrayref
// ==========================================================================

#[test]
fn dollar_hash_deref_simple() {
    assert_clean_parse("my $last = $#$ref;");
}

#[test]
fn dollar_hash_deref_in_range() {
    assert_clean_parse("my @slice = @$self[0..$#$self];");
}

#[test]
fn dollar_hash_deref_in_block_range() {
    assert_clean_parse("my @rest = @{$self}[2..$#$self];");
}

#[test]
fn dollar_hash_deref_in_complex_range() {
    assert_clean_parse("unshift @cycle, @{$path}[$i+1 .. $#$path];");
}

#[test]
fn dollar_hash_deref_in_pair_slice() {
    assert_clean_parse("my %attrs = (value => $pair->[1], @$pair[2 .. $#$pair]);");
}

#[test]
fn dollar_hash_deref_in_return_slice() {
    assert_clean_parse("return $self->new(@$self[(0 - $size) .. $#$self]);");
}

#[test]
fn dollar_hash_deref_in_array_ref() {
    assert_clean_parse("my $x = [0..$#$ref];");
}

// ==========================================================================
// Fix 2: Bare function calls inside array ref [...]
// Perl: [dirname $filename] is [dirname($filename)]
// ==========================================================================

#[test]
fn bare_func_in_arrayref() {
    assert_clean_parse("$search_paths = [dirname $filename];");
}

#[test]
fn bare_func_multiple_args_in_arrayref() {
    assert_clean_parse("my $x = [join \', \', @items];");
}

// ==========================================================================
// Fix 3: Declarations inside bracket subscripts
// Perl: $ref->[my $n = expr] is valid
// ==========================================================================

#[test]
fn my_decl_in_arrow_bracket() {
    assert_clean_parse("$i->[ my $n = $m->[ _n ]++ ] = $_;");
}

#[test]
fn my_decl_in_bracket_subscript() {
    assert_clean_parse("$ref->[my $x = 0];");
}

// ==========================================================================
// Fix 4: Concatenation chains inside array ref (IP-address-like patterns)
// Perl: [127.0.0.1] contains concatenation (127 . 0 . 0 . 1)
// ==========================================================================

#[test]
fn ip_address_in_arrayref() {
    assert_clean_parse("$args{no_proxy} = [127.0.0.1, 127.0.0.11];");
}

#[test]
fn dotted_numbers_in_arrayref() {
    assert_clean_parse("my $x = [1.2.3];");
}

// ==========================================================================
// Fix 5: Complex deref expressions inside array ref wrapper
// ==========================================================================

#[test]
fn deref_method_call_in_arrayref() {
    assert_clean_parse(
        "return [@{Mojo::Cookie::Response->parse($headers->set_cookie)}] unless @_;",
    );
}

// ==========================================================================
// Fix 6: x repetition operator inside array ref
// ==========================================================================

#[test]
fn x_repetition_in_arrayref() {
    assert_clean_parse("my @x = [ (undef) x scalar(@ordered_values) ];");
}

// ==========================================================================
// Fix 7: Block-list functions inside array ref
// ==========================================================================

#[test]
fn block_func_indexes_in_arrayref() {
    assert_clean_parse("[indexes {$_ eq $search} unpack(\'(A)\', $_[0])];");
}

#[test]
fn block_func_indexes_deref_in_arrayref() {
    assert_clean_parse("[indexes {$_ eq $search} @{$_[0]}];");
}

// ==========================================================================
// Fix 8: Complex map inside array ref
// ==========================================================================

#[test]
fn map_complex_in_arrayref() {
    assert_clean_parse(r#"my $methods = [map uc($_), @{ref $_[0] ? $_[0] : [@_]}];"#);
}

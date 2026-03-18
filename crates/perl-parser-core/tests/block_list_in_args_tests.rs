mod cpan_test_helpers;
use cpan_test_helpers::*;

// ===== FIRST-ERROR patterns from CPAN corpus =====
// These are patterns where unclosed_paren_identifier is the FIRST error in the file.

// Pattern 1: $#$var in parens
#[test]
fn test_dollar_hash_deref_in_if_condition() {
    // From Mojo::Content::MultiPart line 88
    assert_clean_parse("if ($#$parts == $i) { }");
}

// Pattern 2: reftype $var in parens
#[test]
fn test_reftype_in_parens() {
    // From YAML::PP::Representer line 77
    assert_clean_parse("$node->{reftype} = (reftype $node->{data}) || '';");
}

#[test]
fn test_reftype_in_elsif() {
    // From YAML::PP::Dumper line 140
    assert_clean_parse("if (reftype $node->{value} eq 'HASH') { }");
}

// Pattern 3: blessed $var in parens
#[test]
fn test_blessed_in_complex_condition() {
    // From Future.pm line 287
    assert_clean_parse(
        "if( @values == 1 and blessed $values[0] and $values[0]->isa( __PACKAGE__ ) ) { }",
    );
}

// Pattern 4: split with regex in complex paren context
#[test]
fn test_split_regex_complex_paren_list() {
    // From URI::_ldap line 18
    assert_clean_parse(r#"my @bits = (split(/\?/, defined($query) ? $query : ""), ("")x4);"#);
}

// Pattern 5: defined + and + my in condition
#[test]
fn test_defined_and_my_in_condition() {
    // From Catmandu::Plugin::Versioning line 72
    assert_clean_parse(
        r#"if (defined $data->{$id_key} and my $d = $self->get($data->{$id_key})) { }"#,
    );
}

// ===== More patterns to find =====

#[test]
fn test_dollar_hash_deref_standalone() {
    // Simplest possible $#$var test
    assert_clean_parse("my $x = $#$arr;");
}

#[test]
fn test_dollar_hash_deref_in_range_parens() {
    assert_clean_parse("for my $i (0 .. $#$arr) { }");
}

// ===== Block-list functions inside parenthesized argument lists =====

#[test]
fn test_map_block_inside_parenthesized_args() {
    assert_clean_parse("some_function(map { $_ + 1 } @list);");
}

#[test]
fn test_grep_block_inside_parenthesized_args() {
    assert_clean_parse("another_fn(grep { $cond } @items);");
}

#[test]
fn test_push_with_map_block() {
    assert_clean_parse("push @result, map { $_ * 2 } @input;");
}

#[test]
fn test_sort_block_assignment() {
    assert_clean_parse("my @sorted = sort { $a <=> $b } @array;");
}

#[test]
fn test_grep_block_in_for_loop() {
    assert_clean_parse("for my $item (grep { defined } @list) { }");
}

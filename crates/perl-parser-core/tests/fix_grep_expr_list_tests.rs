mod cpan_test_helpers;
use cpan_test_helpers::*;

// ==========================================================================
// grep EXPR, LIST — the expression form of grep (no block)
// ==========================================================================

#[test]
fn grep_defined_comma_list() {
    let source = r#"grep defined, @list;"#;
    assert_clean_parse(source);
}

#[test]
fn grep_defined_comma_at_underscore() {
    let source = r#"grep defined, @_;"#;
    assert_clean_parse(source);
}

#[test]
fn grep_length_comma_list() {
    let source = r#"grep length, @list;"#;
    assert_clean_parse(source);
}

#[test]
fn grep_ref_comma_list() {
    let source = r#"my @refs = grep ref, @mixed;"#;
    assert_clean_parse(source);
}

// ==========================================================================
// map EXPR, LIST — the expression form of map (no block)
// ==========================================================================

#[test]
fn map_uc_comma_list() {
    let source = r#"map uc, @list;"#;
    assert_clean_parse(source);
}

#[test]
fn map_lc_comma_list() {
    let source = r#"my @lower = map lc, @words;"#;
    assert_clean_parse(source);
}

#[test]
fn map_chr_comma_list() {
    let source = r#"my @chars = map chr, @codes;"#;
    assert_clean_parse(source);
}

#[test]
fn map_int_comma_list() {
    let source = r#"my @ints = map int, @floats;"#;
    assert_clean_parse(source);
}

// ==========================================================================
// CPAN corpus patterns — real-world uses from popular modules
// ==========================================================================

#[test]
fn cpan_grep_defined_in_join() {
    // From CLI::Osprey::Role
    let source = r#"my $getopt = join('|', grep defined, ($option_name, $attributes{short}));"#;
    assert_clean_parse(source);
}

#[test]
fn cpan_grep_length_splitdir() {
    // From Catmandu
    let source = r#"my @dirs = grep length, File::Spec->splitdir($script_path);"#;
    assert_clean_parse(source);
}

#[test]
fn cpan_map_grep_chained() {
    // From Catmandu::Fix::Parser
    let source = r#"my @result = grep defined, map { is_array_ref($_) ? @$_ : $_ } @$statements;"#;
    assert_clean_parse(source);
}

#[test]
fn cpan_grep_defined_in_if_condition() {
    // From Text::Trim
    let source = r#"if (my @def = grep defined, @_) { return "@def" } else { return }"#;
    assert_clean_parse(source);
}

#[test]
fn cpan_nested_map_grep_expr() {
    // From Catmandu::Importer::Modules
    let source = r#"my $parts = [map {grep length, split(/::/, $_)} $ns];"#;
    assert_clean_parse(source);
}

// ==========================================================================
// Named unary builtins with comma: should default to $_
// ==========================================================================

#[test]
fn grep_chomp_comma_list() {
    let source = r#"grep chomp, @lines;"#;
    assert_clean_parse(source);
}

#[test]
fn grep_abs_comma_list() {
    let source = r#"my @positive = grep abs, @numbers;"#;
    assert_clean_parse(source);
}

#[test]
fn map_ord_comma_list() {
    let source = r#"my @codes = map ord, @chars;"#;
    assert_clean_parse(source);
}

#[test]
fn map_hex_comma_list() {
    let source = r#"my @values = map hex, @hex_strings;"#;
    assert_clean_parse(source);
}

// ==========================================================================
// Ensure named unary builtins with actual arguments still work
// ==========================================================================

#[test]
fn grep_defined_with_arg_still_works() {
    // defined $var should still parse $var as the argument to defined
    let source = r#"grep defined $_, @list;"#;
    assert_clean_parse(source);
}

#[test]
fn defined_variable_in_expression() {
    let source = r#"my $x = defined $y;"#;
    assert_clean_parse(source);
}

#[test]
fn length_variable_in_expression() {
    let source = r#"my $len = length $str;"#;
    assert_clean_parse(source);
}

#[test]
fn defined_hash_element() {
    let source = r#"if (defined $hash{key}) { }"#;
    assert_clean_parse(source);
}

// ==========================================================================
// Block form still works (regression guard)
// ==========================================================================

#[test]
fn grep_block_form_still_works() {
    let source = r#"my @filtered = grep { defined $_ } @list;"#;
    assert_clean_parse(source);
}

#[test]
fn map_block_form_still_works() {
    let source = r#"my @upper = map { uc $_ } @list;"#;
    assert_clean_parse(source);
}

#[test]
fn sort_block_form_still_works() {
    let source = r#"my @sorted = sort { $a cmp $b } @list;"#;
    assert_clean_parse(source);
}

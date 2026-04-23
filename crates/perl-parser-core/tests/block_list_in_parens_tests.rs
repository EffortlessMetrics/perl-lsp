mod cpan_test_helpers;
use cpan_test_helpers::*;

// Issue #1897: block-list functions in parenthesized argument lists

#[test]
fn test_sort_block_inside_parenthesized_args() {
    assert_clean_parse("foo(sort { $a <=> $b } @numbers);");
}

#[test]
fn test_map_block_with_other_args_in_parens() {
    assert_clean_parse("foo($x, map { $_ * 2 } @list, $y);");
}

#[test]
fn test_method_call_with_grep_block() {
    assert_clean_parse("$obj->method(grep { $_->is_valid } @items);");
}

#[test]
fn test_push_paren_map_block() {
    assert_clean_parse("push(@result, map { $_ + 1 } @input);");
}

#[test]
fn test_nested_join_sort_in_print() {
    let source = r#"print(join(", ", sort { $a cmp $b } @names));"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_block_in_array_ref() {
    assert_clean_parse("my $ref = [map { $_ * 2 } @list];");
}

#[test]
fn test_grep_block_in_hash_value_array_ref() {
    let source = r#"my %h = (items => [grep { defined $_ } @raw]);"#;
    assert_clean_parse(source);
}

#[test]
fn test_chained_sort_map_in_parens() {
    let source = r#"foo(sort { $a <=> $b } map { $_->{val} } @items);"#;
    assert_clean_parse(source);
}

#[test]
fn test_return_map_block_in_parens() {
    assert_clean_parse("return (map { $_ + 1 } @list);");
}

#[test]
fn test_grep_block_in_list_parens() {
    let source = r#"my @r = (grep { /pattern/ } @strings);"#;
    assert_clean_parse(source);
}

#[test]
fn test_multiple_block_lists_as_separate_args() {
    let source = r#"foo(grep { $_ > 0 } @a, sort { $a <=> $b } @b);"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_block_with_hash_slice_in_block() {
    let source = r#"my @vals = map { $hash{$_} } @keys;"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_in_if_condition() {
    let source = r#"if (grep { $_ eq $target } @items) { }"#;
    assert_clean_parse(source);
}

#[test]
fn test_any_block_in_if() {
    let source = r#"if (any { $_ eq 'admin' } @roles) { }"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_in_sprintf() {
    let source = r#"sprintf("(%s)", join(", ", map { $_ * 2 } @nums));"#;
    assert_clean_parse(source);
}

// CPAN-sourced patterns

#[test]
fn test_map_block_with_fat_arrow_in_parens() {
    // From ExtUtils::InstallPaths
    let source = r#"my %h = (map { $_ => _merge_shallow($_, $deep_filter{$_}) } qw/original_prefix install_base_relpaths/);"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_block_escape_table_in_parens() {
    // From Catmandu::Exporter::Text
    let source = r#"my %e = (map {$_ => $_} ('\\', '"', '$', '@'));"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_block_with_unpack_in_parens() {
    // From Catmandu::Exporter::Text
    let source = r#"my %e = (map {'x' . unpack('H2', chr($_)) => chr($_)} (0 .. 255));"#;
    assert_clean_parse(source);
}

#[test]
fn test_map_block_ordinal_suffix_in_parens() {
    // From Date::Language::Turkish
    let source = r#"my %s = (map {$_ => 'inci', $_+10 => 'inci', $_+20 => 'inci' } 1,2,5,8 );"#;
    assert_clean_parse(source);
}

// More actual CPAN patterns

#[test]
fn test_grep_block_assignment_splitdir() {
    // From ExtUtils::Helpers
    assert_clean_parse(r#"my @dirs = grep { length } splitdir($dirs);"#);
}

#[test]
fn test_grep_block_in_if_qw() {
    // From ExtUtils::PkgConfig
    let source = r#"if (grep {$_ eq $function} qw/libs cflags/) { }"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_block_opentype_comparison() {
    // From Path::Tiny
    let source = r#"if ( grep { $opentype eq $_ } qw( > +> ) ) { }"#;
    assert_clean_parse(source);
}

#[test]
fn test_return_grep_block_ref_check() {
    // From Catmandu::Fix
    let source = r#"return [grep {!(ref $_ && $_ == $reject)} @$list];"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_block_file_test() {
    // From Catmandu::Env
    let source = r#"if (grep {-r File::Spec->catfile($path, $_)} @files) { }"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_is_value_join() {
    // From Catmandu::Fix::error
    let source = r#"my $str = join "\n", grep {is_value($_)} @$vals;"#;
    assert_clean_parse(source);
}

#[test]
fn test_sort_map_grep_chain() {
    // From Catmandu::Fix::include
    let source = r#"return [sort map {realpath($_)} grep {-r $_} glob $path];"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_defined_map_chain() {
    // From Catmandu::Validator
    let source = r#"return [grep {defined} map {$self->_process_record($_)} @$data];"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_ref_ne_in_parens() {
    // From Log::Any::Adapter::Multiplex
    let source = r#"( grep { ref($_) ne 'ARRAY' } values %$adapters );"#;
    assert_clean_parse(source);
}

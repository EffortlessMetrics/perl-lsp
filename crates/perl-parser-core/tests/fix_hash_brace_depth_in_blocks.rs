mod cpan_test_helpers;
use cpan_test_helpers::*;

// Issue #2833: hash_brace_depth was incremented for ALL `{` in ExpectOperator mode,
// not just those that follow a variable token ($x, @x, %x). This caused quote-op
// suppression inside sub/if/else/while/for block bodies, leading to parse errors
// on m//, s///, qr//, etc. wherever hash_brace_depth > 0.

// Pattern 1: m// inside sub body (the core regression)
#[test]
fn test_regex_in_sub_body() {
    let source = r#"
sub foo {
    $x =~ m/abc/;
}
"#;
    assert_clean_parse(source);
}

// Pattern 2: m// inside else block
#[test]
fn test_regex_in_else_block() {
    let source = r#"
if (1) {
    $x = 1;
} else {
    $y =~ m/pattern/;
}
"#;
    assert_clean_parse(source);
}

// Pattern 3: m// inside elsif block
#[test]
fn test_regex_in_elsif_block() {
    let source = r#"
sub check {
    local(*FH);
    if ($x) {
    } elsif ($fd =~ m#^\d+$#) {
        return 1;
    }
}
"#;
    assert_clean_parse(source);
}

// Pattern 4: nested map with regex inside sub body
#[test]
fn test_regex_in_nested_map_in_sub() {
    let source = r#"
sub foo {
    my @r = map { m/::/ ? 1 : 0 } @list;
}
"#;
    assert_clean_parse(source);
}

// Pattern 5: m// with brace quantifier inside sub body
#[test]
fn test_regex_brace_quantifier_in_sub() {
    let source = r#"
sub foo {
    if ($d =~ m/\d{3}/) {
        return 1;
    }
}
"#;
    assert_clean_parse(source);
}

// Pattern 6: s/// inside sub body
#[test]
fn test_substitution_in_sub_body() {
    let source = r#"
sub normalize {
    (my $copy = $input) =~ s/foo/bar/g;
    return $copy;
}
"#;
    assert_clean_parse(source);
}

// Pattern 7: while loop with m// inside sub body
#[test]
fn test_regex_in_while_in_sub() {
    let source = r#"
sub parse_tokens {
    while ($line =~ m/(\w+)/g) {
        push @tokens, $1;
    }
}
"#;
    assert_clean_parse(source);
}

// Pattern 8: deeply nested blocks with regex
// Uses string match operator to avoid a pre-existing regex-parsing limitation
// with `/` delimiters across statement boundaries in deeply nested code.
#[test]
fn test_regex_deeply_nested() {
    let source = r#"
sub foo {
    for my $item (@list) {
        if ($item) {
            while (1) {
                last unless $item =~ m/\w+/g;
                push @r, $item;
            }
        }
    }
}
"#;
    assert_clean_parse(source);
}

// Pattern 9: m{} delimiter inside sub body (m with brace delimiter)
#[test]
fn test_regex_brace_delim_in_sub_body() {
    let source = r#"
sub check_prefix {
    if ($cmd =~ m{^perl}) {
        return 1;
    }
}
"#;
    assert_clean_parse(source);
}

// Pattern 10: elsif with m{} then nested m// (from plan-review confirmed failing pattern)
#[test]
fn test_elsif_mbrace_then_mslash() {
    let source = r#"
sub foo {
    if (0) {
    } elsif ($cmd =~ m{^perl}) {
        if ($x =~ m/abc/) { return 1; }
    }
}
"#;
    assert_clean_parse(source);
}

// Pattern 11: for loop block with regex inside sub
#[test]
fn test_regex_in_for_block_in_sub() {
    let source = r#"
sub process {
    for my $line (@lines) {
        next unless $line =~ m/^\s*\w/;
        push @result, $line;
    }
}
"#;
    assert_clean_parse(source);
}

// Pattern 12: qr// inside sub body
#[test]
fn test_qr_in_sub_body() {
    let source = r#"
sub make_regex {
    my $re = qr/^\d+$/;
    return $re;
}
"#;
    assert_clean_parse(source);
}

// Pattern 13: tr/// inside sub body
#[test]
fn test_tr_in_sub_body() {
    let source = r#"
sub upcase_vowels {
    (my $copy = $str) =~ tr/aeiou/AEIOU/;
    return $copy;
}
"#;
    assert_clean_parse(source);
}

// --- REGRESSION TESTS: hash subscript keys must still work ---

#[test]
fn test_hash_key_m_regression() {
    assert_clean_parse("$h{m} = 1;");
}

#[test]
fn test_hash_key_s_regression() {
    assert_clean_parse("$h{s} = 1;");
}

#[test]
fn test_hash_slice_m_s_regression() {
    assert_clean_parse("my @v = @h{m, s};");
}

#[test]
fn test_chained_hash_subscript_regression() {
    assert_clean_parse("my $v = $h{a}{b};");
}

#[test]
fn test_arrow_hash_subscript_m_regression() {
    assert_clean_parse("my $v = $ref->{m};");
}

#[test]
fn test_hash_subscript_with_regex_inside_sub() {
    // hash subscript inside a sub body — must NOT suppress the hash behavior
    let source = r#"
sub foo {
    my $v = $h{m};
    return $v;
}
"#;
    assert_clean_parse(source);
}

#[test]
fn test_hash_subscript_and_regex_in_same_sub() {
    // Both should work correctly in same sub body
    let source = r#"
sub foo {
    my $v = $h{m};
    if ($str =~ m/pattern/) {
        return $v;
    }
}
"#;
    assert_clean_parse(source);
}

use perl_parser_core::{Node, NodeKind, Parser};
use perl_tdd_support::must;

/// Assert a clean parse — no Error or Missing* nodes in the AST.
fn assert_no_errors(source: &str) {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let mut errors = Vec::new();
    walk_all_errors(&ast, &mut errors);

    assert!(
        errors.is_empty(),
        "Found {} error node(s) for source:\n  {}\nerrors:\n{}\nsexp:\n{}",
        errors.len(),
        source,
        errors
            .iter()
            .map(|(pos, msg)| format!("  byte {pos}: {msg}"))
            .collect::<Vec<_>>()
            .join("\n"),
        ast.to_sexp(),
    );
}

/// Assert at least one error is present — either an Error/Missing* AST node
/// or a collected parse error.
fn assert_has_errors(source: &str) {
    let mut parser = Parser::new(source);
    let ast = must(parser.parse());

    let mut ast_errors = Vec::new();
    walk_all_errors(&ast, &mut ast_errors);
    let collected_errors = parser.errors();

    assert!(
        !ast_errors.is_empty() || !collected_errors.is_empty(),
        "Expected parse errors for source:\n  {}\nsexp:\n{}",
        source,
        ast.to_sexp(),
    );
}

fn walk_all_errors(node: &Node, errors: &mut Vec<(usize, String)>) {
    if let NodeKind::Error { message, .. } = &node.kind {
        errors.push((node.location.start, message.clone()));
    }
    node.for_each_child(|child| {
        walk_all_errors(child, errors);
    });
}

// ============================================================
// Sub-bucket A: is_block_list_func trailing comma regression guard
// The block-list-func arg loop (sort, grep, map, first, any, all, none,
// reduce) must not fail when given a trailing comma before `)`.
// is_at_statement_end() includes RightParen, so the while loop exits
// cleanly after consuming the trailing comma.
// ============================================================

#[test]
fn test_sort_trailing_comma_in_parens() {
    // Trailing comma after last argument — valid Perl
    assert_no_errors("my @r = sort(@list,);");
}

#[test]
fn test_grep_trailing_comma_in_parens() {
    // Trailing comma after last argument — valid Perl
    assert_no_errors("my @r = grep(sub { $_ > 0 }, @list,);");
}

#[test]
fn test_map_trailing_comma_in_parens() {
    // Trailing comma after last argument — valid Perl
    assert_no_errors("my @r = map(sub { $_ * 2 }, @list,);");
}

#[test]
fn test_sort_block_then_trailing_comma() {
    // Block arg then list arg with trailing comma
    assert_no_errors("my @r = sort({ $a <=> $b } @list,);");
}

#[test]
fn test_grep_single_arg_trailing_comma() {
    // Single list arg with trailing comma
    assert_no_errors("my @r = grep($_ > 0, @list,);");
}

// ============================================================
// Sub-bucket B: empty conditions in if/elsif/unless/until
// For if/elsif/unless/until, empty conditions are Perl syntax errors.
// The parser should report an error (not cascade into unexpected_rparen_expr).
// Contrast: while () is valid Perl (infinite loop idiom, equivalent to
// while (1)) and must parse cleanly — see test below.
// ============================================================

#[test]
fn test_empty_if_condition_produces_error() {
    // `if ()` is a Perl syntax error — should report an error, not cascade
    assert_has_errors("if () { 1 }");
}

#[test]
fn test_empty_unless_condition_produces_error() {
    // `unless ()` is a Perl syntax error — should report an error
    assert_has_errors("unless () { 1 }");
}

#[test]
fn test_empty_until_condition_produces_error() {
    // `until ()` is a Perl syntax error — should report an error
    assert_has_errors("until () { last }");
}

#[test]
fn test_empty_elsif_in_if_chain_produces_error() {
    // `elsif ()` is a Perl syntax error — should report an error
    assert_has_errors("if (1) { } elsif () { }");
}

#[test]
fn test_empty_elsif_in_unless_chain_produces_error() {
    // `elsif ()` after `unless` is a Perl syntax error — should report an error
    assert_has_errors("unless (1) { } elsif () { }");
}

// ============================================================
// Sanity check: while () MUST remain valid (infinite loop idiom)
// This is the model implementation that Sub-bucket B fixes must NOT disturb.
// ============================================================

#[test]
fn test_while_empty_condition_is_valid_infinite_loop() {
    // while () is valid Perl — infinite loop, equivalent to while (1)
    // This must parse cleanly with no errors
    assert_no_errors("while () { last }");
}

// ============================================================
// Regression: well-formed block-list-func calls must still work
// ============================================================

#[test]
fn test_sort_block_no_trailing_comma_clean() {
    assert_no_errors("my @r = sort { $a <=> $b } @list;");
}

#[test]
fn test_grep_block_no_trailing_comma_clean() {
    assert_no_errors("my @r = grep { $_ > 0 } @list;");
}

#[test]
fn test_map_block_no_trailing_comma_clean() {
    assert_no_errors("my @r = map { $_ * 2 } @list;");
}

#[test]
fn test_nested_grep_clean() {
    // Nested block-list funcs must still work after the fix
    assert_no_errors("my @r = grep { grep { $_ } @inner } @outer;");
}

#[test]
fn test_if_with_normal_condition_clean() {
    // Normal if condition must not be affected by Sub-bucket B fix
    assert_no_errors("if ($x > 0) { 1 }");
}

#[test]
fn test_unless_with_normal_condition_clean() {
    assert_no_errors("unless ($x) { 1 }");
}

#[test]
fn test_until_with_normal_condition_clean() {
    assert_no_errors("until ($done) { last }");
}

#[test]
fn test_if_with_var_decl_condition_clean() {
    // Variable declaration in condition — existing guard must not be disturbed
    assert_no_errors("if (my $x = foo()) { }");
}

// ============================================================
// Corpus-derived patterns — remaining unexpected_rparen_expr
// These are from actual corpus files listed in the baseline
// ============================================================

#[test]
fn test_qualified_function_call_empty_parens() {
    // Net/hostent.pm: Socket::AF_INET() — qualified method with empty parens
    assert_no_errors("$addrtype = @_ ? shift : Socket::AF_INET();");
}

#[test]
fn test_qualified_function_call_empty_parens_in_expr() {
    // General: qualified package call with empty parens
    assert_no_errors("my $x = Foo::Bar::baz();");
}

#[test]
fn test_method_call_empty_parens_in_ternary() {
    // Pattern from Net::* corpus files
    assert_no_errors("my $x = defined($y) ? $y : Some::Package::default_value();");
}

#[test]
fn test_split_dollar_gid_in_grep() {
    // POSIX.pm line 135: grep !$seen{$_}++, split " ", $)
    // $) = effective GID special variable
    assert_no_errors(r#"my @r = grep !$seen{$_}++, split " ", $);"#);
}

#[test]
fn test_posix_map_function_name() {
    // POSIX.pm line 188: map { "POSIX::$_()" } @unimpl
    // Empty parens inside double-quoted string inside map block
    assert_no_errors(r#"my @r = map { "POSIX::$_()" } @list;"#);
}

// ============================================================
// B/Deparse.pm patterns — constant sub with empty prototype
// `sub NAME () { VALUE }` is Perl constant sub syntax
// ============================================================

#[test]
fn test_constant_sub_empty_prototype() {
    // B/Deparse.pm: sub POSTFIX () { 1 }
    // Sub with empty prototype () — valid Perl constant declaration
    assert_no_errors("sub POSTFIX () { 1 }");
}

#[test]
fn test_constant_sub_empty_prototype_in_package() {
    // B/Deparse.pm: sub ASSIGN () { 2 }
    assert_no_errors("sub ASSIGN () { 2 }");
}

#[test]
fn test_glob_assign_constant_sub() {
    // B/Deparse.pm line 76: *{$_} = sub () {0} unless *{$_}{CODE};
    assert_no_errors(r#"*{$_} = sub () {0} unless *{$_}{CODE};"#);
}

// ============================================================
// POSIX.pm patterns — hash slice assigned empty list
// ============================================================

#[test]
fn test_hash_slice_assigned_empty_list() {
    // POSIX.pm: @export{map {@$_} values %tags} = ();
    // Hash slice assigned an empty list ()
    assert_no_errors("@export{map {@$_} values %default_export_tags} = ();");
}

#[test]
fn test_simple_hash_slice_assigned_empty_list() {
    // Simplified: @hash{@keys} = ();
    assert_no_errors("@hash{@keys} = ();");
}

#[test]
fn test_array_assigned_empty_list() {
    // @array = () assignment
    assert_no_errors("@array = ();");
}

// ============================================================
// X11/Auth.pm patterns
// ============================================================

#[test]
fn test_return_empty_list() {
    // X11/Auth.pm: return ();
    assert_no_errors("return ();");
}

#[test]
fn test_array_reset_empty_list() {
    // X11/Auth.pm: @a = ();
    assert_no_errors("@a = ();");
}

#[test]
fn test_qualified_hostname_call() {
    // X11/Auth.pm: Sys::Hostname::hostname()
    assert_no_errors("$host = Sys::Hostname::hostname();");
}

// ============================================================
// TAP/Parser/YAMLish/Reader.pm patterns — coderef invocation
// ============================================================

#[test]
fn test_hash_coderef_call() {
    // TAP/Parser/YAMLish/Reader.pm: $self->{reader}->()
    // Calling a coderef stored in a hash value
    assert_no_errors("my $line = $self->{reader}->();");
}

// ============================================================
// Sub-bucket C: $( (real GID) special variable
// $( is a valid Perl special variable (real group ID of the process).
// The lexer must consume the '(' as part of the $( token, not treat
// it as the start of a parenthesized expression.
// ============================================================

#[test]
fn test_dollar_lparen_assign() {
    // PgCommon.pm: $( = $gid;
    // $( is the real group ID special variable — assignable
    assert_no_errors("$( = $gid;");
}

#[test]
fn test_dollar_lparen_in_comparison() {
    // PgCommon.pm: if ($( != $gid) { die }
    assert_no_errors("if ($( != $gid) { die }");
}

#[test]
fn test_dollar_lparen_postfix_if() {
    // PgCommon.pm: error 'Could not change group id' if $( != $gid;
    assert_no_errors("error 'Could not change group id' if $( != $gid;");
}

#[test]
fn test_dollar_lparen_read_in_expr() {
    // Read $( in arithmetic context
    assert_no_errors("my $x = $( + 0;");
}

#[test]
fn test_dollar_lparen_chain_assign() {
    // PgCommon.pm: $) = $groups; $( = $gid; $> = $< = $uid;
    assert_no_errors("$) = $groups; $( = $gid; $> = $< = $uid;");
}

#[test]
fn test_dollar_lparen_print() {
    // print $( — reading real GID
    assert_no_errors(r#"print "GID: $(\n";"#);
}

// Regression guard: $) (effective GID) must remain valid after the fix
#[test]
fn test_dollar_rparen_still_valid() {
    assert_no_errors("$) = $groups;");
}

#[test]
fn test_dollar_rparen_in_split() {
    // POSIX.pm: grep !$seen{$_}++, split " ", $)
    assert_no_errors(r#"my @r = grep !$seen{$_}++, split " ", $);"#);
}

// ============================================================
// Sub-bucket D: s/// with single-quote content and / delimiter
// When the s/// replacement contains a single quote character and the
// delimiter is '/', the lexer must not mistake the single quote for
// the start of an inner string literal.
// ============================================================

#[test]
fn test_subst_replace_single_quote_with_slash_delim() {
    // Log::Log4perl: $literal =~ s/''/'/g;
    // Delimiter is '/', pattern is '' (two single quotes), replacement is ' (one single quote)
    assert_no_errors(r#"$literal =~ s/''/'/g;"#);
}

#[test]
fn test_subst_replace_single_quote_simple() {
    // Simplified: just replace two single quotes with one
    assert_no_errors(r#"$x =~ s/''/'/;"#);
}

#[test]
fn test_subst_double_quote_to_single() {
    // Variant: double-quote replacement containing single quote
    assert_no_errors(r#"$x =~ s/foo/bar'baz/;"#);
}

#[test]
fn test_subst_single_quote_in_pattern_and_replacement() {
    // TAP/Parser: ( my $rv = $1 ) =~ s/''/'/g;
    // Cascaded match-then-subst with single-quote replacement
    assert_no_errors(r#"( my $rv = $1 ) =~ s/''/'/g;"#);
}

#[test]
fn test_log4perl_combined_pattern() {
    // Full pattern from Log::Log4perl::DateFormat.pm
    assert_no_errors(
        r#"
if ( $chunk =~ /\A'(.*)'\z/ ) {
    my $literal = $1;
    $literal =~ s/''/'/g;
    $literal =~ s/\%/\%\%/g;
    my $fmt2 = $literal;
} elsif ( $chunk =~ /'/ ) {
    croak "bad format";
}
"#,
    );
}

// Regression guard: s/// with non-quote delimiters must not be affected
#[test]
fn test_subst_slash_delim_no_quotes_clean() {
    assert_no_errors(r#"$x =~ s/foo/bar/g;"#);
}

#[test]
fn test_subst_brace_delim_clean() {
    assert_no_errors(r#"$x =~ s{foo}{bar}g;"#);
}

#[test]
fn test_subst_comma_delim_clean() {
    assert_no_errors(r#"$x =~ s,foo,bar,g;"#);
}

// ============================================================
// Edge cases: s/// with single-quote as delimiter
// When ' is the delimiter, it is the closing char — NOT an inner string opener.
// The guard `ch != repl_closing` must prevent string-skip for the delimiter itself.
// ============================================================

#[test]
fn test_subst_single_quote_delimiter() {
    // s'pattern'replacement' — single-quote IS the delimiter
    // The ' in the replacement section closes it; no string-skip should occur.
    assert_no_errors(r#"$x =~ s'foo'bar'g;"#);
}

#[test]
fn test_subst_single_quote_delimiter_empty_replacement() {
    // s'pattern'' — empty replacement with single-quote delimiter
    assert_no_errors(r#"$x =~ s'foo'';"#);
}

#[test]
fn test_subst_double_quote_delimiter() {
    // s"pattern"replacement" — double-quote IS the delimiter
    // The " in the replacement section closes it; no string-skip should occur.
    assert_no_errors(r#"$x =~ s"foo"bar"g;"#);
}

// ============================================================
// Edge cases: s/// replacement containing double-quoted string with delimiter
// When the replacement contains "str/with/slashes", the lookahead must enter
// string-skip mode (contains_delim=true) to protect the inner slashes.
// ============================================================

#[test]
fn test_subst_double_quoted_replacement_with_delimiter() {
    // Replacement contains a double-quoted string literal that has slashes
    // The lookahead should enter string-skip for "foo/bar" because it contains /
    assert_no_errors(r#"$x =~ s/old/"new\/replacement"/;"#);
}

#[test]
fn test_subst_double_quoted_replacement_no_delimiter() {
    // Replacement contains a double-quoted string literal with no slash
    // The lookahead should NOT enter string-skip (no delimiter inside)
    // The quotes are treated as literal chars in the replacement
    assert_no_errors(r#"$x =~ s/old/"new_replacement"/;"#);
}

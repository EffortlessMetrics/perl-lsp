mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for issue #4169: ternary branches with call-like expressions
//
// Root cause: The postfix chain did not handle `LeftParen` after non-Identifier
// expressions (subscripts, `undef`, deref results).  This caused
// "expected ':', found '('" errors when a ternary then-branch ended with
// a subscript or keyword that was immediately followed by a call `()`.
//
// Fix: added two cases to `parse_postfix_chain`:
//   - `NodeKind::Undef` + `(` → `undef(LIST)` function call
//   - `NodeKind::Binary` + `(` → implicit coderef invocation

// === Core issue patterns from the bug report ===

#[test]
fn test_nested_ternary_simple_chain() {
    // $a ? $b : $c ? $d : $e — right-associative nested ternary
    assert_clean_parse(r#"my $result = $a ? $b : $c ? $d : $e;"#);
}

#[test]
fn test_nested_ternary_fat_arrow_branches() {
    // Ternary with fat arrow in branches: `key => 1` in then/else
    assert_clean_parse(r#"my $pair = $flag ? key => 1 : other => 2;"#);
}

#[test]
fn test_ternary_in_statement_modifier_position() {
    // Ternary in statement modifier position with bare identifiers
    assert_clean_parse(r#"print $a ? x : y if $cond;"#);
}

// === The actual bug: call syntax after ternary branches ===

#[test]
fn test_undef_parens_in_ternary_else_branch() {
    // From POE — undef() with explicit parens in ternary else-branch.
    // Previously: ERROR "expected ':', found '(' at position 41"
    assert_clean_parse(r#"StderrEvent => ($conduit eq 'pty' ? undef() : 'stderr');"#);
}

#[test]
fn test_coderef_hashref_call_in_ternary_then_branch() {
    // From IO/Socket/SSL/Intercept.pm: $self->{serial}($old_cert,$hash)
    // Previously: ERROR "expected ':', found '(' at position 61"
    assert_clean_parse(
        r#"my $serial = ref($self->{serial}) eq 'CODE' ? $self->{serial}($old_cert,$hash) : ++$self->{serial};"#,
    );
}

#[test]
fn test_array_subscript_coderef_call_in_ternary_then_branch() {
    // From Mojo::Log::_format: ref $_[0] eq 'CODE' ? $_[0]() : @_
    // Previously: ERROR "expected ':', found '(' at position 92"
    assert_clean_parse(r#"my @msgs = ref $_[0] eq 'CODE' ? $_[0]() : @_;"#);
}

// === undef() as a function call (not just as a literal) ===

#[test]
fn test_undef_parens_standalone() {
    // undef() called with no args — clears $_
    assert_clean_parse(r#"undef();"#);
}

#[test]
fn test_undef_parens_with_args() {
    // undef($var) — undefines the variable
    assert_clean_parse(r#"undef($x);"#);
}

#[test]
fn test_undef_parens_in_assignment() {
    // Assignment from undef()
    assert_clean_parse(r#"my $x = undef();"#);
}

// === Implicit coderef calls (no arrow) ===

#[test]
fn test_array_subscript_coderef_call_no_arrow() {
    // $handlers[0]($arg) — calling array element as coderef
    assert_clean_parse(r#"$handlers[0]($arg);"#);
}

#[test]
fn test_hash_subscript_coderef_call_no_arrow() {
    // $dispatch{$key}($arg) — calling hash value as coderef
    assert_clean_parse(r#"$dispatch{$key}($arg);"#);
}

#[test]
fn test_hash_arrow_subscript_coderef_call_no_arrow() {
    // $obj->{method}($self) — calling hash arrow value as coderef (without ->)
    assert_clean_parse(r#"$self->{serial}($old, $hash);"#);
}

// === Regression guard: existing patterns still work ===

#[test]
fn test_nested_ternary_bare_identifiers() {
    // Bare identifiers in ternary branches (no call parens)
    assert_clean_parse(r#"my $x = $a ? foo : bar;"#);
}

#[test]
fn test_triple_chained_ternary() {
    // Three-level chain
    assert_clean_parse(r#"my $r = $a ? $b : $c ? $d : $e ? $f : $g;"#);
}

#[test]
fn test_multiline_chained_ternary() {
    // CPAN-style multiline ternary with leading colons
    assert_clean_parse(
        r#"my $x = $a eq "foo" ? alpha
         : $a eq "bar" ? beta
         : gamma;"#,
    );
}

#[test]
fn test_ternary_with_method_call_in_then() {
    // Method call in then-branch (already worked via arrow chain)
    assert_clean_parse(r#"my $v = $a ? $obj->method($x) : $obj->other($y);"#);
}

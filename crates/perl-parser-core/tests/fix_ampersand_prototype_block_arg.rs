mod cpan_test_helpers;
use cpan_test_helpers::*;

// Tests for:
// 1. sub declarations with & in prototype where call syntax passes
//    a bare block `{ ... }` as the coderef argument.
// 2. nullary builtins like `ref` followed by string-comparison operators
//    (ne, eq, etc.) which are tokenized as identifiers, not dedicated tokens.
// 3. comma expressions on the right side of word operators (or/and/xor):
//    `COND or $x = VALUE, 0` where `, 0` is part of the rhs list.
// Issue #2388

#[test]
fn test_ampersand_prototype_basic_call() {
    let source = r#"sub foo (&;@) { $_[0]->() } foo { 1 } 'bar', 'baz';"#;
    assert_clean_parse(source);
}

#[test]
fn test_ampersand_prototype_declaration() {
    let source = r#"sub foo (&;@) { $_[0]->() }"#;
    assert_clean_parse(source);
}

#[test]
fn test_try_catch_pattern() {
    let source = r#"
sub try (&;@) {
    my ($try, @handlers) = @_;
    $try->();
}
sub catch (&;@) {
    my ($handler) = @_;
    return $handler;
}
try { die "oops" } catch { warn "caught: $_" };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_ampersand_prototype_only() {
    let source = r#"sub build (&;$) { my ($block, $server) = @_; $block->() }"#;
    assert_clean_parse(source);
}

#[test]
fn test_ampersand_prototype_call_with_block_body() {
    let source = r#"
sub apply (&@) { my $f = shift; $f->() for @_ }
apply { print $_ } @items;
"#;
    assert_clean_parse(source);
}

#[test]
fn test_continuation_pattern() {
    let source = r#"
sub continuation (&;@) {
    my $block = shift;
    return $block;
}
my $cont = continuation { 42 };
"#;
    assert_clean_parse(source);
}

#[test]
fn test_ref_ne_in_grep_block() {
    // `ref` is a nullary builtin; `ne` is tokenized as Identifier.
    // The parser must NOT consume `ne` as ref's argument.
    let source = r#"grep { ref ne 'Foo' } @list;"#;
    assert_clean_parse(source);
}

#[test]
fn test_grep_block_or_comma() {
    // `$cond or $x = VALUE, 0` -- comma > or in precedence.
    let source = r#"grep { $x > 5 or $y = 42, 0 } @rest;"#;
    assert_clean_parse(source);
}

#[test]
fn test_ref_ne_or_comma_in_grep_block() {
    let source = r#"grep { ref ne 'Foo' or $x = 1, 0 } @list;"#;
    assert_clean_parse(source);
}

#[test]
fn test_dancer_exception_catch() {
    // Key excerpt from Dancer::Exception.pm (issue #2388)
    let source = r#"
sub catch (&;@) {
    my ( $block, @rest ) = @_;
    my $continuation_code;
    my @new_rest = grep { ref ne 'Try::Tiny::Catch' or $continuation_code = $$_, 0 } @rest;
}
"#;
    assert_clean_parse(source);
}

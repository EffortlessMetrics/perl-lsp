/// Tests for unbraced scalar/array/hash dereference of a scalar ref.
///
/// Per perlref/perlop, `$$ref`, `@$ref`, and `%$ref` are 100% equivalent to
/// their braced forms `${$ref}`, `@{$ref}`, and `%{$ref}`.
///
/// Bug: before the fix these produced `Variable{sigil:"$",name:"$ref"}` etc.
/// instead of a `Unary` dereference node matching the braced form.
use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

fn parse_sexp(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    ast.to_sexp()
}

/// Helper: extract the sexp of the first top-level expression statement's
/// expression from a source snippet.
fn first_expr_sexp(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let NodeKind::Program { statements } = &ast.kind else {
        return format!("not-a-program: {}", ast.to_sexp());
    };
    let Some(stmt) = statements.first() else {
        return "no-statements".to_string();
    };
    let NodeKind::ExpressionStatement { expression } = &stmt.kind else {
        return stmt.to_sexp();
    };
    expression.to_sexp()
}

// ---------------------------------------------------------------------------
// Braced forms (correct before and after the fix — used as expected baseline)
// ---------------------------------------------------------------------------

#[test]
fn braced_scalar_deref_is_unary() {
    // ${$ref} must produce (unary_${} (variable $ ref))
    let sexp = first_expr_sexp("${$ref};");
    assert!(sexp.contains("unary_${}"), "braced scalar deref should be Unary: got {sexp}");
    assert!(!sexp.contains("variable $ $ref"), "inner var must not include leading $: got {sexp}");
}

#[test]
fn braced_simple_scalar_name_is_variable_not_deref() {
    let sexp = first_expr_sexp("${sep};");
    assert!(
        sexp.contains("(variable $ sep)"),
        "simple braced scalar should parse as a variable, got {sexp}"
    );
    assert!(
        !sexp.contains("unary_${}"),
        "simple braced scalar must not parse as symbolic deref, got {sexp}"
    );
}

#[test]
fn braced_simple_scalar_name_with_internal_whitespace_both_sides_is_variable_not_deref() {
    // ${ sep } (whitespace on both sides of the name) must fold the same as
    // the no-space form ${sep} — must NOT become a symbolic dereference.
    let sexp = first_expr_sexp("${ sep };");
    assert!(
        sexp.contains("(variable $ sep)"),
        "whitespace-padded braced scalar should parse as a variable, got {sexp}"
    );
    assert!(
        !sexp.contains("unary_${}"),
        "whitespace-padded braced scalar must not parse as symbolic deref, got {sexp}"
    );
}

#[test]
fn braced_simple_scalar_name_with_leading_whitespace_is_variable_not_deref() {
    // ${sep } (whitespace only before the closing brace).
    let sexp = first_expr_sexp("${sep };");
    assert!(
        sexp.contains("(variable $ sep)"),
        "leading-space braced scalar should parse as a variable, got {sexp}"
    );
    assert!(
        !sexp.contains("unary_${}"),
        "leading-space braced scalar must not parse as symbolic deref, got {sexp}"
    );
}

#[test]
fn braced_simple_scalar_name_with_trailing_whitespace_is_variable_not_deref() {
    // ${ sep} (whitespace only after the opening brace).
    let sexp = first_expr_sexp("${ sep};");
    assert!(
        sexp.contains("(variable $ sep)"),
        "trailing-space braced scalar should parse as a variable, got {sexp}"
    );
    assert!(
        !sexp.contains("unary_${}"),
        "trailing-space braced scalar must not parse as symbolic deref, got {sexp}"
    );
}

#[test]
fn braced_array_deref_is_unary() {
    // @{$ref} must produce (unary_@{} (variable $ ref))
    let sexp = first_expr_sexp("@{$ref};");
    assert!(sexp.contains("unary_@{}"), "braced array deref should be Unary: got {sexp}");
}

#[test]
fn braced_hash_deref_is_unary() {
    // %{$ref} must produce (unary_%{} (variable $ ref))
    let sexp = first_expr_sexp("%{$ref};");
    assert!(sexp.contains("unary_%{}"), "braced hash deref should be Unary: got {sexp}");
}

// ---------------------------------------------------------------------------
// Unbraced forms — these are the ones that were wrong before the fix.
// Each must produce the SAME node shape as its braced equivalent.
// ---------------------------------------------------------------------------

#[test]
fn unbraced_scalar_deref_equals_braced() {
    // $$ref and ${$ref} must produce the same sexp
    let unbraced = first_expr_sexp("$$ref;");
    let braced = first_expr_sexp("${$ref};");
    assert_eq!(unbraced, braced, "$$ref should parse identically to ${{$ref}}");
}

#[test]
fn unbraced_scalar_deref_is_unary() {
    // $$ref must be a Unary ${{}} node, not Variable with name="$ref"
    let sexp = first_expr_sexp("$$ref;");
    assert!(sexp.contains("unary_${}"), "$$ref should produce unary_${{}} node, got: {sexp}");
    assert!(
        !sexp.contains("variable $ $ref"),
        "$$ref must NOT produce variable with name $ref, got: {sexp}"
    );
}

#[test]
fn unbraced_array_deref_equals_braced() {
    // @$ref and @{$ref} must produce the same sexp
    let unbraced = first_expr_sexp("@$ref;");
    let braced = first_expr_sexp("@{$ref};");
    assert_eq!(unbraced, braced, "@$ref should parse identically to @{{$ref}}");
}

#[test]
fn unbraced_array_deref_is_unary() {
    // @$ref must be a Unary @{{}} node, not Variable with name="$ref"
    let sexp = first_expr_sexp("@$ref;");
    assert!(sexp.contains("unary_@{}"), "@$ref should produce unary_@{{}} node, got: {sexp}");
    assert!(
        !sexp.contains("variable @ $ref"),
        "@$ref must NOT produce variable with name $ref, got: {sexp}"
    );
}

#[test]
fn unbraced_hash_deref_equals_braced() {
    // %$ref and %{$ref} must produce the same sexp
    let unbraced = first_expr_sexp("%$ref;");
    let braced = first_expr_sexp("%{$ref};");
    assert_eq!(unbraced, braced, "%$ref should parse identically to %{{$ref}}");
}

#[test]
fn unbraced_hash_deref_is_unary() {
    // %$ref must be a Unary %{{}} node, not Variable with name="$ref"
    let sexp = first_expr_sexp("%$ref;");
    assert!(sexp.contains("unary_%{}"), "%$ref should produce unary_%{{}} node, got: {sexp}");
    assert!(
        !sexp.contains("variable % $ref"),
        "%$ref must NOT produce variable with name $ref, got: {sexp}"
    );
}

// ---------------------------------------------------------------------------
// Hash-based OOP access — the real-world impact of this bug.
// $$self{field} is the most common OOP pattern in hash-based Perl OO.
// ---------------------------------------------------------------------------

#[test]
fn unbraced_self_hash_access_equals_braced() {
    // $$self{field} and ${$self}{field} must produce the same sexp
    let unbraced = first_expr_sexp("$$self{field};");
    let braced = first_expr_sexp("${$self}{field};");
    assert_eq!(unbraced, braced, "$$self{{field}} should parse identically to ${{$self}}{{field}}");
}

#[test]
fn unbraced_self_hash_access_is_not_wrong_variable() {
    // $$self{field} must NOT produce (binary_{} (variable $ $self) ...)
    let sexp = first_expr_sexp("$$self{field};");
    assert!(
        !sexp.contains("variable $ $self"),
        "$$self{{field}} must NOT produce variable with name $self, got: {sexp}"
    );
}

// ---------------------------------------------------------------------------
// Regression guards — things that must NOT break.
// ---------------------------------------------------------------------------

#[test]
fn pid_special_var_unchanged() {
    // $$ alone (with space after) is the PID special variable — must stay Variable
    let sexp = first_expr_sexp("$$ + 1;");
    // The $$ here is a Variable, not a Unary
    assert!(sexp.contains("variable"), "$$ (PID) should still parse as a variable, got: {sexp}");
    // Must not misparse the whole expression
    assert!(!sexp.contains("ERROR"), "$$ + 1 must parse cleanly, got: {sexp}");
}

#[test]
fn plain_scalar_var_unchanged() {
    // A plain $ref must still be Variable{sigil:"$", name:"ref"}
    let sexp = first_expr_sexp("$ref;");
    assert_eq!(sexp, "(variable $ ref)", "$ref must still be a plain variable");
}

#[test]
fn plain_array_var_unchanged() {
    // A plain @array must still be Variable
    let sexp = first_expr_sexp("@array;");
    assert!(
        sexp.contains("variable @ array"),
        "@array must still be a plain variable, got: {sexp}"
    );
}

#[test]
fn braced_scalar_deref_no_regression() {
    // Make sure braced deref still works after the fix
    let sexp = first_expr_sexp("${$ref};");
    assert!(sexp.contains("unary_${}"), "${{$ref}} still works after the fix, got: {sexp}");
}

#[test]
fn pid_semicolon_terminates() {
    // $$ followed by ; must be PID, not start of a deref
    let full = parse_sexp("my $pid = $$;");
    assert!(!full.contains("ERROR"), "my $pid = $$ must parse cleanly: {full}");
}

#[test]
fn debug_catfile_unbraced() {
    // Diagnostic: check what $$self parses to in a catfile context
    let unbraced = parse_sexp("@files = map { catfile $$self, $_ } @files;");
    let braced = parse_sexp("@files = map { catfile ${$self}, $_ } @files;");
    // Both should be clean (no ERROR nodes)
    assert!(!unbraced.contains("ERROR"), "unbraced catfile $$self must parse cleanly: {unbraced}");
    assert!(!braced.contains("ERROR"), "braced catfile $${{$self}} must parse cleanly: {braced}");
}

// ---------------------------------------------------------------------------
// Package-qualified braced scalar folding (issue #3593).
//
// Per perlref ("Not-so-symbolic references"), `${Foo::bar}` === `$Foo::bar`
// for any bareword name, including package-qualified ones — the same
// `${name}` == `$name` folding rule as the plain-name case, just with `::`
// segments. Verified against real perl:
//   perl -e '$Foo::bar = 42; print ${Foo::bar}, "\n"; print $Foo::bar, "\n";'
//   -> 42 / 42
//
// Bug: before the fix, `${Foo::bar}` (no internal whitespace) produced
// `(unary_${} (variable $ Foo::bar))` — a symbolic-dereference wrapper —
// instead of folding to the bare scalar `(variable $ Foo::bar)`. Root cause
// was lexer-level: the braced-variable scan didn't consume `::`-delimited
// segments, splitting the token stream as `Identifier("${Foo")`,
// `Operator("::")`, `Identifier("bar")`, `RightBrace`.
// ---------------------------------------------------------------------------

#[test]
fn braced_qualified_scalar_no_space_folds_to_variable() {
    // ${Foo::bar} (no internal whitespace) must fold to (variable $ Foo::bar),
    // not stay a symbolic dereference.
    let sexp = first_expr_sexp("${Foo::bar};");
    assert_eq!(
        sexp, "(variable $ Foo::bar)",
        "${{Foo::bar}} must fold to a plain qualified scalar variable, got {sexp}"
    );
    assert!(!sexp.contains("unary_${}"), "${{Foo::bar}} must NOT be a symbolic deref, got {sexp}");
}

#[test]
fn braced_qualified_scalar_with_whitespace_folds_to_variable() {
    // ${ Foo::bar } (whitespace on both sides) must fold the same way.
    let sexp = first_expr_sexp("${ Foo::bar };");
    assert_eq!(
        sexp, "(variable $ Foo::bar)",
        "${{ Foo::bar }} must fold to a plain qualified scalar variable, got {sexp}"
    );
    assert!(
        !sexp.contains("unary_${}"),
        "${{ Foo::bar }} must NOT be a symbolic deref, got {sexp}"
    );
}

#[test]
fn braced_qualified_scalar_matches_bare_form() {
    // ${Foo::bar} and $Foo::bar must parse identically.
    let braced = first_expr_sexp("${Foo::bar};");
    let bare = first_expr_sexp("$Foo::bar;");
    assert_eq!(braced, bare, "${{Foo::bar}} should parse identically to $Foo::bar");
}

#[test]
fn braced_qualified_scalar_three_segments_folds_to_variable() {
    // Multi-level package paths (Foo::Bar::baz) must fold the same way.
    let sexp = first_expr_sexp("${Foo::Bar::baz};");
    assert_eq!(
        sexp, "(variable $ Foo::Bar::baz)",
        "${{Foo::Bar::baz}} must fold to a plain qualified scalar variable, got {sexp}"
    );
}

// ---------------------------------------------------------------------------
// Regression guards: real dereferences must NOT be affected by the fix.
// ---------------------------------------------------------------------------

#[test]
fn braced_scalar_ref_deref_still_a_dereference_after_qualified_fix() {
    // ${$ref} must remain a symbolic/scalar-ref dereference.
    let sexp = first_expr_sexp("${$ref};");
    assert!(
        sexp.contains("unary_${}"),
        "${{$ref}} must remain a dereference after the qualified-scalar fix, got {sexp}"
    );
}

#[test]
fn braced_array_qualified_deref_still_a_dereference() {
    // @{Foo::bar} must remain an array dereference (not affected by the
    // scalar-only qualified-name fold).
    let sexp = first_expr_sexp("@{Foo::bar};");
    assert!(
        sexp.contains("unary_@{}"),
        "@{{Foo::bar}} must remain an array dereference, got {sexp}"
    );
}

#[test]
fn braced_hash_qualified_deref_still_a_dereference() {
    // %{Foo::bar} must remain a hash dereference (not affected by the
    // scalar-only qualified-name fold).
    let sexp = first_expr_sexp("%{Foo::bar};");
    assert!(sexp.contains("unary_%{}"), "%{{Foo::bar}} must remain a hash dereference, got {sexp}");
}

// ---------------------------------------------------------------------------
// Regression guards: partial-deref/postfix-chain cases must NOT lose the
// qualified name as a variable operand (issue #3939 — the lexer's `::`
// folding above must not swallow `::` when a postfix operator follows the
// qualified name inside the braces, before the closing `}`).
// ---------------------------------------------------------------------------

#[test]
fn braced_qualified_scalar_with_arrow_hash_deref_keeps_variable_operand() {
    // ${Foo::bar->{baz}} must keep `Foo::bar` as a `(variable $ Foo::bar)`
    // operand of the arrow-hash-deref postfix, not lose it to a bareword
    // `(identifier Foo::bar)`.
    let sexp = first_expr_sexp("${Foo::bar->{baz}};");
    assert!(
        sexp.contains("(variable $ Foo::bar)"),
        "${{Foo::bar->{{baz}}}} must keep Foo::bar as a variable operand, got {sexp}"
    );
    assert!(
        !sexp.contains("(identifier Foo::bar)"),
        "${{Foo::bar->{{baz}}}} must not fold Foo::bar into a bareword identifier, got {sexp}"
    );
}

#[test]
fn braced_qualified_scalar_with_subscript_keeps_variable_operand() {
    // ${Foo::bar[0]} — same partial-deref concern via a bare `[...]`
    // subscript (no `->`) instead of an arrow.
    let sexp = first_expr_sexp("${Foo::bar[0]};");
    assert!(
        sexp.contains("(variable $ Foo::bar)"),
        "${{Foo::bar[0]}} must keep Foo::bar as a variable operand, got {sexp}"
    );
    assert!(
        !sexp.contains("(identifier Foo::bar)"),
        "${{Foo::bar[0]}} must not fold Foo::bar into a bareword identifier, got {sexp}"
    );
}

#[test]
fn braced_qualified_scalar_with_trailing_double_colon_reports_error() {
    // ${Foo::} — a `::` with no identifier segment after it. The lexer's
    // qualified_name_closes_brace_from_here() lookahead must NOT treat this
    // as "the `::` chain leads directly to `}`" (it would if the "did we
    // actually consume a segment" guard were dropped, since the next char
    // after `::` really is `}`). Folding here would hand the parser a
    // malformed single token instead of the clean, already-tested
    // "Expected identifier after :: in package-qualified variable"
    // diagnostic that the pre-existing bare $Foo:: handling produces.
    // Verified against a clean origin/main checkout (pre-#3593, before any
    // braced-qualified-scalar folding existed): this exact ERROR is the
    // pre-existing baseline output for `${Foo::};`, so this test pins that
    // the new lookahead doesn't regress this edge case, not a new parser
    // behavior. (`${Foo::}` is itself rare/obscure Perl -- an empty-named
    // variable in package Foo -- and the parser's ERROR-node response to
    // it is a separate, pre-existing limitation unrelated to this fix.)
    let sexp = first_expr_sexp("${Foo::};");
    assert!(
        sexp.contains("Expected identifier after :: in package-qualified variable"),
        "${{Foo::}} must report the standard qualified-name error, got {sexp}"
    );
}

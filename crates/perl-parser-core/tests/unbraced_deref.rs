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

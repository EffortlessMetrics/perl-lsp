//! Mutation-proof boundary tests for five merged parser seams.
//!
//! Each test pins ONE decision boundary so that a single-line mutation of the
//! guarding sub-condition causes exactly that test to fail.  Expectations were
//! derived from the actual binary output of `perl-parse` on `origin/main` after
//! the five parser fixes shipped — never from predicted behaviour.
//!
//! Seam 1 — angle-bracket disambiguation (`primary.rs`, `is_simple_scalar_variable`)
//!   Issue: #741 — `<$fh>` was wrongly classified as Glob; the fix adds an
//!   `is_simple_scalar_variable` guard that returns Readline for bare `$name`.
//!
//! Seam 2 — `parse_class` VERSION token (`declarations.rs`, #711)
//!   Issue: `class Foo 1.0 {}` and `class Foo v1.2.3 {}` were rejected; the
//!   fix consumes the optional version before attributes and body.
//!
//! Seam 3 — builtin call + ternary (`statements.rs`, #715)
//!   Issue: `defined($x) ? $x : "d"` absorbed the ternary inside the call's
//!   arg-list instead of making the ternary the top-level expression.
//!
//! Seam 4 — unbraced deref + indirect-call comma (`variables.rs` + `calls.rs`, #725)
//!   Issue: `$$ref` was parsed as `Variable{sigil:"$",name:"$ref"}` instead of
//!   a `Unary` deref node; `catfile $$self, $_` lost the second argument.
//!
//! Seam 5 — s///e embedded-code marker (`primary.rs` + `quotes.rs`, #975)
//!   Issue: `has_embedded_code` was derived solely from `analyze_regex_body_for_ast`,
//!   which only detects `(?{...})` inline code blocks.  The `e`/`ee` modifiers
//!   evaluate the replacement as Perl code (equivalent to `eval`) but were never
//!   consulted.  Fix: OR in `modifiers.contains('e')` at both originating sites.
//!   The `(risk:code)` sexp marker is the discriminating signal — a mutation
//!   removing `|| modifiers.contains('e')` would make it disappear for s///e.

mod cpan_test_helpers;
use cpan_test_helpers::*;

use perl_parser_core::{NodeKind, Parser};
use perl_tdd_support::must;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Parse `src` and return the sexp string of the whole source_file.
fn sexp(src: &str) -> String {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    ast.to_sexp()
}

/// Parse `src` and return the `NodeKind` of the first top-level expression
/// (unwrapping Program → ExpressionStatement → expression).
fn first_expr_kind(src: &str) -> NodeKind {
    let mut parser = Parser::new(src);
    let ast = must(parser.parse());
    let NodeKind::Program { ref statements } = ast.kind else {
        return NodeKind::MissingExpression;
    };
    let Some(stmt) = statements.first() else {
        return NodeKind::MissingExpression;
    };
    let NodeKind::ExpressionStatement { ref expression } = stmt.kind else {
        return stmt.kind.clone();
    };
    expression.kind.clone()
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEAM 1 — Angle-bracket: Readline vs Glob (primary.rs, is_simple_scalar_variable)
// ═══════════════════════════════════════════════════════════════════════════════

// ── BOUNDARY A: sigil present + bare name → Readline, NOT Glob ───────────────

/// `<$fh>` — the canonical case that triggered the bug.
/// Verified sexp: `(source_file (my_declaration (variable $ x)(readline $fh)))`
/// Pinned boundary: `is_simple_scalar_variable` returns true for `$fh`.
#[test]
fn seam1_angle_simple_scalar_lowercase_is_readline() {
    let s = sexp("my $x = <$fh>;");
    assert!(s.contains("(readline $fh)"), "expected (readline $fh) but got: {s}");
    assert!(!s.contains("(glob"), "<$fh> must NOT be Glob; got: {s}");
}

/// `<$FH>` — uppercase sigilled name is still a simple scalar.
/// Boundary: the sigil `$` alone (not uppercase-without-sigil) determines Readline.
#[test]
fn seam1_angle_simple_scalar_uppercase_sigil_is_readline() {
    let s = sexp("my $x = <$FH>;");
    assert!(s.contains("(readline $FH)"), "expected (readline $FH) but got: {s}");
    assert!(!s.contains("(glob"), "<$FH> must NOT be Glob; got: {s}");
}

/// `<$pattern>` — name happens to look like a glob pattern word, but `$` wins.
/// Boundary: `is_simple_scalar_variable` checked BEFORE glob-metachar scan.
#[test]
fn seam1_angle_simple_scalar_pattern_name_is_readline() {
    let s = sexp("my $x = <$pattern>;");
    assert!(s.contains("(readline $pattern)"), "expected (readline $pattern) but got: {s}");
    assert!(!s.contains("(glob"), "<$pattern> must NOT be Glob; got: {s}");
}

/// `<$Foo::bar>` — package-qualified scalar is still a simple scalar.
/// Boundary: `::` in the name must not disqualify it from Readline.
#[test]
fn seam1_angle_qualified_scalar_is_readline() {
    let s = sexp("my $x = <$Foo::bar>;");
    assert!(s.contains("(readline $Foo::bar)"), "expected (readline $Foo::bar) but got: {s}");
    assert!(!s.contains("(glob"), "<$Foo::bar> must NOT be Glob; got: {s}");
}

// ── BOUNDARY B: glob metacharacter forces Glob even when `$` is present ──────

/// `<$dir/*>` — the `/` and `*` after the scalar name → Glob.
/// Boundary: `is_simple_scalar_variable` returns false when `/` or `*` follow.
#[test]
fn seam1_angle_scalar_with_glob_star_is_glob() {
    let s = sexp(r"my @f = <$dir/*>;");
    assert!(s.contains("(glob"), "<$dir/*> must be Glob; got: {s}");
    assert!(!s.contains("(readline"), "<$dir/*> must NOT be Readline; got: {s}");
}

/// `<$h{key}>` — hash subscript (brace) → Glob.
/// Boundary: `{` inside the angle-bracket content disqualifies simple-scalar.
#[test]
fn seam1_angle_hash_subscript_is_glob() {
    let s = sexp(r"my @v = <$h{key}>;");
    assert!(s.contains("(glob"), "<$h{{key}}> must be Glob; got: {s}");
    assert!(!s.contains("(readline"), "<$h{{key}}> must NOT be Readline; got: {s}");
}

/// `<conf/*.ini>` — no sigil, has glob chars → Glob.
/// Boundary: absence of leading `$` keeps the glob path.
#[test]
fn seam1_angle_bareword_glob_pattern_is_glob() {
    let s = sexp(r"my @f = <conf/*.ini>;");
    assert!(s.contains("(glob conf/*.ini)"), "<conf/*.ini> must be Glob; got: {s}");
    assert!(!s.contains("(readline"), "<conf/*.ini> must NOT be Readline; got: {s}");
}

/// `<*.pm>` — classic glob, no scalar.
/// Boundary: `*` at the start forces Glob path.
#[test]
fn seam1_angle_star_pattern_is_glob() {
    let s = sexp(r"my @f = <*.pm>;");
    assert!(s.contains("(glob *.pm)"), "<*.pm> must be Glob; got: {s}");
    assert!(!s.contains("(readline"), "<*.pm> must NOT be Readline; got: {s}");
}

// ── BOUNDARY C: bareword filehandles → Readline ───────────────────────────────

/// `<STDIN>` — uppercase bareword → Readline (pre-fix path, must not regress).
#[test]
fn seam1_angle_stdin_bareword_is_readline() {
    let s = sexp("my $x = <STDIN>;");
    assert!(s.contains("(readline STDIN)"), "expected (readline STDIN) but got: {s}");
}

/// `<FH>` — short uppercase bareword → Readline (pre-fix path).
#[test]
fn seam1_angle_fh_bareword_is_readline() {
    let s = sexp("my $x = <FH>;");
    assert!(s.contains("(readline FH)"), "expected (readline FH) but got: {s}");
}

// ── BOUNDARY D: diamond operators ────────────────────────────────────────────

/// `<>` — empty angle → Diamond.
/// Boundary: empty content is handled before any scalar/glob check.
#[test]
fn seam1_angle_empty_is_diamond() {
    let kind = first_expr_kind("my $x = <>;");
    // After my_declaration we get the RHS — but first_expr_kind returns the
    // top-level expression of a statement.  Use sexp matching instead.
    let s = sexp("my $x = <>;");
    assert!(s.contains("(diamond)"), "<> must be Diamond; got: {s}");
    assert!(!s.contains("(glob"), "<> must NOT be Glob; got: {s}");
    assert!(!s.contains("(readline"), "<> must NOT be Readline; got: {s}");
    // Suppress unused-variable warning
    let _ = kind;
}

/// `<<>>` — double-diamond → Diamond (#744, same NodeKind::Diamond).
/// Boundary: the `<<>>` lexer token routes to the same Diamond node as `<>`.
#[test]
fn seam1_double_diamond_is_diamond() {
    let s = sexp("my $x = <<>>;");
    assert!(s.contains("(diamond)"), "<<>> must be Diamond; got: {s}");
    assert!(!s.contains("(glob"), "<<>> must NOT be Glob; got: {s}");
    assert!(!s.contains("(readline"), "<<>> must NOT be Readline; got: {s}");
}

// ── NodeKind-level checks: Readline vs Glob (no string matching) ──────────────

/// `<$fh>` — confirm the AST node is literally `NodeKind::Readline`.
/// Pinned against sexp-string tricks; requires the real variant.
#[test]
fn seam1_angle_fh_nodekind_is_readline_variant() {
    let mut parser = Parser::new("my $x = <$fh>;");
    let ast = must(parser.parse());
    let NodeKind::Program { ref statements } = ast.kind else {
        panic!("expected Program");
    };
    let stmt = statements.first().expect("no statements");
    // The my_declaration's RHS child holds the Readline node.
    let NodeKind::VariableDeclaration { ref initializer, .. } = stmt.kind else {
        panic!("expected VariableDeclaration");
    };
    let Some(init_node) = initializer else {
        panic!("initializer missing");
    };
    assert!(
        matches!(init_node.kind, NodeKind::Readline { .. }),
        "expected NodeKind::Readline for <$fh>, got {:?}",
        init_node.kind.kind_name()
    );
}

/// `<$dir/*>` — confirm the AST node is literally `NodeKind::Glob`.
#[test]
fn seam1_angle_dir_star_nodekind_is_glob_variant() {
    let mut parser = Parser::new(r"my @f = <$dir/*>;");
    let ast = must(parser.parse());
    let NodeKind::Program { ref statements } = ast.kind else {
        panic!("expected Program");
    };
    let stmt = statements.first().expect("no statements");
    let NodeKind::VariableDeclaration { ref initializer, .. } = stmt.kind else {
        panic!("expected VariableDeclaration");
    };
    let Some(init_node) = initializer else {
        panic!("initializer missing");
    };
    assert!(
        matches!(init_node.kind, NodeKind::Glob { .. }),
        "expected NodeKind::Glob for <$dir/*>, got {:?}",
        init_node.kind.kind_name()
    );
}

// ── Clean-parse guards ────────────────────────────────────────────────────────

#[test]
fn seam1_all_readline_forms_parse_cleanly() {
    assert_clean_parse("my $x = <$fh>;");
    assert_clean_parse("my $x = <$FH>;");
    assert_clean_parse("my $x = <$pattern>;");
    assert_clean_parse("my $x = <$Foo::bar>;");
    assert_clean_parse("my $x = <STDIN>;");
    assert_clean_parse("my $x = <FH>;");
    assert_clean_parse("my $x = <>;");
    assert_clean_parse("my $x = <<>>;");
}

#[test]
fn seam1_all_glob_forms_parse_cleanly() {
    assert_clean_parse(r"my @f = <*.pm>;");
    assert_clean_parse(r"my @f = <$dir/*>;");
    assert_clean_parse(r"my @v = <$h{key}>;");
    assert_clean_parse(r"my @f = <conf/*.ini>;");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEAM 2 — parse_class VERSION token (declarations.rs, #711)
// ═══════════════════════════════════════════════════════════════════════════════

// ── BOUNDARY A: version forms produce a Class node with a Block ──────────────

/// `class Foo 1.0 {}` — decimal version consumed, Block preserved.
/// Verified sexp: `(source_file (class Foo (block )))`
#[test]
fn seam2_class_decimal_version_produces_class_block() {
    let s = sexp("class Foo 1.0 {}");
    assert!(s.contains("(class Foo"), "expected class Foo node; got: {s}");
    assert!(s.contains("(block"), "class body block must be present; got: {s}");
    assert!(!s.contains("ERROR"), "class with decimal version must not error; got: {s}");
}

/// `class Foo v1.2.3 {}` — v-string version consumed.
#[test]
fn seam2_class_vstring_version_produces_class_block() {
    let s = sexp("class Foo v1.2.3 {}");
    assert!(s.contains("(class Foo"), "expected class Foo node; got: {s}");
    assert!(s.contains("(block"), "class body block must be present; got: {s}");
    assert!(!s.contains("ERROR"), "class with v-string must not error; got: {s}");
}

/// `class Foo 2 {}` — bare integer version consumed.
#[test]
fn seam2_class_integer_version_produces_class_block() {
    let s = sexp("class Foo 2 {}");
    assert!(s.contains("(class Foo"), "expected class Foo node; got: {s}");
    assert!(s.contains("(block"), "class body block must be present; got: {s}");
    assert!(!s.contains("ERROR"), "class with integer version must not error; got: {s}");
}

// ── BOUNDARY B: version token doesn't corrupt body or attributes ──────────────

/// `class Foo 1.0 {}` — parser must emit no errors.
/// Boundary: the version token is fully consumed before body parsing.
#[test]
fn seam2_class_decimal_version_no_parser_errors() {
    let mut parser = Parser::new("class Foo 1.0 {}");
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "class Foo 1.0 must produce no errors; got: {:?}",
        parser.errors()
    );
}

/// `class Foo v1.2.3 {}` — parser must emit no errors.
#[test]
fn seam2_class_vstring_version_no_parser_errors() {
    let mut parser = Parser::new("class Foo v1.2.3 {}");
    let _ast = must(parser.parse());
    assert!(
        parser.errors().is_empty(),
        "class Foo v1.2.3 must produce no errors; got: {:?}",
        parser.errors()
    );
}

/// `class Foo v1.0 :isa(Bar) {}` — version then attribute still works.
/// Boundary: version consumed BEFORE attribute parsing, not instead of it.
#[test]
fn seam2_class_version_then_attribute_no_errors() {
    assert_clean_parse("class Foo v1.0 :isa(Bar) {}");
}

// ── BOUNDARY C: non-versioned forms must not regress ─────────────────────────

/// `class Foo {}` — no version; must still work.
#[test]
fn seam2_class_no_version_unchanged() {
    let s = sexp("class Foo {}");
    assert!(s.contains("(class Foo"), "class without version must produce Class; got: {s}");
    assert!(s.contains("(block"), "class without version must have Block body; got: {s}");
    assert_clean_parse("class Foo {}");
}

/// `class Foo :isa(Bar) {}` — attribute only, no version.
#[test]
fn seam2_class_attribute_no_version_unchanged() {
    let s = sexp("class Foo :isa(Bar) {}");
    assert!(s.contains(":isa(Bar)"), "isa attribute must survive; got: {s}");
    assert!(s.contains("(block"), "class body must be present; got: {s}");
    assert_clean_parse("class Foo :isa(Bar) {}");
}

// ── NodeKind-level check ──────────────────────────────────────────────────────

/// Verify the AST node is literally `NodeKind::Class` with a `NodeKind::Block` body.
#[test]
fn seam2_class_version_nodekind_is_class_with_block() {
    let mut parser = Parser::new("class Foo 1.0 { }");
    let ast = must(parser.parse());
    let NodeKind::Program { ref statements } = ast.kind else {
        panic!("expected Program");
    };
    let class_node = statements
        .iter()
        .find(|s| matches!(s.kind, NodeKind::Class { .. }))
        .expect("expected Class node");
    let NodeKind::Class { ref body, .. } = class_node.kind else {
        panic!("expected Class kind");
    };
    assert!(
        matches!(body.kind, NodeKind::Block { .. }),
        "class body must be Block, got {}",
        body.kind.kind_name()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEAM 3 — Builtin call + ternary (statements.rs, #715)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Key sexp shapes (verified against binary):
//   chr($x) ? 1 : 0    → (ternary (ambiguous_function_call_expression ...) ...)
//   defined($x) ? ...  → (ternary (call defined (...)) ...)
//   ref($x) ? ...      → (ternary (call ref (...)) ...)
//   length($s) ? ...   → (ternary (ambiguous_function_call_expression ...) ...)

// ── BOUNDARY A: ternary is at the root (call is the CONDITION) ────────────────

/// `chr($x) ? 1 : 0` — ternary wraps the call, not the other way around.
/// Boundary: the ternary-check fires AFTER the builtin call is parsed.
#[test]
fn seam3_chr_paren_ternary_at_root() {
    let s = sexp("chr($x) ? 1 : 0;");
    assert!(s.contains("(ternary"), "expected ternary at root for chr($x) ? 1 : 0; got: {s}");
    // The call must NOT absorb the ternary as an argument
    assert!(
        !s.contains("(ambiguous_function_call_expression (function) (ternary"),
        "chr must NOT absorb ternary into args; got: {s}"
    );
}

/// `defined($x) ? $x : "d"` — defined uses `(call defined ...)` format.
#[test]
fn seam3_defined_paren_ternary_at_root() {
    let s = sexp(r#"defined($x) ? $x : "d";"#);
    assert!(s.contains("(ternary"), "expected ternary at root for defined($x) ternary; got: {s}");
    assert!(
        !s.contains("(call defined ((ternary"),
        "defined must NOT absorb ternary into args; got: {s}"
    );
}

/// `ref($x) ? 1 : 0` — ref uses `(call ref ...)` format.
#[test]
fn seam3_ref_paren_ternary_at_root() {
    let s = sexp("ref($x) ? 1 : 0;");
    assert!(s.contains("(ternary"), "expected ternary at root for ref($x) ternary; got: {s}");
    assert!(!s.contains("(call ref ((ternary"), "ref must NOT absorb ternary into args; got: {s}");
}

/// `length($s) ? "a" : "b"` — length uses ambiguous_function_call_expression.
#[test]
fn seam3_length_paren_ternary_at_root() {
    let s = sexp(r#"length($s) ? "a" : "b";"#);
    assert!(s.contains("(ternary"), "expected ternary at root for length($s) ternary; got: {s}");
    assert!(
        !s.contains("(ambiguous_function_call_expression (function) (ternary"),
        "length must NOT absorb ternary into args; got: {s}"
    );
}

// ── BOUNDARY B: call structure is correct inside the ternary condition ────────

/// `chr($x) ? 1 : 0` — the ternary condition is the call, not a bare variable.
/// Boundary: full call node (with `function` child) must be the condition.
#[test]
fn seam3_chr_ternary_condition_is_call() {
    let s = sexp("chr($x) ? 1 : 0;");
    // Verified exact sexp:
    // (ternary (ambiguous_function_call_expression (function) (variable $ x)) (number 1) (number 0))
    assert!(
        s.contains("(ternary (ambiguous_function_call_expression (function) (variable $ x))"),
        "ternary condition must be the chr call; got: {s}"
    );
}

/// `defined($x) ? $x : "d"` — exact ternary structure verified.
#[test]
fn seam3_defined_ternary_exact_sexp() {
    let s = sexp(r#"defined($x) ? $x : "d";"#);
    // Verified exact sexp (outer source_file wrapper stripped by assertion):
    // (ternary (call defined ((variable $ x))) (variable $ x) (string_interpolated "\"d\""))
    assert!(
        s.contains("(ternary (call defined ((variable $ x)))"),
        "defined ternary must have (call defined ...) as condition; got: {s}"
    );
}

// ── BOUNDARY C: builtin without ternary still parses correctly ────────────────

/// `chr($x)` alone — no ternary in output (regression guard).
#[test]
fn seam3_chr_no_ternary_parses_as_call() {
    let s = sexp("chr($x);");
    assert!(!s.contains("(ternary"), "chr($x) alone must NOT produce ternary; got: {s}");
    assert!(
        s.contains("(ambiguous_function_call_expression (function)"),
        "chr($x) must parse as a call; got: {s}"
    );
    assert_clean_parse("chr($x);");
}

/// `defined($x)` alone — no ternary (regression guard).
#[test]
fn seam3_defined_no_ternary_parses_as_call() {
    let s = sexp("defined($x);");
    assert!(!s.contains("(ternary"), "defined($x) alone must NOT produce ternary; got: {s}");
    assert!(s.contains("(call defined"), "defined($x) must parse as call; got: {s}");
    assert_clean_parse("defined($x);");
}

// ── BOUNDARY D: user-defined function already worked — must not regress ───────

/// `foo($x) ? 1 : 0` — user function ternary behaviour must be unchanged.
#[test]
fn seam3_user_func_ternary_at_root_unchanged() {
    let s = sexp("foo($x) ? 1 : 0;");
    assert!(s.contains("(ternary"), "user func ternary must be at root; got: {s}");
}

// ── Clean-parse guard ─────────────────────────────────────────────────────────

#[test]
fn seam3_all_builtin_ternary_forms_parse_cleanly() {
    assert_clean_parse("chr($x) ? 1 : 0;");
    assert_clean_parse(r#"defined($x) ? $x : "d";"#);
    assert_clean_parse("ref($x) ? 1 : 0;");
    assert_clean_parse(r#"length($s) ? "a" : "b";"#);
    assert_clean_parse("foo($x) ? 1 : 0;");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEAM 4 — Unbraced deref + indirect-call comma (variables.rs + calls.rs, #725)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Key sexp shapes verified against the binary:
//   $$ref          → (unary_${} (variable $ ref))
//   @$ref          → (unary_@{} (variable $ ref))
//   %$ref          → (unary_%{} (variable $ ref))
//   $$self{field}  → (binary_{} (unary_${} (variable $ self)) (identifier field))
//   ${$self}{field}→ (binary_{} (unary_${} (variable $ self)) (identifier field))
//   $$             → (variable $ $)   [PID special var]
//   $ref           → (variable $ ref)
//   catfile $$self, $_ → (indirect_call catfile (unary_${} (variable $ self)) ((variable $ _)))

// ── BOUNDARY A: unbraced scalar deref → Unary node (not wrong Variable) ──────

/// `$$ref` — must be Unary dereference, NOT `Variable{name:"$ref"}`.
/// Verified sexp: `(unary_${} (variable $ ref))`
/// Boundary: `$` followed by another `$` triggers unary deref path.
#[test]
fn seam4_unbraced_scalar_deref_is_unary() {
    let s = sexp("$$ref;");
    assert!(
        s.contains("(unary_${} (variable $ ref))"),
        "$$ref must be (unary_${{}} (variable $ ref)); got: {s}"
    );
    assert!(!s.contains("(variable $ $ref)"), "$$ref must NOT be (variable $ $ref); got: {s}");
}

/// `@$ref` — unbraced array deref → Unary.
/// Verified sexp: `(unary_@{} (variable $ ref))`
#[test]
fn seam4_unbraced_array_deref_is_unary() {
    let s = sexp("@$ref;");
    assert!(
        s.contains("(unary_@{} (variable $ ref))"),
        "@$ref must be (unary_@{{}} (variable $ ref)); got: {s}"
    );
    assert!(!s.contains("(variable @ $ref)"), "@$ref must NOT be (variable @ $ref); got: {s}");
}

/// `%$ref` — unbraced hash deref → Unary.
/// Verified sexp: `(unary_%{} (variable $ ref))`
#[test]
fn seam4_unbraced_hash_deref_is_unary() {
    let s = sexp("%$ref;");
    assert!(
        s.contains("(unary_%{} (variable $ ref))"),
        "%$ref must be (unary_%{{}} (variable $ ref)); got: {s}"
    );
    assert!(!s.contains("(variable % $ref)"), "%$ref must NOT be (variable % $ref); got: {s}");
}

// ── BOUNDARY B: unbraced form matches braced form exactly ────────────────────

/// `$$ref` must produce the same sexp as `${$ref}`.
/// Boundary: both paths converge on the same Unary node shape.
#[test]
fn seam4_unbraced_scalar_equals_braced() {
    let unbraced = sexp("$$ref;");
    let braced = sexp("${$ref};");
    assert_eq!(unbraced, braced, "$$ref and ${{$ref}} must produce identical sexp");
}

/// `@$ref` must produce the same sexp as `@{$ref}`.
#[test]
fn seam4_unbraced_array_equals_braced() {
    let unbraced = sexp("@$ref;");
    let braced = sexp("@{$ref};");
    assert_eq!(unbraced, braced, "@$ref and @{{$ref}} must produce identical sexp");
}

/// `%$ref` must produce the same sexp as `%{$ref}`.
#[test]
fn seam4_unbraced_hash_equals_braced() {
    let unbraced = sexp("%$ref;");
    let braced = sexp("%{$ref};");
    assert_eq!(unbraced, braced, "%$ref and %{{$ref}} must produce identical sexp");
}

// ── BOUNDARY C: hash subscript after unbraced deref ──────────────────────────

/// `$$self{field}` — subscript after unbraced deref matches braced form.
/// Verified sexp: `(binary_{} (unary_${} (variable $ self)) (identifier field))`
#[test]
fn seam4_unbraced_self_hash_subscript_matches_braced() {
    let unbraced = sexp("$$self{field};");
    let braced = sexp("${$self}{field};");
    assert_eq!(
        unbraced, braced,
        "$$self{{field}} and ${{$self}}{{field}} must produce identical sexp"
    );
}

/// `$$self{field}` — must not produce `(variable $ $self)` as the deref target.
#[test]
fn seam4_unbraced_self_hash_subscript_not_wrong_variable() {
    let s = sexp("$$self{field};");
    assert!(
        !s.contains("(variable $ $self)"),
        "$$self{{field}} must NOT produce (variable $ $self); got: {s}"
    );
    assert!(
        s.contains("(unary_${} (variable $ self))"),
        "$$self{{field}} must use unary_${{}} node; got: {s}"
    );
}

// ── BOUNDARY D: $$ (PID) special variable must not be affected ───────────────

/// `$$` alone — PID special variable, NOT a deref.
/// Verified sexp: `(variable $ $)`
/// Boundary: `$$` with nothing following is the special var, not `$` + deref.
#[test]
fn seam4_dollar_dollar_pid_is_special_var() {
    let s = sexp("$$;");
    assert!(s.contains("(variable $ $)"), "$$ must be special var (variable $ $); got: {s}");
    assert!(!s.contains("(unary_${}"), "$$ (PID) must NOT be parsed as unary deref; got: {s}");
}

/// `my $pid = $$` — PID in declaration RHS must stay clean.
#[test]
fn seam4_dollar_dollar_pid_in_declaration_clean() {
    let s = sexp("my $pid = $$;");
    assert!(!s.contains("ERROR"), "my $pid = $$ must parse cleanly; got: {s}");
    assert!(s.contains("(variable $ $)"), "my $pid = $$ must preserve PID var; got: {s}");
}

// ── BOUNDARY E: plain scalar unchanged ───────────────────────────────────────

/// `$ref` alone — plain variable, NOT a deref.
/// Boundary: single `$` without a following `$` stays as Variable.
#[test]
fn seam4_plain_scalar_unchanged() {
    let s = sexp("$ref;");
    assert_eq!(
        s, "(source_file (variable $ ref))",
        "$ref must still be a plain variable; got: {s}"
    );
}

// ── BOUNDARY F: indirect call with unbraced deref keeps both arguments ────────

/// `catfile $$self, $_` — indirect call must keep BOTH args.
/// Verified sexp: `(indirect_call catfile (unary_${} (variable $ self)) ((variable $ _)))`
/// Boundary: the comma after `$$self` must not be lost by the deref fix.
#[test]
fn seam4_indirect_call_with_deref_keeps_both_args() {
    let s = sexp("catfile $$self, $_;");
    assert!(
        s.contains("(indirect_call catfile"),
        "catfile call must parse as indirect_call; got: {s}"
    );
    assert!(
        s.contains("(unary_${} (variable $ self))"),
        "catfile first arg must be unary_${{}} deref; got: {s}"
    );
    assert!(s.contains("(variable $ _)"), "catfile second arg $_ must be present; got: {s}");
}

/// `catfile $$self, $_` — exact verified sexp.
#[test]
fn seam4_indirect_call_exact_sexp() {
    let s = sexp("catfile $$self, $_;");
    // Verified against binary: (source_file (indirect_call catfile (unary_${} (variable $ self)) ((variable $ _))))
    assert!(
        s.contains("(indirect_call catfile (unary_${} (variable $ self)) ((variable $ _)))"),
        "catfile sexp mismatch; got: {s}"
    );
}

// ── NodeKind-level checks for deref ──────────────────────────────────────────

/// `$$ref` — confirm the AST node is `NodeKind::Unary`.
#[test]
fn seam4_dollar_dollar_ref_nodekind_is_unary() {
    let kind = first_expr_kind("$$ref;");
    assert!(
        matches!(kind, NodeKind::Unary { .. }),
        "$$ref must be NodeKind::Unary, got {}",
        kind.kind_name()
    );
}

/// `$ref` — confirm the AST node is `NodeKind::Variable`.
#[test]
fn seam4_plain_scalar_nodekind_is_variable() {
    let kind = first_expr_kind("$ref;");
    assert!(
        matches!(kind, NodeKind::Variable { .. }),
        "$ref must be NodeKind::Variable, got {}",
        kind.kind_name()
    );
}

// ── Clean-parse guards ────────────────────────────────────────────────────────

#[test]
fn seam4_all_deref_forms_parse_cleanly() {
    assert_clean_parse("$$ref;");
    assert_clean_parse("@$ref;");
    assert_clean_parse("%$ref;");
    assert_clean_parse("$$self{field};");
    assert_clean_parse("${$self}{field};");
    assert_clean_parse("my $pid = $$;");
    assert_clean_parse("$ref;");
    assert_clean_parse("catfile $$self, $_;");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEAM 5 — s///e embedded-code marker (`primary.rs` + `quotes.rs`, #975)
// ═══════════════════════════════════════════════════════════════════════════════
//
// The `e` modifier evaluates the replacement as Perl code (equiv. to `eval`),
// so the substitution carries embedded code regardless of the pattern body.
// Fix: `has_embedded_code = analyze_regex_body_for_ast(...) || modifiers.contains('e')`
// applied at two sites:
//   primary.rs — s/// as a standalone expression (bound via =~ later)
//   quotes.rs  — s{}{} quote-operator form (no =~)
//
// Discriminating signal: `(risk:code)` marker in the sexp.
// A mutation that removes `|| modifiers.contains('e')` causes every e-modifier
// test below to fail — the `(risk:code)` annotation disappears from the sexp.
//
// Key sexp shapes:
//   $s =~ s/a/b/e;   → (substitution (variable $ s) "a" "b" "e" (risk:code))
//   $s =~ s/a/b/g;   → (substitution (variable $ s) "a" "b" "g")   [no marker]
//   s{a}{b}e;        → (substitution (identifier $_) "a" "b" "e" (risk:code))
//   s{a}{b}g;        → (substitution (identifier $_) "a" "b" "g")   [no marker]

// ── BOUNDARY A: Site 1 (primary.rs) — e modifier → risk:code present ─────────

/// `$s =~ s/a/b/e` — single `e` modifier must emit `(risk:code)` in the sexp.
/// Pinned boundary: `modifiers.contains('e')` true → `has_embedded_code = true`.
/// A mutation removing `|| modifiers.contains('e')` makes this test fail.
#[test]
fn seam5_primary_e_modifier_emits_risk_code_marker() {
    let s = sexp(r#"$s =~ s/a/b/e;"#);
    assert!(s.contains("(risk:code)"), "s///e must emit (risk:code) in sexp; got: {s}");
    assert!(s.contains("(substitution"), "must produce a substitution node; got: {s}");
}

/// `$s =~ s/a/b/ee` — double-eval form must also emit `(risk:code)`.
/// Boundary: `'e' in "ee"` is still true for `modifiers.contains('e')`.
#[test]
fn seam5_primary_ee_modifier_emits_risk_code_marker() {
    let s = sexp(r#"$s =~ s/a/b/ee;"#);
    assert!(s.contains("(risk:code)"), "s///ee must emit (risk:code) in sexp; got: {s}");
}

// ── BOUNDARY B: Site 1 (primary.rs) — no e modifier → risk:code absent ───────

/// `$s =~ s/a/b/g` — no `e` modifier: `(risk:code)` must NOT appear.
/// Boundary: `modifiers.contains('e')` false, pattern has no `(?{...})`.
/// Verifies the guard does not over-trigger.
#[test]
fn seam5_primary_no_e_modifier_no_risk_code_marker() {
    let s = sexp(r#"$s =~ s/a/b/g;"#);
    assert!(!s.contains("(risk:code)"), "s///g must NOT emit (risk:code) in sexp; got: {s}");
    assert!(s.contains("(substitution"), "must still produce a substitution node; got: {s}");
}

// ── BOUNDARY C: Site 2 (quotes.rs) — e modifier → risk:code present ──────────

/// `s{a}{b}e` — brace-delimited form (quotes.rs site) with `e` modifier.
/// Boundary: quotes.rs `|| modifiers.contains('e')` path must also fire.
/// Without the fix at the quotes.rs site, this test would fail independently
/// of the primary.rs fix.
#[test]
fn seam5_quotes_e_modifier_emits_risk_code_marker() {
    let s = sexp(r#"s{a}{b}e;"#);
    assert!(s.contains("(risk:code)"), "s{{}}{{}}e must emit (risk:code) in sexp; got: {s}");
    assert!(s.contains("(substitution"), "must produce a substitution node; got: {s}");
}

/// `s{a}{b}ee` — brace-delimited double-eval form must emit `(risk:code)`.
#[test]
fn seam5_quotes_ee_modifier_emits_risk_code_marker() {
    let s = sexp(r#"s{a}{b}ee;"#);
    assert!(s.contains("(risk:code)"), "s{{}}{{}}ee must emit (risk:code) in sexp; got: {s}");
}

// ── BOUNDARY D: Site 2 (quotes.rs) — no e modifier → risk:code absent ────────

/// `s{a}{b}g` — brace-delimited form with no `e` modifier: `(risk:code)` absent.
/// Verifies the quotes.rs guard does not over-trigger.
#[test]
fn seam5_quotes_no_e_modifier_no_risk_code_marker() {
    let s = sexp(r#"s{a}{b}g;"#);
    assert!(!s.contains("(risk:code)"), "s{{}}{{}}g must NOT emit (risk:code) in sexp; got: {s}");
}

// ── BOUNDARY E: pattern-body `(?{...})` path unchanged ───────────────────────

/// `$s =~ s/(?{1+1})/b/g` — embedded code in pattern body, no `e` modifier.
/// Verifies the original `analyze_regex_body_for_ast` path is not broken.
/// `(risk:code)` must appear because of the `(?{...})`, not because of `e`.
#[test]
fn seam5_pattern_body_embedded_code_unchanged() {
    let s = sexp(r#"$s =~ s/(?{1+1})/b/g;"#);
    assert!(
        s.contains("(risk:code)"),
        "s///g with (?{{...}}) in pattern must still emit (risk:code); got: {s}"
    );
}

// ── Clean-parse guards ────────────────────────────────────────────────────────

#[test]
fn seam5_all_subst_e_forms_parse_cleanly() {
    assert_clean_parse(r#"$s =~ s/a/b/e;"#);
    assert_clean_parse(r#"$s =~ s/a/b/ee;"#);
    assert_clean_parse(r#"$s =~ s/a/b/ge;"#);
    assert_clean_parse(r#"$s =~ s/a/b/g;"#);
    assert_clean_parse(r#"s{a}{b}e;"#);
    assert_clean_parse(r#"s{a}{b}ee;"#);
    assert_clean_parse(r#"s{a}{b}g;"#);
}

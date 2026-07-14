//! Mutation-proof boundary tests for unparenthesized declaration lists.
//!
//! Real Perl declares ONLY the first variable in an unparenthesized `my` /
//! `our` / `state` declaration list. Verified live on `perl 5.42.2`:
//!
//!   $ perl -MO=Deparse,-p -e 'my $a, $b, $c = 1;'
//!   (my($a), $b, ($c = 1));
//!
//! perlsub ("Private Variables via my()"): "If more than one value is
//! listed, the list must be placed in parentheses" and "`my $foo, $bar = 1;`
//! has the same effect as `my $foo; $bar = 1;`". Perl also emits
//! "Parentheses missing around \"my\" list" (perldiag).
//!
//! A comma immediately following the declared variable therefore does NOT
//! extend the declaration — it starts the surrounding comma expression,
//! which the parser represents the same way it represents any other
//! statement-level comma list (`print $a, $b, $c;`): an `ArrayLiteral` of
//! the individual terms, wrapped in an `ExpressionStatement`. Only the
//! first element of that list is a `VariableDeclaration`; the rest are
//! ordinary expression terms.
//!
//! Parenthesized lists (`my ($a, $b) = ...`) are a DIFFERENT grammar form
//! and are unaffected: they still fold every variable into one
//! `NodeKind::VariableListDeclaration`, which is correct Perl.
//!
//! This file previously (incorrectly, per issue #3728) asserted the
//! over-declaring behavior introduced by #3627 — every comma-separated
//! variable folded into the declaration. These assertions are corrected to
//! match real Perl semantics.

mod cpan_test_helpers;

use cpan_test_helpers::{assert_clean_parse, assert_has_error};
use perl_parser_core::hir::{HirKind, lower_ast};
use perl_parser_core::{Node, NodeKind, Parser};

fn parse_program(source: &str) -> Result<Node, String> {
    let mut parser = Parser::new(source);
    parser.parse().map_err(|error| error.to_string())
}

fn statements(source: &str) -> Result<Vec<Node>, String> {
    let ast = parse_program(source)?;
    match ast.kind {
        NodeKind::Program { statements } => Ok(statements),
        other => Err(format!("expected Program node, got {other:?}")),
    }
}

fn variable_name(variable: &Node) -> Result<String, String> {
    match &variable.kind {
        NodeKind::Variable { sigil, name } => Ok(format!("{sigil}{name}")),
        NodeKind::VariableWithAttributes { variable, .. } => variable_name(variable),
        other => Err(format!("expected declared variable, got {other:?}")),
    }
}

/// Unwrap the `ArrayLiteral` elements built by the statement-level comma
/// continuation for `declarator $first, <rest...>;`.
fn comma_list_elements(declaration: &Node) -> Result<&[Node], String> {
    match &declaration.kind {
        NodeKind::ExpressionStatement { expression } => match &expression.kind {
            NodeKind::ArrayLiteral { elements } => Ok(elements),
            other => Err(format!("expected ArrayLiteral, got {other:?}")),
        },
        other => Err(format!("expected ExpressionStatement, got {other:?}")),
    }
}

#[test]
fn unparenthesized_my_list_declares_only_first_variable() -> Result<(), String> {
    let source = "my $exit, $exit_arg = values();";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let program_statements = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    let declaration =
        program_statements.first().ok_or_else(|| "expected declaration statement".to_string())?;

    let elements = comma_list_elements(declaration)?;
    assert_eq!(elements.len(), 2, "expected [my($exit), $exit_arg = values()]");

    // Only `$exit` is declared; it has no initializer of its own (`= values()`
    // belongs to `$exit_arg`, a separate assignment term).
    match &elements[0].kind {
        NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
            assert_eq!(declarator, "my");
            assert_eq!(variable_name(variable)?, "$exit");
            assert!(initializer.is_none(), "$exit itself is not initialized");
        }
        other => return Err(format!("expected VariableDeclaration for $exit, got {other:?}")),
    }

    // `$exit_arg = values()` is an ordinary assignment, NOT a declaration.
    match &elements[1].kind {
        NodeKind::Assignment { lhs, .. } => {
            assert_eq!(variable_name(lhs)?, "$exit_arg");
        }
        other => return Err(format!("expected Assignment for $exit_arg, got {other:?}")),
    }

    // HIR: exactly one `my` declaration, binding only `$exit`.
    let hir = lower_ast(&ast);
    let my_decls = hir
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::VariableDecl(declaration) if declaration.declarator == "my" => {
                Some(declaration)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(my_decls.len(), 1, "expected exactly one my-declaration");
    assert!(!my_decls[0].is_list, "unparenthesized single-variable declaration is not a list");
    assert_eq!(my_decls[0].variables.len(), 1);
    assert_eq!(
        format!("{}{}", my_decls[0].variables[0].sigil, my_decls[0].variables[0].name),
        "$exit"
    );
    Ok(())
}

#[test]
fn our_and_state_declare_only_first_variable() -> Result<(), String> {
    let source = "our $package_name, $package_value; state $cached_name, $cached_value;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let declarations = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    assert_eq!(declarations.len(), 2);

    let mut declared = Vec::new();
    let mut bare = Vec::new();
    for declaration in declarations {
        let elements = comma_list_elements(declaration)?;
        assert_eq!(elements.len(), 2, "expected [declarator $first, $second]");

        match &elements[0].kind {
            NodeKind::VariableDeclaration { declarator, variable, .. } => {
                declared.push((declarator.clone(), variable_name(variable)?));
            }
            other => return Err(format!("expected VariableDeclaration, got {other:?}")),
        }
        bare.push(variable_name(&elements[1])?);
    }

    assert_eq!(
        declared,
        vec![
            ("our".to_string(), "$package_name".to_string()),
            ("state".to_string(), "$cached_name".to_string()),
        ]
    );
    // The second variable in each list is an ordinary (undeclared) reference.
    assert_eq!(bare, vec!["$package_value".to_string(), "$cached_value".to_string()]);

    let hir = lower_ast(&ast);
    let hir_declarators = hir
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            HirKind::VariableDecl(declaration) => {
                Some((declaration.declarator.clone(), declaration.variables.len()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(hir_declarators, vec![("our".to_string(), 1), ("state".to_string(), 1)]);
    Ok(())
}

#[test]
fn initializer_argument_commas_remain_a_single_declaration() -> Result<(), String> {
    let source = "my $only = values($first, $second);";
    assert_clean_parse(source);

    let declarations = statements(source)?;
    let declaration =
        declarations.first().ok_or_else(|| "expected declaration statement".to_string())?;
    match &declaration.kind {
        NodeKind::VariableDeclaration { declarator, initializer, .. } => {
            assert_eq!(declarator, "my");
            assert!(initializer.is_some(), "expected initializer");
        }
        other => return Err(format!("expected VariableDeclaration, got {other:?}")),
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Defect #2 (regressed by #3627, tracked by the issue accompanying this
// fix): `my`/`our`/`state` unconditionally postfix-chained the declared
// variable, so a DIRECT (arrow-less) subscript right after the variable —
// `my $cache[0]`, `my $cache{key}` — was silently accepted as the
// declaration target (an "array/hash element" declaration). Real Perl
// rejects this outright; `my`/`our`/`state` may only declare a whole
// variable, never an element. Ground truth, `perl 5.42.2`:
//
//   $ perl -c -e 'my $cache[0] = 5;'
//   syntax error at -e line 1, near "$cache["
//   $ perl -c -e 'my $cache{key} = 5;'
//   syntax error at -e line 1, near "$cache{key"
//   $ perl -c -e 'our $cache[0] = 5;'
//   syntax error at -e line 1, near "$cache["
//   $ perl -c -e 'use feature "state"; sub f { state $cache[0] = 5; }'
//   syntax error at -e line 1, near "$cache["
//
// This file previously (incorrectly) asserted that
// `my $first[0], $second{key};` parsed CLEANLY with `$first[0]` as the
// declaration's subscripted target — that assertion pinned the bug. Real
// Perl rejects that exact source with a syntax error (verified above), so
// the corrected test asserts rejection instead.
#[test]
fn unparenthesized_declaration_with_direct_subscript_is_rejected() {
    // Bare `$cache[0]` (array-element subscript, no arrow) directly after
    // the declared variable must be rejected, not folded into the
    // declaration target.
    assert_has_error("my $cache[0] = 5;", "Can't declare array element");
    // Bare `$cache{key}` (hash-element subscript, no arrow) is likewise
    // rejected.
    assert_has_error("my $cache{key} = 5;", "Can't declare hash element");
    // Same rule applies to `our` and `state`, matching the oracle above.
    assert_has_error("our $cache[0] = 5;", "Can't declare array element");
    assert_has_error("state $cache[0] = 5;", "Can't declare array element");
    // The exact source this test previously mis-asserted as a clean parse.
    assert_has_error("my $first[0], $second{key};", "Can't declare array element");
}

// Guard: the ARROW-postfix form is a genuinely different (and valid) Perl
// idiom — `my $cache->{key} = ...` / `my $cache->[0] = ...` declare a plain
// lexical scalar and then autovivify through it via `->`. This is NOT an
// element declaration and must keep parsing cleanly after the fix above.
// Ground truth:
//
//   $ perl -c -e 'my $cache->{key} = [1,2,3];'
//   -e syntax OK
//   $ perl -c -e 'my $cache->[0] = 5;'
//   -e syntax OK
#[test]
fn arrow_postfix_after_declared_variable_still_parses_cleanly() -> Result<(), String> {
    assert_clean_parse("my $cache->{key} = [1,2,3];");
    assert_clean_parse("my $cache->[0] = 5;");

    let declarations = statements("my $cache->[0] = 5;")?;
    let declaration =
        declarations.first().ok_or_else(|| "expected declaration statement".to_string())?;
    match &declaration.kind {
        NodeKind::VariableDeclaration { declarator, variable, .. } => {
            assert_eq!(declarator, "my");
            assert!(
                matches!(&variable.kind, NodeKind::Binary { op, .. } if op == "->[]"),
                "expected arrow-chained subscript target, got {:?}",
                variable.kind
            );
        }
        other => return Err(format!("expected VariableDeclaration, got {other:?}")),
    }
    Ok(())
}

// NOTE (issue #3742): the fixture below is INVALID Perl, not a valid
// multi-declaration form. Ground truth, `perl 5.42.2`:
//
//   $ perl -c -e 'my $first :Attr, %second;'
//   Invalid separator character ',' in attribute list at -e line 1, near "$first :Attr"
//   syntax error at -e line 1, near "$first :Attr"
//   -e had compilation errors.
//
// Real Perl attribute lists are separated by `:` (or whitespace before the
// next `:name`), never by a bare `,` — a comma directly after an attribute
// name is a syntax error, not a continuation into a second declared
// variable. `assert_clean_parse` below is verifying an AST-shape property
// (no Error/Missing nodes) of the parser's current PERMISSIVE recovery for
// this malformed input, not that the source is syntactically valid Perl.
// Inherited unchanged from the #3627-era file; do not read this as an
// oracle-verified valid-Perl fixture. See `attributed_first_variable_fixture_documents_invalid_perl_status`
// below, which pins this comment against silent removal.
#[test]
fn attributed_first_variable_declares_only_that_variable_with_its_attribute() -> Result<(), String>
{
    let source = "my $first :Attr, %second;";
    assert_clean_parse(source);

    let ast = parse_program(source)?;
    let declarations = match &ast.kind {
        NodeKind::Program { statements } => statements,
        other => return Err(format!("expected Program node, got {other:?}")),
    };
    let declaration =
        declarations.first().ok_or_else(|| "expected declaration statement".to_string())?;
    let elements = comma_list_elements(declaration)?;
    assert_eq!(elements.len(), 2, "expected [my $first :Attr, %second]");

    match &elements[0].kind {
        NodeKind::VariableDeclaration { declarator, variable, attributes, .. } => {
            assert_eq!(declarator, "my");
            assert_eq!(variable_name(variable)?, "$first");
            assert_eq!(attributes, &["Attr".to_string()]);
        }
        other => return Err(format!("expected VariableDeclaration, got {other:?}")),
    }
    // `%second` is a bare (undeclared) reference, not part of the declaration.
    assert_eq!(variable_name(&elements[1])?, "%second");

    let hir = lower_ast(&ast);
    let my_decl = hir.items.iter().find_map(|item| match &item.kind {
        HirKind::VariableDecl(declaration) if declaration.declarator == "my" => Some(declaration),
        _ => None,
    });
    let my_decl = my_decl.ok_or_else(|| "expected my-declaration HIR item".to_string())?;
    assert_eq!(my_decl.variables.len(), 1);
    assert_eq!(my_decl.attribute_count, 1);
    Ok(())
}

/// Regression guard for issue #3742: `my $first :Attr, %second;` is invalid
/// Perl (`perl -c` rejects it with "Invalid separator character ',' in
/// attribute list"), yet the test above exercises the parser's permissive
/// recovery on it via `assert_clean_parse`. Without the explanatory comment
/// directly above that test, a reader could mistake the fixture for a
/// verified-valid multi-declaration form. This meta-test fails if that
/// label comment is ever silently dropped or detached from the test it
/// documents.
#[test]
fn attributed_first_variable_fixture_documents_invalid_perl_status() -> Result<(), String> {
    let own_source = include_str!("ripr_seam_proof_unparenthesized_declarations.rs");
    let label_marker = "NOTE (issue #3742): the fixture below is INVALID Perl";
    let oracle_marker = "Invalid separator character ',' in attribute list at -e line 1";
    let fn_marker = "fn attributed_first_variable_declares_only_that_variable_with_its_attribute";

    let label_pos = own_source
        .find(label_marker)
        .ok_or_else(|| "expected the #3742 invalid-Perl label comment to be present".to_string())?;
    let oracle_pos = own_source.find(oracle_marker).ok_or_else(|| {
        "expected the perl -c oracle output to be quoted in the comment".to_string()
    })?;
    let fn_pos = own_source.find(fn_marker).ok_or_else(|| {
        "expected the attributed_first_variable test fn to be present".to_string()
    })?;

    assert!(
        label_pos < oracle_pos && oracle_pos < fn_pos,
        "expected the invalid-Perl label comment (with its perl -c oracle quote) to \
         immediately precede the attributed_first_variable test function"
    );
    Ok(())
}

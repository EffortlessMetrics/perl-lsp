//! Targeted branch-coverage tests for `perl-symbol`.
//!
//! These tests fill the three specific gap areas identified from llvm-cov:
//!   - `surface/ref.rs`:  71.88% branch (9 of 32 missed)
//!   - `surface/decl.rs`: 79.69% branch (13 of 64 missed)
//!   - `cursor/mod.rs`:   81.58% branch (14 of 76 missed)
//!
//! No production code is modified; this file only adds tests.

use perl_ast::{GotoTargetForm, Node, NodeKind, SourceLocation};
use perl_symbol::SymbolKind;
use perl_symbol::VarKind;
use perl_symbol::cursor::{
    CursorSymbolKind, extract_symbol_from_source, get_symbol_range_at_position, is_word_boundary,
    token_under_cursor,
};
use perl_symbol::surface::{SymbolDecl, SymbolRefKind, extract_symbol_decls, extract_symbol_refs};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// ── helpers ────────────────────────────────────────────────────────────────────

fn loc(start: usize, end: usize) -> SourceLocation {
    SourceLocation { start, end }
}

// ══════════════════════════════════════════════════════════════════════════════
// surface/ref.rs — branch gaps
// ══════════════════════════════════════════════════════════════════════════════

// ── static_method_target: empty identifier name → None ─────────────────────
//
// `static_method_target` guards against `name.is_empty()` (line 179 True-branch).
// When the Identifier node has an empty name the function must return `None` so
// the call is treated as an instance-method call, not a static one.

#[test]
fn static_method_target_empty_identifier_name_gives_instance_call() -> Result<()> {
    // object is an Identifier with an empty name — simulates a degenerate AST
    // that the guard at `static_method_target` must reject.
    let object = Node::new(NodeKind::Identifier { name: String::new() }, loc(0, 0));
    let call = Node::new(
        NodeKind::MethodCall { object: Box::new(object), method: "run".to_string(), args: vec![] },
        loc(0, 5),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(0, 5));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    // Must fall back to MethodCall (not StaticMethodCall) because name is empty.
    assert_eq!(refs[0].kind, SymbolRefKind::MethodCall);
    assert_eq!(refs[0].name, "run");
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

// ── static_method_target: empty method name → None ─────────────────────────
//
// `static_method_target` also guards `method.is_empty()` (line 179 rhs True-branch).

#[test]
fn static_method_target_empty_method_name_gives_instance_call() -> Result<()> {
    let object = Node::new(NodeKind::Identifier { name: "MyPkg".to_string() }, loc(0, 5));
    let call = Node::new(
        NodeKind::MethodCall {
            object: Box::new(object),
            method: String::new(), // degenerate: empty method name
            args: vec![],
        },
        loc(0, 7),
    );
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(0, 7));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    // Empty method name → static_method_target returns None → MethodCall.
    assert_eq!(refs[0].kind, SymbolRefKind::MethodCall);
    assert_eq!(refs[0].name, "");
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

// ── var_kind_from_sigil: hash sigil → VarKind::Hash ─────────────────────────
//
// The `%` branch (line 266) was executed 0 times in the baseline.

#[test]
fn hash_sigil_variable_ref_emits_varkind_hash() -> Result<()> {
    let var = Node::new(
        NodeKind::Variable { sigil: "%".to_string(), name: "opts".to_string() },
        loc(0, 5),
    );
    let program = Node::new(NodeKind::Program { statements: vec![var] }, loc(0, 5));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::Variable(VarKind::Hash));
    assert_eq!(refs[0].name, "opts");
    assert_eq!(refs[0].sigil.as_deref(), Some("%"));
    Ok(())
}

// ── var_kind_from_sigil: unknown sigil → skipped (None branch) ──────────────
//
// The `_` arm returning `None` (line 267) was executed 0 times.
// A Variable node with a sigil that doesn't map to any known VarKind must
// be silently skipped (not pushed into the output vec).

#[test]
fn variable_with_unknown_sigil_is_silently_skipped() -> Result<()> {
    // `?` is not a real Perl sigil — var_kind_from_sigil returns None → no ref emitted.
    let unknown_sigil_var = Node::new(
        NodeKind::Variable { sigil: "?".to_string(), name: "weird".to_string() },
        loc(0, 6),
    );
    // Place a valid scalar ref alongside so we can tell the walker still runs.
    let valid_var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "valid".to_string() },
        loc(7, 13),
    );
    let program =
        Node::new(NodeKind::Program { statements: vec![unknown_sigil_var, valid_var] }, loc(0, 13));

    let refs = extract_symbol_refs(&program);
    // Only the valid variable ref should appear; the unknown-sigil one is skipped.
    assert_eq!(refs.len(), 1, "unknown sigil must be silently skipped");
    assert_eq!(refs[0].name, "valid");
    Ok(())
}

// ── coderef_target_name: Variable with & sigil ──────────────────────────────
//
// Branch (227:47) in `coderef_target_name` — the `sigil == "&"` guard on
// a plain Variable node — was executed 0 times. This exercises it through
// `push_coderef_target` via the `Goto` handler.

#[test]
fn goto_with_ampersand_variable_is_classified_as_coderef() -> Result<()> {
    // `goto &foo` where the target is an `&`-sigil Variable (not a FunctionCall).
    let amp_var = Node::new(
        NodeKind::Variable { sigil: "&".to_string(), name: "handler".to_string() },
        loc(5, 13),
    );
    let goto = Node::new(
        NodeKind::Goto { target: Box::new(amp_var), form: GotoTargetForm::Sub },
        loc(0, 13),
    );
    let program = Node::new(NodeKind::Program { statements: vec![goto] }, loc(0, 13));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, SymbolRefKind::CoderefReference);
    assert_eq!(refs[0].name, "handler");
    assert_eq!(refs[0].sigil.as_deref(), Some("&"));
    Ok(())
}

// ── split_qualified_name: empty package component ───────────────────────────
//
// `"::bar"` has an empty package part → the `!package.is_empty()` guard
// at line 251 is False → falls through to bare-name path.

#[test]
fn qualified_name_with_empty_package_component_treated_as_bare() -> Result<()> {
    let var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "::bar".to_string() },
        loc(0, 6),
    );
    let program = Node::new(NodeKind::Program { statements: vec![var] }, loc(0, 6));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    // `::bar` splits as package="" bare="bar"; empty package → treat as bare.
    assert_eq!(refs[0].name, "::bar");
    assert_eq!(refs[0].qualified_name, "::bar");
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

// ── split_qualified_name: trailing `::` empty bare component ────────────────
//
// `"Foo::"` has an empty bare part → the `!bare.is_empty()` guard at line 252
// is False → falls through to bare-name path.

#[test]
fn qualified_name_with_trailing_colons_treated_as_bare() -> Result<()> {
    let call =
        Node::new(NodeKind::FunctionCall { name: "Foo::".to_string(), args: vec![] }, loc(0, 5));
    let program = Node::new(NodeKind::Program { statements: vec![call] }, loc(0, 5));

    let refs = extract_symbol_refs(&program);
    assert_eq!(refs.len(), 1);
    // `Foo::` splits as package="Foo" bare=""; empty bare → treat as bare.
    assert_eq!(refs[0].name, "Foo::");
    assert_eq!(refs[0].qualified_name, "Foo::");
    assert_eq!(refs[0].package_qualifier, None);
    Ok(())
}

// ── VariableListDeclaration walks initializer via walk() ────────────────────
//
// The `VariableListDeclaration { initializer, .. }` arm in the refs walker
// was exercised, but let's explicitly add a test that verifies the
// initializer branch (Some vs None) both work for refs extraction.

#[test]
fn variable_list_declaration_initializer_refs_are_walked() -> Result<()> {
    // `my ($a, $b) = @source`
    let var_a =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "a".to_string() }, loc(4, 6));
    let var_b =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "b".to_string() }, loc(8, 10));
    let init = Node::new(
        NodeKind::Variable { sigil: "@".to_string(), name: "source".to_string() },
        loc(14, 21),
    );
    let list_decl = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var_a, var_b],
            attributes: vec![],
            initializer: Some(Box::new(init)),
        },
        loc(0, 21),
    );
    let program = Node::new(NodeKind::Program { statements: vec![list_decl] }, loc(0, 21));

    let refs = extract_symbol_refs(&program);
    // Only the initializer `@source` should be a ref; $a and $b are declaration targets.
    assert_eq!(refs.len(), 1, "only the initializer variable ref must appear");
    assert_eq!(refs[0].kind, SymbolRefKind::Variable(VarKind::Array));
    assert_eq!(refs[0].name, "source");
    Ok(())
}

// ── VariableListDeclaration with no initializer emits no refs ───────────────

#[test]
fn variable_list_declaration_without_initializer_emits_no_refs() -> Result<()> {
    // `my ($x, $y);` — no initializer, no refs.
    let var_x =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc(4, 6));
    let var_y =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "y".to_string() }, loc(8, 10));
    let list_decl = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var_x, var_y],
            attributes: vec![],
            initializer: None,
        },
        loc(0, 11),
    );
    let program = Node::new(NodeKind::Program { statements: vec![list_decl] }, loc(0, 11));

    let refs = extract_symbol_refs(&program);
    assert!(refs.is_empty(), "no initializer → no refs");
    Ok(())
}

// ── NamedParameter is skipped (declaration, not ref) ────────────────────────
//
// Exercises the `NamedParameter` arm in the refs walker.

#[test]
fn named_parameter_node_is_not_emitted_as_ref() -> Result<()> {
    let param_var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "key".to_string() },
        loc(8, 12),
    );
    let named_param =
        Node::new(NodeKind::NamedParameter { variable: Box::new(param_var) }, loc(8, 12));
    let sig = Node::new(NodeKind::Signature { parameters: vec![named_param] }, loc(7, 13));
    let body = Node::new(NodeKind::Block { statements: vec![] }, loc(14, 16));
    let sub_node = Node::new(
        NodeKind::Subroutine {
            name: Some("process".to_string()),
            name_span: None,
            declarator: None,
            prototype: None,
            signature: Some(Box::new(sig)),
            attributes: vec![],
            body: Box::new(body),
        },
        loc(0, 16),
    );
    let program = Node::new(NodeKind::Program { statements: vec![sub_node] }, loc(0, 16));

    let refs = extract_symbol_refs(&program);
    let named_refs: Vec<_> = refs.iter().filter(|r| r.name == "key").collect();
    assert!(
        named_refs.is_empty(),
        "NamedParameter is a declaration site; must not appear as a ref, got: {:?}",
        refs,
    );
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// surface/decl.rs — branch gaps
// ══════════════════════════════════════════════════════════════════════════════

// ── push_const_fast_decl: VariableListDeclaration arm ───────────────────────
//
// `const my ($A, $B) = (1, 2)` — the list-declaration variant of the
// `const` constant wrapper. Lines 416-424 (push_const_fast_decl / list arm)
// were executed 0 times.

#[test]
fn const_fast_list_declaration_emits_multiple_constants() -> Result<()> {
    let var_a =
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "A".to_string() }, loc(9, 11));
    let var_b = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "B".to_string() },
        loc(13, 15),
    );
    let list_decl = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var_a, var_b],
            attributes: vec![],
            initializer: None,
        },
        loc(6, 16),
    );
    // `use Const::Fast;`
    let use_const_fast = Node::new(
        NodeKind::Use { module: "Const::Fast".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 17),
    );
    // `const my ($A, $B) = ...`
    let const_call = Node::new(
        NodeKind::FunctionCall { name: "const".to_string(), args: vec![list_decl] },
        loc(18, 38),
    );
    let program =
        Node::new(NodeKind::Program { statements: vec![use_const_fast, const_call] }, loc(0, 38));

    let decls = extract_symbol_decls(&program, None);
    let constant_decls: Vec<&SymbolDecl> =
        decls.iter().filter(|d| d.kind == SymbolKind::Constant).collect();
    assert_eq!(constant_decls.len(), 2, "both constants from list decl must be projected");
    let names: Vec<&str> = constant_decls.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"A"), "A must be projected");
    assert!(names.contains(&"B"), "B must be projected");
    Ok(())
}

// ── push_readonly_decl: VariableListDeclaration arm ─────────────────────────
//
// `Readonly my ($X, $Y) => (10, 20)` — exercises lines 438-446
// (push_readonly_decl / list arm), executed 0 times in baseline.

#[test]
fn readonly_list_declaration_emits_multiple_constants() -> Result<()> {
    let var_x = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "X".to_string() },
        loc(12, 14),
    );
    let var_y = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "Y".to_string() },
        loc(16, 18),
    );
    let list_decl = Node::new(
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![var_x, var_y],
            attributes: vec![],
            initializer: None,
        },
        loc(9, 19),
    );
    let use_readonly = Node::new(
        NodeKind::Use { module: "Readonly".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 20),
    );
    let readonly_call = Node::new(
        NodeKind::FunctionCall { name: "Readonly".to_string(), args: vec![list_decl] },
        loc(21, 45),
    );
    let program =
        Node::new(NodeKind::Program { statements: vec![use_readonly, readonly_call] }, loc(0, 45));

    let decls = extract_symbol_decls(&program, None);
    let constant_decls: Vec<&SymbolDecl> =
        decls.iter().filter(|d| d.kind == SymbolKind::Constant).collect();
    assert_eq!(constant_decls.len(), 2, "both constants from Readonly list decl must be projected");
    let names: Vec<&str> = constant_decls.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"X"), "X must be projected");
    assert!(names.contains(&"Y"), "Y must be projected");
    Ok(())
}

// ── constant_wrapper_decl_from_node: VariableWithAttributes arm ─────────────
//
// Line 471: `NodeKind::VariableWithAttributes` arm inside
// `constant_wrapper_decl_from_node` was executed 0 times. Exercises the
// recursive unwrapping path.

#[test]
fn const_fast_variable_with_attributes_is_unwrapped() -> Result<()> {
    let inner_var = Node::new(
        NodeKind::Variable { sigil: "$".to_string(), name: "MAGIC".to_string() },
        loc(9, 15),
    );
    let var_with_attrs = Node::new(
        NodeKind::VariableWithAttributes {
            variable: Box::new(inner_var),
            attributes: vec!["shared".to_string()],
        },
        loc(6, 15),
    );
    let var_decl = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(var_with_attrs),
            attributes: vec![],
            initializer: None,
        },
        loc(3, 15),
    );
    let use_const_fast = Node::new(
        NodeKind::Use { module: "Const::Fast".to_string(), args: vec![], has_filter_risk: false },
        loc(0, 16),
    );
    let const_call = Node::new(
        NodeKind::FunctionCall { name: "const".to_string(), args: vec![var_decl] },
        loc(17, 35),
    );
    let program =
        Node::new(NodeKind::Program { statements: vec![use_const_fast, const_call] }, loc(0, 35));

    let decls = extract_symbol_decls(&program, None);
    let constant_decls: Vec<&SymbolDecl> =
        decls.iter().filter(|d| d.kind == SymbolKind::Constant).collect();
    assert_eq!(constant_decls.len(), 1, "MAGIC must be projected through VariableWithAttributes");
    assert_eq!(constant_decls[0].name, "MAGIC");
    Ok(())
}

// ── variable_decl_from_node: `_` arm (returns None) ────────────────────────
//
// Line 518 `_ => None` in `variable_decl_from_node` — when the child of a
// VariableDeclaration is neither Variable nor VariableWithAttributes, the
// function returns `None` and no decl is emitted. This happens in practice
// with degenerate ASTs.

#[test]
fn variable_decl_with_non_variable_child_emits_no_decl() -> Result<()> {
    // A Number node as the "variable" of a VariableDeclaration is degenerate but
    // lets us reach the `_ => None` arm in `variable_decl_from_node`.
    let number_child = Node::new(NodeKind::Number { value: "42".to_string() }, loc(3, 5));
    let decl_node = Node::new(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(number_child),
            attributes: vec![],
            initializer: None,
        },
        loc(0, 5),
    );
    let program = Node::new(NodeKind::Program { statements: vec![decl_node] }, loc(0, 5));

    let decls = extract_symbol_decls(&program, None);
    assert!(
        decls.is_empty(),
        "degenerate variable child must produce no SymbolDecl, got: {:?}",
        decls,
    );
    Ok(())
}

// ── is_constant_name_candidate: rejects args starting with `{`, `}` ─────────
//
// Lines 395-396 (starts_with('{') / starts_with('}') False branches) were
// executed 0 times for the False path. Exact `{` and `}` tokens are handled as
// depth markers before candidate filtering, so this test also includes
// malformed brace-prefixed arguments that must reach and fail the candidate
// predicate.

#[test]
fn constant_names_rejects_brace_prefixed_args() -> Result<()> {
    // `use constant { FOO => 1, BAR => 2 };` where the brace tokens appear in `args`.
    // Exact braces are depth markers; malformed brace-prefixed entries must not
    // become constant names.
    let use_node = Node::new(
        NodeKind::Use {
            module: "constant".to_string(),
            args: vec![
                "{".to_string(),
                "{BAD".to_string(),
                ",".to_string(),
                "FOO".to_string(),
                "=>".to_string(),
                "1".to_string(),
                ",".to_string(),
                "}BAD".to_string(),
                ",".to_string(),
                "BAR".to_string(),
                "=>".to_string(),
                "2".to_string(),
                "}".to_string(),
            ],
            has_filter_risk: false,
        },
        loc(0, 30),
    );
    let program = Node::new(NodeKind::Program { statements: vec![use_node] }, loc(0, 30));

    let decls = extract_symbol_decls(&program, None);
    let const_decls: Vec<&SymbolDecl> =
        decls.iter().filter(|d| d.kind == SymbolKind::Constant).collect();
    assert_eq!(const_decls.len(), 2, "must extract FOO and BAR");
    let names: Vec<&str> = const_decls.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"FOO"));
    assert!(names.contains(&"BAR"));
    Ok(())
}

// ── is_constant_name_candidate: rejects `-`, `$`, `@`, `%` prefixes ─────────
//
// Lines 397-400 False branches — each flag-like and sigil prefix must not be
// treated as a constant name candidate.

#[test]
fn constant_names_rejects_flag_and_sigil_prefixed_args() -> Result<()> {
    // `use constant VALID => 1` alongside spurious args with sigil/flag prefixes.
    let use_node = Node::new(
        NodeKind::Use {
            module: "constant".to_string(),
            args: vec![
                "VALID".to_string(),
                "=>".to_string(),
                "$scalar".to_string(),
                "@array".to_string(),
                "%hash".to_string(),
                "-flag".to_string(),
            ],
            has_filter_risk: false,
        },
        loc(0, 40),
    );
    let program = Node::new(NodeKind::Program { statements: vec![use_node] }, loc(0, 40));

    let decls = extract_symbol_decls(&program, None);
    let const_decls: Vec<&SymbolDecl> =
        decls.iter().filter(|d| d.kind == SymbolKind::Constant).collect();
    // Only VALID should be picked as a constant name.
    assert_eq!(const_decls.len(), 1, "only VALID must be a constant, got: {:?}", const_decls);
    assert_eq!(const_decls[0].name, "VALID");
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// cursor/mod.rs — branch gaps
// ══════════════════════════════════════════════════════════════════════════════

// ── token_under_cursor: empty line returns None ──────────────────────────────
//
// `bytes.is_empty()` guard (line 123) was executed 0 times for the True path.

#[test]
fn token_under_cursor_empty_line_returns_none() {
    // A text with a blank line; targeting line 0 which is empty.
    let text = "\nsecond line\n";
    let result = token_under_cursor(text, 0, 0);
    assert_eq!(result, None, "empty line must return None");
}

// ── token_under_cursor: cursor at end of line snaps back ─────────────────────
//
// `byte_pos >= bytes.len()` (line 130 else branch) — cursor is past the
// last byte of the line, so `anchor` snaps to `bytes.len() - 1`.

#[test]
fn token_under_cursor_cursor_past_end_snaps_back_to_last_char() {
    // Line is "foo" (3 bytes). UTF-16 column 3 equals byte offset 3 = bytes.len().
    let text = "foo";
    // col 3 is exactly bytes.len() for ASCII; `anchor` must snap to len-1.
    let result = token_under_cursor(text, 0, 3);
    assert_eq!(result, Some("foo".to_string()), "past-end cursor must snap and extract token");
}

// ── token_under_cursor: cursor on non-modchar, adjacent modchar ──────────────
//
// Line 135: `else if anchor > 0 && is_modchar(bytes[anchor - 1])` — executed
// 0 times in baseline. Triggered when `bytes[anchor]` is not a modchar or
// sigil but `bytes[anchor - 1]` is.

#[test]
fn token_under_cursor_cursor_on_delimiter_after_identifier() {
    // "abc(" — cursor on '(' (col 3). '(' is neither modchar nor sigil;
    // but 'c' at col 2 IS a modchar → snap to anchor-1.
    let text = "abc(";
    let result = token_under_cursor(text, 0, 3);
    assert_eq!(result, Some("abc".to_string()), "cursor after identifier must snap left");
}

// ── token_under_cursor: cursor on space after identifier → None ──────────────
//
// The `else { return None }` branch (line 138) fires when neither condition
// in the if/else-if is satisfied.

#[test]
fn token_under_cursor_cursor_on_space_not_adjacent_to_token_returns_none() {
    // "  x" — cursor at col 0 (space), anchor > 0 is false, so the second
    // condition `anchor > 0 && is_modchar(bytes[anchor - 1])` is also false → None.
    let text = "  x";
    let result = token_under_cursor(text, 0, 0);
    assert_eq!(result, None, "space at start of line must return None");
}

// ── get_symbol_range_at_position: sigil with underscore name ─────────────────
//
// Line 80 branch (80:65) `chars[end] == '_'` True path was 0 times.
// The inner loop in `get_symbol_range_at_position` exits on non-alnum
// non-underscore, but the `_ ==` True arm within the loop was not exercised.

#[test]
fn get_symbol_range_underscore_name_includes_underscores() -> Result<()> {
    let source = "$my_var = 1";
    // position 1 = 'm' (first char of name after '$')
    let (start, end) = get_symbol_range_at_position(1, source)
        .ok_or("get_symbol_range_at_position must return Some for a valid identifier")?;
    assert_eq!((start, end), (0, 7), "range must include '$my_var' exactly");
    Ok(())
}

// ── is_word_boundary: pos > 0 and preceding char is modchar → false ──────────
//
// Line 170: `pos > 0 && is_modchar(text[pos - 1])` True-branch in
// `is_word_boundary` was 0 times for one of the executions.

#[test]
fn word_boundary_embedded_word_not_at_boundary() {
    // "foobar" — "bar" at offset 3, preceded by 'o' which is alphanumeric → not a boundary.
    let text = b"foobar";
    assert!(
        !is_word_boundary(text, 3, "bar".len()),
        "'bar' inside 'foobar' is not at a word boundary"
    );
}

// ── is_word_boundary: end_pos < text.len() and following char is modchar → false

#[test]
fn word_boundary_word_followed_by_modchar_not_at_boundary() {
    // "foo::" — "foo" at offset 0, followed by ':' which is modchar → not a boundary.
    let text = b"foo::bar";
    // "foo" at position 0, length 3; text[3] = ':' which is_modchar → false.
    assert!(!is_word_boundary(text, 0, 3), "'foo' followed by '::' is not at a word boundary");
}

// ── extract_symbol_from_source: position 0 with no sigil, bare name ──────────
//
// Additional coverage for the `else` branch when `position == 0` and there's
// no sigil at position-1 (because there's no previous char).

#[test]
fn extract_symbol_at_position_zero_without_sigil() {
    // cursor at 0, no previous char, no sigil at current char — bare word
    let source = "hello";
    let result = extract_symbol_from_source(0, source);
    assert_eq!(result, Some(("hello".to_string(), CursorSymbolKind::Subroutine)));
}

// ── extract_symbol_from_source: sigil at position-1, cursor in name ──────────
//
// Exercises the `position > 0` branch with all four sigil chars in the
// preceding position, in addition to the no-sigil predecessor case.

#[test]
fn extract_symbol_percent_sigil_before_position() -> Result<()> {
    // cursor on the 'c' in %cfg — position 1, sigil '%' at 0
    let source = "%cfg";
    let (name, kind) = extract_symbol_from_source(1, source)
        .ok_or("extract_symbol_from_source must return Some for %cfg at position 1")?;
    assert_eq!(name, "cfg");
    assert_eq!(kind, CursorSymbolKind::Hash);
    Ok(())
}

// ── byte_offset_utf16: units > col_utf16 mid-surrogate path ─────────────────
//
// Line 110 `if units > col_utf16` True branch was executed 0 times in
// baseline (test 1). This path fires when col_utf16 falls inside a
// multi-unit surrogate pair (col points to the low surrogate).

#[test]
fn byte_offset_utf16_col_inside_surrogate_pair_returns_char_start() {
    // "A😀B" — 😀 (U+1F600) encodes as 2 UTF-16 units.
    // col 0 → byte 0 (A)
    // col 1 → byte 1 (start of 😀, 4 bytes)
    // col 2 → still inside 😀 — the `units > col_utf16` branch fires here
    //         and should return byte 1 (the start of the emoji).
    // col 3 → byte 5 (B)
    use perl_symbol::cursor::byte_offset_utf16;
    let line = "A😀B";
    // col 2 lands mid-surrogate; must return byte 1 (the emoji start).
    assert_eq!(byte_offset_utf16(line, 2), 1, "mid-surrogate column must point to char start");
}

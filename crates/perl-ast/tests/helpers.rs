//! Shared test infrastructure for perl-ast integration tests.
//!
//! [`all_nodekind_instances`] returns one representative instance of every
//! `NodeKind` variant. It is the **single source of truth** for all-variant
//! fixture vecs used across integration test files. Each test binary that
//! needs it includes this module via:
//!
//! ```
//! #[path = "helpers.rs"]
//! mod helpers;
//! ```
//!
//! # Maintenance
//!
//! When adding a new `NodeKind` variant:
//! 1. Add it to the `NodeKind` enum in `src/ast.rs`.
//! 2. Update `kind_name()` match arm in `src/ast.rs` (compiler-enforced).
//! 3. Update `ALL_KIND_NAMES` slice in `src/ast.rs`.
//! 4. Add one instance here — this is the **only** fixture update needed
//!    across all integration tests.
#![allow(dead_code)]

use perl_ast::ast::{Node, NodeKind, SourceLocation};

pub fn loc() -> SourceLocation {
    SourceLocation { start: 0, end: 0 }
}

/// A minimal placeholder `Node` used to satisfy required child fields.
/// Field values are intentionally empty — callers that need semantically
/// meaningful content should construct their own nodes.
pub fn dummy() -> Node {
    Node::new(NodeKind::Undef, loc())
}

/// Returns one representative instance of every [`NodeKind`] variant.
///
/// Instances use empty/minimal field values — enough to identify the variant,
/// not meant to represent valid Perl. Tests that depend on specific field
/// content should build their own fixtures.
pub fn all_nodekind_instances() -> Vec<NodeKind> {
    vec![
        NodeKind::Program { statements: vec![] },
        NodeKind::ExpressionStatement { expression: Box::new(dummy()) },
        NodeKind::VariableDeclaration {
            declarator: String::new(),
            variable: Box::new(dummy()),
            attributes: vec![],
            initializer: None,
        },
        NodeKind::VariableListDeclaration {
            declarator: String::new(),
            variables: vec![],
            attributes: vec![],
            initializer: None,
        },
        NodeKind::NestedVariableList { items: vec![] },
        NodeKind::Variable { sigil: String::new(), name: String::new() },
        NodeKind::VariableWithAttributes { variable: Box::new(dummy()), attributes: vec![] },
        NodeKind::Assignment { lhs: Box::new(dummy()), rhs: Box::new(dummy()), op: String::new() },
        NodeKind::Binary { op: String::new(), left: Box::new(dummy()), right: Box::new(dummy()) },
        NodeKind::Ternary {
            condition: Box::new(dummy()),
            then_expr: Box::new(dummy()),
            else_expr: Box::new(dummy()),
        },
        NodeKind::Unary { op: String::new(), operand: Box::new(dummy()) },
        NodeKind::Diamond,
        NodeKind::Ellipsis,
        NodeKind::Undef,
        NodeKind::Readline { filehandle: None },
        NodeKind::Glob { pattern: String::new() },
        NodeKind::Typeglob { name: String::new() },
        NodeKind::Number { value: String::new() },
        NodeKind::String { value: String::new(), interpolated: false },
        NodeKind::Heredoc {
            delimiter: String::new(),
            content: String::new(),
            interpolated: false,
            indented: false,
            command: false,
            body_span: None,
        },
        NodeKind::ArrayLiteral { elements: vec![] },
        NodeKind::HashLiteral { pairs: vec![] },
        NodeKind::Block { statements: vec![] },
        NodeKind::Eval { block: Box::new(dummy()) },
        NodeKind::Do { block: Box::new(dummy()) },
        NodeKind::Defer { block: Box::new(dummy()) },
        NodeKind::Try { body: Box::new(dummy()), catch_blocks: vec![], finally_block: None },
        NodeKind::If {
            condition: Box::new(dummy()),
            then_branch: Box::new(dummy()),
            elsif_branches: vec![],
            else_branch: None,
            keyword: None,
        },
        NodeKind::LabeledStatement { label: String::new(), statement: Box::new(dummy()) },
        NodeKind::While {
            condition: Box::new(dummy()),
            body: Box::new(dummy()),
            continue_block: None,
            keyword: None,
        },
        NodeKind::Tie { variable: Box::new(dummy()), package: Box::new(dummy()), args: vec![] },
        NodeKind::Untie { variable: Box::new(dummy()) },
        NodeKind::For {
            init: None,
            condition: None,
            update: None,
            body: Box::new(dummy()),
            continue_block: None,
        },
        NodeKind::Foreach {
            variable: Box::new(dummy()),
            list: Box::new(dummy()),
            body: Box::new(dummy()),
            continue_block: None,
        },
        NodeKind::Given { expr: Box::new(dummy()), body: Box::new(dummy()) },
        NodeKind::When { condition: Box::new(dummy()), body: Box::new(dummy()) },
        NodeKind::Default { body: Box::new(dummy()) },
        NodeKind::StatementModifier {
            statement: Box::new(dummy()),
            modifier: String::new(),
            condition: Box::new(dummy()),
        },
        NodeKind::Subroutine {
            name: None,
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(dummy()),
        },
        NodeKind::Prototype { content: String::new() },
        NodeKind::Signature { parameters: vec![] },
        NodeKind::MandatoryParameter { variable: Box::new(dummy()) },
        NodeKind::OptionalParameter {
            variable: Box::new(dummy()),
            default_value: Box::new(dummy()),
        },
        NodeKind::SlurpyParameter { variable: Box::new(dummy()) },
        NodeKind::NamedParameter { variable: Box::new(dummy()) },
        NodeKind::Method {
            name: String::new(),
            signature: None,
            attributes: vec![],
            body: Box::new(dummy()),
        },
        NodeKind::Return { value: None },
        NodeKind::LoopControl { op: String::new(), label: None },
        NodeKind::Goto { target: Box::new(dummy()) },
        NodeKind::MethodCall { object: Box::new(dummy()), method: String::new(), args: vec![] },
        NodeKind::FunctionCall { name: String::new(), args: vec![] },
        NodeKind::IndirectCall { method: String::new(), object: Box::new(dummy()), args: vec![] },
        NodeKind::Regex {
            pattern: String::new(),
            replacement: None,
            modifiers: String::new(),
            has_embedded_code: false,
        },
        NodeKind::Match {
            expr: Box::new(dummy()),
            pattern: String::new(),
            modifiers: String::new(),
            has_embedded_code: false,
            negated: false,
        },
        NodeKind::Substitution {
            expr: Box::new(dummy()),
            pattern: String::new(),
            replacement: String::new(),
            modifiers: String::new(),
            has_embedded_code: false,
            negated: false,
        },
        NodeKind::Transliteration {
            expr: Box::new(dummy()),
            search: String::new(),
            replace: String::new(),
            modifiers: String::new(),
            negated: false,
        },
        NodeKind::Package { name: String::new(), name_span: loc(), block: None },
        NodeKind::Use { module: String::new(), args: vec![], has_filter_risk: false },
        NodeKind::No { module: String::new(), args: vec![], has_filter_risk: false },
        NodeKind::PhaseBlock { phase: String::new(), phase_span: None, block: Box::new(dummy()) },
        NodeKind::DataSection { marker: String::new(), body: None },
        NodeKind::Class { name: String::new(), parents: vec![], body: Box::new(dummy()) },
        NodeKind::Format { name: String::new(), body: String::new() },
        NodeKind::Identifier { name: String::new() },
        NodeKind::Error { message: String::new(), expected: vec![], found: None, partial: None },
        NodeKind::MissingExpression,
        NodeKind::MissingStatement,
        NodeKind::MissingIdentifier,
        NodeKind::MissingBlock,
        NodeKind::UnknownRest,
    ]
}

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
//! 1. Add it to the `NodeKind` enum in `perl-ast`.
//! 2. Add a representative instance here.
//! 3. All tests that use `all_nodekind_instances()` automatically cover the new variant.

use perl_ast::{Node, NodeKind, SourceLocation};

fn loc() -> SourceLocation {
    SourceLocation { start: 0, end: 1 }
}

fn loc2(s: usize, e: usize) -> SourceLocation {
    SourceLocation { start: s, end: e }
}

fn num(v: &str) -> Node {
    Node::new(NodeKind::Number { value: v.to_string() }, loc())
}

fn var(sigil: &str, name: &str) -> Node {
    Node::new(NodeKind::Variable { sigil: sigil.to_string(), name: name.to_string() }, loc())
}

fn block() -> Node {
    Node::new(NodeKind::Block { statements: vec![] }, loc())
}

/// Returns one representative instance of every `NodeKind` variant.
///
/// Used by coverage tests to ensure every variant is exercised.
#[allow(dead_code)]
pub fn all_nodekind_instances() -> Vec<Node> {
    vec![
        Node::new(NodeKind::Number { value: "42".to_string() }, loc()),
        Node::new(NodeKind::String { value: "hello".to_string(), interpolated: false }, loc()),
        Node::new(NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }, loc()),
        Node::new(NodeKind::Identifier { name: "foo".to_string() }, loc()),
        Node::new(NodeKind::Block { statements: vec![] }, loc()),
        Node::new(NodeKind::Program { statements: vec![] }, loc()),
        Node::new(
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(var("$", "x")),
                attributes: vec![],
                initializer: None,
            },
            loc(),
        ),
        Node::new(
            NodeKind::VariableListDeclaration {
                declarator: "my".to_string(),
                variables: vec![],
                attributes: vec![],
                initializer: None,
            },
            loc(),
        ),
        Node::new(
            NodeKind::Binary {
                op: "+".to_string(),
                left: Box::new(num("1")),
                right: Box::new(num("2")),
            },
            loc(),
        ),
        Node::new(NodeKind::Unary { op: "-".to_string(), operand: Box::new(num("1")) }, loc()),
        Node::new(
            NodeKind::If {
                condition: Box::new(num("1")),
                then_branch: Box::new(block()),
                elsif_branches: vec![],
                else_branch: None,
                keyword: None,
            },
            loc(),
        ),
        Node::new(
            NodeKind::While {
                condition: Box::new(num("1")),
                body: Box::new(block()),
                continue_block: None,
                keyword: None,
            },
            loc(),
        ),
        Node::new(
            NodeKind::For {
                init: None,
                condition: None,
                update: None,
                body: Box::new(block()),
                continue_block: None,
            },
            loc(),
        ),
        Node::new(
            NodeKind::Foreach {
                variable: Box::new(var("$", "i")),
                list: Box::new(var("@", "arr")),
                body: Box::new(block()),
                continue_block: None,
            },
            loc(),
        ),
        Node::new(
            NodeKind::Subroutine {
                name: Some("foo".to_string()),
                name_span: None,
                declarator: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block()),
            },
            loc(),
        ),
        Node::new(NodeKind::Return { value: Some(Box::new(num("1"))) }, loc()),
        Node::new(NodeKind::FunctionCall { name: "print".to_string(), args: vec![] }, loc()),
        Node::new(
            NodeKind::MethodCall {
                object: Box::new(var("$", "obj")),
                method: "run".to_string(),
                args: vec![],
            },
            loc(),
        ),
        Node::new(
            NodeKind::Assignment {
                lhs: Box::new(var("$", "x")),
                rhs: Box::new(num("1")),
                op: "=".to_string(),
            },
            loc(),
        ),
        Node::new(
            NodeKind::Ternary {
                condition: Box::new(num("1")),
                then_expr: Box::new(num("2")),
                else_expr: Box::new(num("3")),
            },
            loc(),
        ),
        Node::new(NodeKind::ArrayLiteral { elements: vec![] }, loc()),
        Node::new(NodeKind::HashLiteral { pairs: vec![] }, loc()),
        Node::new(
            NodeKind::Use { module: "strict".to_string(), args: vec![], has_filter_risk: false },
            loc(),
        ),
        Node::new(
            NodeKind::No { module: "warnings".to_string(), args: vec![], has_filter_risk: false },
            loc(),
        ),
        Node::new(
            NodeKind::Package { name: "Foo".to_string(), name_span: loc2(8, 11), block: None },
            loc(),
        ),
        Node::new(NodeKind::Eval { block: Box::new(block()) }, loc()),
        Node::new(NodeKind::Do { block: Box::new(block()) }, loc()),
        Node::new(
            NodeKind::Try { body: Box::new(block()), catch_blocks: vec![], finally_block: None },
            loc(),
        ),
        Node::new(
            NodeKind::Regex {
                pattern: "foo".to_string(),
                replacement: None,
                modifiers: "".to_string(),
                has_embedded_code: false,
            },
            loc(),
        ),
        Node::new(
            NodeKind::Heredoc {
                delimiter: "EOF".to_string(),
                content: "".to_string(),
                interpolated: false,
                indented: false,
                command: false,
                body_span: None,
            },
            loc(),
        ),
        Node::new(NodeKind::LoopControl { op: "next".to_string(), label: None }, loc()),
        Node::new(
            NodeKind::Error {
                message: "err".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            loc(),
        ),
        Node::new(NodeKind::MissingExpression, loc()),
        Node::new(NodeKind::MissingStatement, loc()),
        Node::new(NodeKind::MissingIdentifier, loc()),
        Node::new(NodeKind::MissingBlock, loc()),
        Node::new(NodeKind::UnknownRest, loc()),
        Node::new(NodeKind::Diamond, loc()),
        Node::new(NodeKind::Ellipsis, loc()),
        Node::new(NodeKind::Undef, loc()),
        Node::new(NodeKind::Readline { filehandle: None }, loc()),
        Node::new(NodeKind::Glob { pattern: "*".to_string() }, loc()),
        Node::new(NodeKind::Typeglob { name: "foo".to_string() }, loc()),
        Node::new(NodeKind::DataSection { marker: "__DATA__".to_string(), body: None }, loc()),
        Node::new(
            NodeKind::Class { name: "Foo".to_string(), parents: vec![], body: Box::new(block()) },
            loc(),
        ),
        Node::new(NodeKind::ExpressionStatement { expression: Box::new(num("1")) }, loc()),
        Node::new(
            NodeKind::StatementModifier {
                statement: Box::new(num("1")),
                modifier: "if".to_string(),
                condition: Box::new(num("1")),
            },
            loc(),
        ),
        Node::new(
            NodeKind::LabeledStatement {
                label: "LOOP".to_string(),
                statement: Box::new(Node::new(
                    NodeKind::While {
                        condition: Box::new(num("1")),
                        body: Box::new(block()),
                        continue_block: None,
                        keyword: None,
                    },
                    loc(),
                )),
            },
            loc(),
        ),
        Node::new(NodeKind::Given { expr: Box::new(num("1")), body: Box::new(block()) }, loc()),
        Node::new(NodeKind::When { condition: Box::new(num("1")), body: Box::new(block()) }, loc()),
        Node::new(NodeKind::Default { body: Box::new(block()) }, loc()),
        Node::new(
            NodeKind::PhaseBlock {
                phase: "BEGIN".to_string(),
                phase_span: None,
                block: Box::new(block()),
            },
            loc(),
        ),
        Node::new(
            NodeKind::Match {
                expr: Box::new(var("$", "x")),
                pattern: "foo".to_string(),
                modifiers: "".to_string(),
                has_embedded_code: false,
                negated: false,
            },
            loc(),
        ),
        Node::new(
            NodeKind::Substitution {
                expr: Box::new(var("$", "x")),
                pattern: "old".to_string(),
                replacement: "new".to_string(),
                modifiers: "".to_string(),
                has_embedded_code: false,
                negated: false,
            },
            loc(),
        ),
        Node::new(
            NodeKind::Transliteration {
                expr: Box::new(var("$", "x")),
                search: "a".to_string(),
                replace: "b".to_string(),
                modifiers: "".to_string(),
                negated: false,
            },
            loc(),
        ),
        Node::new(
            NodeKind::IndirectCall {
                method: "new".to_string(),
                object: Box::new(Node::new(
                    NodeKind::Identifier { name: "Foo".to_string() },
                    loc(),
                )),
                args: vec![],
            },
            loc(),
        ),
        Node::new(NodeKind::Signature { parameters: vec![] }, loc()),
        Node::new(NodeKind::MandatoryParameter { variable: Box::new(var("$", "x")) }, loc()),
        Node::new(
            NodeKind::OptionalParameter {
                variable: Box::new(var("$", "x")),
                default_value: Box::new(num("0")),
            },
            loc(),
        ),
        Node::new(NodeKind::SlurpyParameter { variable: Box::new(var("@", "rest")) }, loc()),
        Node::new(
            NodeKind::Tie {
                variable: Box::new(var("%", "h")),
                package: Box::new(Node::new(
                    NodeKind::Identifier { name: "DB_File".to_string() },
                    loc(),
                )),
                args: vec![],
            },
            loc(),
        ),
        Node::new(NodeKind::Untie { variable: Box::new(var("%", "h")) }, loc()),
        Node::new(NodeKind::Format { name: "STDOUT".to_string(), body: "".to_string() }, loc()),
        Node::new(NodeKind::NestedVariableList { items: vec![] }, loc()),
        Node::new(
            NodeKind::VariableWithAttributes {
                variable: Box::new(var("$", "x")),
                attributes: vec![":lvalue".to_string()],
            },
            loc(),
        ),
        Node::new(NodeKind::Defer { block: Box::new(block()) }, loc()),
        Node::new(NodeKind::Prototype { content: "$@".to_string() }, loc()),
        Node::new(NodeKind::NamedParameter { variable: Box::new(var("$", "x")) }, loc()),
        Node::new(
            NodeKind::Method {
                name: "foo".to_string(),
                signature: None,
                attributes: vec![],
                body: Box::new(block()),
            },
            loc(),
        ),
        Node::new(NodeKind::Goto { target: Box::new(var("$", "sub_ref")) }, loc()),
    ]
}

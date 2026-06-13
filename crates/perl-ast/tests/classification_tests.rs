// Acceptance tests for the NodeKindCategory + NodeKindFlags classification API.
//
// These tests were written BEFORE the implementation (TDD red phase).
// They encode the spec table from issue #911 plan-review comment id 4583604229.
//
// safe_for_breakpoint rule (plan-reviewer corrected):
//   false = recovery_artifact || "pure variant-level classification says no"
//   The corrected table has 43 true / 26 false.
//
// All assertions are deterministic (no randomness, no parser invocation).

use perl_ast::classification::NodeKindCategory;
use perl_ast::{Node, NodeKind, SourceLocation};

fn loc() -> SourceLocation {
    SourceLocation::new(0, 1)
}

fn leaf() -> Node {
    Node::new(NodeKind::Identifier { name: "x".to_string() }, loc())
}

fn block_node() -> Node {
    Node::new(NodeKind::Block { statements: vec![] }, loc())
}

// ────────────────────────────────────────────────────────
// Helper: produce one representative of each NodeKind variant
// ────────────────────────────────────────────────────────

fn all_variants() -> Vec<NodeKind> {
    vec![
        NodeKind::Program { statements: vec![] },
        NodeKind::ExpressionStatement { expression: Box::new(leaf()) },
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(leaf()),
            attributes: vec![],
            initializer: None,
        },
        NodeKind::VariableListDeclaration {
            declarator: "my".to_string(),
            variables: vec![],
            attributes: vec![],
            initializer: None,
        },
        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
        NodeKind::VariableWithAttributes { variable: Box::new(leaf()), attributes: vec![] },
        NodeKind::Assignment { lhs: Box::new(leaf()), rhs: Box::new(leaf()), op: "=".to_string() },
        NodeKind::Binary { op: "+".to_string(), left: Box::new(leaf()), right: Box::new(leaf()) },
        NodeKind::Ternary {
            condition: Box::new(leaf()),
            then_expr: Box::new(leaf()),
            else_expr: Box::new(leaf()),
        },
        NodeKind::Unary { op: "-".to_string(), operand: Box::new(leaf()) },
        NodeKind::Diamond,
        NodeKind::Ellipsis,
        NodeKind::Undef,
        NodeKind::Readline { filehandle: None },
        NodeKind::Glob { pattern: "*.pl".to_string() },
        NodeKind::Typeglob { name: "foo".to_string() },
        NodeKind::Number { value: "42".to_string() },
        NodeKind::String { value: "hello".to_string(), interpolated: false },
        NodeKind::Heredoc {
            delimiter: "EOF".to_string(),
            content: "body".to_string(),
            interpolated: false,
            indented: false,
            command: false,
            body_span: None,
        },
        NodeKind::ArrayLiteral { elements: vec![] },
        NodeKind::HashLiteral { pairs: vec![] },
        NodeKind::Block { statements: vec![] },
        NodeKind::Eval { block: Box::new(block_node()) },
        NodeKind::Do { block: Box::new(block_node()) },
        NodeKind::Defer { block: Box::new(block_node()) },
        NodeKind::Try { body: Box::new(block_node()), catch_blocks: vec![], finally_block: None },
        NodeKind::If {
            condition: Box::new(leaf()),
            then_branch: Box::new(block_node()),
            elsif_branches: vec![],
            else_branch: None,
            keyword: None,
        },
        NodeKind::LabeledStatement {
            label: "OUTER".to_string(),
            statement: Box::new(Node::new(
                NodeKind::LoopControl { op: "next".to_string(), label: None },
                loc(),
            )),
        },
        NodeKind::While {
            condition: Box::new(leaf()),
            body: Box::new(block_node()),
            continue_block: None,
            keyword: None,
        },
        NodeKind::Tie { variable: Box::new(leaf()), package: Box::new(leaf()), args: vec![] },
        NodeKind::Untie { variable: Box::new(leaf()) },
        NodeKind::For {
            init: None,
            condition: None,
            update: None,
            body: Box::new(block_node()),
            continue_block: None,
        },
        NodeKind::Foreach {
            variable: Box::new(leaf()),
            list: Box::new(leaf()),
            body: Box::new(block_node()),
            continue_block: None,
        },
        NodeKind::Given { expr: Box::new(leaf()), body: Box::new(block_node()) },
        NodeKind::When { condition: Box::new(leaf()), body: Box::new(block_node()) },
        NodeKind::Default { body: Box::new(block_node()) },
        NodeKind::StatementModifier {
            statement: Box::new(leaf()),
            modifier: "if".to_string(),
            condition: Box::new(leaf()),
        },
        NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block_node()),
        },
        NodeKind::Prototype { content: "$@".to_string() },
        NodeKind::Signature { parameters: vec![] },
        NodeKind::MandatoryParameter { variable: Box::new(leaf()) },
        NodeKind::OptionalParameter { variable: Box::new(leaf()), default_value: Box::new(leaf()) },
        NodeKind::SlurpyParameter { variable: Box::new(leaf()) },
        NodeKind::NamedParameter { variable: Box::new(leaf()) },
        NodeKind::Method {
            name: "bar".to_string(),
            signature: None,
            attributes: vec![],
            body: Box::new(block_node()),
        },
        NodeKind::Return { value: None },
        NodeKind::LoopControl { op: "next".to_string(), label: None },
        NodeKind::Goto { target: Box::new(leaf()) },
        NodeKind::MethodCall { object: Box::new(leaf()), method: "foo".to_string(), args: vec![] },
        NodeKind::FunctionCall { name: "print".to_string(), args: vec![] },
        NodeKind::IndirectCall {
            method: "new".to_string(),
            object: Box::new(leaf()),
            args: vec![],
        },
        NodeKind::Regex {
            pattern: "foo".to_string(),
            replacement: None,
            modifiers: "".to_string(),
            has_embedded_code: false,
        },
        NodeKind::Match {
            expr: Box::new(leaf()),
            pattern: "foo".to_string(),
            modifiers: "".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        NodeKind::Substitution {
            expr: Box::new(leaf()),
            pattern: "foo".to_string(),
            replacement: "bar".to_string(),
            modifiers: "".to_string(),
            has_embedded_code: false,
            negated: false,
        },
        NodeKind::Transliteration {
            expr: Box::new(leaf()),
            search: "a".to_string(),
            replace: "b".to_string(),
            modifiers: "".to_string(),
            negated: false,
        },
        NodeKind::Package { name: "Foo".to_string(), name_span: loc(), block: None },
        NodeKind::Use { module: "strict".to_string(), args: vec![], has_filter_risk: false },
        NodeKind::No { module: "strict".to_string(), args: vec![], has_filter_risk: false },
        NodeKind::PhaseBlock {
            phase: "BEGIN".to_string(),
            phase_span: None,
            block: Box::new(block_node()),
        },
        NodeKind::DataSection { marker: "__DATA__".to_string(), body: None },
        NodeKind::Class { name: "Foo".to_string(), parents: vec![], body: Box::new(block_node()) },
        NodeKind::Format { name: "STDOUT".to_string(), body: "".to_string() },
        NodeKind::Identifier { name: "foo".to_string() },
        NodeKind::Error {
            message: "oops".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        },
        NodeKind::MissingExpression,
        NodeKind::MissingStatement,
        NodeKind::MissingIdentifier,
        NodeKind::MissingBlock,
        NodeKind::UnknownRest,
    ]
}

// ────────────────────────────────────────────────────────
// Test (a): every variant returns a category and flags without panicking
// ────────────────────────────────────────────────────────

#[test]
fn all_variants_have_category_and_flags() {
    for kind in all_variants() {
        let _cat = kind.category();
        let flags = kind.flags();
        // flags.validate() is checked separately
        let _ = flags;
    }
}

// ────────────────────────────────────────────────────────
// Test (b): recovery_artifact == true implies safe_for_breakpoint == false
// ────────────────────────────────────────────────────────

#[test]
fn recovery_implies_not_safe_for_breakpoint() {
    for kind in all_variants() {
        let flags = kind.flags();
        if flags.recovery_artifact {
            assert!(
                !flags.safe_for_breakpoint,
                "variant {:?} has recovery_artifact=true but safe_for_breakpoint=true",
                kind.kind_name(),
            );
        }
    }
}

// ────────────────────────────────────────────────────────
// Test (c): exact safe_for_breakpoint true/false sets from the spec table
// ────────────────────────────────────────────────────────

/// The exact set of variant names that must be safe_for_breakpoint=TRUE
/// per the plan-reviewer corrected table (43 variants).
const SAFE_FOR_BREAKPOINT_TRUE: &[&str] = &[
    "ExpressionStatement",
    "VariableDeclaration",
    "VariableListDeclaration",
    "Assignment",
    "Binary",
    "Ternary",
    "Unary",
    "Diamond",
    "Readline",
    "Glob",
    "Heredoc",
    "Block",
    "Eval",
    "Do",
    "Defer",
    "Try",
    "If",
    "LabeledStatement",
    "While",
    "Tie",
    "Untie",
    "For",
    "Foreach",
    "Given",
    "When",
    "Default",
    "StatementModifier",
    "Subroutine",
    "Method",
    "Return",
    "LoopControl",
    "Goto",
    "MethodCall",
    "FunctionCall",
    "IndirectCall",
    "Match",
    "Substitution",
    "Transliteration",
    "Package",
    "Use",
    "No",
    "PhaseBlock",
    "Class",
];

/// The exact set of variant names that must be safe_for_breakpoint=FALSE
/// per the plan-reviewer corrected table (26 variants).
const SAFE_FOR_BREAKPOINT_FALSE: &[&str] = &[
    "Program",
    "Variable",
    "VariableWithAttributes",
    "Ellipsis",
    "Undef",
    "Number",
    "String",
    "ArrayLiteral",
    "HashLiteral",
    "Typeglob",
    "Regex",
    "Prototype",
    "Signature",
    "MandatoryParameter",
    "OptionalParameter",
    "SlurpyParameter",
    "NamedParameter",
    "DataSection",
    "Format",
    "Identifier",
    // Recovery nodes (6)
    "Error",
    "MissingExpression",
    "MissingStatement",
    "MissingIdentifier",
    "MissingBlock",
    "UnknownRest",
];

#[test]
fn safe_for_breakpoint_exact_true_set() {
    for kind in all_variants() {
        let name = kind.kind_name();
        let flags = kind.flags();
        if SAFE_FOR_BREAKPOINT_TRUE.contains(&name) {
            assert!(
                flags.safe_for_breakpoint,
                "expected {name} to be safe_for_breakpoint=true, got false",
            );
        }
    }
}

#[test]
fn safe_for_breakpoint_exact_false_set() {
    for kind in all_variants() {
        let name = kind.kind_name();
        let flags = kind.flags();
        if SAFE_FOR_BREAKPOINT_FALSE.contains(&name) {
            assert!(
                !flags.safe_for_breakpoint,
                "expected {name} to be safe_for_breakpoint=false, got true",
            );
        }
    }
}

#[test]
fn safe_for_breakpoint_covers_all_69_variants() {
    // Every variant must appear in exactly one of the two lists.
    for kind in all_variants() {
        let name = kind.kind_name();
        let in_true = SAFE_FOR_BREAKPOINT_TRUE.contains(&name);
        let in_false = SAFE_FOR_BREAKPOINT_FALSE.contains(&name);
        assert!(
            in_true ^ in_false,
            "variant {name} must appear in exactly one of the safe_for_breakpoint lists"
        );
    }
}

// ────────────────────────────────────────────────────────
// Test (d): spot-check category assignments for representative variants
// ────────────────────────────────────────────────────────

#[test]
fn category_spot_checks() {
    // Program → Program
    assert_eq!(NodeKind::Program { statements: vec![] }.category(), NodeKindCategory::Program);

    // ExpressionStatement → Statement
    assert_eq!(
        NodeKind::ExpressionStatement { expression: Box::new(leaf()) }.category(),
        NodeKindCategory::Statement,
    );

    // VariableDeclaration → Declaration
    assert_eq!(
        NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(leaf()),
            attributes: vec![],
            initializer: None,
        }
        .category(),
        NodeKindCategory::Declaration
    );

    // Variable → Expression
    assert_eq!(
        NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() }.category(),
        NodeKindCategory::Expression
    );

    // Number → Literal
    assert_eq!(NodeKind::Number { value: "1".to_string() }.category(), NodeKindCategory::Literal);

    // Ellipsis → Operator
    assert_eq!(NodeKind::Ellipsis.category(), NodeKindCategory::Operator);

    // Block → Scope
    assert_eq!(NodeKind::Block { statements: vec![] }.category(), NodeKindCategory::Scope);

    // Error → Recovery
    assert_eq!(
        NodeKind::Error { message: "e".to_string(), expected: vec![], found: None, partial: None }
            .category(),
        NodeKindCategory::Recovery
    );

    // DataSection → Declaration (plan-reviewer correction: NOT Statement)
    assert_eq!(
        NodeKind::DataSection { marker: "__DATA__".to_string(), body: None }.category(),
        NodeKindCategory::Declaration
    );

    // PhaseBlock → Declaration (plan-reviewer correction)
    assert_eq!(
        NodeKind::PhaseBlock {
            phase: "BEGIN".to_string(),
            phase_span: None,
            block: Box::new(block_node()),
        }
        .category(),
        NodeKindCategory::Declaration
    );

    // Class → Declaration
    assert_eq!(
        NodeKind::Class { name: "Foo".to_string(), parents: vec![], body: Box::new(block_node()) }
            .category(),
        NodeKindCategory::Declaration
    );

    // Subroutine → Declaration
    assert_eq!(
        NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block_node()),
        }
        .category(),
        NodeKindCategory::Declaration
    );
}

// ────────────────────────────────────────────────────────
// Test: convenience methods on NodeKind
// ────────────────────────────────────────────────────────

#[test]
fn convenience_accessors_match_flags() {
    for kind in all_variants() {
        let flags = kind.flags();
        assert_eq!(
            kind.is_executable(),
            flags.executable,
            "is_executable mismatch for {}",
            kind.kind_name()
        );
        assert_eq!(
            kind.introduces_scope(),
            flags.introduces_scope,
            "introduces_scope mismatch for {}",
            kind.kind_name()
        );
        assert_eq!(
            kind.declares_symbol(),
            flags.declares_symbol,
            "declares_symbol mismatch for {}",
            kind.kind_name()
        );
        assert_eq!(
            kind.references_symbol(),
            flags.references_symbol,
            "references_symbol mismatch for {}",
            kind.kind_name()
        );
        assert_eq!(
            kind.safe_for_breakpoint(),
            flags.safe_for_breakpoint,
            "safe_for_breakpoint mismatch for {}",
            kind.kind_name()
        );
        assert_eq!(
            kind.is_recovery(),
            flags.recovery_artifact,
            "is_recovery mismatch for {}",
            kind.kind_name()
        );
    }
}

// ────────────────────────────────────────────────────────
// Test: flags validate() — recovery_artifact && safe_for_breakpoint is forbidden
// ────────────────────────────────────────────────────────

#[test]
fn flags_validate_all_variants() {
    for kind in all_variants() {
        let flags = kind.flags();
        assert!(
            flags.validate().is_ok(),
            "flags.validate() failed for variant {}: {:?}",
            kind.kind_name(),
            flags
        );
    }
}

// ────────────────────────────────────────────────────────
// Test: recovery category variants have recovery_artifact=true
// ────────────────────────────────────────────────────────

#[test]
fn recovery_category_implies_recovery_artifact() {
    for kind in all_variants() {
        let flags = kind.flags();
        let cat = kind.category();
        if cat == NodeKindCategory::Recovery {
            assert!(
                flags.recovery_artifact,
                "variant {} has Recovery category but recovery_artifact=false",
                kind.kind_name()
            );
        }
    }
}

// ────────────────────────────────────────────────────────
// Test: specific flags spot-checks against spec table
// ────────────────────────────────────────────────────────

#[test]
fn flags_spot_checks() {
    // Program: introduces_scope=true, contains_children=true
    {
        let kind = NodeKind::Program { statements: vec![] };
        let f = kind.flags();
        assert!(!f.executable, "Program.executable should be false");
        assert!(f.introduces_scope, "Program.introduces_scope should be true");
        assert!(f.contains_children, "Program.contains_children should be true");
        assert!(!f.safe_for_breakpoint, "Program.safe_for_breakpoint should be false");
    }

    // Subroutine: introduces_scope=true, declares_symbol=true
    {
        let kind = NodeKind::Subroutine {
            name: Some("foo".to_string()),
            name_span: None,
            prototype: None,
            signature: None,
            attributes: vec![],
            body: Box::new(block_node()),
        };
        let f = kind.flags();
        assert!(!f.executable, "Subroutine.executable should be false");
        assert!(f.introduces_scope, "Subroutine.introduces_scope should be true");
        assert!(f.declares_symbol, "Subroutine.declares_symbol should be true");
        assert!(f.safe_for_breakpoint, "Subroutine.safe_for_breakpoint should be true");
    }

    // Error: recovery_artifact=true, safe_for_breakpoint=false
    {
        let kind = NodeKind::Error {
            message: "e".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        };
        let f = kind.flags();
        assert!(f.recovery_artifact, "Error.recovery_artifact should be true");
        assert!(!f.safe_for_breakpoint, "Error.safe_for_breakpoint should be false");
    }

    // Package: executable=true, introduces_scope=true, declares_symbol=true
    {
        let kind = NodeKind::Package { name: "Foo".to_string(), name_span: loc(), block: None };
        let f = kind.flags();
        assert!(f.executable, "Package.executable should be true");
        assert!(f.introduces_scope, "Package.introduces_scope should be true");
        assert!(f.declares_symbol, "Package.declares_symbol should be true");
        assert!(f.safe_for_breakpoint, "Package.safe_for_breakpoint should be true");
    }

    // Identifier: references_symbol=true, safe_for_breakpoint=false
    {
        let kind = NodeKind::Identifier { name: "foo".to_string() };
        let f = kind.flags();
        assert!(!f.executable, "Identifier.executable should be false");
        assert!(f.references_symbol, "Identifier.references_symbol should be true");
        assert!(!f.safe_for_breakpoint, "Identifier.safe_for_breakpoint should be false");
    }

    // Heredoc: executable=true (the <<EOF line is stoppable), safe_for_breakpoint=true
    {
        let kind = NodeKind::Heredoc {
            delimiter: "EOF".to_string(),
            content: "body".to_string(),
            interpolated: false,
            indented: false,
            command: false,
            body_span: None,
        };
        let f = kind.flags();
        assert!(f.executable, "Heredoc.executable should be true");
        assert!(f.safe_for_breakpoint, "Heredoc.safe_for_breakpoint should be true");
    }
}

// ────────────────────────────────────────────────────────
// Test: the ALL_KIND_NAMES count matches the variants we constructed
// ────────────────────────────────────────────────────────

#[test]
fn all_kind_names_count_matches_variants() {
    let constructed = all_variants().len();
    let named = NodeKind::ALL_KIND_NAMES.len();
    assert_eq!(
        constructed, named,
        "all_variants() produced {constructed} variants but ALL_KIND_NAMES has {named} entries"
    );
}

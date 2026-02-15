use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

use perl_parser::{
    Parser, SourceLocation,
    ast::{Node, NodeKind},
    semantic::SemanticAnalyzer,
};
use perl_tdd_support::{must, must_some};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Source-of-truth list: update ONLY when NodeKind changes.
/// (Do not include HeredocDepthLimit — that is a lexer/token budget error path, not a NodeKind.)
const ALL_NODE_KIND_NAMES: &[&str] = &[
    "Program",
    "ExpressionStatement",
    "VariableDeclaration",
    "VariableListDeclaration",
    "Variable",
    "VariableWithAttributes",
    "Assignment",
    "Binary",
    "Ternary",
    "Unary",
    "Diamond",
    "Ellipsis",
    "Undef",
    "Readline",
    "Glob",
    "Typeglob",
    "Number",
    "String",
    "Heredoc",
    "ArrayLiteral",
    "HashLiteral",
    "Block",
    "Eval",
    "Do",
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
    "Prototype",
    "Signature",
    "MandatoryParameter",
    "OptionalParameter",
    "SlurpyParameter",
    "NamedParameter",
    "Method",
    "Return",
    "LoopControl",
    "MethodCall",
    "FunctionCall",
    "IndirectCall",
    "Regex",
    "Match",
    "Substitution",
    "Transliteration",
    "Package",
    "Use",
    "No",
    "PhaseBlock",
    "DataSection",
    "Class",
    "Format",
    "Identifier",
    "Error",
    "MissingExpression",
    "MissingStatement",
    "MissingIdentifier",
    "MissingBlock",
    "UnknownRest",
];

/// These are intentionally covered by a manual AST fixture.
/// (UnknownRest is lexer-budget driven and not a stable parser fixture; Missing* is not guaranteed to be
/// produced by the v3 engine error recovery hot path.)
const MANUAL_ONLY_NODE_KIND_NAMES: &[&str] = &[
    "Error",
    "MissingExpression",
    "MissingStatement",
    "MissingIdentifier",
    "MissingBlock",
    "UnknownRest",
];

fn parse_ast(source: &str) -> Node {
    let mut parser = Parser::new(source);
    must(parser.parse())
}

fn parse_ast_with_recovery(source: &str) -> Node {
    let mut parser = Parser::new(source);
    parser.parse_with_recovery().ast
}

fn collect_node_kinds(node: &Node, out: &mut HashSet<&'static str>) {
    out.insert(node.kind.kind_name());
    node.for_each_child(|child| collect_node_kinds(child, out));
}

fn has_node_kind(ast: &Node, expected: &str) -> bool {
    if ast.kind.kind_name() == expected {
        return true;
    }
    let mut found = false;
    ast.for_each_child(|child| {
        if !found && has_node_kind(child, expected) {
            found = true;
        }
    });
    found
}

fn find_first_node_of_kind<'a>(node: &'a Node, expected: &str) -> Option<&'a Node> {
    if node.kind.kind_name() == expected {
        return Some(node);
    }
    let mut found: Option<&'a Node> = None;
    node.for_each_child(|child| {
        if found.is_none() {
            found = find_first_node_of_kind(child, expected);
        }
    });
    found
}

fn manual_recovery_nodekind_fixture(location: SourceLocation) -> Node {
    Node::new(
        NodeKind::Program {
            statements: vec![
                Node::new(
                    NodeKind::Error {
                        message: "manual recovery node".to_string(),
                        expected: vec![],
                        found: None,
                        partial: None,
                    },
                    location,
                ),
                Node::new(NodeKind::MissingExpression, location),
                Node::new(NodeKind::MissingStatement, location),
                Node::new(NodeKind::MissingIdentifier, location),
                Node::new(NodeKind::MissingBlock, location),
                Node::new(NodeKind::UnknownRest, location),
            ],
        },
        location,
    )
}

#[test]
fn test_semantic_nodekind_coverage_is_total() -> TestResult {
    // Use known-good corpus syntax patterns (no indentation-sensitive heredoc terminators, etc.).
    let cases: Vec<(&str, &str)> = vec![
        (
            "core_literals_and_structures",
            r#"
                1;
                my $scalar = 41;
                $scalar = 42;

                my ($x :shared, $y :locked) = @_;
                my $list = (1, 2, 3);
                my $hash = (one => 1, two => 2);

                my $aryref = [1, 2, 3];
                my $href   = { one => 1, two => 2 };

                my $undef = undef;
                my $unary = -$scalar;
                my $bin = 1 + 2;
                my $ternary = 1 ? 2 : 3;

                my $stdin = <STDIN>;
                my $diamond = <>;
                my $paths = <*.txt>;
                my $tg = *STDOUT;

                my $id = Foo::Bar;

                my $heredoc = <<'EOF';
alpha
EOF

                sub unimplemented { ... }
            "#,
        ),
        (
            "signatures_calls_and_prototypes",
            r#"
                use feature 'signatures';

                sub identity($required, $optional = 1, @rest, :$named) {
                    return $required + $optional;
                }

                sub legacy :prototype($) { 1 }

                identity(7, 8, 9, named => 1);
                identity->legacy(4);

                my $obj = new Some::Class;
            "#,
        ),
        (
            "control_flow_and_modifiers",
            r#"
                eval { my $v = 0; };
                do { my $w = 1; };

                for (my $j = 0; $j < 2; $j++) {
                    next if $j == 1;
                }

                foreach my $entry ((1, 2, 3)) {
                    redo if $entry == 0;
                    last if $entry == 1;
                }

                LABEL: while (1) {
                    last LABEL;
                }

                try {
                    die "boom";
                } catch ($e) {
                    "caught";
                } finally {
                    "finally";
                }
            "#,
        ),
        (
            "given_when_default",
            r#"
                use feature 'switch';
                no warnings 'experimental::smartmatch';

                my $x = 2;

                given ($x) {
                    when (1) { say 'one'; }
                    when (2) { say 'two'; }
                    default { say 'other'; }
                }
            "#,
        ),
        (
            "packages_class_tie_format_data",
            r#"
                package Demo;
                use strict;
                no warnings;

                BEGIN { 1; }

                use feature 'class';
                use feature 'signatures';

                class Builder {
                    method build($left, $right = 1, @rest) {
                        return $left + $right;
                    }
                }

                tie my %tied, 'Tie::IxHash';
                untie %tied;

                my ($name, $age, $salary) = ("Ada", 37, 1234.50);

                format STDOUT =
@<<<<<< @>>>>  @####.##
$name, $age, $salary
.

                write;

                __DATA__
                payload
            "#,
        ),
        (
            "regex_operators",
            r#"
                my $subject = "hello world";
                my $regex = qr/hello/;

                my $matched = $subject =~ /world/;
                my $repl    = $subject =~ s/world/perl/;
                my $trans   = $subject =~ tr/hello/world/;
            "#,
        ),
    ];

    let mut observed = HashSet::new();

    for (name, source) in cases {
        let ast = parse_ast(source);
        collect_node_kinds(&ast, &mut observed);

        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, source);
        assert!(
            !analyzer.semantic_tokens().is_empty(),
            "[{name}] expected semantic analyzer to emit some semantic tokens"
        );
    }

    // Manual-only coverage (Missing* + UnknownRest + Error).
    let manual_ast = manual_recovery_nodekind_fixture(SourceLocation { start: 0, end: 0 });
    collect_node_kinds(&manual_ast, &mut observed);

    let missing: Vec<_> =
        ALL_NODE_KIND_NAMES.iter().copied().filter(|k| !observed.contains(k)).collect();

    assert!(missing.is_empty(), "Missing NodeKind coverage: {missing:?}");

    Ok(())
}

#[test]
fn test_manual_only_nodekinds_exist_and_analyze_without_panic() {
    let location = SourceLocation { start: 0, end: 0 };
    let manual_ast = manual_recovery_nodekind_fixture(location);

    // 1) Ensure the fixture still contains the intended manual-only kinds.
    for kind in MANUAL_ONLY_NODE_KIND_NAMES {
        assert!(has_node_kind(&manual_ast, kind), "manual fixture must include NodeKind::{kind}");
    }

    // 2) Ensure semantic analysis doesn't panic on any manual-only kind in isolation.
    for kind in MANUAL_ONLY_NODE_KIND_NAMES {
        let node = must_some(find_first_node_of_kind(&manual_ast, kind)).clone();

        let single = Node::new(NodeKind::Program { statements: vec![node] }, location);

        let ok = catch_unwind(AssertUnwindSafe(|| {
            let _ = SemanticAnalyzer::analyze_with_source(&single, "");
        }))
        .is_ok();

        assert!(ok, "Semantic analysis panicked on manual NodeKind::{kind}");
    }
}

#[test]
fn test_parser_recovery_produces_error_nodes_and_does_not_panic_semantic() {
    // These are intentionally malformed; we only require:
    // - parse_with_recovery returns an AST
    // - the AST contains an Error node
    // - semantic analysis does not panic
    let cases: &[(&str, &str)] = &[
        ("missing_expression", "my $x = ;"),
        ("missing_identifier", "my $ = 1;"),
        ("unclosed_block", "if (1) {"),
    ];

    for (name, source) in cases {
        let ast = parse_ast_with_recovery(source);
        assert!(
            has_node_kind(&ast, "Error"),
            "[{name}] expected parse_with_recovery() AST to include NodeKind::Error"
        );

        let ok = catch_unwind(AssertUnwindSafe(|| {
            let _ = SemanticAnalyzer::analyze_with_source(&ast, source);
        }))
        .is_ok();

        assert!(ok, "[{name}] semantic analysis panicked on recovery AST");
    }
}

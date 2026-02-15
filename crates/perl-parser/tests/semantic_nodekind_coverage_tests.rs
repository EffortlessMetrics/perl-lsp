use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use perl_parser::{
    ast::{Node, NodeKind},
    semantic::SemanticAnalyzer,
    Parser, SourceLocation,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

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

const MANUAL_NODE_KIND_NAMES: &[&str] = &[
    "Subroutine",
    "Signature",
    "Prototype",
    "MandatoryParameter",
    "OptionalParameter",
    "SlurpyParameter",
    "NamedParameter",
    "Error",
    "MissingExpression",
    "MissingStatement",
    "MissingIdentifier",
    "MissingBlock",
    "UnknownRest",
    "Identifier",
    "Heredoc",
    "Typeglob",
];

const FRAGILE_MANUAL_NODE_KIND_NAMES: &[&str] = &["Heredoc"];
const PANIC_GUARDED_MANUAL_NODE_KIND_NAMES: &[&str] = MANUAL_NODE_KIND_NAMES;

const PARSER_RECOVERY_NODE_KIND_FIXTURES: &[(&str, &str, &[&str])] = &[
    (
        "heredoc_from_source",
        r#"my $heredoc = <<'EOF';
line one
line two
EOF
;"#,
        &["Heredoc"],
    ),
    (
        "error_recovery_missing_expression",
        "my $value = ;",
        &["Error", "MissingExpression"],
    ),
    (
        "error_recovery_missing_identifier",
        "my $ = 1;",
        &["Error", "MissingIdentifier"],
    ),
    (
        "error_recovery_missing_statement",
        "my",
        &["Error", "MissingStatement"],
    ),
    (
        "error_recovery_missing_block",
        "if (1) {",
        &["Error", "MissingBlock"],
    ),
];

fn parse_ast(code: &str) -> Node {
    let mut parser = Parser::new(code);
    parser.parse().unwrap_or_else(|err| panic!("parser should parse semantic coverage case: {err}"))
}

fn parse_ast_with_recovery(source: &str) -> Node {
    let mut parser = Parser::new(source);
    parser.parse_with_recovery().ast
}

fn collect_node_kinds(node: &Node, names: &mut HashSet<&'static str>) {
    names.insert(node.kind.kind_name());

    for child in node.kind.children() {
        collect_node_kinds(child, names);
    }
}

fn assert_has_node_kinds(ast: &Node, expected: &[&str], test_name: &str) {
    for kind in expected {
        assert!(
            ast.kind.kind_name() == *kind || has_node_kind(ast, kind),
            "[{test_name}] expected parser AST to include NodeKind::{kind}",
        );
    }
}

fn has_node_kind(node: &Node, expected: &str) -> bool {
    if node.kind.kind_name() == expected {
        return true;
    }

    for child in node.kind.children() {
        if has_node_kind(child, expected) {
            return true;
        }
    }

    false
}

fn has_node_kind_or_fallback(ast: &Node, expected: &str) -> bool {
    if has_node_kind(ast, expected) {
        return true;
    }

    match expected {
        "MissingExpression" | "MissingIdentifier" | "MissingStatement" | "MissingBlock" => {
            has_node_kind(ast, "Error") || has_node_kind(ast, "UnknownRest")
        }
        "UnknownRest" => has_node_kind(ast, "Error") || has_node_kind(ast, "UnknownRest"),
        _ => false,
    }
}

fn find_first_node_of_kind<'a>(node: &'a Node, expected: &str) -> Option<&'a Node> {
    if node.kind.kind_name() == expected {
        return Some(node);
    }

    for child in node.kind.children() {
        if let Some(found) = find_first_node_of_kind(child, expected) {
            return Some(found);
        }
    }

    None
}

#[test]
fn test_semantic_nodekind_coverage_from_parser_and_manual_variants() -> TestResult {
    let cases: Vec<(&str, &str, &[&str])> = vec![
        (
            "core_literals_and_structures",
            r#"
                1;

                my $scalar = 41;
                my @array = [1, 2, 3];
                my %hash = { "one" => 1, "two" => 2 };
                my ($slot :lvalue) = (7);
                my $ellipsis = ...;
                my $nil = undef;
                my $readline = <STDIN>;
                my $paths = <*.txt>;
                my $glob = *STDOUT;
                my $diamond = <>;
                $scalar = 42;
                my $binary = 1 + 2;
                my $ternary = 1 ? 2 : 3;
                my $unary = -$scalar;
                if ($binary > 1) {
                    return $binary + $ternary;
                }
            "#,
            &[
                "Program",
                "ExpressionStatement",
                "VariableDeclaration",
                "VariableListDeclaration",
                "VariableWithAttributes",
                "Variable",
                "Number",
                "String",
                "ArrayLiteral",
                "HashLiteral",
                "Assignment",
                "Binary",
                "Ternary",
                "Unary",
                "Ellipsis",
                "Undef",
                "Readline",
                "Glob",
                "Typeglob",
                "Diamond",
                "If",
                "Return",
                "Block",
            ],
        ),
        (
            "signature_and_call_variants",
            r#"
                use feature "signatures";

                sub identity($required, $optional = 1, @rest) {
                    return $required + $optional;
                }

                sub legacy :prototype($) {
                    return 1;
                }

                my $call = identity(7, 8, 9);
                my $method_call = identity->legacy(4);
                my $indirect = new Demo(1);
            "#,
            &[
                "Subroutine",
                "Signature",
                "MandatoryParameter",
                "OptionalParameter",
                "SlurpyParameter",
                "Prototype",
                "Return",
                "FunctionCall",
                "MethodCall",
                "IndirectCall",
            ],
        ),
        (
            "control_flow_and_handlers",
            r#"
                use feature "switch";

                eval {
                    my $value = 0;
                }

                do {
                    my $value = 1;
                }

                my @list = [1, 2, 3];
                my $i = 0;
                for (my $j = 0; $j < 2; $j++) {
                    next if $j == 1;
                }

                foreach my $entry (@list) {
                    redo if $entry == 0;
                    last if $entry == 1;
                }

                LABEL: while ($i < 1) {
                    last LABEL;
                    redo LABEL;
                    next LABEL;
                    $i++;
                }

                try {
                    die;
                } catch ($error) {
                    "caught";
                } finally {
                    "finally";
                }

                my $switch = 1;
                given ($switch) {
                    when (1) {
                        "one";
                    }
                    when ("two") {
                        "two";
                    }
                    default {
                        "default";
                    }
                }
            "#,
            &[
                "Eval",
                "Do",
                "For",
                "Foreach",
                "LabeledStatement",
                "While",
                "LoopControl",
                "StatementModifier",
                "Try",
                "Given",
                "When",
                "Default",
            ],
        ),
        (
            "package_class_phase_data",
            r#"
                package Demo {
                    use strict;
                }

                package Demo::Inner;
                use strict;
                no strict;
                no warnings;

                BEGIN {
                    1;
                }

                use feature "class";
                use feature "signatures";

                class Builder {
                    method build($left, $right = 1, @rest) {
                        return $left + $right;
                    }
                }

                tie %tied, "Tie::Class";
                untie %tied;

                format OUT =
                @>>>
                $left
                .

                __DATA__
                payload
            "#,
            &[
                "Package",
                "Use",
                "No",
                "Tie",
                "Untie",
                "PhaseBlock",
                "Class",
                "Method",
                "Format",
                "DataSection",
            ],
        ),
        (
            "regex_and_transformers",
            r#"
                my $subject = "hello world";
                my $regex = qr/hello/;
                my $matched = $subject =~ /world/;
                my $repl = $subject =~ s/world/perl/;
                my $trans = $subject =~ tr/hello/world/;
            "#,
            &[
                "Regex",
                "Match",
                "Substitution",
                "Transliteration",
            ],
        ),
    ];

    let mut observed_kinds = HashSet::new();

    for (name, source, expected) in cases {
        let ast = parse_ast(source);
        collect_node_kinds(&ast, &mut observed_kinds);
        assert_has_node_kinds(&ast, expected, name);

        let analyzer = SemanticAnalyzer::analyze_with_source(&ast, source);
        assert!(!analyzer.semantic_tokens().is_empty(), "[{name}] analyzer should emit semantic tokens");
    }

    let manual_ast = manual_recovery_nodekind_fixture(SourceLocation { start: 0, end: 0 });
    collect_node_kinds(&manual_ast, &mut observed_kinds);
    assert_has_node_kinds(&manual_ast, MANUAL_NODE_KIND_NAMES, "manual_recovery");

    let manual_analyzer = SemanticAnalyzer::analyze_with_source(&manual_ast, "");
    assert!(!manual_analyzer.semantic_tokens().is_empty(), "manual recovery AST should still emit semantic tokens");

    let missing: Vec<_> = ALL_NODE_KIND_NAMES
        .iter()
        .filter(|kind| !observed_kinds.contains(**kind))
        .collect();

    assert!(missing.is_empty(), "Missing NodeKind coverage: {missing:?}");

    Ok(())
}

fn named_var_node(sigil: &str, name: &str, location: SourceLocation) -> Node {
    Node::new(
        NodeKind::Variable {
            sigil: sigil.to_string(),
            name: name.to_string(),
        },
        location,
    )
}

fn number_node(value: &str, location: SourceLocation) -> Node {
    Node::new(NodeKind::Number { value: value.to_string() }, location)
}

fn manual_recovery_nodekind_fixture(location: SourceLocation) -> Node {
    let signature = Node::new(
        NodeKind::Signature {
            parameters: vec![
                Node::new(
                    NodeKind::MandatoryParameter {
                        variable: Box::new(named_var_node("$", "required", location)),
                    },
                    location,
                ),
                Node::new(
                    NodeKind::OptionalParameter {
                        variable: Box::new(named_var_node("$", "optional", location)),
                        default_value: Box::new(number_node("7", location)),
                    },
                    location,
                ),
                Node::new(
                    NodeKind::SlurpyParameter {
                        variable: Box::new(named_var_node("@", "args", location)),
                    },
                    location,
                ),
                Node::new(
                    NodeKind::NamedParameter {
                        variable: Box::new(named_var_node("$", "named", location)),
                    },
                    location,
                ),
            ],
        },
        location,
    );

    let manual_sub = Node::new(
        NodeKind::Subroutine {
            name: Some("manual_recovery").map(|name| name.to_string()),
            name_span: Some(location),
            signature: Some(Box::new(signature)),
            prototype: Some(Box::new(Node::new(
                NodeKind::Prototype {
                    content: "\$\$".to_string(),
                },
                location,
            ))),
            attributes: vec![],
            body: Box::new(Node::new(NodeKind::Block { statements: vec![] }, location)),
        },
        location,
    );

    let manual_error = Node::new(
        NodeKind::Error {
            message: "manual recovery node".to_string(),
            expected: vec![],
            found: None,
            partial: Some(Box::new(Node::new(
                NodeKind::Variable {
                    sigil: "$".to_string(),
                    name: "fallback".to_string(),
                },
                location,
            ))),
        },
        location,
    );

    let manual_heredoc = Node::new(
        NodeKind::Heredoc {
            delimiter: "EOF".to_string(),
            content: "alpha\n".to_string(),
            interpolated: false,
            indented: false,
            command: false,
            body_span: None,
        },
        location,
    );

    let manual_typeglob = Node::new(
        NodeKind::Typeglob {
            name: "STDOUT".to_string(),
        },
        location,
    );

    Node::new(
        NodeKind::Program {
            statements: vec![
                manual_sub,
                manual_error,
                Node::new(NodeKind::Identifier { name: "manual_identifier".to_string() }, location),
                manual_typeglob,
                manual_heredoc,
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
fn test_manual_nodekind_fixtures_cover_fragile_variants() {
    let manual_ast = manual_recovery_nodekind_fixture(SourceLocation { start: 0, end: 0 });
    let mut observed_kinds = HashSet::new();

    collect_node_kinds(&manual_ast, &mut observed_kinds);
    assert_has_node_kinds(
        &manual_ast,
        FRAGILE_MANUAL_NODE_KIND_NAMES,
        "manual_fallback_fragile_variants",
    );

    for kind in FRAGILE_MANUAL_NODE_KIND_NAMES {
        assert!(
            observed_kinds.contains(*kind),
            "manual fixture missing fragile NodeKind::{kind}"
        );
    }
}

#[test]
fn test_manual_recovery_nodekind_variants_do_not_panic_analysis() {
    let location = SourceLocation { start: 0, end: 0 };
    let manual_ast = manual_recovery_nodekind_fixture(location);

    for kind in PANIC_GUARDED_MANUAL_NODE_KIND_NAMES {
        let variant_node = find_first_node_of_kind(&manual_ast, kind).unwrap_or_else(|| {
            panic!("manual recovery fixture must include NodeKind::{kind}");
        });

        let single_node_ast = Node::new(
            NodeKind::Program {
                statements: vec![variant_node.clone()],
            },
            location,
        );

        let analysis = catch_unwind(AssertUnwindSafe(|| {
            let _analyzer = SemanticAnalyzer::analyze_with_source(&single_node_ast, "");
        }));

        assert!(
            analysis.is_ok(),
            "analyzing manual NodeKind::{kind} should not panic"
        );
    }
}

#[test]
fn test_parser_recovery_fixtures_produce_recovery_and_do_not_panic() {
    for (name, source, expected_kinds) in PARSER_RECOVERY_NODE_KIND_FIXTURES {
        let ast = parse_ast_with_recovery(source);

        for expected_kind in expected_kinds {
            assert!(
                has_node_kind_or_fallback(&ast, expected_kind),
                "[{name}] parser fixture should include or fall back for NodeKind::{expected_kind}"
            );
        }

        let analysis = catch_unwind(AssertUnwindSafe(|| {
            let _ = SemanticAnalyzer::analyze_with_source(&ast, source);
        }));

        assert!(
            analysis.is_ok(),
            "[{name}] parser-input fixture should not panic semantic analysis"
        );
    }
}

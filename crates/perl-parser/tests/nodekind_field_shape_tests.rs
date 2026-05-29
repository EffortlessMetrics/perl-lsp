//! Focused NodeKind field-shape coverage for parser-produced ASTs.
//!
//! Corpus coverage proves every required kind appears somewhere.  These tests go
//! a step deeper for low-frequency variants whose important behavior lives in
//! fields such as negation flags, modifiers, optional blocks, and child lists.

use perl_parser::{
    Node, Parser,
    ast::{NodeKind, SourceLocation},
};

fn parse(source: &str) -> Result<Node, perl_parser::ParseError> {
    let mut parser = Parser::new(source);
    parser.parse()
}

fn find_nodes<'a, F>(node: &'a Node, predicate: &F, out: &mut Vec<&'a Node>)
where
    F: Fn(&NodeKind) -> bool,
{
    if predicate(&node.kind) {
        out.push(node);
    }
    node.for_each_child(|child| find_nodes(child, predicate, out));
}

fn matching_nodes<'a, F>(node: &'a Node, predicate: F) -> Vec<&'a Node>
where
    F: Fn(&NodeKind) -> bool,
{
    let mut out = Vec::new();
    find_nodes(node, &predicate, &mut out);
    out
}

fn first_matching_node<'a, F>(node: &'a Node, predicate: F, label: &str) -> Result<&'a Node, String>
where
    F: Fn(&NodeKind) -> bool,
{
    matching_nodes(node, predicate).first().copied().ok_or_else(|| format!("missing {label}"))
}

fn assert_variable(node: &Node, sigil: &str, name: &str) -> Result<(), String> {
    match &node.kind {
        NodeKind::Variable { sigil: actual_sigil, name: actual_name } => {
            assert_eq!(actual_sigil, sigil);
            assert_eq!(actual_name, name);
            Ok(())
        }
        other => Err(format!("expected Variable {sigil}{name}, got {other:?}")),
    }
}

fn assert_span_text(source: &str, span: SourceLocation, expected: &str) {
    assert_eq!(&source[span.start..span.end], expected);
}

#[test]
fn regex_binding_nodekinds_preserve_flags_and_subjects() -> Result<(), String> {
    let source = r#"
my $text = "abc";
$text =~ /a(?{ $seen++ })/x;
$text !~ s/a/b/gr;
$text !~ tr/a-z/A-Z/cdsr;
"#;
    let ast = parse(source).map_err(|err| err.to_string())?;

    let match_node =
        first_matching_node(&ast, |kind| matches!(kind, NodeKind::Match { .. }), "Match")?;
    match &match_node.kind {
        NodeKind::Match { expr, pattern, modifiers, has_embedded_code, negated } => {
            assert_variable(expr, "$", "text")?;
            assert_eq!(pattern, "/a(?{ $seen++ })/");
            assert_eq!(modifiers, "x");
            assert!(*has_embedded_code, "embedded-code regex flag should be preserved");
            assert!(!negated, "=~ match should not be marked negated");
        }
        other => return Err(format!("expected Match, got {other:?}")),
    }

    let substitution_node = first_matching_node(
        &ast,
        |kind| matches!(kind, NodeKind::Substitution { .. }),
        "Substitution",
    )?;
    match &substitution_node.kind {
        NodeKind::Substitution {
            expr,
            pattern,
            replacement,
            modifiers,
            has_embedded_code,
            negated,
        } => {
            assert_variable(expr, "$", "text")?;
            assert_eq!(pattern, "a");
            assert_eq!(replacement, "b");
            assert_eq!(modifiers, "gr");
            assert!(!has_embedded_code, "plain substitution should not report embedded code");
            assert!(*negated, "!~ substitution should preserve negation");
        }
        other => return Err(format!("expected Substitution, got {other:?}")),
    }

    let transliteration_node = first_matching_node(
        &ast,
        |kind| matches!(kind, NodeKind::Transliteration { .. }),
        "Transliteration",
    )?;
    match &transliteration_node.kind {
        NodeKind::Transliteration { expr, search, replace, modifiers, negated } => {
            assert_variable(expr, "$", "text")?;
            assert_eq!(search, "a-z");
            assert_eq!(replace, "A-Z");
            assert_eq!(modifiers, "cdsr");
            assert!(*negated, "!~ transliteration should preserve negation");
        }
        other => return Err(format!("expected Transliteration, got {other:?}")),
    }

    Ok(())
}

#[test]
fn package_phase_and_data_nodekinds_preserve_optional_shapes() -> Result<(), String> {
    let source = "package Local::Pkg { BEGIN { $ready = 1; } sub answer { return 42; } }\n__END__\nfixture tail\n";
    let ast = parse(source).map_err(|err| err.to_string())?;

    let package_node =
        first_matching_node(&ast, |kind| matches!(kind, NodeKind::Package { .. }), "Package")?;
    match &package_node.kind {
        NodeKind::Package { name, name_span, block } => {
            assert_eq!(name, "Local::Pkg");
            assert_span_text(source, *name_span, "Local::Pkg");
            let block = block.as_ref().ok_or("block-form package should keep its block")?;
            assert!(matches!(block.kind, NodeKind::Block { .. }));
        }
        other => return Err(format!("expected Package, got {other:?}")),
    }

    let phase_node = first_matching_node(
        &ast,
        |kind| matches!(kind, NodeKind::PhaseBlock { .. }),
        "PhaseBlock",
    )?;
    match &phase_node.kind {
        NodeKind::PhaseBlock { phase, phase_span, block } => {
            assert_eq!(phase, "BEGIN");
            let span = phase_span.ok_or("phase block should keep phase_span")?;
            assert_span_text(source, span, "BEGIN");
            assert!(matches!(block.kind, NodeKind::Block { .. }));
        }
        other => return Err(format!("expected PhaseBlock, got {other:?}")),
    }

    let data_node = first_matching_node(
        &ast,
        |kind| matches!(kind, NodeKind::DataSection { .. }),
        "DataSection",
    )?;
    match &data_node.kind {
        NodeKind::DataSection { marker, body } => {
            assert_eq!(marker, "__END__");
            let body = body.as_ref().ok_or("data section should keep body")?;
            assert!(body.contains("fixture tail"));
        }
        other => return Err(format!("expected DataSection, got {other:?}")),
    }

    Ok(())
}

#[test]
fn tie_untie_and_indirect_nodekinds_preserve_child_shapes() -> Result<(), String> {
    let source = r#"
tie my %cache, 'Tie::StdHash', size => 10;
untie %cache;
my $obj = new Widget $arg, 42;
print $fh "log";
"#;
    let ast = parse(source).map_err(|err| err.to_string())?;

    let tie_node = first_matching_node(&ast, |kind| matches!(kind, NodeKind::Tie { .. }), "Tie")?;
    match &tie_node.kind {
        NodeKind::Tie { variable, package, args } => {
            assert!(
                matches!(variable.kind, NodeKind::VariableDeclaration { .. }),
                "tie my %cache should keep declaration as tie variable"
            );
            assert!(matches!(package.kind, NodeKind::String { .. }));
            assert!(args.len() >= 2, "tie arguments should include named option and value");
            assert_eq!(tie_node.children().len(), 2 + args.len());
        }
        other => return Err(format!("expected Tie, got {other:?}")),
    }

    let untie_node =
        first_matching_node(&ast, |kind| matches!(kind, NodeKind::Untie { .. }), "Untie")?;
    match &untie_node.kind {
        NodeKind::Untie { variable } => {
            assert_variable(variable, "%", "cache")?;
            assert_eq!(untie_node.children().len(), 1);
        }
        other => return Err(format!("expected Untie, got {other:?}")),
    }

    let indirect_nodes = matching_nodes(&ast, |kind| matches!(kind, NodeKind::IndirectCall { .. }));
    assert!(
        indirect_nodes.len() >= 2,
        "new Widget and print $fh should both surface as indirect calls"
    );

    let new_call = indirect_nodes
        .iter()
        .find(|node| matches!(&node.kind, NodeKind::IndirectCall { method, .. } if method == "new"))
        .copied()
        .ok_or("missing indirect new call")?;
    match &new_call.kind {
        NodeKind::IndirectCall { object, args, .. } => {
            assert!(matches!(object.kind, NodeKind::Identifier { .. }));
            assert_eq!(args.len(), 1);
        }
        other => return Err(format!("expected IndirectCall, got {other:?}")),
    }

    let print_call = indirect_nodes
        .iter()
        .find(
            |node| matches!(&node.kind, NodeKind::IndirectCall { method, .. } if method == "print"),
        )
        .copied()
        .ok_or("missing indirect print call")?;
    match &print_call.kind {
        NodeKind::IndirectCall { object, args, .. } => {
            assert_variable(object, "$", "fh")?;
            assert_eq!(args.len(), 1);
        }
        other => return Err(format!("expected IndirectCall, got {other:?}")),
    }

    Ok(())
}

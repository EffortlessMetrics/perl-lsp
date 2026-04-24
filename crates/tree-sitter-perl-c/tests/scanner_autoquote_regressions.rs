use std::error::Error;

use tree_sitter::Node;
use tree_sitter_perl_c::parse_perl_code;

fn count_kind(node: Node<'_>, wanted: &str) -> usize {
    let mut total = usize::from(node.kind() == wanted);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        total += count_kind(child, wanted);
    }
    total
}

#[test]
fn brace_autoquote_skips_comment_and_pod_extras() -> Result<(), Box<dyn Error>> {
    let source = "my $value = $h{foo # trailing comment\n=pod\nignored\n=cut\n};\n";
    let tree = parse_perl_code(source)?;

    assert!(!tree.root_node().has_error());
    assert!(
        count_kind(tree.root_node(), "autoquoted_bareword") >= 1,
        "expected autoquoted_bareword for foo in hash subscript"
    );
    Ok(())
}

#[test]
fn fat_comma_autoquote_does_not_cross_pod_directives() -> Result<(), Box<dyn Error>> {
    let source = "my %h = (foo # trailing comment\n=pod\nignored\n=cut\n=> 1);\n";
    let tree = parse_perl_code(source)?;

    assert_eq!(count_kind(tree.root_node(), "autoquoted_bareword"), 0);
    Ok(())
}

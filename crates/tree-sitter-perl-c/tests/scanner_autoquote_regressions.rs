use std::error::Error;

use tree_sitter::{Query, QueryCursor, StreamingIterator};
use tree_sitter_perl_c::{language, parse_perl_code};

fn captured_autoquoted_barewords(source: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let tree = parse_perl_code(source)?;
    assert!(
        !tree.root_node().has_error(),
        "unexpected parse error: {}",
        tree.root_node().to_sexp()
    );

    let query = Query::new(&language(), "(autoquoted_bareword) @autoquoted")?;
    let mut cursor = QueryCursor::new();
    let mut captures = Vec::new();

    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let text = capture.node.utf8_text(source.as_bytes())?;
            captures.push(text.to_string());
        }
    }

    Ok(captures)
}

#[test]
fn fat_comma_autoquote_skips_comment_gap() -> Result<(), Box<dyn Error>> {
    let source = "my %h = ( key # comment between key and fat comma\n  => 1 );\n";
    let captures = captured_autoquoted_barewords(source)?;

    assert!(
        captures.iter().any(|capture| capture == "key"),
        "expected autoquoted_bareword capture for `key`, got {captures:?}"
    );
    Ok(())
}

#[test]
fn brace_autoquote_skips_comment_gap() -> Result<(), Box<dyn Error>> {
    let source = "my %h = ( key => 1 );\nmy $v = $h{key # comment before closing brace\n};\n";
    let captures = captured_autoquoted_barewords(source)?;

    assert!(
        captures.iter().any(|capture| capture == "key"),
        "expected autoquoted_bareword capture for `key`, got {captures:?}"
    );
    Ok(())
}

#[test]
fn fat_comma_autoquote_skips_pod_like_gap() -> Result<(), Box<dyn Error>> {
    let source = "my %h = ( key\n=head1 Note\nintervening pod text\n=cut\n  => 1 );\n";
    let captures = captured_autoquoted_barewords(source)?;

    assert!(
        captures.iter().any(|capture| capture == "key"),
        "expected autoquoted_bareword capture for `key`, got {captures:?}"
    );
    Ok(())
}

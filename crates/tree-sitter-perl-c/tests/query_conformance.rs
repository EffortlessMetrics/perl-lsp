use std::{collections::BTreeMap, error::Error};

use tree_sitter::{Query, QueryCursor, StreamingIterator};
use tree_sitter_perl_c::{language, parse_perl_code};

type CaptureMap = BTreeMap<String, Vec<String>>;

fn run_query(source: &str, query_source: &str) -> Result<CaptureMap, Box<dyn Error>> {
    let tree = parse_perl_code(source)?;
    let query = Query::new(&language(), query_source)?;
    let mut cursor = QueryCursor::new();

    let mut captures: CaptureMap = BTreeMap::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    while let Some(matched) = matches.next() {
        for capture in matched.captures {
            let capture_name = query
                .capture_names()
                .get(capture.index as usize)
                .copied()
                .unwrap_or_default()
                .to_string();
            let capture_text = capture.node.utf8_text(source.as_bytes())?;
            captures
                .entry(capture_name)
                .or_default()
                .push(format!("{}: {capture_text:?}", capture.node.kind()));
        }
    }

    Ok(captures)
}

fn assert_capture_contains(captures: &CaptureMap, name: &str, needle: &str) {
    let haystack = captures.get(name).cloned().unwrap_or_default();
    assert!(
        haystack.iter().any(|capture| capture.contains(needle)),
        "expected capture '{name}' to include {needle:?}, got: {haystack:#?}"
    );
}

#[test]
fn query_conformance_injections_inline_c_and_cpp_heredocs() -> Result<(), Box<dyn Error>> {
    let query = include_str!("../../../tree-sitter-perl/queries/injections.scm");
    let inline_c = r#"use Inline C => <<'END_C';
#include <math.h>
double calc(double x) { return sqrt(x); }
END_C
"#;
    let inline_cpp = r#"use Inline CPP => <<'END_CPP';
#include <string>
class Greet {};
END_CPP
"#;

    let c_captures = run_query(inline_c, query)?;
    assert_capture_contains(&c_captures, "inline.package", "\"Inline\"");
    assert_capture_contains(&c_captures, "inline.language", "\"C\"");
    assert_capture_contains(&c_captures, "injection.content", "#include <math.h>");

    let cpp_captures = run_query(inline_cpp, query)?;
    assert_capture_contains(&cpp_captures, "inline.package", "\"Inline\"");
    assert_capture_contains(&cpp_captures, "inline.language", "\"CPP\"");
    assert_capture_contains(&cpp_captures, "injection.content", "#include <string>");

    Ok(())
}

#[test]
fn query_conformance_highlights_pod_heredoc_and_quote_like() -> Result<(), Box<dyn Error>> {
    let highlights = include_str!("../../../tree-sitter-perl/queries/highlights.scm");
    let source = r#"=head1 NAME
Demo - highlight capture fixture
=cut

my $value = <<'SQL';
select * from users;
SQL

my $rx = qr/foo+/;
my @items = qw(alpha beta);
"#;

    let pod_clause = "(pod) @text";
    let string_clause = r#"[
  (string_literal)
  (interpolated_string_literal)
  (quoted_word_list)
  (command_string)
  (heredoc_content)
  (replacement)
  (transliteration_content)
] @string"#;
    let regex_clause = r#"[
 (quoted_regexp)
 (match_regexp)
 (regexp_content)
] @string.regex"#;

    for (label, clause) in [("pod", pod_clause), ("string", string_clause), ("regex", regex_clause)]
    {
        assert!(
            highlights.contains(clause),
            "expected upstream highlights.scm to contain {label} clause"
        );
    }

    let focused_query = [pod_clause, string_clause, regex_clause].join("\n\n");
    let captures = run_query(source, &focused_query)?;

    assert_capture_contains(&captures, "text", "Demo - highlight capture fixture");
    assert_capture_contains(&captures, "string", "select * from users");
    assert_capture_contains(&captures, "string.regex", "foo+");
    assert_capture_contains(&captures, "string", "qw(alpha beta)");

    Ok(())
}

#[test]
fn query_conformance_folds_comments_pod_heredoc_and_blocks() -> Result<(), Box<dyn Error>> {
    let query = include_str!("../../../tree-sitter-perl/queries/folds.scm");
    let source = r#"# heading comment
# another comment

=head1 TITLE
Fold this POD block
=cut

my $template = <<'EOT';
line one
line two
EOT

sub greet {
    my ($name) = @_;
    return "hi $name";
}
"#;

    let captures = run_query(source, query)?;
    assert_capture_contains(&captures, "fold", "heading comment");
    assert_capture_contains(&captures, "fold", "Fold this POD block");
    assert_capture_contains(&captures, "fold", "line one");
    assert_capture_contains(&captures, "fold", "sub greet");

    Ok(())
}

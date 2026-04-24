use std::error::Error;

use tree_sitter::{Query, QueryCursor, StreamingIterator};
use tree_sitter_perl_c::{language, parse_perl_code};

#[derive(Debug, Clone)]
struct CaptureRecord {
    name: String,
    text: String,
    kind: String,
    row: usize,
}

fn run_query(source: &str, query_source: &str) -> Result<Vec<CaptureRecord>, Box<dyn Error>> {
    let tree = parse_perl_code(source)?;
    let query = Query::new(&language(), query_source)?;
    let mut cursor = QueryCursor::new();
    let capture_names = query.capture_names();

    let mut records = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let name = capture_names
                .get(capture.index as usize)
                .map(|value| (*value).to_owned())
                .unwrap_or_default();
            let text =
                capture.node.utf8_text(source.as_bytes()).map(str::to_owned).unwrap_or_default();
            let kind = capture.node.kind().to_owned();
            let row = capture.node.start_position().row;

            records.push(CaptureRecord { name, text, kind, row });
        }
    }

    Ok(records)
}

fn assert_has_capture(
    captures: &[CaptureRecord],
    name: &str,
    predicate: impl Fn(&CaptureRecord) -> bool,
    detail: &str,
) {
    let found = captures.iter().any(|capture| capture.name == name && predicate(capture));

    let diagnostic = captures
        .iter()
        .map(|capture| {
            format!(
                "{} kind={} row={} text={:?}",
                capture.name, capture.kind, capture.row, capture.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(found, "expected capture `{name}` with {detail}.\nCaptured records:\n{diagnostic}");
}

#[test]
fn highlights_query_covers_pod_heredoc_and_quote_like_tokens() -> Result<(), Box<dyn Error>> {
    let source = r#"=pod
Heading
=cut

my $doc = <<'END_HEREDOC';
line one
END_HEREDOC

my $regexp = qr/foo+/;
my $replacement = $value =~ s/foo/bar/;
"#;
    let highlights_upstream = include_str!("../../../tree-sitter-perl/queries/highlights.scm");
    assert!(highlights_upstream.contains("(pod) @text"));
    assert!(highlights_upstream.contains("(heredoc_content)"));
    assert!(highlights_upstream.contains("(replacement)"));
    assert!(highlights_upstream.contains("(quoted_regexp)"));

    // The full upstream highlights query currently includes patterns not accepted by the
    // vendored grammar, so this conformance check executes a targeted subset verbatim
    // from the upstream file and validates the key captures we rely on.
    let highlights_query = r#"
(pod) @text
[(heredoc_content) (replacement)] @string
[(quoted_regexp) (match_regexp) (regexp_content)] @string.regex
"#;

    let captures = run_query(source, highlights_query)?;

    assert_has_capture(&captures, "text", |capture| capture.kind == "pod", "POD blocks");
    assert_has_capture(
        &captures,
        "string",
        |capture| capture.kind == "heredoc_content" && capture.text.contains("line one"),
        "heredoc content",
    );
    assert_has_capture(
        &captures,
        "string.regex",
        |capture| capture.kind == "quoted_regexp",
        "quote-like regexp literals",
    );
    assert_has_capture(
        &captures,
        "string",
        |capture| capture.kind == "replacement" && capture.text == "bar",
        "substitution replacement text",
    );

    Ok(())
}

#[test]
fn folds_query_marks_pod_heredoc_and_control_blocks() -> Result<(), Box<dyn Error>> {
    let source = r#"=pod
Fold this section
=cut

sub greet {
    my $name = shift;
    return $name;
}

my $doc = <<'END_HEREDOC';
fold me too
END_HEREDOC
"#;
    let folds_query = include_str!("../../../tree-sitter-perl/queries/folds.scm");

    let captures = run_query(source, folds_query)?;

    assert_has_capture(&captures, "fold", |capture| capture.kind == "pod", "POD sections");
    assert_has_capture(
        &captures,
        "fold",
        |capture| capture.kind == "heredoc_content" && capture.text.contains("fold me too"),
        "heredoc content",
    );
    assert_has_capture(
        &captures,
        "fold",
        |capture| capture.kind == "subroutine_declaration_statement",
        "subroutine declaration blocks",
    );

    Ok(())
}

#[test]
fn injections_query_covers_comments_pod_substitution_and_inline_heredocs()
-> Result<(), Box<dyn Error>> {
    let source = r#"# comment injection
=pod
Inline pod
=cut

my $single_eval = $value =~ s/foo/bar/e;
my $double_eval = $value =~ s/foo/bar/ee;

use Inline C => <<'END_C';
#include <math.h>
END_C

use Inline CPP => <<'END_CPP';
#include <string>
END_CPP
"#;
    let injections_query = include_str!("../../../tree-sitter-perl/queries/injections.scm");

    let captures = run_query(source, injections_query)?;

    assert_has_capture(
        &captures,
        "injection.content",
        |capture| capture.kind == "comment" && capture.text.contains("comment injection"),
        "comment injections",
    );
    assert_has_capture(
        &captures,
        "injection.content",
        |capture| capture.kind == "pod" && capture.text.contains("Inline pod"),
        "POD injections",
    );
    assert_has_capture(
        &captures,
        "injection.content",
        |capture| capture.kind == "replacement" && capture.text == "bar",
        "single-e substitution eval injections",
    );
    assert_has_capture(
        &captures,
        "inline.package",
        |capture| capture.text == "Inline",
        "Inline package capture",
    );
    assert_has_capture(
        &captures,
        "inline.language",
        |capture| capture.text == "C",
        "Inline::C language capture",
    );
    assert_has_capture(
        &captures,
        "inline.language",
        |capture| capture.text == "CPP",
        "Inline::CPP language capture",
    );
    assert_has_capture(
        &captures,
        "injection.content",
        |capture| capture.kind == "heredoc_content" && capture.text.contains("#include <math.h>"),
        "Inline::C heredoc content",
    );
    assert_has_capture(
        &captures,
        "injection.content",
        |capture| capture.kind == "heredoc_content" && capture.text.contains("#include <string>"),
        "Inline::CPP heredoc content",
    );

    let replacement_injections = captures
        .iter()
        .filter(|capture| capture.name == "injection.content" && capture.kind == "replacement")
        .count();
    assert_eq!(
        replacement_injections, 1,
        "expected exactly one replacement injection capture from the /e substitution"
    );

    Ok(())
}

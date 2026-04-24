use std::error::Error;

use tree_sitter::{Query, QueryCursor, StreamingIterator};
use tree_sitter_perl_c::{language, parse_perl_code};

#[derive(Debug)]
struct CaptureHit {
    name: String,
    kind: String,
    text: String,
}

fn run_query(query_source: &str, source: &str) -> Result<Vec<CaptureHit>, Box<dyn Error>> {
    let tree = parse_perl_code(source)?;
    let query = Query::new(&language(), query_source)?;
    let mut cursor = QueryCursor::new();

    let mut hits = Vec::new();
    let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    while let Some(matched) = matches.next() {
        for capture in matched.captures {
            if let Some(name) = query.capture_names().get(capture.index as usize) {
                let text = capture.node.utf8_text(source.as_bytes())?.to_owned();
                hits.push(CaptureHit {
                    name: (*name).to_owned(),
                    kind: capture.node.kind().to_owned(),
                    text,
                });
            }
        }
    }

    Ok(hits)
}

fn assert_has_capture(
    hits: &[CaptureHit],
    capture_name: &str,
    required_kind: &str,
    required_text_fragment: &str,
) {
    let found = hits.iter().any(|hit| {
        hit.name == capture_name
            && hit.kind == required_kind
            && hit.text.contains(required_text_fragment)
    });

    let rendered_hits = hits
        .iter()
        .map(|hit| format!("{} [{}] => {:?}", hit.name, hit.kind, hit.text))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        found,
        "missing capture '{capture_name}' on kind '{required_kind}' containing {required_text_fragment:?}.\nObserved captures:\n{rendered_hits}"
    );
}

#[test]
fn query_conformance_injections_cover_inline_c_and_cpp_heredocs() -> Result<(), Box<dyn Error>> {
    let injections_query = include_str!("../../../tree-sitter-perl/queries/injections.scm");

    let inline_c = "use Inline C => <<'END_C';\n#include <math.h>\nEND_C\n";
    let inline_cpp =
        "use Inline CPP => <<'END_CPP';\n#include <string>\nclass Greet {};\nEND_CPP\n";

    let c_hits = run_query(injections_query, inline_c)?;
    assert_has_capture(&c_hits, "inline.package", "package", "Inline");
    assert_has_capture(&c_hits, "inline.language", "autoquoted_bareword", "C");
    assert_has_capture(&c_hits, "injection.content", "heredoc_content", "#include <math.h>");

    let cpp_hits = run_query(injections_query, inline_cpp)?;
    assert_has_capture(&cpp_hits, "inline.package", "package", "Inline");
    assert_has_capture(&cpp_hits, "inline.language", "autoquoted_bareword", "CPP");
    assert_has_capture(&cpp_hits, "injection.content", "heredoc_content", "#include <string>");

    Ok(())
}

#[test]
fn query_conformance_highlights_cover_pod_heredoc_and_quote_like_nodes()
-> Result<(), Box<dyn Error>> {
    let upstream_highlights_query =
        include_str!("../../../tree-sitter-perl/queries/highlights.scm");
    assert!(upstream_highlights_query.contains("(pod) @text"));
    assert!(upstream_highlights_query.contains("(heredoc_content)"));
    assert!(upstream_highlights_query.contains("(quoted_regexp)"));

    let highlights_query = r#"
(pod) @text
(heredoc_content) @string
(quoted_word_list) @string
[(quoted_regexp) (regexp_content) (match_regexp)] @string.regex
"#;
    let source = r#"=pod
A POD section
=cut

my $doc = <<'TXT';
hello heredoc
TXT

my $regex = qr/abc+/;
my @words = qw(alpha beta);
"#;

    let hits = run_query(highlights_query, source)?;

    assert_has_capture(&hits, "text", "pod", "A POD section");
    assert_has_capture(&hits, "string", "heredoc_content", "hello heredoc");
    assert_has_capture(&hits, "string.regex", "quoted_regexp", "qr/abc+/");
    assert_has_capture(&hits, "string", "quoted_word_list", "qw(alpha beta)");

    Ok(())
}

#[test]
fn query_conformance_folds_cover_comments_pod_heredoc_and_blocks() -> Result<(), Box<dyn Error>> {
    let folds_query = include_str!("../../../tree-sitter-perl/queries/folds.scm");
    let source = r#"# fold me 1
# fold me 2

=pod
Fold this POD
=cut

sub greet {
    return 1;
}

my $doc = <<'DOC';
fold heredoc body
DOC
"#;

    let hits = run_query(folds_query, source)?;

    assert_has_capture(&hits, "fold", "comment", "fold me 1");
    assert_has_capture(&hits, "fold", "pod", "Fold this POD");
    assert_has_capture(&hits, "fold", "subroutine_declaration_statement", "sub greet");
    assert_has_capture(&hits, "fold", "heredoc_content", "fold heredoc body");

    Ok(())
}

#[test]
fn query_conformance_injections_cover_pod_and_subst_eval_cases() -> Result<(), Box<dyn Error>> {
    let injections_query = include_str!("../../../tree-sitter-perl/queries/injections.scm");
    let source = r#"=pod
Injected as POD
=cut

my $value = "abc";
$value =~ s/a/uc($1)/e;
"#;

    let hits = run_query(injections_query, source)?;

    assert_has_capture(&hits, "injection.content", "pod", "Injected as POD");
    assert_has_capture(&hits, "injection.content", "replacement", "uc($1)");

    Ok(())
}

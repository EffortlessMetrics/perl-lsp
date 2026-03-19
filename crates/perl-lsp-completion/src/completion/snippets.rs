//! Perl idiom snippet completions
//!
//! Context-aware snippet completions for common Perl patterns including
//! subroutine definitions, control flow, module boilerplate, test patterns,
//! and data structure initializers.

use super::{context::CompletionContext, items::CompletionItem};

/// Scope in which a snippet should be offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnippetScope {
    /// Anywhere (general purpose).
    Any,
    /// Only at file/package scope (not inside a sub body).
    FileScope,
    /// Only in `.t` test files or files that use Test::More / Test2.
    TestOnly,
}

/// A single snippet definition.
struct Snippet {
    /// Trigger prefix the user types.
    trigger: &'static str,
    /// LSP snippet body with tab stops.
    body: &'static str,
    /// Short detail shown inline.
    detail: &'static str,
    /// Longer documentation.
    documentation: &'static str,
    /// Where this snippet is valid.
    scope: SnippetScope,
}

/// All built-in Perl idiom snippets.
const SNIPPETS: &[Snippet] = &[
    // -- Subroutine patterns --
    Snippet {
        trigger: "subm",
        body: "sub ${1:name} {\n    my ($self${2}) = @_;\n    ${3}\n}",
        detail: "method subroutine",
        documentation: "Subroutine with `$self` as first argument (method pattern).",
        scope: SnippetScope::Any,
    },
    Snippet {
        trigger: "suba",
        body: "sub ${1:name} {\n    my (${2}) = @_;\n    ${3}\n}",
        detail: "subroutine with args",
        documentation: "Subroutine that unpacks `@_` into named variables.",
        scope: SnippetScope::Any,
    },
    // -- Control flow --
    Snippet {
        trigger: "fore",
        body: "foreach my \\$${1:item} (@${2:list}) {\n    ${3}\n}",
        detail: "foreach loop",
        documentation: "Iterate over a list with a lexical loop variable.",
        scope: SnippetScope::Any,
    },
    Snippet {
        trigger: "whi",
        body: "while (${1:condition}) {\n    ${2}\n}",
        detail: "while loop",
        documentation: "Loop while a condition is true.",
        scope: SnippetScope::Any,
    },
    Snippet {
        trigger: "iff",
        body: "if (${1:condition}) {\n    ${2}\n} elsif (${3}) {\n    ${4}\n} else {\n    ${5}\n}",
        detail: "if/elsif/else",
        documentation: "Full if-elsif-else chain.",
        scope: SnippetScope::Any,
    },
    // -- Module patterns (file scope only) --
    Snippet {
        trigger: "pkg",
        body: "package ${1:Name};\nuse strict;\nuse warnings;\n\n${2}\n\n1;",
        detail: "package boilerplate",
        documentation: "Minimal Perl package with strict/warnings and trailing `1;`.",
        scope: SnippetScope::FileScope,
    },
    Snippet {
        trigger: "moose",
        body: "package ${1:Name};\nuse Moose;\n\nhas '${2:attr}' => (\n    is => '${3:ro}',\n    isa => '${4:Str}',\n);\n\n__PACKAGE__->meta->make_immutable;\n1;",
        detail: "Moose class",
        documentation: "Moose-based class with an attribute and `make_immutable`.",
        scope: SnippetScope::FileScope,
    },
    // -- Test patterns (test files only) --
    Snippet {
        trigger: "test",
        body: "use strict;\nuse warnings;\nuse Test::More;\n\n${1}\n\ndone_testing;",
        detail: "test file boilerplate",
        documentation: "Minimal test file with strict/warnings, Test::More, and `done_testing`.",
        scope: SnippetScope::TestOnly,
    },
    Snippet {
        trigger: "subt",
        body: "subtest '${1:description}' => sub {\n    ${2}\n};",
        detail: "subtest block",
        documentation: "Named subtest block for grouping related assertions.",
        scope: SnippetScope::TestOnly,
    },
    // -- Data structures --
    Snippet {
        trigger: "hashr",
        body: "my \\$${1:hash} = {\n    ${2:key} => ${3:value},\n};",
        detail: "hash reference",
        documentation: "Anonymous hash reference assigned to a scalar.",
        scope: SnippetScope::Any,
    },
    Snippet {
        trigger: "arrayr",
        body: "my \\$${1:array} = [\n    ${2}\n];",
        detail: "array reference",
        documentation: "Anonymous array reference assigned to a scalar.",
        scope: SnippetScope::Any,
    },
];

/// Add Perl idiom snippet completions to the completion list.
///
/// Snippets are filtered by prefix match and scope context:
/// - `FileScope` snippets only appear when the cursor is at the top level
///   (not inside a subroutine body).
/// - `TestOnly` snippets only appear in test files (`.t`) or files that
///   import Test::More / Test2.
pub fn add_snippet_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    filepath: Option<&str>,
) {
    let is_test = is_test_file(source, filepath);
    let is_file_scope = is_at_file_scope(source, context.position);

    for snippet in SNIPPETS {
        // Skip if prefix doesn't match.
        if !context.prefix.is_empty() && !snippet.trigger.starts_with(&*context.prefix) {
            continue;
        }

        // Scope gating.
        match snippet.scope {
            SnippetScope::FileScope if !is_file_scope => continue,
            SnippetScope::TestOnly if !is_test => continue,
            _ => {}
        }

        completions.push(CompletionItem {
            label: snippet.trigger.to_string(),
            kind: crate::completion::items::CompletionItemKind::Snippet,
            detail: Some(snippet.detail.to_string()),
            documentation: Some(snippet.documentation.to_string()),
            insert_text: Some(snippet.body.to_string()),
            // Sort after keywords (4_) but with a snippet sub-prefix for grouping.
            sort_text: Some(format!("4_snip_{}", snippet.trigger)),
            filter_text: Some(snippet.trigger.to_string()),
            additional_edits: vec![],
            text_edit_range: Some((context.prefix_start, context.position)),
        });
    }
}

/// Check if the file is a test file based on path extension or source imports.
fn is_test_file(source: &str, filepath: Option<&str>) -> bool {
    if let Some(path) = filepath {
        if path.ends_with(".t") {
            return true;
        }
    }
    source.contains("use Test::More") || source.contains("use Test2::V0")
}

/// Heuristic: the cursor is at file scope if it is not inside a `sub { ... }` block.
///
/// We walk backwards from the cursor and track brace depth. If we encounter a `sub`
/// keyword at brace depth 0 (meaning we never closed out of a block), we are inside
/// a subroutine body.
fn is_at_file_scope(source: &str, position: usize) -> bool {
    let before = &source[..position];
    let mut depth: i32 = 0;

    // Walk backwards through characters.
    for ch in before.chars().rev() {
        match ch {
            '}' => depth += 1,
            '{' => {
                depth -= 1;
                if depth < 0 {
                    // We are inside an unclosed block. Check if it belongs to `sub`.
                    let brace_pos = before.rfind('{').and_then(|p| {
                        let remaining = &before[..p];
                        let pre = remaining.trim_end();
                        if pre.ends_with("sub")
                            || pre
                                .rsplit_once(char::is_whitespace)
                                .is_some_and(|(rest, _)| rest.trim_end().ends_with("sub"))
                        {
                            Some(())
                        } else {
                            None
                        }
                    });
                    if brace_pos.is_some() {
                        return false;
                    }
                    return false;
                }
            }
            _ => {}
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::must;

    fn get_snippets(source: &str, filepath: Option<&str>) -> Vec<CompletionItem> {
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        let provider = crate::CompletionProvider::new(&ast);
        provider.get_completions_with_path(source, source.len(), filepath)
    }

    #[test]
    fn snippet_subm_available_at_file_scope() {
        let code = "subm";
        let completions = get_snippets(code, Some("lib/Foo.pm"));
        assert!(
            completions.iter().any(|c| c.label == "subm"),
            "expected subm snippet at file scope: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn snippet_suba_available() {
        let completions = get_snippets("suba", Some("lib/Foo.pm"));
        assert!(completions.iter().any(|c| c.label == "suba"), "expected suba snippet");
    }

    #[test]
    fn snippet_fore_available() {
        let completions = get_snippets("fore", Some("lib/Foo.pm"));
        assert!(completions.iter().any(|c| c.label == "fore"), "expected fore snippet");
    }

    #[test]
    fn snippet_whi_available() {
        let completions = get_snippets("whi", Some("lib/Foo.pm"));
        assert!(completions.iter().any(|c| c.label == "whi"), "expected whi snippet");
    }

    #[test]
    fn snippet_iff_available() {
        let completions = get_snippets("iff", Some("lib/Foo.pm"));
        assert!(completions.iter().any(|c| c.label == "iff"), "expected iff snippet");
    }

    #[test]
    fn snippet_pkg_only_at_file_scope() {
        let completions = get_snippets("pkg", Some("lib/Foo.pm"));
        assert!(completions.iter().any(|c| c.label == "pkg"), "expected pkg at file scope");
    }

    #[test]
    fn snippet_pkg_hidden_inside_sub() {
        let completions = get_snippets("sub foo {\n    pkg", Some("lib/Foo.pm"));
        assert!(!completions.iter().any(|c| c.label == "pkg"), "pkg should NOT appear inside sub");
    }

    #[test]
    fn snippet_moose_only_at_file_scope() {
        let completions = get_snippets("moose", Some("lib/Foo.pm"));
        assert!(completions.iter().any(|c| c.label == "moose"), "expected moose at file scope");
    }

    #[test]
    fn snippet_test_only_in_test_file() {
        let completions = get_snippets("test", Some("t/basic.t"));
        assert!(completions.iter().any(|c| c.label == "test"), "expected test in .t file");
    }

    #[test]
    fn snippet_test_hidden_in_non_test_file() {
        let completions = get_snippets("test", Some("lib/Foo.pm"));
        let has = completions
            .iter()
            .any(|c| c.label == "test" && c.detail.as_deref() == Some("test file boilerplate"));
        assert!(!has, "test boilerplate should NOT appear in non-test files");
    }

    #[test]
    fn snippet_subt_only_in_test_file() {
        let completions = get_snippets("subt", Some("t/basic.t"));
        assert!(completions.iter().any(|c| c.label == "subt"), "expected subt in .t file");
    }

    #[test]
    fn snippet_subt_hidden_in_non_test_file() {
        let completions = get_snippets("subt", Some("lib/Foo.pm"));
        assert!(
            !completions.iter().any(|c| c.label == "subt"),
            "subt should NOT appear in non-test"
        );
    }

    #[test]
    fn snippet_hashr_available() {
        let completions = get_snippets("hashr", Some("lib/Foo.pm"));
        assert!(completions.iter().any(|c| c.label == "hashr"), "expected hashr snippet");
    }

    #[test]
    fn snippet_arrayr_available() {
        let completions = get_snippets("arrayr", Some("lib/Foo.pm"));
        assert!(completions.iter().any(|c| c.label == "arrayr"), "expected arrayr snippet");
    }

    #[test]
    fn snippet_items_have_snippet_kind() {
        let completions = get_snippets("subm", Some("lib/Foo.pm"));
        let subm = completions.iter().find(|c| c.label == "subm");
        assert!(subm.is_some(), "expected subm in completions");
        assert_eq!(subm.map(|c| c.kind), Some(crate::CompletionItemKind::Snippet));
    }

    #[test]
    fn snippet_items_have_documentation() {
        let completions = get_snippets("fore", Some("lib/Foo.pm"));
        let fore = completions.iter().find(|c| c.label == "fore");
        assert!(fore.is_some());
        assert!(fore.and_then(|c| c.documentation.as_ref()).is_some());
    }

    #[test]
    fn snippet_test_appears_with_test_import() {
        let completions = get_snippets("use Test::More;\ntest", Some("lib/Foo.pm"));
        let has = completions
            .iter()
            .any(|c| c.label == "test" && c.detail.as_deref() == Some("test file boilerplate"));
        assert!(has, "test snippet should appear when Test::More is imported");
    }

    #[test]
    fn is_at_file_scope_empty_source() {
        assert!(is_at_file_scope("", 0));
    }

    #[test]
    fn is_at_file_scope_top_level() {
        let code = "my $x = 1;\n";
        assert!(is_at_file_scope(code, code.len()));
    }

    #[test]
    fn is_at_file_scope_inside_sub() {
        let code = "sub foo {\n    ";
        assert!(!is_at_file_scope(code, code.len()));
    }

    #[test]
    fn is_at_file_scope_after_closed_sub() {
        let code = "sub foo {\n    my $x;\n}\n";
        assert!(is_at_file_scope(code, code.len()));
    }
}

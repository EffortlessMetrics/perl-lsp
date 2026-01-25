//! Keyword completion for Perl
//!
//! Provides completion for Perl keywords with snippet expansion.

use crate::ide::lsp_compat::completion::{context::CompletionContext, items::CompletionItem};
use std::collections::HashSet;

/// Create the keywords HashSet
pub fn create_keywords() -> HashSet<&'static str> {
    [
        "my",
        "our",
        "local",
        "state",
        "sub",
        "package",
        "use",
        "require",
        "if",
        "elsif",
        "else",
        "unless",
        "while",
        "until",
        "for",
        "foreach",
        "do",
        "eval",
        "goto",
        "last",
        "next",
        "redo",
        "return",
        "die",
        "warn",
        "exit",
        "and",
        "or",
        "not",
        "xor",
        "eq",
        "ne",
        "lt",
        "le",
        "gt",
        "ge",
        "cmp",
        "defined",
        "undef",
        "ref",
        "blessed",
        "scalar",
        "wantarray",
        "__PACKAGE__",
        "__FILE__",
        "__LINE__",
        "BEGIN",
        "END",
        "CHECK",
        "INIT",
        "UNITCHECK",
        "DESTROY",
        "AUTOLOAD",
    ]
    .into_iter()
    .collect()
}

/// Add keyword completions
pub fn add_keyword_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    keywords: &HashSet<&'static str>,
) {
    for keyword in keywords {
        if keyword.starts_with(&context.prefix) {
            let (insert_text, snippet) = match *keyword {
                "sub" => ("sub ${1:name} {\n    $0\n}", true),
                "if" => ("if ($1) {\n    $0\n}", true),
                "elsif" => ("elsif ($1) {\n    $0\n}", true),
                "else" => ("else {\n    $0\n}", true),
                "unless" => ("unless ($1) {\n    $0\n}", true),
                "while" => ("while ($1) {\n    $0\n}", true),
                "for" => ("for (my $i = 0; $i < $1; $i++) {\n    $0\n}", true),
                "foreach" => ("foreach my $${1:item} (@${2:array}) {\n    $0\n}", true),
                "package" => ("package ${1:Name};\n\n$0", true),
                "use" => ("use ${1:Module};\n$0", true),
                _ => (*keyword, false),
            };

            completions.push(CompletionItem {
                label: keyword.to_string(),
                kind: if snippet {
                    crate::ide::lsp_compat::completion::items::CompletionItemKind::Snippet
                } else {
                    crate::ide::lsp_compat::completion::items::CompletionItemKind::Keyword
                },
                detail: Some("keyword".to_string()),
                documentation: None,
                insert_text: Some(insert_text.to_string()),
                sort_text: Some(format!("4_{}", keyword)),
                filter_text: Some(keyword.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
            });
        }
    }
}

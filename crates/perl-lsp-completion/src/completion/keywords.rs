//! Keyword completion for Perl
//!
//! Provides completion for Perl keywords with snippet expansion.

use super::{context::CompletionContext, items::CompletionItem};
use perl_keywords::LSP_COMPLETION_KEYWORDS;
use perl_lsp_keyword_snippets::template_for_keyword;

/// Canonical Perl keywords for completion.
#[must_use]
pub fn keywords() -> &'static [&'static str] {
    LSP_COMPLETION_KEYWORDS
}

/// Add keyword completions
pub fn add_keyword_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    keywords: &[&'static str],
) {
    for &keyword in keywords {
        if keyword.starts_with(&context.prefix) {
            let template = template_for_keyword(keyword);
            let (insert_text, snippet) =
                if template.is_snippet { (template.insert_text, true) } else { (keyword, false) };

            completions.push(CompletionItem {
                label: keyword.to_string(),
                kind: if snippet {
                    crate::completion::items::CompletionItemKind::Snippet
                } else {
                    crate::completion::items::CompletionItemKind::Keyword
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

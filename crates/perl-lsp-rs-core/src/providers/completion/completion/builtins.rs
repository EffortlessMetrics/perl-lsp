//! Built-in function completion for Perl
//!
//! Provides completion for Perl built-in functions with signatures.

mod catalog;
mod metadata;

use super::{context::CompletionContext, items::CompletionItem};
use std::collections::HashSet;

/// Create the builtins HashSet
pub fn create_builtins() -> HashSet<&'static str> {
    catalog::all_names().collect()
}

/// Add built-in function completions
pub fn add_builtin_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    builtins: &HashSet<&'static str>,
) {
    for builtin in builtins {
        if builtin.starts_with(&context.prefix) {
            let (insert_text, detail, documentation) = metadata::builtin_info(builtin);

            completions.push(CompletionItem {
                label: builtin.to_string(),
                kind: super::items::CompletionItemKind::Function,
                detail: Some(detail.to_string()),
                documentation: documentation.map(str::to_string),
                insert_text: Some(insert_text.to_string()),
                sort_text: Some(format!("3_{}", builtin)),
                filter_text: Some(builtin.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
                label_details: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn context_for(prefix: &str) -> CompletionContext {
        CompletionContext {
            position: prefix.len(),
            trigger_character: None,
            in_string: false,
            in_regex: false,
            in_comment: false,
            in_use_statement: false,
            current_package: "main".to_string(),
            prefix: prefix.to_string(),
            prefix_start: 0,
            cursor_scope_id: 0,
        }
    }

    #[test]
    fn builtin_completion_uses_cataloged_metadata() -> TestResult {
        let builtins = create_builtins();
        let mut completions = Vec::new();
        add_builtin_completions(&mut completions, &context_for("op"), &builtins);

        let open = completions
            .iter()
            .find(|item| item.label == "open")
            .ok_or("open completion missing")?;

        assert_eq!(open.detail.as_deref(), Some("open FILEHANDLE, MODE, FILENAME"));
        assert_eq!(
            open.insert_text.as_deref(),
            Some("open(my $fh, '${1:<}', ${2:\\$file}) or die \"Cannot open ${2:\\$file}: $!\";")
        );
        assert!(open.documentation.as_deref().is_some_and(|doc| doc.contains("Three-arg")));

        Ok(())
    }

    #[test]
    fn builtin_completion_keeps_fallback_for_catalog_entries_without_metadata() -> TestResult {
        let builtins = create_builtins();
        let mut completions = Vec::new();
        add_builtin_completions(&mut completions, &context_for("-r"), &builtins);

        let file_test =
            completions.iter().find(|item| item.label == "-r").ok_or("-r completion missing")?;

        assert_eq!(file_test.detail.as_deref(), Some("built-in function"));
        assert_eq!(file_test.insert_text.as_deref(), Some("-r"));
        assert_eq!(file_test.documentation.as_deref(), Some("Perl built-in function."));

        Ok(())
    }
}

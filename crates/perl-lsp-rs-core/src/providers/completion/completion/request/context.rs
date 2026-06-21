use super::super::{CompletionContext, CompletionItem, CompletionProvider, regex_patterns, sort};

pub(super) fn prepare_context(
    provider: &CompletionProvider,
    source: &str,
    position: usize,
) -> Option<CompletionContext> {
    if position > source.len() {
        return None;
    }

    let context = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        provider.analyze_context(source, position)
    }))
    .ok()
    .map(|mut context| {
        context.in_use_statement = CompletionProvider::is_use_statement_context(source, position);
        context
    })?;

    (!context.in_comment).then_some(context)
}

pub(super) fn rejects_dash_trigger(context: &CompletionContext) -> bool {
    context.trigger_character == Some('-')
        && !(context.prefix.ends_with("->") && context.prefix.len() > 2)
}

pub(super) fn rejects_lexical_block(source: &str, position: usize) -> bool {
    CompletionProvider::is_in_heredoc(source, position)
        || CompletionProvider::is_in_pod(source, position)
}

pub(super) fn complete_regex_context(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
) -> Option<Vec<CompletionItem>> {
    if CompletionProvider::is_in_regex_flags(source, context.position) {
        regex_patterns::add_regex_flag_completions(completions, context, source);
        return Some(sort::deduplicate_and_sort(std::mem::take(completions)));
    }

    if context.in_regex && has_regex_sigil_prefix(&context.prefix) {
        return Some(Vec::new());
    }

    if context.in_regex {
        regex_patterns::add_regex_completions(completions, context, source);
        return Some(sort::deduplicate_and_sort(std::mem::take(completions)));
    }

    None
}

fn has_regex_sigil_prefix(prefix: &str) -> bool {
    matches!(prefix.chars().next(), Some('$' | '@' | '%'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regex_context(prefix: &str) -> CompletionContext {
        CompletionContext {
            position: 25,
            trigger_character: None,
            in_string: false,
            in_regex: true,
            in_comment: false,
            in_use_statement: false,
            current_package: "main".to_string(),
            prefix: prefix.to_string(),
            prefix_start: 23,
            cursor_scope_id: 0,
        }
    }

    #[test]
    fn complete_regex_context_suppresses_scalar_sigil_prefix() {
        let context = regex_context("$m");
        let mut completions = Vec::new();

        let result =
            complete_regex_context(&mut completions, &context, "if ($str =~ /match $m/) {");

        assert!(result.as_ref().is_some_and(Vec::is_empty));
    }

    #[test]
    fn complete_regex_context_suppresses_array_sigil_prefix() {
        let context = regex_context("@a");
        let mut completions = Vec::new();

        let result =
            complete_regex_context(&mut completions, &context, "if ($str =~ /pattern @a/) {");

        assert!(result.as_ref().is_some_and(Vec::is_empty));
    }

    #[test]
    fn complete_regex_context_suppresses_hash_sigil_prefix() {
        let context = regex_context("%h");
        let mut completions = Vec::new();

        let result =
            complete_regex_context(&mut completions, &context, "if ($str =~ /pattern %h/) {");

        assert!(result.as_ref().is_some_and(Vec::is_empty));
    }
}

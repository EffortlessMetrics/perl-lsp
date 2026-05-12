mod context;
mod dispatch;

use super::{CompletionItem, CompletionProvider, sort};

pub(super) fn complete(
    provider: &CompletionProvider,
    source: &str,
    position: usize,
    filepath: Option<&str>,
    is_cancelled: &dyn Fn() -> bool,
) -> Vec<CompletionItem> {
    let Some(context) = context::prepare_context(provider, source, position) else {
        return vec![];
    };

    if is_cancelled() || context::rejects_dash_trigger(&context) {
        return vec![];
    }

    let mut completions = Vec::new();
    if let Some(regex_completions) =
        context::complete_regex_context(&mut completions, &context, source)
    {
        return regex_completions;
    }

    match dispatch::complete_dispatch(
        provider,
        &mut completions,
        &context,
        source,
        position,
        filepath,
        is_cancelled,
    ) {
        CompletionFlow::SortAndReturn => sort::deduplicate_and_sort(completions),
        CompletionFlow::Return(items) => items,
        CompletionFlow::Cancelled => vec![],
    }
}

pub(super) enum CompletionFlow {
    SortAndReturn,
    Return(Vec<CompletionItem>),
    Cancelled,
}

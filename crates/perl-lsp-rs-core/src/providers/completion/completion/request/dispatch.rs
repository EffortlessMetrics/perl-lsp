use super::super::{
    CompletionContext, CompletionItem, CompletionProvider, builtins, file_path, functions,
    keywords, methods, packages, snippets, test_more, variables, workspace, xs_api,
};
use super::CompletionFlow;
use perl_semantic_analyzer::symbol::SymbolKind;

pub(super) fn complete_dispatch(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    position: usize,
    filepath: Option<&str>,
    is_cancelled: &dyn Fn() -> bool,
) -> CompletionFlow {
    if complete_use_or_structural_context(
        provider,
        completions,
        context,
        source,
        position,
        is_cancelled,
    ) {
        return CompletionFlow::SortAndReturn;
    }

    if let Some(flow) = complete_sigil_context(provider, completions, context, is_cancelled) {
        return flow;
    }

    if complete_symbol_namespace_context(provider, completions, context) {
        return CompletionFlow::SortAndReturn;
    }

    if context.in_string {
        complete_file_path_context(completions, context, source, is_cancelled);
        return CompletionFlow::SortAndReturn;
    }

    complete_general_context(provider, completions, context, source, filepath, is_cancelled)
}

fn complete_use_or_structural_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    position: usize,
    is_cancelled: &dyn Fn() -> bool,
) -> bool {
    if let Some((module_name, qw_prefix)) =
        CompletionProvider::detect_use_qw_import_context(source, position)
    {
        workspace::add_use_qw_import_completions(
            completions,
            context,
            &provider.workspace_index,
            &module_name,
            &qw_prefix,
        );
        return true;
    }

    if context.in_use_statement && !context.prefix.starts_with('$') {
        workspace::add_use_module_completions_with_cache(
            completions,
            context,
            &provider.workspace_index,
            &provider.include_paths,
            &provider.system_inc_paths,
            provider.include_system_inc,
            provider.scan_cache.as_deref(),
            is_cancelled,
        );
        return true;
    }

    if provider.is_has_type_value_context(source, position) {
        provider.add_has_type_completions(completions, context);
        return true;
    }

    if provider.is_has_options_key_context(source, position) {
        provider.add_has_option_completions(completions, context);
        return true;
    }

    if !context.in_string
        && !context.in_regex
        && let Some((varname, key_prefix)) =
            CompletionProvider::detect_hash_key_context(source, position)
    {
        CompletionProvider::add_hash_key_completions(
            completions,
            context,
            source,
            &varname,
            &key_prefix,
        );
        return true;
    }

    if let Some(package_name) = provider.object_pad_constructor_package(source, position) {
        provider.add_object_pad_constructor_completions(completions, context, &package_name);
        return true;
    }

    if is_method_arrow_context(context) {
        methods::add_method_completions(completions, context, source, &provider.symbol_table);
        workspace::add_workspace_method_completions(
            completions,
            context,
            source,
            provider.type_engine.as_ref(),
            &provider.workspace_index,
            &provider.used_modules,
        );
        return true;
    }

    false
}

fn is_method_arrow_context(context: &CompletionContext) -> bool {
    (context.trigger_character == Some('>') || context.trigger_character == Some('-'))
        && context.prefix.ends_with("->")
        && context.prefix.len() > 2
}

fn complete_sigil_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    is_cancelled: &dyn Fn() -> bool,
) -> Option<CompletionFlow> {
    let (sigil, kind) = sigil_kind(context)?;

    if context.prefix.contains("::") {
        packages::add_package_completions(
            completions,
            context,
            &provider.symbol_table,
            &provider.workspace_index,
        );
        if !completions.is_empty() {
            return Some(CompletionFlow::Return(std::mem::take(completions)));
        }
    }

    variables::add_variable_completions(completions, context, kind, &provider.symbol_table);
    if is_cancelled() {
        return Some(CompletionFlow::Cancelled);
    }
    variables::add_special_variables(completions, context, sigil);
    Some(CompletionFlow::SortAndReturn)
}

fn sigil_kind(context: &CompletionContext) -> Option<(&'static str, SymbolKind)> {
    if context.prefix.starts_with('$') {
        Some(("$", SymbolKind::scalar()))
    } else if context.prefix.starts_with('@') {
        Some(("@", SymbolKind::array()))
    } else if context.prefix.starts_with('%') {
        Some(("%", SymbolKind::hash()))
    } else {
        None
    }
}

fn complete_symbol_namespace_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
) -> bool {
    if context.prefix.starts_with('&') {
        functions::add_function_completions(completions, context, &provider.symbol_table);
        return true;
    }

    if context.prefix.contains("::") {
        packages::add_package_completions(
            completions,
            context,
            &provider.symbol_table,
            &provider.workspace_index,
        );
        return true;
    }

    false
}

fn complete_file_path_context(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    is_cancelled: &dyn Fn() -> bool,
) {
    let line_prefix = &source[..context.position];
    let Some(start) = line_prefix.rfind(['"', '\'']) else {
        return;
    };

    let Some(quote_char) = source.get(start..).and_then(|text| text.chars().next()) else {
        return;
    };
    let string_end =
        source[start + 1..].find(quote_char).map(|index| start + 1 + index).unwrap_or(source.len());
    let full_string_content = &source[start + 1..string_end];
    if full_string_content.contains('\0') {
        return;
    }

    let path_prefix = &line_prefix[start + 1..];
    if looks_like_path_prefix(path_prefix) {
        let file_context =
            file_path::FileCompletionContext::new(path_prefix, start + 1, context.position);
        completions.extend(file_path::complete_file_paths(&file_context, is_cancelled));
    }
}

fn looks_like_path_prefix(path_prefix: &str) -> bool {
    path_prefix.contains('/')
        || path_prefix.contains('\\')
        || path_prefix.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-')
}

fn complete_general_context(
    provider: &CompletionProvider,
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    source: &str,
    filepath: Option<&str>,
    is_cancelled: &dyn Fn() -> bool,
) -> CompletionFlow {
    let keyword_set = keywords::keywords();
    if context.prefix.is_empty() || provider.could_be_keyword(&context.prefix, keyword_set) {
        keywords::add_keyword_completions(completions, context, keyword_set);
        if is_cancelled() {
            return CompletionFlow::Cancelled;
        }
    }

    let builtin_set = builtins::create_builtins();
    xs_api::add_xs_api_completions(completions, context, source, filepath);
    if context.prefix.is_empty() || provider.could_be_function(&context.prefix, &builtin_set) {
        builtins::add_builtin_completions(completions, context, &builtin_set);
        if is_cancelled() {
            return CompletionFlow::Cancelled;
        }
        functions::add_function_completions(completions, context, &provider.symbol_table);
        if is_cancelled() {
            return CompletionFlow::Cancelled;
        }
    }

    snippets::add_snippet_completions(completions, context);
    if is_cancelled() {
        return CompletionFlow::Cancelled;
    }

    variables::add_all_variables(completions, context, &provider.symbol_table);
    if is_cancelled() {
        return CompletionFlow::Cancelled;
    }

    workspace::add_visible_symbol_completions(
        completions,
        context,
        &provider.workspace_index,
        filepath,
    );
    if is_cancelled() {
        return CompletionFlow::Cancelled;
    }

    workspace::add_workspace_symbol_completions(
        completions,
        context,
        &provider.workspace_index,
        &provider.import_map,
    );
    if is_cancelled() {
        return CompletionFlow::Cancelled;
    }

    if provider.is_test_context(source, filepath) {
        test_more::add_test_more_completions(completions, context);
    }

    CompletionFlow::SortAndReturn
}

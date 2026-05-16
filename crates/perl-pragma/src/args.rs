use crate::PragmaState;

const MAX_DISABLED_WARNING_CATEGORIES: usize = 256;

pub(crate) fn builtin_import_names(arg: &str) -> Vec<String> {
    let trimmed = arg.trim();

    if let Some(inner) = trimmed.strip_prefix("qw(").and_then(|s| s.strip_suffix(')')) {
        return inner
            .split_whitespace()
            .filter(|name| !name.is_empty())
            .map(|name| name.trim_matches('\'').trim_matches('"').to_string())
            .collect();
    }

    let name = trimmed.trim_matches('\'').trim_matches('"');
    if name.is_empty() { Vec::new() } else { vec![name.to_string()] }
}

pub(crate) fn apply_builtin_imports(state: &mut PragmaState, args: &[String]) {
    for arg in args {
        for name in builtin_import_names(arg) {
            if !state.builtin_imports.iter().any(|import| import == &name) {
                state.builtin_imports.push(name);
            }
        }
    }
}

/// Insert `category` into `state.disabled_warning_categories` if not already present and
/// within the hard cap of [`MAX_DISABLED_WARNING_CATEGORIES`].
///
/// Categories beyond the cap are silently dropped. In valid Perl code this is never reached
/// (Perl's own warning hierarchy has ~30 leaf categories); the cap is a safety guard against
/// pathological or adversarial AST input that would otherwise cause O(n²) clone cost.
pub(crate) fn add_disabled_warning_category(state: &mut PragmaState, category: &str) {
    if category.is_empty() {
        return;
    }

    if state.disabled_warning_categories.iter().any(|c| c == category) {
        return;
    }

    if state.disabled_warning_categories.len() >= MAX_DISABLED_WARNING_CATEGORIES {
        return;
    }

    state.disabled_warning_categories.push(category.to_string());
}

pub(crate) fn remove_builtin_imports(state: &mut PragmaState, args: &[String]) {
    if args.is_empty() {
        state.builtin_imports.clear();
        return;
    }

    let names_to_remove: Vec<String> =
        args.iter().flat_map(|arg| builtin_import_names(arg)).collect();
    state.builtin_imports.retain(|import| !names_to_remove.iter().any(|name| name == import));
}

pub(crate) fn pragma_arg_items(arg: &str) -> Vec<String> {
    let trimmed = arg.trim().trim_matches('\'').trim_matches('"');

    if let Some(inner) = trimmed.strip_prefix("qw(").and_then(|s| s.strip_suffix(')')) {
        return inner.split_whitespace().map(|item| item.to_string()).collect();
    }

    if trimmed.contains(char::is_whitespace) {
        return trimmed.split_whitespace().map(|item| item.to_string()).collect();
    }

    vec![trimmed.to_string()]
}

pub(crate) fn normalized_pragma_token(arg: &str) -> &str {
    arg.trim().trim_matches('\'').trim_matches('"')
}

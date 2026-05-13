use crate::{
    PragmaState, add_disabled_warning_category, apply_builtin_imports, apply_feature_state,
    conditional_pragma_target, enable_effective_version_semantics, normalized_pragma_token,
    parse_perl_version, pragma_arg_items, remove_builtin_imports,
};
use std::ops::Range;

pub(super) fn apply_use_directive(
    range: Range<usize>,
    module: &str,
    args: &[String],
    state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    if apply_conditional_use(module, args, range.clone(), state, ranges) {
        return;
    }

    match module {
        "strict" => {
            set_strict_categories(state, args, true);
            push_state(range, state, ranges);
        }
        "warnings" => {
            state.warnings = true;
            state.disabled_warning_categories.clear();
            push_state(range, state, ranges);
        }
        "utf8" => {
            state.utf8 = true;
            push_state(range, state, ranges);
        }
        "encoding" => {
            state.encoding = first_normalized_arg(args);
            push_state(range, state, ranges);
        }
        "locale" => {
            state.locale = true;
            state.locale_scope = first_normalized_arg(args);
            push_state(range, state, ranges);
        }
        "feature" => {
            if apply_feature_state(state, args, true) {
                push_state(range, state, ranges);
            }
        }
        "builtin" => {
            apply_builtin_imports(state, args);
            push_state(range, state, ranges);
        }
        _ => {
            if let Some(version) = parse_perl_version(module) {
                enable_effective_version_semantics(state, version);
                push_state(range, state, ranges);
            }
        }
    }
}

pub(super) fn apply_no_directive(
    range: Range<usize>,
    module: &str,
    args: &[String],
    state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    if apply_conditional_no(module, args, range.clone(), state, ranges) {
        return;
    }

    match module {
        "strict" => {
            set_strict_categories(state, args, false);
            push_state(range, state, ranges);
        }
        "warnings" => {
            apply_no_warnings(range, args, state, ranges);
        }
        "utf8" => {
            state.utf8 = false;
            push_state(range, state, ranges);
        }
        "encoding" => {
            state.encoding = None;
            push_state(range, state, ranges);
        }
        "locale" => {
            state.locale = false;
            state.locale_scope = None;
            push_state(range, state, ranges);
        }
        "feature" => {
            if apply_feature_state(state, args, false) {
                push_state(range, state, ranges);
            }
        }
        "builtin" => {
            remove_builtin_imports(state, args);
            push_state(range, state, ranges);
        }
        _ => {}
    }
}

fn apply_conditional_use(
    module: &str,
    args: &[String],
    range: Range<usize>,
    state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) -> bool {
    if !matches!(module, "if" | "unless") {
        return false;
    }

    if let Some((target, target_args)) = conditional_pragma_target(args) {
        apply_conditional_use_target(range, target, target_args, state, ranges);
    }
    true
}

fn apply_conditional_no(
    module: &str,
    args: &[String],
    range: Range<usize>,
    state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) -> bool {
    if !matches!(module, "if" | "unless") {
        return false;
    }

    if let Some((target, target_args)) = conditional_pragma_target(args) {
        apply_conditional_no_target(range, target, target_args, state, ranges);
    }
    true
}

fn apply_conditional_use_target(
    range: Range<usize>,
    module: &str,
    args: &[String],
    state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    match module {
        "strict" => set_strict_categories(state, args, true),
        "warnings" => {
            state.warnings = true;
            state.disabled_warning_categories.clear();
        }
        "utf8" => state.utf8 = true,
        "encoding" => state.encoding = first_normalized_arg(args),
        "locale" => {
            state.locale = true;
            state.locale_scope = first_normalized_arg(args);
        }
        "feature" => {
            if !apply_feature_state(state, args, true) {
                return;
            }
        }
        "builtin" => apply_builtin_imports(state, args),
        _ => {
            if let Some(version) = parse_perl_version(module) {
                enable_effective_version_semantics(state, version);
            } else {
                return;
            }
        }
    }
    push_state(range, state, ranges);
}

fn apply_conditional_no_target(
    range: Range<usize>,
    module: &str,
    args: &[String],
    state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    match module {
        "strict" => set_strict_categories(state, args, false),
        "warnings" => disable_warnings_categories(args, state),
        "utf8" => state.utf8 = false,
        "encoding" => state.encoding = None,
        "locale" => {
            state.locale = false;
            state.locale_scope = None;
        }
        "feature" => {
            if !apply_feature_state(state, args, false) {
                return;
            }
        }
        "builtin" => remove_builtin_imports(state, args),
        _ => return,
    }
    push_state(range, state, ranges);
}

fn set_strict_categories(state: &mut PragmaState, args: &[String], enabled: bool) {
    if args.is_empty() {
        state.strict_vars = enabled;
        state.strict_subs = enabled;
        state.strict_refs = enabled;
        return;
    }

    for arg in args {
        for item in pragma_arg_items(arg) {
            match item.as_str() {
                "vars" => state.strict_vars = enabled,
                "subs" => state.strict_subs = enabled,
                "refs" => state.strict_refs = enabled,
                _ => {}
            }
        }
    }
}

fn apply_no_warnings(
    range: Range<usize>,
    args: &[String],
    state: &mut PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    let warnings_before = state.warnings;
    let had_disabled_before = !state.disabled_warning_categories.is_empty();
    let before = state.disabled_warning_categories.len();
    disable_warnings_categories(args, state);

    let changed = if args.is_empty() {
        warnings_before || had_disabled_before
    } else {
        state.disabled_warning_categories.len() != before
    };
    if changed {
        push_state(range, state, ranges);
    }
}

fn disable_warnings_categories(args: &[String], state: &mut PragmaState) {
    if args.is_empty() {
        state.warnings = false;
        state.disabled_warning_categories.clear();
        return;
    }

    for arg in args {
        add_disabled_warning_category(state, normalized_pragma_token(arg));
    }
}

fn first_normalized_arg(args: &[String]) -> Option<String> {
    args.first().map(|arg| normalized_pragma_token(arg).to_string())
}

fn push_state(
    range: Range<usize>,
    state: &PragmaState,
    ranges: &mut Vec<(Range<usize>, PragmaState)>,
) {
    ranges.push((range, state.clone()));
}

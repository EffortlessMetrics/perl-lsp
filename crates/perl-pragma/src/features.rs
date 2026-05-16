use crate::{PragmaState, features_enabled_by_version, parse_perl_version, pragma_arg_items};

fn feature_items(arg: &str) -> Vec<String> {
    pragma_arg_items(arg)
}

fn known_feature_name(name: &str) -> Option<&'static str> {
    match name {
        "say" => Some("say"),
        "state" => Some("state"),
        "switch" => Some("switch"),
        "unicode_strings" => Some("unicode_strings"),
        "unicode_eval" => Some("unicode_eval"),
        "evalbytes" => Some("evalbytes"),
        "current_sub" => Some("current_sub"),
        "fc" => Some("fc"),
        "postfix_deref" => Some("postfix_deref"),
        "try" => Some("try"),
        "signatures" => Some("signatures"),
        "defer" => Some("defer"),
        "isa" => Some("isa"),
        "class" => Some("class"),
        "field" => Some("field"),
        "method" => Some("method"),
        "builtin" => Some("builtin"),
        _ => None,
    }
}

const ALL_KNOWN_FEATURES: &[&str] = &[
    "say",
    "state",
    "switch",
    "unicode_strings",
    "unicode_eval",
    "evalbytes",
    "current_sub",
    "fc",
    "postfix_deref",
    "try",
    "signatures",
    "defer",
    "isa",
    "class",
    "field",
    "method",
    "builtin",
];

fn enable_feature_name(state: &mut PragmaState, name: &str) -> bool {
    if name == "signatures" {
        state.signatures_strict = true;
    }
    if name == "unicode_strings" {
        state.unicode_strings = true;
    }

    if let Some(feature) = known_feature_name(name) {
        if state.features.iter().all(|existing| existing != &feature) {
            state.features.push(feature);
        }
        true
    } else {
        false
    }
}

fn disable_feature_name(state: &mut PragmaState, name: &str) -> bool {
    if name == "signatures" {
        state.signatures_strict = false;
    }
    if name == "unicode_strings" {
        state.unicode_strings = false;
    }

    if let Some(feature) = known_feature_name(name) {
        let before = state.features.len();
        state.features.retain(|existing| *existing != feature);
        before != state.features.len()
    } else {
        false
    }
}

pub(crate) fn apply_feature_state(state: &mut PragmaState, args: &[String], enabled: bool) -> bool {
    if !enabled && args.is_empty() {
        let changed =
            !state.features.is_empty() || state.unicode_strings || state.signatures_strict;
        state.features.clear();
        state.unicode_strings = false;
        state.signatures_strict = false;
        return changed;
    }

    let mut changed = false;

    for arg in args {
        for item in feature_items(arg) {
            if enabled && item == ":all" {
                for feature in ALL_KNOWN_FEATURES {
                    changed |= enable_feature_name(state, feature);
                }
                continue;
            }

            if !enabled && item == ":all" {
                let had_features =
                    !state.features.is_empty() || state.unicode_strings || state.signatures_strict;
                state.features.clear();
                state.unicode_strings = false;
                state.signatures_strict = false;
                changed |= had_features;
                continue;
            }

            if let Some(version) = item.strip_prefix(':').and_then(parse_perl_version) {
                for feature in features_enabled_by_version(version) {
                    changed |= if enabled {
                        enable_feature_name(state, feature)
                    } else {
                        disable_feature_name(state, feature)
                    };
                }
                continue;
            }

            changed |= if enabled {
                enable_feature_name(state, &item)
            } else {
                disable_feature_name(state, &item)
            };
        }
    }

    changed
}

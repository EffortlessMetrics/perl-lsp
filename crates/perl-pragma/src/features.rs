use crate::{
    PragmaState, features_enabled_by_version, parse_perl_version, pragma_arg_items,
    version::DEFAULT_FEATURES,
};

fn feature_items(arg: &str) -> Vec<String> {
    pragma_arg_items(arg)
}

fn known_feature_name(name: &str) -> Option<&'static str> {
    match name {
        "say" => Some("say"),
        "state" => Some("state"),
        "switch" => Some("switch"),
        "smartmatch" => Some("smartmatch"),
        "unicode_strings" => Some("unicode_strings"),
        "unicode_eval" => Some("unicode_eval"),
        "evalbytes" => Some("evalbytes"),
        "current_sub" => Some("current_sub"),
        "fc" => Some("fc"),
        "lexical_subs" => Some("lexical_subs"),
        "postderef" => Some("postderef"),
        "postderef_qq" | "postfix_deref" => Some("postderef_qq"),
        "refaliasing" => Some("refaliasing"),
        "bitwise" => Some("bitwise"),
        "declared_refs" => Some("declared_refs"),
        "try" => Some("try"),
        "signatures" => Some("signatures"),
        "defer" => Some("defer"),
        "isa" => Some("isa"),
        "class" => Some("class"),
        "field" => Some("field"),
        "method" => Some("method"),
        "builtin" | "module_true" => Some("module_true"),
        "indirect" => Some("indirect"),
        "multidimensional" => Some("multidimensional"),
        "bareword_filehandles" => Some("bareword_filehandles"),
        "extra_paired_delimiters" => Some("extra_paired_delimiters"),
        "apostrophe_as_package_separator" => Some("apostrophe_as_package_separator"),
        "keyword_any" => Some("keyword_any"),
        "keyword_all" => Some("keyword_all"),
        _ => None,
    }
}

const ALL_KNOWN_FEATURES: &[&str] = &[
    "say",
    "state",
    "smartmatch",
    "switch",
    "unicode_strings",
    "unicode_eval",
    "evalbytes",
    "current_sub",
    "fc",
    "lexical_subs",
    "postderef",
    "postderef_qq",
    "signatures",
    "refaliasing",
    "bitwise",
    "declared_refs",
    "isa",
    "indirect",
    "multidimensional",
    "bareword_filehandles",
    "try",
    "defer",
    "extra_paired_delimiters",
    "module_true",
    "class",
    "field",
    "method",
    "apostrophe_as_package_separator",
    "keyword_any",
    "keyword_all",
];

pub(crate) fn canonical_feature_query(feature: &str) -> &str {
    match feature {
        "builtin" => "module_true",
        "postfix_deref" => "postderef_qq",
        _ => feature,
    }
}

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
        let default_features = DEFAULT_FEATURES.to_vec();
        let changed =
            state.features != default_features || state.unicode_strings || state.signatures_strict;
        state.features = default_features;
        state.unicode_strings = state.has_feature("unicode_strings");
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

            if enabled && item == ":default" {
                for feature in DEFAULT_FEATURES {
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

            if !enabled && item == ":default" {
                for feature in DEFAULT_FEATURES {
                    changed |= disable_feature_name(state, feature);
                }
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

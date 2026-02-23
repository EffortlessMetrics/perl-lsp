//! Deterministic module-import rename edit planning.
//!
//! This crate isolates the small but critical responsibility of computing line
//! edits for Perl module file-rename workflows.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

/// A full-line replacement edit for a module rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleLineEdit {
    /// Zero-based source line index.
    pub line: usize,
    /// Start column (always `0` for full-line replacement).
    pub start_character: usize,
    /// End column of the original line in bytes.
    pub end_character: usize,
    /// Replacement text for the full line.
    pub new_text: String,
}

/// Plan full-line edits needed to update module imports after file rename.
///
/// Supported import forms:
/// - `use Module::Name;`
/// - `require Module::Name;`
/// - `use parent 'Module::Name';`
/// - `use parent "Module::Name";`
/// - `use parent qw(Module::Name Other);`
/// - `use base 'Module::Name';`
/// - `use base "Module::Name";`
/// - `use base qw(Module::Name Other);`
///
/// Legacy package separators (`Foo'Bar`) are also handled.
#[must_use]
pub fn plan_module_rename_edits(
    source: &str,
    old_module: &str,
    new_module: &str,
) -> Vec<ModuleLineEdit> {
    if source.is_empty()
        || old_module.is_empty()
        || new_module.is_empty()
        || old_module == new_module
    {
        return Vec::new();
    }

    let variants = module_variants(old_module, new_module);
    let mut edits = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let mut rewritten: Option<String> = None;

        for (old_variant, new_variant) in &variants {
            let current_line = rewritten.as_deref().unwrap_or(line);
            if !should_rewrite_line(current_line, old_variant) {
                continue;
            }

            let (candidate, changed) = replace_module_token(current_line, old_variant, new_variant);
            if changed {
                rewritten = Some(candidate);
            }
        }

        if let Some(new_text) = rewritten {
            edits.push(ModuleLineEdit {
                line: line_idx,
                start_character: 0,
                end_character: line.len(),
                new_text,
            });
        }
    }

    edits
}

/// Apply full-line `ModuleLineEdit` replacements to source text.
#[must_use]
pub fn apply_module_rename_edits(source: &str, edits: &[ModuleLineEdit]) -> String {
    if edits.is_empty() {
        return source.to_string();
    }

    let mut lines: Vec<String> = source.split('\n').map(ToString::to_string).collect();

    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|edit| edit.line);

    for edit in sorted {
        if let Some(line) = lines.get_mut(edit.line) {
            *line = edit.new_text;
        }
    }

    lines.join("\n")
}

fn module_variants(old_module: &str, new_module: &str) -> Vec<(String, String)> {
    let canonical = (old_module.to_string(), new_module.to_string());

    let legacy_old = old_module.replace("::", "'");
    let legacy_new = new_module.replace("::", "'");
    let legacy = (legacy_old, legacy_new);

    if legacy == canonical { vec![canonical] } else { vec![canonical, legacy] }
}

fn should_rewrite_line(line: &str, module_name: &str) -> bool {
    let trimmed = line.trim_start();

    if let Some(rest) = trimmed.strip_prefix("use ") {
        let first = first_token(rest);
        if first == module_name {
            return true;
        }

        if (first == "parent" || first == "base") && contains_module_token(line, module_name) {
            return true;
        }

        return false;
    }

    if let Some(rest) = trimmed.strip_prefix("require ") {
        let first = first_token(rest);
        return first == module_name;
    }

    false
}

fn first_token(input: &str) -> &str {
    input
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ';' | '(' | ')'))
        .next()
        .unwrap_or_default()
}

fn contains_module_token(line: &str, module_name: &str) -> bool {
    replace_module_token(line, module_name, module_name).1
}

fn replace_module_token(line: &str, from: &str, to: &str) -> (String, bool) {
    if from.is_empty() || line.is_empty() {
        return (line.to_string(), false);
    }

    let mut out = String::with_capacity(line.len());
    let mut search_start = 0usize;
    let mut replaced = false;

    while let Some(rel_pos) = line[search_start..].find(from) {
        let start = search_start + rel_pos;
        let end = start + from.len();

        if has_module_boundaries(line, start, end) {
            out.push_str(&line[search_start..start]);
            out.push_str(to);
            replaced = true;
        } else {
            out.push_str(&line[search_start..end]);
        }

        search_start = end;
    }

    if replaced {
        out.push_str(&line[search_start..]);
        (out, true)
    } else {
        (line.to_string(), false)
    }
}

fn has_module_boundaries(line: &str, start: usize, end: usize) -> bool {
    let left_ok = if start == 0 {
        true
    } else {
        !line[..start].chars().next_back().is_some_and(is_module_char)
    };

    let right_ok = if end >= line.len() {
        true
    } else {
        !line[end..].chars().next().is_some_and(is_module_char)
    };

    left_ok && right_ok
}

fn is_module_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'
}

#[cfg(test)]
mod tests {
    use super::{
        ModuleLineEdit, apply_module_rename_edits, module_variants, plan_module_rename_edits,
        replace_module_token,
    };

    #[test]
    fn plans_basic_use_and_require_edits() {
        let source = "use Foo::Bar;\nrequire Foo::Bar;\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Module");

        assert_eq!(
            edits,
            vec![
                ModuleLineEdit {
                    line: 0,
                    start_character: 0,
                    end_character: "use Foo::Bar;".len(),
                    new_text: "use New::Module;".to_string(),
                },
                ModuleLineEdit {
                    line: 1,
                    start_character: 0,
                    end_character: "require Foo::Bar;".len(),
                    new_text: "require New::Module;".to_string(),
                },
            ]
        );
    }

    #[test]
    fn plans_parent_and_base_edits() {
        let source = "use parent 'Foo::Bar';\nuse base \"Foo::Bar\";\nuse parent qw(Foo::Bar Other::Base);\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "Renamed::Base");
        let rewritten = apply_module_rename_edits(source, &edits);

        let expected = "use parent 'Renamed::Base';\nuse base \"Renamed::Base\";\nuse parent qw(Renamed::Base Other::Base);\n";
        assert_eq!(rewritten, expected);
    }

    #[test]
    fn handles_legacy_separator_variants() {
        let source = "use Foo'Bar;\nuse parent \"Foo'Bar\";\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Path");
        let rewritten = apply_module_rename_edits(source, &edits);

        assert_eq!(rewritten, "use New'Path;\nuse parent \"New'Path\";\n");
    }

    #[test]
    fn does_not_touch_partial_module_names() {
        let source = "use Foo::Barista;\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "Renamed::Module");
        assert!(edits.is_empty());
    }

    #[test]
    fn apply_edits_replaces_target_lines_only() {
        let source = "line1\nline2\nline3\n";
        let edits = vec![ModuleLineEdit {
            line: 1,
            start_character: 0,
            end_character: 5,
            new_text: "updated".to_string(),
        }];

        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "line1\nupdated\nline3\n");
    }

    #[test]
    fn module_variant_generation_deduplicates_when_not_needed() {
        let variants = module_variants("strict", "warnings");
        assert_eq!(variants.len(), 1);
    }

    #[test]
    fn token_replacement_requires_boundaries() {
        let (rewritten, changed) = replace_module_token("use Foo::Barista;", "Foo::Bar", "X::Y");
        assert_eq!(rewritten, "use Foo::Barista;");
        assert!(!changed);

        let (rewritten, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "X::Y");
        assert_eq!(rewritten, "use X::Y;");
        assert!(changed);
    }
}

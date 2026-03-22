//! Deterministic module-import rename edit planning.
//!
//! This crate isolates the small but critical responsibility of computing line
//! edits for Perl module file-rename workflows.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

use perl_module_import_match::line_references_module_import;
use perl_module_token::{module_variant_pairs, replace_module_token};

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

    let variants = module_variant_pairs(old_module, new_module);
    let mut edits = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let mut rewritten: Option<String> = None;

        for (old_variant, new_variant) in &variants {
            let current_line = rewritten.as_deref().unwrap_or(line);
            if !line_references_module_import(current_line, old_variant) {
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
#[cfg(test)]
mod tests {
    use super::{ModuleLineEdit, apply_module_rename_edits, plan_module_rename_edits};
    use perl_module_token::{module_variant_pairs, replace_module_token};

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
        let variants = module_variant_pairs("strict", "warnings");
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

    #[test]
    fn plans_use_parent_simple_name_no_colons() {
        // Regression for #2747: use parent with simple name (no ::)
        let source = "package Child;\nuse parent 'MyBase';\n1;\n";
        let edits = plan_module_rename_edits(source, "MyBase", "RenamedBase");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert!(
            rewritten.contains("use parent 'RenamedBase'"),
            "Expected rewrite of use parent 'MyBase' to 'RenamedBase', got: {:?}",
            rewritten
        );
    }
}

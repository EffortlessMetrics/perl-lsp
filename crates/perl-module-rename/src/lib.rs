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
/// - `package Module::Name;`
/// - `use parent 'Module::Name';`
/// - `use parent "Module::Name";`
/// - `use parent qw(Module::Name Other);`
/// - `use base 'Module::Name';`
/// - `use base "Module::Name";`
/// - `use base qw(Module::Name Other);`
///
/// Also rewrites:
/// - Qualified function calls: `Module::Name::func()` → `NewName::func()`
/// - Static method calls: `Module::Name->method()` → `NewName->method()`
/// - `@ISA` array assignments: `@ISA = ('Module::Name')` → `@ISA = ('NewName')`
/// - `our @ISA = qw(Module::Name)` → `our @ISA = qw(NewName)`
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
            // Re-read after each replacement so patterns compose correctly.
            // A single source line may contain multiple pattern types
            // (e.g. `@ISA = qw(Foo::Bar); Foo::Bar::init();`), and each
            // replacement must see the latest working text.

            // Check import forms (use/require/use parent/use base)
            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_module_import(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }

            // Check package declarations in moved module files.
            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_package_declaration(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }

            // Check @ISA assignments: module name appears standalone in the line
            // (e.g. `@ISA = ('Foo::Bar')`, `our @ISA = qw(Foo::Bar)`)
            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_isa_assignment(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }

            // Check qualified function calls: `Foo::Bar::func()` — the module
            // name appears as a namespace prefix followed by `::`.
            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_qualified_call(current_line, old_variant) {
                    let candidate =
                        replace_module_name_prefix(current_line, old_variant, new_variant);
                    if candidate != current_line {
                        rewritten = Some(candidate);
                    }
                }
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

/// Return `true` when `line` contains a `package` declaration that references
/// `module_name` as a standalone token.
#[must_use]
pub fn line_references_package_declaration(line: &str, module_name: &str) -> bool {
    if line.is_empty() || module_name.is_empty() {
        return false;
    }
    let trimmed = line.trim_start();
    if !trimmed.starts_with("package ") {
        return false;
    }

    perl_module_token::contains_module_token(trimmed, module_name)
}

/// Return `true` when `line` contains an `@ISA` assignment that references
/// `module_name` as a standalone token.
///
/// Detects patterns such as:
/// - `@ISA = ('Foo::Bar');`
/// - `our @ISA = qw(Foo::Bar Other::Base);`
/// - `push @ISA, 'Foo::Bar';`
#[must_use]
pub fn line_references_isa_assignment(line: &str, module_name: &str) -> bool {
    if line.is_empty() || module_name.is_empty() {
        return false;
    }
    // Must contain @ISA (case-sensitive — Perl's @ISA is always uppercase)
    if !line.contains("@ISA") {
        return false;
    }
    // The module name must appear as a standalone token on this line.
    // We reuse the boundary-aware check from perl_module_token.
    perl_module_token::contains_module_token(line, module_name)
}

/// Return `true` when `line` contains a qualified call that uses `module_name`
/// as a namespace prefix.
///
/// Detects patterns such as:
/// - `Foo::Bar::baz()` — direct qualified function call
/// - `Foo::Bar->method()` is handled by `replace_module_token` directly
///   (boundary check passes when `->` follows the module name)
///
/// A qualified call is identified when `module_name::` (with `::` suffix)
/// appears in the line, preceded by a non-module-token character (or the
/// start of the line), and followed by an identifier character.
#[must_use]
pub fn line_references_qualified_call(line: &str, module_name: &str) -> bool {
    if line.is_empty() || module_name.is_empty() {
        return false;
    }
    // Build the search needle: `module_name::`
    let needle = format!("{}::", module_name);
    let needle_bytes = needle.as_bytes();
    let line_bytes = line.as_bytes();
    let needle_len = needle_bytes.len();

    if line_bytes.len() < needle_len {
        return false;
    }

    let mut start = 0usize;
    while start + needle_len <= line_bytes.len() {
        let Some(rel) = line[start..].find(needle.as_str()) else {
            break;
        };
        let abs = start + rel;
        let after = abs + needle_len;

        // Verify: the character before the match is not a module-token char
        // (rejects matches embedded in a longer module path like `Extra::Foo::Bar::baz`)
        let before_ok = abs == 0 || {
            let ch = line_bytes[abs - 1] as char;
            !ch.is_alphanumeric() && ch != '_' && ch != ':'
        };

        // Verify: the character after `module_name::` is an identifier start
        // (rejects trailing `::` that is not followed by a function name)
        let after_ok = after < line_bytes.len() && {
            let ch = line_bytes[after] as char;
            ch.is_alphabetic() || ch == '_'
        };

        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }

    false
}

/// Replace `old_module::` namespace prefixes in `line` with `new_module::`.
///
/// This handles qualified function calls like `Foo::Bar::baz()` where
/// `Foo::Bar` is used as a namespace prefix (i.e., followed by `::`).
/// The replacement is boundary-aware: it only replaces occurrences where
/// the match is not embedded inside a longer namespace path.
#[must_use]
pub fn replace_module_name_prefix(line: &str, old_module: &str, new_module: &str) -> String {
    if old_module.is_empty() || new_module.is_empty() || line.is_empty() {
        return line.to_string();
    }

    let needle = format!("{}::", old_module);
    let replacement = format!("{}::", new_module);
    let needle_bytes = needle.as_bytes();
    let needle_len = needle_bytes.len();
    let line_bytes = line.as_bytes();

    if line_bytes.len() < needle_len {
        return line.to_string();
    }

    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize;

    while cursor + needle_len <= line_bytes.len() {
        let Some(rel) = line[cursor..].find(needle.as_str()) else {
            break;
        };
        let abs = cursor + rel;
        let after = abs + needle_len;

        let before_ok = abs == 0 || {
            let ch = line_bytes[abs - 1] as char;
            !ch.is_alphanumeric() && ch != '_' && ch != ':'
        };

        let after_ok = after < line_bytes.len() && {
            let ch = line_bytes[after] as char;
            ch.is_alphabetic() || ch == '_'
        };

        if before_ok && after_ok {
            out.push_str(&line[cursor..abs]);
            out.push_str(&replacement);
            cursor = after;
        } else {
            // Advance past this non-matching occurrence
            out.push_str(&line[cursor..abs + 1]);
            cursor = abs + 1;
        }
    }

    out.push_str(&line[cursor..]);
    out
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
    use super::{
        ModuleLineEdit, apply_module_rename_edits, line_references_isa_assignment,
        line_references_package_declaration, line_references_qualified_call,
        plan_module_rename_edits, replace_module_name_prefix,
    };
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
    fn rewrites_later_import_statement_on_same_line() {
        let source = "use Foo::Bar; use Foo::Baz;\n";
        let edits = plan_module_rename_edits(source, "Foo::Baz", "Renamed::Baz");
        let rewritten = apply_module_rename_edits(source, &edits);

        assert_eq!(rewritten, "use Foo::Bar; use Renamed::Baz;\n");
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

    #[test]
    fn package_declaration_detected() {
        assert!(line_references_package_declaration("package Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn package_declaration_rewritten() {
        let source = "package Foo::Bar;\n1;\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "Renamed::Pkg");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "package Renamed::Pkg;\n1;\n");
    }

    // ── @ISA assignment detection ─────────────────────────────────────────────

    #[test]
    fn isa_assignment_detected_single_quoted() {
        assert!(line_references_isa_assignment("@ISA = ('Foo::Bar');", "Foo::Bar"));
    }

    #[test]
    fn isa_assignment_detected_double_quoted() {
        assert!(line_references_isa_assignment("@ISA = (\"Foo::Bar\");", "Foo::Bar"));
    }

    #[test]
    fn isa_assignment_detected_qw() {
        assert!(line_references_isa_assignment("our @ISA = qw(Foo::Bar Other::Base);", "Foo::Bar"));
    }

    #[test]
    fn isa_push_detected() {
        assert!(line_references_isa_assignment("push @ISA, 'Foo::Bar';", "Foo::Bar"));
    }

    #[test]
    fn isa_assignment_rejects_absent_module() {
        assert!(!line_references_isa_assignment("@ISA = ('Other::Base');", "Foo::Bar"));
    }

    #[test]
    fn isa_assignment_rejects_no_isa() {
        assert!(!line_references_isa_assignment("use Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn isa_assignment_rejects_partial_module_name() {
        // "Bar" must NOT match inside "Foo::Bar" when searching for "Bar"
        // @ISA contains Foo::Bar, not Bar standalone
        assert!(!line_references_isa_assignment("@ISA = ('Foo::Bar');", "Bar"));
    }

    // ── @ISA edit planning ────────────────────────────────────────────────────

    #[test]
    fn plans_isa_assignment_single_quoted() {
        let source = "@ISA = ('Foo::Bar');\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Module");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "@ISA = ('New::Module');\n");
    }

    #[test]
    fn plans_isa_assignment_qw() {
        let source = "our @ISA = qw(Foo::Bar Other::Base);\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Module");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "our @ISA = qw(New::Module Other::Base);\n");
    }

    #[test]
    fn plans_isa_push() {
        let source = "push @ISA, 'Foo::Bar';\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Module");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "push @ISA, 'New::Module';\n");
    }

    // ── Qualified call detection ──────────────────────────────────────────────

    #[test]
    fn qualified_call_detected_direct_function() {
        assert!(line_references_qualified_call("Foo::Bar::baz();", "Foo::Bar"));
    }

    #[test]
    fn qualified_call_detected_in_expression() {
        assert!(line_references_qualified_call("my $x = Foo::Bar::create($arg);", "Foo::Bar"));
    }

    #[test]
    fn qualified_call_rejects_standalone_module() {
        // "use Foo::Bar;" — Foo::Bar is not a namespace prefix here
        assert!(!line_references_qualified_call("use Foo::Bar;", "Foo::Bar"));
    }

    #[test]
    fn qualified_call_rejects_deeper_prefix() {
        // "Extra::Foo::Bar::baz()" — Foo::Bar is not at the boundary
        assert!(!line_references_qualified_call("Extra::Foo::Bar::baz();", "Foo::Bar"));
    }

    #[test]
    fn qualified_call_rejects_empty_inputs() {
        assert!(!line_references_qualified_call("", "Foo::Bar"));
        assert!(!line_references_qualified_call("Foo::Bar::baz();", ""));
    }

    // ── Qualified call edit planning ──────────────────────────────────────────

    #[test]
    fn plans_qualified_function_call() {
        let source = "my $x = Foo::Bar::create($arg);\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Module");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "my $x = New::Module::create($arg);\n");
    }

    #[test]
    fn plans_qualified_call_preserves_function_name() {
        let source = "Foo::Bar::baz();\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "Renamed::Pkg");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "Renamed::Pkg::baz();\n");
    }

    #[test]
    fn plans_qualified_call_does_not_affect_deeper_prefix() {
        // Extra::Foo::Bar::baz() — "Foo::Bar" is not a top-level namespace here
        let source = "Extra::Foo::Bar::baz();\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Module");
        // Should produce no edits
        assert!(edits.is_empty(), "Expected no edits, got: {:?}", edits);
    }

    #[test]
    fn plans_multiple_qualified_calls_on_same_line() {
        let source = "my $a = Foo::Bar::new(); my $b = Foo::Bar::clone();\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "my $a = New::Mod::new(); my $b = New::Mod::clone();\n");
    }

    // ── replace_module_name_prefix unit tests ─────────────────────────────────

    #[test]
    fn prefix_replace_basic() {
        let result = replace_module_name_prefix("Foo::Bar::baz();", "Foo::Bar", "New::Mod");
        assert_eq!(result, "New::Mod::baz();");
    }

    #[test]
    fn prefix_replace_multiple_occurrences() {
        let result =
            replace_module_name_prefix("Foo::Bar::a() + Foo::Bar::b()", "Foo::Bar", "New::Mod");
        assert_eq!(result, "New::Mod::a() + New::Mod::b()");
    }

    #[test]
    fn prefix_replace_rejects_deeper_prefix() {
        // Extra::Foo::Bar::baz() — Foo::Bar is not at a boundary
        let result = replace_module_name_prefix("Extra::Foo::Bar::baz();", "Foo::Bar", "New::Mod");
        assert_eq!(result, "Extra::Foo::Bar::baz();");
    }

    #[test]
    fn prefix_replace_empty_inputs_are_noop() {
        assert_eq!(replace_module_name_prefix("", "Foo::Bar", "New::Mod"), "");
        assert_eq!(
            replace_module_name_prefix("Foo::Bar::baz();", "", "New::Mod"),
            "Foo::Bar::baz();"
        );
    }

    // ── Combined: mixed import + @ISA + qualified call ────────────────────────

    #[test]
    fn plans_full_file_with_all_patterns() {
        let source = "use Foo::Bar;\nour @ISA = qw(Foo::Bar);\nmy $x = Foo::Bar::create();\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Module");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(
            rewritten,
            "use New::Module;\nour @ISA = qw(New::Module);\nmy $x = New::Module::create();\n"
        );
    }

    #[test]
    fn plans_isa_and_qualified_call_on_same_line() {
        // When @ISA assignment and a qualified function call appear on the same
        // source line, both must be rewritten. The ISA branch must not short-
        // circuit via `continue` and skip the qualified-call branch.
        let source = "@ISA = qw(Foo::Bar); Foo::Bar::init();\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "@ISA = qw(New::Mod); New::Mod::init();\n");
    }

    #[test]
    fn plans_import_and_qualified_call_on_same_line() {
        // Same principle: a `use` import and a qualified call on the same line.
        // The import branch fires first via `continue`; the qualified call must
        // also be rewritten in a subsequent pass.
        let source = "use Foo::Bar; Foo::Bar::init();\n";
        let edits = plan_module_rename_edits(source, "Foo::Bar", "New::Mod");
        let rewritten = apply_module_rename_edits(source, &edits);
        assert_eq!(rewritten, "use New::Mod; New::Mod::init();\n");
    }
}

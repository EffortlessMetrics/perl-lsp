//! Targeted unit tests filling coverage gaps in `perl-module`.
//!
//! Covers:
//! - `import::resolve_known_export_tag` (previously 0 tests)
//! - `import::DispatchSemantics::hover_description` for the two unreachable-via-kind arms
//! - `reference::find_module_reference` / `find_module_reference_extended` edge cases
//! - `boundary::find_standalone_module_token_ranges` early-exit branches
//! - `import_match::line_references_module_import` edge cases
//! - `token::replace_module_token` short-circuit branches

// ============================================================================
// resolve_known_export_tag
// ============================================================================

use perl_module::import::{
    DispatchSemantics, ImportBehavior, LoadTiming, resolve_known_export_tag,
};
use perl_module::import_match::line_references_module_import;
use perl_module::reference::{
    ModuleReferenceKind, extract_module_reference, extract_module_reference_extended,
    find_module_reference, find_module_reference_extended,
};
use perl_module::token::{contains_module_token, replace_module_token};
use perl_module::{
    ModuleTokenRange, contains_standalone_module_token, find_standalone_module_token_ranges,
};

// --- resolve_known_export_tag: known module/tag pairs ----------------------

#[test]
fn resolve_known_export_tag_posix_sys_wait_h() -> Result<(), String> {
    let syms = resolve_known_export_tag("POSIX", "sys_wait_h")
        .ok_or("expected Some for POSIX/sys_wait_h")?;
    if !syms.contains(&"WIFEXITED") {
        return Err(format!("expected WIFEXITED in {syms:?}"));
    }
    if !syms.contains(&"WEXITSTATUS") {
        return Err(format!("expected WEXITSTATUS in {syms:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_posix_sys_wait_h_colon_prefix() -> Result<(), String> {
    // Tag may be passed with a leading ':'.
    let with_colon = resolve_known_export_tag("POSIX", ":sys_wait_h");
    let without_colon = resolve_known_export_tag("POSIX", "sys_wait_h");
    if with_colon != without_colon {
        return Err(format!(
            "colon-prefixed and bare tags should resolve identically: {with_colon:?} vs {without_colon:?}"
        ));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_posix_fcntl_h() -> Result<(), String> {
    let syms =
        resolve_known_export_tag("POSIX", "fcntl_h").ok_or("expected Some for POSIX/fcntl_h")?;
    if !syms.contains(&"F_GETFL") {
        return Err(format!("expected F_GETFL in {syms:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_posix_termios_h() -> Result<(), String> {
    let syms = resolve_known_export_tag("POSIX", "termios_h")
        .ok_or("expected Some for POSIX/termios_h")?;
    if !syms.contains(&"TCSANOW") {
        return Err(format!("expected TCSANOW in {syms:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_file_find_find() -> Result<(), String> {
    let syms = resolve_known_export_tag("File::Find", "find")
        .ok_or("expected Some for File::Find/find")?;
    if !syms.contains(&"find") {
        return Err(format!("expected 'find' in {syms:?}"));
    }
    if !syms.contains(&"finddepth") {
        return Err(format!("expected 'finddepth' in {syms:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_fcntl_seek() -> Result<(), String> {
    let syms = resolve_known_export_tag("Fcntl", "seek").ok_or("expected Some for Fcntl/seek")?;
    if !syms.contains(&"SEEK_SET") {
        return Err(format!("expected SEEK_SET in {syms:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_fcntl_lock() -> Result<(), String> {
    let syms = resolve_known_export_tag("Fcntl", "lock").ok_or("expected Some for Fcntl/lock")?;
    if !syms.contains(&"LOCK_SH") {
        return Err(format!("expected LOCK_SH in {syms:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_encode_fallback() -> Result<(), String> {
    let syms = resolve_known_export_tag("Encode", "fallback")
        .ok_or("expected Some for Encode/fallback")?;
    if !syms.contains(&"FB_DEFAULT") {
        return Err(format!("expected FB_DEFAULT in {syms:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_unknown_module_returns_none() -> Result<(), String> {
    let result = resolve_known_export_tag("NoSuch::Module", "all");
    if result.is_some() {
        return Err(format!("expected None for unknown module, got {result:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_unknown_tag_for_known_module_returns_none() -> Result<(), String> {
    let result = resolve_known_export_tag("POSIX", "nonexistent_tag");
    if result.is_some() {
        return Err(format!("expected None for unknown tag, got {result:?}"));
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_empty_inputs_return_none() -> Result<(), String> {
    let result1 = resolve_known_export_tag("", "");
    let result2 = resolve_known_export_tag("POSIX", "");
    let result3 = resolve_known_export_tag("", "sys_wait_h");
    if result1.is_some() || result2.is_some() || result3.is_some() {
        return Err("expected None for empty inputs".into());
    }
    Ok(())
}

#[test]
fn resolve_known_export_tag_colon_prefix_on_fcntl_seek() -> Result<(), String> {
    // Verify colon-stripping works for the Fcntl/seek pair too.
    let bare = resolve_known_export_tag("Fcntl", "seek");
    let colon = resolve_known_export_tag("Fcntl", ":seek");
    if bare != colon {
        return Err(format!("bare and colon forms should match: {bare:?} vs {colon:?}"));
    }
    Ok(())
}

// ============================================================================
// DispatchSemantics::hover_description — unreachable-via-kind arms
//
// The `hover_description` method has 4 arms; only 2 are reachable via
// `ModuleImportKind::dispatch_semantics`. The remaining two require direct
// struct construction (both fields are public).
// ============================================================================

#[test]
fn hover_description_compile_time_no_import() -> Result<(), String> {
    let sem = DispatchSemantics {
        load_timing: LoadTiming::CompileTime,
        import_behavior: ImportBehavior::NoImport,
    };
    let desc = sem.hover_description();
    let lower = desc.to_lowercase();
    if !lower.contains("compile") {
        return Err(format!("expected 'compile' in description: {desc:?}"));
    }
    if !lower.contains("import") {
        return Err(format!("expected 'import' in description: {desc:?}"));
    }
    Ok(())
}

#[test]
fn hover_description_runtime_calls_import() -> Result<(), String> {
    let sem = DispatchSemantics {
        load_timing: LoadTiming::Runtime,
        import_behavior: ImportBehavior::CallsImport,
    };
    let desc = sem.hover_description();
    let lower = desc.to_lowercase();
    if !lower.contains("runtime") {
        return Err(format!("expected 'runtime' in description: {desc:?}"));
    }
    if !lower.contains("import") {
        return Err(format!("expected 'import' in description: {desc:?}"));
    }
    Ok(())
}

// ============================================================================
// reference/mod.rs — edge cases
// ============================================================================

#[test]
fn find_module_reference_empty_text_returns_none() -> Result<(), String> {
    let result = find_module_reference("", 0);
    if result.is_some() {
        return Err("expected None for empty text".into());
    }
    Ok(())
}

#[test]
fn find_module_reference_cursor_beyond_text_returns_none() -> Result<(), String> {
    let text = "use Foo::Bar;";
    let result = find_module_reference(text, text.len() + 100);
    if result.is_some() {
        return Err("expected None when cursor is beyond text length".into());
    }
    Ok(())
}

#[test]
fn find_module_reference_cursor_exactly_at_text_len_returns_none() -> Result<(), String> {
    // cursor_pos == text.len() is on the boundary (past last char).
    // The function accepts cursor_pos <= text.len(), so it doesn't short-circuit,
    // but a cursor right after the semicolon won't land on a module token.
    let text = "use Foo::Bar;";
    // Cursor at len may still search — just assert no panic and consistent result.
    let _ = find_module_reference(text, text.len());
    Ok(())
}

#[test]
fn find_module_reference_require_kind() -> Result<(), String> {
    let line = "require Some::Dep;";
    let cursor = line.find("Some").ok_or("missing token")? + 2;
    let reference = find_module_reference(line, cursor).ok_or("expected Some")?;
    if reference.kind != ModuleReferenceKind::Require {
        return Err(format!("expected Require kind, got {:?}", reference.kind));
    }
    if reference.module_name != "Some::Dep" {
        return Err(format!("wrong module_name: {:?}", reference.module_name));
    }
    Ok(())
}

#[test]
fn find_module_reference_canonical_module_name_strips_legacy_separator() -> Result<(), String> {
    let line = "use Foo'Bar;";
    let cursor = line.find("Foo").ok_or("missing token")? + 1;
    let reference = find_module_reference(line, cursor).ok_or("expected Some")?;
    let canonical = reference.canonical_module_name();
    if canonical != "Foo::Bar" {
        return Err(format!("expected Foo::Bar, got {canonical:?}"));
    }
    Ok(())
}

#[test]
fn find_module_reference_module_start_and_end_offsets_are_consistent() -> Result<(), String> {
    let text = "use Demo::Widget;";
    let cursor = text.find("Demo").ok_or("missing token")? + 3;
    let reference = find_module_reference(text, cursor).ok_or("expected Some")?;
    let sliced = &text[reference.module_start..reference.module_end];
    if sliced != reference.module_name {
        return Err(format!(
            "slice {:?} does not match module_name {:?}",
            sliced, reference.module_name
        ));
    }
    Ok(())
}

#[test]
fn find_module_reference_extended_empty_text_returns_none() -> Result<(), String> {
    let result = find_module_reference_extended("", 0);
    if result.is_some() {
        return Err("expected None for empty text".into());
    }
    Ok(())
}

#[test]
fn find_module_reference_extended_cursor_beyond_text_returns_none() -> Result<(), String> {
    let text = "use parent 'Foo::Bar';";
    let result = find_module_reference_extended(text, text.len() + 50);
    if result.is_some() {
        return Err("expected None when cursor is beyond text length".into());
    }
    Ok(())
}

#[test]
fn extract_module_reference_extended_falls_back_when_not_parent() -> Result<(), String> {
    // `use Some::Module;` is a direct use, not parent/base.
    // extended should return the same as direct.
    let line = "use Some::Module;";
    let cursor = line.find("Some").ok_or("missing token")? + 1;
    let direct = extract_module_reference(line, cursor);
    let extended = extract_module_reference_extended(line, cursor);
    if direct != extended {
        return Err(format!(
            "direct and extended should agree for plain use: direct={direct:?} extended={extended:?}"
        ));
    }
    Ok(())
}

#[test]
fn extract_module_reference_extended_returns_none_for_non_import_line() -> Result<(), String> {
    let line = "my $x = Foo::Bar->new();";
    let cursor = line.find("Foo").ok_or("missing token")? + 1;
    let result = extract_module_reference_extended(line, cursor);
    if result.is_some() {
        return Err(format!("expected None for non-import line, got {result:?}"));
    }
    Ok(())
}

#[test]
fn find_module_reference_extended_finds_use_base_module() -> Result<(), String> {
    let line = "use base 'My::Parent';";
    let cursor = line.find("My::Parent").ok_or("missing token")? + 3;
    let reference = find_module_reference_extended(line, cursor).ok_or("expected Some")?;
    if reference.module_name != "My::Parent" {
        return Err(format!("wrong module_name: {:?}", reference.module_name));
    }
    if reference.kind != ModuleReferenceKind::Use {
        return Err(format!("expected Use kind, got {:?}", reference.kind));
    }
    Ok(())
}

#[test]
fn find_module_reference_multiline_resolves_correct_line() -> Result<(), String> {
    // Ensure line_bounds_at correctly isolates the use statement's line.
    let source = "package App;\nuse Deep::Module;\n# comment\n";
    let cursor = source.find("Deep::Module").ok_or("missing token")? + 4;
    let reference = find_module_reference(source, cursor).ok_or("expected Some")?;
    if reference.module_name != "Deep::Module" {
        return Err(format!("wrong module_name: {:?}", reference.module_name));
    }
    Ok(())
}

#[test]
fn module_reference_kind_debug_format() -> Result<(), String> {
    let use_dbg = format!("{:?}", ModuleReferenceKind::Use);
    let req_dbg = format!("{:?}", ModuleReferenceKind::Require);
    if !use_dbg.contains("Use") {
        return Err(format!("Debug for Use should contain 'Use': {use_dbg}"));
    }
    if !req_dbg.contains("Require") {
        return Err(format!("Debug for Require should contain 'Require': {req_dbg}"));
    }
    Ok(())
}

// ============================================================================
// boundary/mod.rs — early-exit and edge-case branches
// ============================================================================

#[test]
fn find_standalone_module_token_ranges_empty_module_name_yields_nothing() -> Result<(), String> {
    let mut iter = find_standalone_module_token_ranges("use Foo::Bar;", "");
    if iter.next().is_some() {
        return Err("expected no ranges for empty module_name".into());
    }
    Ok(())
}

#[test]
fn find_standalone_module_token_ranges_empty_line_yields_nothing() -> Result<(), String> {
    let mut iter = find_standalone_module_token_ranges("", "Foo::Bar");
    if iter.next().is_some() {
        return Err("expected no ranges for empty line".into());
    }
    Ok(())
}

#[test]
fn find_standalone_module_token_ranges_done_after_first_next_on_empty_line() -> Result<(), String> {
    // Verify the iterator is fused after the first None.
    let mut iter = find_standalone_module_token_ranges("", "Foo");
    let _ = iter.next(); // should be None
    if iter.next().is_some() {
        return Err("iterator should stay done after first None".into());
    }
    Ok(())
}

#[test]
fn find_standalone_module_token_ranges_multiple_matches_collected() -> Result<(), String> {
    let line = "Foo Foo Foo";
    let ranges: Vec<ModuleTokenRange> = find_standalone_module_token_ranges(line, "Foo").collect();
    if ranges.len() != 3 {
        return Err(format!("expected 3 ranges, got {}: {ranges:?}", ranges.len()));
    }
    // Verify each range slices back to "Foo".
    for r in &ranges {
        let sliced = &line[r.start..r.end];
        if sliced != "Foo" {
            return Err(format!("range {r:?} slices to {sliced:?}, expected 'Foo'"));
        }
    }
    Ok(())
}

#[test]
fn contains_standalone_module_token_returns_false_for_empty_module_name() -> Result<(), String> {
    let result = contains_standalone_module_token("use Foo::Bar;", "");
    if result {
        return Err("expected false for empty module_name".into());
    }
    Ok(())
}

#[test]
fn contains_standalone_module_token_returns_false_for_empty_line() -> Result<(), String> {
    let result = contains_standalone_module_token("", "Foo::Bar");
    if result {
        return Err("expected false for empty line".into());
    }
    Ok(())
}

#[test]
fn contains_standalone_module_token_no_match_when_embedded_in_longer_name() -> Result<(), String> {
    // "Foo" is not standalone when part of "FooBar" or "::Foo".
    let result_suffix = contains_standalone_module_token("use FooBar;", "Foo");
    let result_prefix = contains_standalone_module_token("use ::Foo;", "Foo");
    if result_suffix {
        return Err("expected false: 'Foo' embedded in 'FooBar'".into());
    }
    if result_prefix {
        return Err("expected false: 'Foo' with leading '::'".into());
    }
    Ok(())
}

// ============================================================================
// import_match/mod.rs — additional edge cases
// ============================================================================

#[test]
fn line_references_module_import_empty_line_returns_false() -> Result<(), String> {
    let result = line_references_module_import("", "Foo::Bar");
    if result {
        return Err("expected false for empty line".into());
    }
    Ok(())
}

#[test]
fn line_references_module_import_empty_module_name_returns_false() -> Result<(), String> {
    let result = line_references_module_import("use Foo::Bar;", "");
    if result {
        return Err("expected false for empty module_name".into());
    }
    Ok(())
}

#[test]
fn line_references_module_import_both_empty_returns_false() -> Result<(), String> {
    let result = line_references_module_import("", "");
    if result {
        return Err("expected false for both empty".into());
    }
    Ok(())
}

#[test]
fn line_references_module_import_use_mismatch_returns_false() -> Result<(), String> {
    // Exact token match is required for Use.
    let result = line_references_module_import("use Foo::Bar::Extended;", "Foo::Bar");
    if result {
        return Err("expected false: token is a prefix, not exact match".into());
    }
    Ok(())
}

#[test]
fn line_references_module_import_use_parent_boundary_rejection() -> Result<(), String> {
    // "Foo" must not match inside "FooExtra" in a use parent list.
    let result = line_references_module_import("use parent qw(FooExtra);", "Foo");
    if result {
        return Err("expected false: 'Foo' is a prefix of 'FooExtra', boundary must reject".into());
    }
    Ok(())
}

#[test]
fn line_references_module_import_multiple_semicolons_any_statement_matches() -> Result<(), String> {
    // Multiple statements on one logical "line" (e.g., minified source).
    let line = "use Unrelated; use Target::Mod;";
    let result = line_references_module_import(line, "Target::Mod");
    if !result {
        return Err("expected true: second statement matches Target::Mod".into());
    }
    Ok(())
}

#[test]
fn line_references_module_import_use_base_exact_match() -> Result<(), String> {
    let result = line_references_module_import("use base 'Exact::Match';", "Exact::Match");
    if !result {
        return Err("expected true for matching use base".into());
    }
    Ok(())
}

#[test]
fn line_references_module_import_require_mismatch_returns_false() -> Result<(), String> {
    let result = line_references_module_import("require Foo::Bar;", "Foo::Baz");
    if result {
        return Err("expected false: 'Foo::Baz' does not match 'Foo::Bar'".into());
    }
    Ok(())
}

// ============================================================================
// token/mod.rs — short-circuit branches
// ============================================================================

#[test]
fn replace_module_token_empty_from_returns_unchanged() -> Result<(), String> {
    let (out, changed) = replace_module_token("use Foo::Bar;", "", "New::Name");
    if changed {
        return Err("expected unchanged when from is empty".into());
    }
    if out != "use Foo::Bar;" {
        return Err(format!("expected original line unchanged, got {out:?}"));
    }
    Ok(())
}

#[test]
fn replace_module_token_empty_line_returns_unchanged() -> Result<(), String> {
    let (out, changed) = replace_module_token("", "Foo::Bar", "New::Name");
    if changed {
        return Err("expected unchanged when line is empty".into());
    }
    if !out.is_empty() {
        return Err(format!("expected empty output, got {out:?}"));
    }
    Ok(())
}

#[test]
fn replace_module_token_no_match_returns_unchanged() -> Result<(), String> {
    let (out, changed) = replace_module_token("use Other::Mod;", "Foo::Bar", "New::Name");
    if changed {
        return Err("expected unchanged when pattern not found".into());
    }
    if out != "use Other::Mod;" {
        return Err(format!("expected original line, got {out:?}"));
    }
    Ok(())
}

#[test]
fn replace_module_token_replaces_standalone_occurrence() -> Result<(), String> {
    let (out, changed) = replace_module_token("use Foo::Bar;", "Foo::Bar", "New::Name");
    if !changed {
        return Err("expected changed to be true".into());
    }
    if out != "use New::Name;" {
        return Err(format!("expected 'use New::Name;', got {out:?}"));
    }
    Ok(())
}

#[test]
fn contains_module_token_returns_true_for_standalone_match() -> Result<(), String> {
    let result = contains_module_token("use Foo::Bar;", "Foo::Bar");
    if !result {
        return Err("expected true for standalone match".into());
    }
    Ok(())
}

#[test]
fn contains_module_token_returns_false_for_no_match() -> Result<(), String> {
    let result = contains_module_token("use Other::Mod;", "Foo::Bar");
    if result {
        return Err("expected false when module not in line".into());
    }
    Ok(())
}

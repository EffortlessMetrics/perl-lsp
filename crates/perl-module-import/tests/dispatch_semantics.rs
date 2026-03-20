//! Tests for dispatch semantics and lazy loading awareness.
//!
//! Covers the distinction between compile-time `use` and runtime `require`,
//! including import call behaviour, load timing, and file-path `require` syntax.

use perl_module_import::{
    ImportBehavior, LoadTiming, ModuleImportKind, RequireForm, parse_module_import_head,
};

// ===========================================================================
// LoadTiming — use is compile-time, require is runtime
// ===========================================================================

#[test]
fn test_use_has_compile_time_load_timing() -> Result<(), String> {
    let timing = ModuleImportKind::Use.dispatch_semantics().load_timing;
    if timing != LoadTiming::CompileTime {
        return Err(format!("expected CompileTime, got {timing:?}"));
    }
    Ok(())
}

#[test]
fn test_require_has_runtime_load_timing() -> Result<(), String> {
    let timing = ModuleImportKind::Require.dispatch_semantics().load_timing;
    if timing != LoadTiming::Runtime {
        return Err(format!("expected Runtime, got {timing:?}"));
    }
    Ok(())
}

#[test]
fn test_use_parent_has_compile_time_load_timing() -> Result<(), String> {
    let timing = ModuleImportKind::UseParent.dispatch_semantics().load_timing;
    if timing != LoadTiming::CompileTime {
        return Err(format!("expected CompileTime, got {timing:?}"));
    }
    Ok(())
}

#[test]
fn test_use_base_has_compile_time_load_timing() -> Result<(), String> {
    let timing = ModuleImportKind::UseBase.dispatch_semantics().load_timing;
    if timing != LoadTiming::CompileTime {
        return Err(format!("expected CompileTime, got {timing:?}"));
    }
    Ok(())
}

// ===========================================================================
// ImportBehavior — use calls import(), require does not
// ===========================================================================

#[test]
fn test_use_calls_import() -> Result<(), String> {
    let behavior = ModuleImportKind::Use.dispatch_semantics().import_behavior;
    if behavior != ImportBehavior::CallsImport {
        return Err(format!("expected CallsImport, got {behavior:?}"));
    }
    Ok(())
}

#[test]
fn test_require_does_not_call_import() -> Result<(), String> {
    let behavior = ModuleImportKind::Require.dispatch_semantics().import_behavior;
    if behavior != ImportBehavior::NoImport {
        return Err(format!("expected NoImport, got {behavior:?}"));
    }
    Ok(())
}

#[test]
fn test_use_parent_calls_import() -> Result<(), String> {
    let behavior = ModuleImportKind::UseParent.dispatch_semantics().import_behavior;
    if behavior != ImportBehavior::CallsImport {
        return Err(format!("expected CallsImport, got {behavior:?}"));
    }
    Ok(())
}

#[test]
fn test_use_base_calls_import() -> Result<(), String> {
    let behavior = ModuleImportKind::UseBase.dispatch_semantics().import_behavior;
    if behavior != ImportBehavior::CallsImport {
        return Err(format!("expected CallsImport, got {behavior:?}"));
    }
    Ok(())
}

// ===========================================================================
// DispatchSemantics struct — derived traits
// ===========================================================================

#[test]
fn test_dispatch_semantics_debug_contains_load_timing() -> Result<(), String> {
    let sem = ModuleImportKind::Use.dispatch_semantics();
    let dbg = format!("{sem:?}");
    if !dbg.contains("CompileTime") {
        return Err(format!("Debug missing CompileTime: {dbg}"));
    }
    Ok(())
}

#[test]
fn test_dispatch_semantics_equality() -> Result<(), String> {
    let a = ModuleImportKind::Use.dispatch_semantics();
    let b = ModuleImportKind::Use.dispatch_semantics();
    if a != b {
        return Err("expected equal DispatchSemantics for same kind".into());
    }
    Ok(())
}

#[test]
fn test_dispatch_semantics_inequality_across_kinds() -> Result<(), String> {
    let use_sem = ModuleImportKind::Use.dispatch_semantics();
    let req_sem = ModuleImportKind::Require.dispatch_semantics();
    if use_sem == req_sem {
        return Err("expected Use and Require semantics to differ".into());
    }
    Ok(())
}

// ===========================================================================
// require "File.pm" — file-path syntax
// ===========================================================================

#[test]
fn test_require_file_path_double_quote_parses() -> Result<(), String> {
    let head = parse_module_import_head(r#"require "Module/File.pm";"#)
        .ok_or("expected Some for quoted require")?;
    if head.kind != ModuleImportKind::Require {
        return Err(format!("expected Require kind, got {:?}", head.kind));
    }
    Ok(())
}

#[test]
fn test_require_file_path_single_quote_parses() -> Result<(), String> {
    let head = parse_module_import_head("require 'Module/File.pm';")
        .ok_or("expected Some for single-quoted require")?;
    if head.kind != ModuleImportKind::Require {
        return Err(format!("expected Require kind, got {:?}", head.kind));
    }
    Ok(())
}

#[test]
fn test_require_file_path_is_file_form() -> Result<(), String> {
    let head = parse_module_import_head(r#"require "Module/File.pm";"#)
        .ok_or("expected Some for quoted require")?;
    if head.require_form() != Some(RequireForm::FilePath) {
        return Err(format!("expected FilePath form, got {:?}", head.require_form()));
    }
    Ok(())
}

#[test]
fn test_require_bare_module_is_module_name_form() -> Result<(), String> {
    let head = parse_module_import_head("require Foo::Bar;")
        .ok_or("expected Some for bare module require")?;
    if head.require_form() != Some(RequireForm::ModuleName) {
        return Err(format!("expected ModuleName form, got {:?}", head.require_form()));
    }
    Ok(())
}

#[test]
fn test_use_has_no_require_form() -> Result<(), String> {
    let head =
        parse_module_import_head("use Foo::Bar;").ok_or("expected Some for use statement")?;
    if head.require_form().is_some() {
        return Err(format!("expected None require_form for use, got {:?}", head.require_form()));
    }
    Ok(())
}

#[test]
fn test_require_file_path_token_strips_quotes() -> Result<(), String> {
    let head = parse_module_import_head(r#"require "Module/File.pm";"#)
        .ok_or("expected Some for quoted require")?;
    // The token should be the content without the surrounding quotes
    if head.token == r#""Module/File.pm""# {
        return Err(format!(
            "token should strip quotes, got {:?} — quotes still present",
            head.token
        ));
    }
    if head.token != "Module/File.pm" {
        return Err(format!("expected 'Module/File.pm' token, got {:?}", head.token));
    }
    Ok(())
}

#[test]
fn test_require_file_path_single_quote_token_strips_quotes() -> Result<(), String> {
    let head = parse_module_import_head("require 'Module/File.pm';")
        .ok_or("expected Some for single-quoted require")?;
    if head.token != "Module/File.pm" {
        return Err(format!("expected 'Module/File.pm' token, got {:?}", head.token));
    }
    Ok(())
}

// ===========================================================================
// hover_description — human-readable semantic label
// ===========================================================================

#[test]
fn test_use_hover_description_mentions_compile_time() -> Result<(), String> {
    let desc = ModuleImportKind::Use.dispatch_semantics().hover_description();
    if !desc.to_lowercase().contains("compile") {
        return Err(format!("hover description for Use should mention compile-time: {desc}"));
    }
    Ok(())
}

#[test]
fn test_require_hover_description_mentions_runtime() -> Result<(), String> {
    let desc = ModuleImportKind::Require.dispatch_semantics().hover_description();
    if !desc.to_lowercase().contains("runtime") {
        return Err(format!("hover description for Require should mention runtime: {desc}"));
    }
    Ok(())
}

#[test]
fn test_require_hover_description_mentions_no_import() -> Result<(), String> {
    let desc = ModuleImportKind::Require.dispatch_semantics().hover_description();
    let lower = desc.to_lowercase();
    if !lower.contains("import") {
        return Err(format!("hover description for Require should mention import: {desc}"));
    }
    Ok(())
}

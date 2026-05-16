//! Tests for `extract_require_import_symbols` — text-level literal
//! `require Module; Module->import(...)` extractor.

use perl_module::{RequireImportEntry, extract_require_import_symbols};

// ── Basic literal patterns ───────────────────────────────────────────────────

#[test]
fn qw_list_produces_entries() -> Result<(), String> {
    let source = "require Foo::Bar;\nFoo::Bar->import(qw(alpha beta));\n";
    let entries = extract_require_import_symbols(source);

    if entries.len() != 2 {
        return Err(format!("expected 2 entries, got {}: {entries:?}", entries.len()));
    }
    for e in &entries {
        if e.module != "Foo::Bar" {
            return Err(format!("unexpected module {:?}", e.module));
        }
    }
    let names: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
    if !names.contains(&"alpha") {
        return Err(format!("missing 'alpha' in {names:?}"));
    }
    if !names.contains(&"beta") {
        return Err(format!("missing 'beta' in {names:?}"));
    }
    Ok(())
}

#[test]
fn single_quoted_args_produce_entries() -> Result<(), String> {
    let source = "require Foo;\nFoo->import('foo', 'bar');\n";
    let entries = extract_require_import_symbols(source);

    if entries.len() != 2 {
        return Err(format!("expected 2 entries, got {}: {entries:?}", entries.len()));
    }
    let names: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
    if !names.contains(&"foo") {
        return Err(format!("missing 'foo' in {names:?}"));
    }
    if !names.contains(&"bar") {
        return Err(format!("missing 'bar' in {names:?}"));
    }
    Ok(())
}

#[test]
fn double_quoted_args_produce_entries() -> Result<(), String> {
    let source = "require Foo;\nFoo->import(\"foo\", \"bar\");\n";
    let entries = extract_require_import_symbols(source);

    if entries.len() != 2 {
        return Err(format!("expected 2 entries, got {}: {entries:?}", entries.len()));
    }
    let names: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
    if !names.contains(&"foo") {
        return Err(format!("missing 'foo' in {names:?}"));
    }
    if !names.contains(&"bar") {
        return Err(format!("missing 'bar' in {names:?}"));
    }
    Ok(())
}

#[test]
fn nested_module_name_is_preserved() -> Result<(), String> {
    let source = "require Module::Nested;\nModule::Nested->import('foo');\n";
    let entries = extract_require_import_symbols(source);

    if entries.len() != 1 {
        return Err(format!("expected 1 entry, got {}: {entries:?}", entries.len()));
    }
    let e = &entries[0];
    if e.module != "Module::Nested" {
        return Err(format!("wrong module {:?}", e.module));
    }
    if e.symbol != "foo" {
        return Err(format!("wrong symbol {:?}", e.symbol));
    }
    Ok(())
}

// ── Non-goals: must not produce entries ─────────────────────────────────────

#[test]
fn dynamic_module_name_is_rejected() -> Result<(), String> {
    // `require $var` — variable receiver, not a literal module name.
    let source = "require $module;\n$module->import('foo');\n";
    let entries = extract_require_import_symbols(source);
    if !entries.is_empty() {
        return Err(format!("expected no entries for dynamic require, got {entries:?}"));
    }
    Ok(())
}

#[test]
fn dynamic_array_arg_is_rejected() -> Result<(), String> {
    // `Module->import(@list)` — dynamic argument list.
    let source = "require Foo;\nFoo->import(@list);\n";
    let entries = extract_require_import_symbols(source);
    if !entries.is_empty() {
        return Err(format!("expected no entries for dynamic arg list, got {entries:?}"));
    }
    Ok(())
}

#[test]
fn dynamic_scalar_arg_is_rejected() -> Result<(), String> {
    // `Module->import($sym)` — dynamic scalar argument.
    let source = "require Foo;\nFoo->import($sym);\n";
    let entries = extract_require_import_symbols(source);
    if !entries.is_empty() {
        return Err(format!("expected no entries for dynamic scalar arg, got {entries:?}"));
    }
    Ok(())
}

#[test]
fn variable_receiver_is_rejected() -> Result<(), String> {
    // `$class->import('x')` — variable receiver, not literal module.
    let source = "require Foo;\n$class->import('x');\n";
    let entries = extract_require_import_symbols(source);
    // The require is literal but the import call uses $class, not Foo.
    // No matching pair is produced.
    if !entries.is_empty() {
        return Err(format!("expected no entries for variable-receiver import, got {entries:?}"));
    }
    Ok(())
}

#[test]
fn quoted_file_path_require_is_rejected() -> Result<(), String> {
    // `require "path/to/file.pm"` — not a bareword module name.
    let source = "require \"Foo/Bar.pm\";\nFoo::Bar->import('baz');\n";
    let entries = extract_require_import_symbols(source);
    if !entries.is_empty() {
        return Err(format!("expected no entries for quoted file-path require, got {entries:?}"));
    }
    Ok(())
}

// ── Byte offset fields are populated ────────────────────────────────────────

#[test]
fn byte_offsets_are_populated() -> Result<(), String> {
    let source = "require Foo;\nFoo->import(qw(bar));\n";
    let entries = extract_require_import_symbols(source);

    let e = entries.first().ok_or("expected at least one entry")?;
    // `require Foo;` starts at byte 0.
    if e.require_byte_offset != 0 {
        return Err(format!("expected require_byte_offset=0, got {}", e.require_byte_offset));
    }
    // `Foo->import(...)` starts after `require Foo;\n` = 13 bytes.
    if e.import_byte_offset != 13 {
        return Err(format!("expected import_byte_offset=13, got {}", e.import_byte_offset));
    }
    Ok(())
}

// ── Multiple pairs in one file ───────────────────────────────────────────────

#[test]
fn multiple_require_import_pairs_all_extracted() -> Result<(), String> {
    let source = "\
require A;
A->import('x');
require B;
B->import(qw(y z));
";
    let entries = extract_require_import_symbols(source);

    let a_entries: Vec<&RequireImportEntry> = entries.iter().filter(|e| e.module == "A").collect();
    let b_entries: Vec<&RequireImportEntry> = entries.iter().filter(|e| e.module == "B").collect();

    if a_entries.len() != 1 {
        return Err(format!("expected 1 entry for A, got {}: {a_entries:?}", a_entries.len()));
    }
    if b_entries.len() != 2 {
        return Err(format!("expected 2 entries for B, got {}: {b_entries:?}", b_entries.len()));
    }
    if a_entries[0].symbol != "x" {
        return Err(format!("wrong symbol for A: {:?}", a_entries[0].symbol));
    }
    let b_names: Vec<&str> = b_entries.iter().map(|e| e.symbol.as_str()).collect();
    if !b_names.contains(&"y") || !b_names.contains(&"z") {
        return Err(format!("wrong symbols for B: {b_names:?}"));
    }
    Ok(())
}

// ── Empty source ──────────────────────────────────────────────────────────────

#[test]
fn empty_source_returns_empty() -> Result<(), String> {
    let entries = extract_require_import_symbols("");
    if !entries.is_empty() {
        return Err(format!("expected empty for empty source, got {entries:?}"));
    }
    Ok(())
}

// ── Require without following import ─────────────────────────────────────────

#[test]
fn require_without_import_produces_no_entries() -> Result<(), String> {
    let source = "require Foo;\n";
    let entries = extract_require_import_symbols(source);
    if !entries.is_empty() {
        return Err(format!("expected no entries for require without import, got {entries:?}"));
    }
    Ok(())
}

#[test]
fn whitespace_around_import_method_call_is_tolerated() -> Result<(), String> {
    let source = "require Foo::Bar;\nFoo::Bar  ->  import  ( 'alpha', \"beta\" );\n";
    let entries = extract_require_import_symbols(source);

    if entries.len() != 2 {
        return Err(format!("expected 2 entries, got {}: {entries:?}", entries.len()));
    }
    let names: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
    if !names.contains(&"alpha") || !names.contains(&"beta") {
        return Err(format!("missing symbols in {names:?}"));
    }
    Ok(())
}

#[test]
fn alternate_qw_delimiters_produce_entries() -> Result<(), String> {
    let source =
        "require Foo;\nFoo->import(qw/alpha beta/);\nrequire Bar;\nBar->import(qw[gamma delta]);\n";
    let entries = extract_require_import_symbols(source);

    let names: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
    for expected in ["alpha", "beta", "gamma", "delta"] {
        if !names.contains(&expected) {
            return Err(format!("missing {expected:?} in {names:?}"));
        }
    }
    Ok(())
}

#[test]
fn malformed_module_separator_is_rejected() -> Result<(), String> {
    let source = "require Foo::::Bar;\nFoo::::Bar->import('x');\n";
    let entries = extract_require_import_symbols(source);

    if !entries.is_empty() {
        return Err(format!("expected malformed module name to be rejected, got {entries:?}"));
    }
    Ok(())
}

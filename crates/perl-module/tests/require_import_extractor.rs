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
fn same_line_require_import_pair_is_extracted() -> Result<(), String> {
    let source = "require Foo; Foo->import(qw(alpha beta));\n";
    let entries = extract_require_import_symbols(source);

    if entries.len() != 2 {
        return Err(format!("expected 2 entries, got {}: {entries:?}", entries.len()));
    }
    let names: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
    if !names.contains(&"alpha") || !names.contains(&"beta") {
        return Err(format!("missing same-line imports in {names:?}"));
    }
    if entries.iter().any(|e| e.import_byte_offset != 13) {
        return Err(format!("expected import_byte_offset=13, got {entries:?}"));
    }
    Ok(())
}

#[test]
fn indented_require_and_import_offsets_are_precise() -> Result<(), String> {
    let source = "  require Foo;\n    Foo->import('bar');\n";
    let entries = extract_require_import_symbols(source);

    let entry = entries.first().ok_or("expected one import entry")?;
    if entry.require_byte_offset != 2 {
        return Err(format!(
            "expected indented require_byte_offset=2, got {}",
            entry.require_byte_offset
        ));
    }
    if entry.import_byte_offset != 19 {
        return Err(format!(
            "expected indented import_byte_offset=19, got {}",
            entry.import_byte_offset
        ));
    }
    Ok(())
}

#[test]
fn malformed_bareword_module_names_are_rejected() -> Result<(), String> {
    let source = "require Foo:::Bar;\nFoo:::Bar->import('baz');\n";
    let entries = extract_require_import_symbols(source);
    if !entries.is_empty() {
        return Err(format!("expected no entries for malformed module name, got {entries:?}"));
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
fn qw_list_with_non_paren_delimiter_produces_entries() -> Result<(), String> {
    let cases = [
        ("Foo->import(qw[alpha beta]);", ["alpha", "beta"]),
        ("Foo->import(qw{gamma delta});", ["gamma", "delta"]),
        ("Foo->import(qw<epsilon zeta>);", ["epsilon", "zeta"]),
        ("Foo->import(qw/eta theta/);", ["eta", "theta"]),
        ("Foo->import(qw!iota kappa!);", ["iota", "kappa"]),
    ];

    for (import_line, expected_names) in cases {
        let source = format!("require Foo;\n{import_line}\n");
        let entries = extract_require_import_symbols(&source);
        let names: Vec<&str> = entries.iter().map(|entry| entry.symbol.as_str()).collect();

        if names != expected_names {
            return Err(format!("expected {expected_names:?} for {import_line:?}, got {names:?}"));
        }
    }

    Ok(())
}

#[test]
fn malformed_qw_list_is_rejected() -> Result<(), String> {
    let source = "require Foo;\nFoo->import(qw[alpha beta));\n";
    let entries = extract_require_import_symbols(source);
    if !entries.is_empty() {
        return Err(format!("expected no entries for malformed qw list, got {entries:?}"));
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

// ── Quote-operator whitespace conformance matrix ──────────────────────────────
//
// Shared case matrix (defined once, applied to this consumer and mirrored in
// perl-semantic-analyzer and perl-workspace):
//
//   qw(foo bar)    → ["foo", "bar"]          compact paren
//   qw (foo bar)   → ["foo", "bar"]          space before paren
//   qw[foo bar]    → ["foo", "bar"]          compact bracket
//   qw [foo bar]   → ["foo", "bar"]          space before bracket
//   qw/foo bar/    → ["foo", "bar"]          compact slash
//   qw /foo bar/   → ["foo", "bar"]          space before slash
//   qw{foo bar}    → ["foo", "bar"]          compact brace
//   qw {foo bar}   → ["foo", "bar"]          space before brace
//   qwfoo          → REJECTED (no entries)   bareword, not a delimiter
//
// This consumer is text-level (no AST).  `q {text}` and `qq [text]` are not
// valid import-call arguments for `require Foo; Foo->import(...)` so they are
// not included in this consumer's matrix.
//
// Invariant: leading-space forms must produce the SAME symbols as compact forms.

#[test]
fn conformance_matrix_qw_all_delimiters_compact() -> Result<(), String> {
    // Compact forms — no whitespace between qw and delimiter.
    let cases: &[(&str, &[&str])] = &[
        ("Foo->import(qw(foo bar));", &["foo", "bar"]),
        ("Foo->import(qw[foo bar]);", &["foo", "bar"]),
        ("Foo->import(qw/foo bar/);", &["foo", "bar"]),
        ("Foo->import(qw{foo bar});", &["foo", "bar"]),
    ];

    for (import_line, expected) in cases {
        let source = format!("require Foo;\n{import_line}\n");
        let entries = extract_require_import_symbols(&source);
        let got: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
        assert_eq!(
            got, *expected,
            "compact form {import_line:?}: expected {expected:?}, got {got:?}"
        );
    }
    Ok(())
}

#[test]
fn conformance_matrix_qw_all_delimiters_with_leading_space() -> Result<(), String> {
    // Space-before-delimiter forms — must yield the same symbols as compact forms.
    let cases: &[(&str, &[&str])] = &[
        ("Foo->import(qw (foo bar));", &["foo", "bar"]),
        ("Foo->import(qw [foo bar]);", &["foo", "bar"]),
        ("Foo->import(qw /foo bar/);", &["foo", "bar"]),
        ("Foo->import(qw {foo bar});", &["foo", "bar"]),
    ];

    for (import_line, expected) in cases {
        let source = format!("require Foo;\n{import_line}\n");
        let entries = extract_require_import_symbols(&source);
        let got: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
        assert_eq!(
            got, *expected,
            "space-before-delimiter form {import_line:?}: expected {expected:?}, got {got:?}"
        );
    }
    Ok(())
}

#[test]
fn conformance_matrix_qw_space_and_compact_forms_are_identical() -> Result<(), String> {
    // Exact parity check: compact and space-before-delimiter must produce the same symbols.
    let pairs: &[(&str, &str)] = &[
        ("Foo->import(qw(foo bar));", "Foo->import(qw (foo bar));"),
        ("Foo->import(qw[foo bar]);", "Foo->import(qw [foo bar]);"),
        ("Foo->import(qw/foo bar/);", "Foo->import(qw /foo bar/);"),
        ("Foo->import(qw{foo bar});", "Foo->import(qw {foo bar});"),
    ];

    for (compact_line, spaced_line) in pairs {
        let compact_src = format!("require Foo;\n{compact_line}\n");
        let spaced_src = format!("require Foo;\n{spaced_line}\n");

        let compact_syms: Vec<String> =
            extract_require_import_symbols(&compact_src).into_iter().map(|e| e.symbol).collect();
        let spaced_syms: Vec<String> =
            extract_require_import_symbols(&spaced_src).into_iter().map(|e| e.symbol).collect();

        assert_eq!(
            compact_syms, spaced_syms,
            "parity: compact {compact_line:?} vs spaced {spaced_line:?}: \
             compact={compact_syms:?}, spaced={spaced_syms:?}"
        );
    }
    Ok(())
}

#[test]
fn conformance_matrix_qwfoo_bareword_is_rejected() -> Result<(), String> {
    // `qwfoo` is NOT a valid qw operator — it is a bareword.  The import call
    // `Foo->import(qwfoo)` does not match any literal arg form, so the extractor
    // must produce no entries.
    let source = "require Foo;\nFoo->import(qwfoo);\n";
    let entries = extract_require_import_symbols(source);
    assert_eq!(
        entries.len(),
        0,
        "qwfoo bareword in import must produce no entries, got {entries:?}"
    );
    Ok(())
}

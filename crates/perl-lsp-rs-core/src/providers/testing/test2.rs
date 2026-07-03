//! Test2 framework awareness — static fact tables for Test2 bundles and tools.
//!
//! This module teaches the LSP about the [Test2](https://metacpan.org/pod/Test2::V0)
//! testing framework: which symbols a `use Test2::V0;` (or a `Test2::Tools::*`
//! module) brings into scope, and whether the import turns on `strict`/`warnings`.
//! It is a *static* fact table, not a Test2 runtime — it does not execute Perl.
//!
//! # Provenance (external-truth gate)
//!
//! The export lists below are verified against the canonical Test2-Suite source
//! rather than reasoned from the diff:
//!
//! - `Test2::V0` default `@EXPORT` and the `use Test2::Tools::* qw/.../;` lines —
//!   `Test-More/Test2-Suite` `lib/Test2/V0.pm`.
//! - Per-tool exports — `lib/Test2/Tools/{Basic,Compare,Subtest,Exception,
//!   Warnings,Class,...}.pm`.
//! - Import-list grammar (`!name` exclusion, `:DEFAULT`/`:ALL` tags,
//!   `name => {-as => 'alias'}` renames, `-prefix`/`-postfix`) — `exodist/Importer`.
//! - `strict`/`warnings` default and the `-no_strict` / `-no_warnings` /
//!   `-no_pragmas` opt-outs — the `Test2::V0` POD.
//!
//! # Scope model (documented simplification)
//!
//! When an import list explicitly selects symbols (positive barewords, a `qw//`
//! list, or a rename), only those symbols are considered imported — matching
//! `Importer`. Otherwise the module's full default set is used. Positive names
//! are trusted verbatim (added to scope even if not in our table), which keeps
//! the LSP from emitting false "unknown subroutine" diagnostics for tools we do
//! not enumerate. Exclusions and renames are applied on top of the default set.

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Per-tool export constants (traceable to individual Test2::Tools::* modules).
// ---------------------------------------------------------------------------

/// `Test2::Tools::Basic` — plan/assert/control primitives.
const BASIC: &[&str] = &[
    "ok",
    "pass",
    "fail",
    "diag",
    "note",
    "todo",
    "skip",
    "plan",
    "skip_all",
    "done_testing",
    "bail_out",
];

/// `Test2::Tools::Compare` — the comparison/check DSL as re-exported by
/// `Test2::V0` (the module's own default `@EXPORT` is only `is`/`like`; the rest
/// are `@EXPORT_OK` that V0 pulls in by name).
const COMPARE: &[&str] = &[
    "is",
    "like",
    "isnt",
    "unlike",
    "match",
    "mismatch",
    "validator",
    "hash",
    "array",
    "bag",
    "object",
    "meta",
    "meta_check",
    "number",
    "float",
    "rounded",
    "within",
    "string",
    "subset",
    "bool",
    "check_isa",
    "number_lt",
    "number_le",
    "number_ge",
    "number_gt",
    "in_set",
    "not_in_set",
    "check_set",
    "item",
    "field",
    "call",
    "call_list",
    "call_hash",
    "prop",
    "check",
    "all_items",
    "all_keys",
    "all_vals",
    "all_values",
    "etc",
    "end",
    "filter_items",
    "T",
    "F",
    "D",
    "DF",
    "E",
    "DNE",
    "FDNE",
    "U",
    "L",
    "event",
    "fail_events",
    "exact_ref",
];

/// The `Test2::Tools::Compare` module's *own* default export set (used when the
/// tool module is imported standalone rather than via a bundle).
const COMPARE_OWN_DEFAULT: &[&str] = &["is", "like"];

/// `Test2::Tools::ClassicCompare` — the `Test::More`-style operator compare.
const CLASSIC_COMPARE: &[&str] = &["cmp_ok"];

/// `Test2::Tools::Warnings`.
const WARNINGS: &[&str] = &["warns", "warning", "warnings", "no_warnings"];

/// `Test2::Tools::Class`.
const CLASS: &[&str] = &["can_ok", "isa_ok", "DOES_ok"];

/// `Test2::Tools::Exception`.
const EXCEPTION: &[&str] = &["dies", "lives", "try_ok"];

/// `Test2::Tools::Defer`.
const DEFER: &[&str] = &["def", "do_def"];

/// `Test2::Tools::Mock`.
const MOCK: &[&str] = &["mock", "mocked"];

/// `Test2::Tools::Ref`.
const REF: &[&str] = &["ref_ok", "ref_is", "ref_is_not"];

/// `Test2::Tools::Encoding`.
const ENCODING: &[&str] = &["set_encoding"];

/// `Test2::Tools::Exports`.
const EXPORTS: &[&str] = &["imported_ok", "not_imported_ok"];

/// `Test2::Tools::Refcount`.
const REFCOUNT: &[&str] = &["is_refcount", "is_oneref", "refcount"];

/// `Test2::Tools::Event`.
const EVENT: &[&str] = &["gen_event"];

/// `Test2::API` symbols re-exported by `Test2::V0`.
const API: &[&str] = &["intercept", "context"];

/// `Test2::Tools::Subtest` — the module's *own* default exports. `Test2::V0`
/// renames `subtest_buffered` to the familiar `subtest`, so a bundle exposes
/// `subtest` while the standalone tool exposes the `*_streamed`/`*_buffered`
/// pair.
const SUBTEST_OWN: &[&str] = &["subtest_streamed", "subtest_buffered"];

/// The `subtest` name as exposed by the `Test2::V0`/`Test2::V1` bundles.
const SUBTEST_BUNDLE: &[&str] = &["subtest"];

/// The complete `Test2::V0` default `@EXPORT` set, composed from the tool
/// modules the bundle pulls in. This is the single source of truth for
/// "what does `use Test2::V0;` put in scope".
static V0_DEFAULT: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut v: Vec<&'static str> = Vec::new();
    for group in [
        BASIC,
        COMPARE,
        CLASSIC_COMPARE,
        WARNINGS,
        CLASS,
        EXCEPTION,
        DEFER,
        MOCK,
        REF,
        ENCODING,
        EXPORTS,
        REFCOUNT,
        EVENT,
        API,
        SUBTEST_BUNDLE,
    ] {
        v.extend_from_slice(group);
    }
    v.sort_unstable();
    v.dedup();
    v
});

// ---------------------------------------------------------------------------
// Module classification.
// ---------------------------------------------------------------------------

/// Whether `module` is any Test2 module the LSP has awareness of.
pub fn is_test2_module(module: &str) -> bool {
    is_test2_bundle(module)
        || module.starts_with("Test2::Tools::")
        || module.starts_with("Test2::Plugin::")
        || module == "Test2::API"
}

/// Whether `module` is a Test2 *bundle* — one that turns on `strict` and
/// `warnings` for the importer (unless disabled by an import option).
///
/// Individual `Test2::Tools::*` modules do **not** enable strict/warnings; only
/// the recommended bundles (`Test2::V0`, `Test2::V1`, `Test2::Bundle::*`) do.
pub fn is_test2_bundle(module: &str) -> bool {
    matches!(module, "Test2::V0" | "Test2::V1" | "Test2::Suite")
        || module.starts_with("Test2::Bundle::")
}

/// The default export set for a known Test2 module, or `None` if the module is
/// a Test2 module we recognize structurally but have no enumerated export table
/// for (e.g. a plugin, or an unfamiliar bundle). `None` means "trust explicit
/// imports, otherwise unknown" — callers should not emit unknown-sub
/// diagnostics for such modules.
pub fn module_default_exports(module: &str) -> Option<&'static [&'static str]> {
    // Bundles: V0/V1 share the recommended default set. Other bundles are
    // recognized as strict/warnings providers but not enumerated.
    if matches!(module, "Test2::V0" | "Test2::V1") {
        return Some(V0_DEFAULT.as_slice());
    }
    let group: &'static [&'static str] = match module {
        "Test2::Tools::Basic" => BASIC,
        "Test2::Tools::Compare" => COMPARE_OWN_DEFAULT,
        "Test2::Tools::ClassicCompare" => CLASSIC_COMPARE,
        "Test2::Tools::Warnings" => WARNINGS,
        "Test2::Tools::Class" => CLASS,
        "Test2::Tools::Exception" => EXCEPTION,
        "Test2::Tools::Defer" => DEFER,
        "Test2::Tools::Mock" => MOCK,
        "Test2::Tools::Ref" => REF,
        "Test2::Tools::Encoding" => ENCODING,
        "Test2::Tools::Exports" => EXPORTS,
        "Test2::Tools::Refcount" => REFCOUNT,
        "Test2::Tools::Event" => EVENT,
        "Test2::Tools::Subtest" => SUBTEST_OWN,
        "Test2::API" => API,
        _ => return None,
    };
    Some(group)
}

// ---------------------------------------------------------------------------
// Import resolution.
// ---------------------------------------------------------------------------

/// The `strict`/`warnings` pragma effect an import applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Test2Pragmas {
    /// `strict` is turned on by this import.
    pub strict: bool,
    /// `warnings` is turned on by this import.
    pub warnings: bool,
}

/// The resolved effect of a single Test2 `use` statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedImport {
    /// Symbols brought into scope by this import.
    pub symbols: BTreeSet<String>,
    /// Pragma effect, present only for bundle imports.
    pub pragmas: Option<Test2Pragmas>,
}

/// Match `name => { ... -as => 'alias' ... }` renames in an import list.
static RENAME_AS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\w+)\s*=>\s*\{[^}]*?-as\s*=>\s*['"]?(\w+)['"]?[^}]*?\}"#)
        .unwrap_or_else(|_| unreachable!("static Test2 -as rename pattern is valid"))
});

/// Match `name => { ... -prefix => 'p' ... }` / `-postfix` renames.
static RENAME_FIX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\w+)\s*=>\s*\{[^}]*?-(prefix|postfix)\s*=>\s*['"]?(\w+)['"]?[^}]*?\}"#)
        .unwrap_or_else(|_| unreachable!("static Test2 -prefix/-postfix pattern is valid"))
});

/// Resolve the imported symbols and pragma effect of a single Test2 `use`
/// statement, given the module name and the raw import-argument text (whatever
/// appears between the module name and the terminating `;`).
///
/// Returns `None` if `module` is not a recognized Test2 module.
pub fn resolve_import(module: &str, raw_args: &str) -> Option<ResolvedImport> {
    if !is_test2_module(module) {
        return None;
    }

    // `use Test2::V0 ();` — an explicit empty import list. Perl does not call
    // `import`, so no symbols are imported and (for bundles) no strict/warnings
    // pragmas are applied. The module is still loaded, so return an empty import
    // rather than `None`.
    let trimmed_args = raw_args.trim();
    if trimmed_args.starts_with('(')
        && trimmed_args.ends_with(')')
        && trimmed_args[1..trimmed_args.len() - 1].trim().is_empty()
    {
        return Some(ResolvedImport { symbols: BTreeSet::new(), pragmas: None });
    }

    let bundle = is_test2_bundle(module);
    let default_set = module_default_exports(module);

    // Pragma resolution (bundles only).
    let pragmas = if bundle {
        let no_pragmas = args_contains_option(raw_args, "no_pragmas");
        let no_strict = no_pragmas || args_contains_option(raw_args, "no_strict");
        let no_warnings = no_pragmas || args_contains_option(raw_args, "no_warnings");
        Some(Test2Pragmas { strict: !no_strict, warnings: !no_warnings })
    } else {
        None
    };

    // Extract renames first (and strip their spans so their bareword names are
    // not double-counted as positive imports).
    let mut renames: Vec<(String, String)> = Vec::new();
    let mut stripped = raw_args.to_string();
    for caps in RENAME_AS.captures_iter(raw_args) {
        if let (Some(name), Some(alias)) = (caps.get(1), caps.get(2)) {
            renames.push((name.as_str().to_string(), alias.as_str().to_string()));
        }
    }
    for caps in RENAME_FIX.captures_iter(raw_args) {
        if let (Some(name), Some(kind), Some(fix)) = (caps.get(1), caps.get(2), caps.get(3)) {
            let base = name.as_str();
            let alias = if kind.as_str() == "prefix" {
                format!("{}{}", fix.as_str(), base)
            } else {
                format!("{}{}", base, fix.as_str())
            };
            renames.push((base.to_string(), alias));
        }
    }
    // Remove matched rename spans so the remaining scan does not see the raw
    // `name => { ... }` text.
    stripped = RENAME_AS.replace_all(&stripped, " ").into_owned();
    stripped = RENAME_FIX.replace_all(&stripped, " ").into_owned();

    let atoms = tokenize_import_args(&stripped);

    let mut positives: Vec<String> = Vec::new();
    let mut exclusions: BTreeSet<String> = BTreeSet::new();
    let mut include_default_tag = false;
    let mut include_all_tag = false;

    for atom in &atoms {
        let atom = atom.trim();
        if atom.is_empty() {
            continue;
        }
        if let Some(rest) = atom.strip_prefix('!') {
            // Exclusion: `!name` (pattern/tag exclusions are ignored — high
            // precision over completeness).
            if is_bareword(rest) {
                exclusions.insert(rest.to_string());
            }
            continue;
        }
        if let Some(tag) = atom.strip_prefix(':') {
            match tag.to_ascii_lowercase().as_str() {
                "default" => include_default_tag = true,
                "all" => include_all_tag = true,
                _ => {}
            }
            continue;
        }
        if atom.starts_with('-') {
            // Import option (`-no_strict`, `-target`, ...): consumed elsewhere.
            continue;
        }
        if is_bareword(atom) {
            positives.push(atom.to_string());
        }
    }

    let rename_aliases: Vec<String> = renames.iter().map(|(_, alias)| alias.clone()).collect();

    // Decide the base set. An explicit selection (positive names, renames, or
    // `:ALL`) replaces the default unless `:DEFAULT` is also present.
    let has_explicit = !positives.is_empty() || !renames.is_empty() || include_all_tag;
    let use_default = !has_explicit || include_default_tag;

    // `:ALL` is handled via `use_default` above: we do not enumerate EXPORT_OK,
    // so the default set is our best-effort superset for known bundles.
    let mut symbols: BTreeSet<String> = BTreeSet::new();
    if use_default {
        if let Some(defaults) = default_set {
            for &sym in defaults {
                symbols.insert(sym.to_string());
            }
        }
    }
    for name in &positives {
        symbols.insert(name.clone());
    }
    for alias in &rename_aliases {
        symbols.insert(alias.clone());
    }
    // Renamed originals are not imported under their original name.
    for (orig, _) in &renames {
        if !positives.iter().any(|p| p == orig) {
            symbols.remove(orig);
        }
    }
    for excluded in &exclusions {
        symbols.remove(excluded);
    }

    Some(ResolvedImport { symbols, pragmas })
}

/// Aggregate Test2 facts for an entire source file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Test2Facts {
    /// Test2 modules imported in the file (in source order).
    pub modules: Vec<String>,
    /// All symbols imported from Test2 modules.
    pub imported_symbols: BTreeSet<String>,
    /// Whether some Test2 bundle turned on `strict`.
    pub strict: bool,
    /// Whether some Test2 bundle turned on `warnings`.
    pub warnings: bool,
}

impl Test2Facts {
    /// Whether the file imports any Test2 module at all.
    pub fn uses_test2(&self) -> bool {
        !self.modules.is_empty()
    }

    /// Whether an imported Test2 bundle turns on the named pragma. Only
    /// `strict` and `warnings` are provided by Test2 bundles; every other
    /// feature returns `false`.
    pub fn provides_pragma(&self, feature: &str) -> bool {
        match feature {
            "strict" => self.strict,
            "warnings" => self.warnings,
            _ => false,
        }
    }

    /// Whether the file imports any Test2 *bundle* (`Test2::V0`, etc.).
    pub fn uses_test2_bundle(&self) -> bool {
        self.modules.iter().any(|m| is_test2_bundle(m))
    }

    /// Whether `name` is a symbol imported from Test2 in this file.
    pub fn is_imported(&self, name: &str) -> bool {
        self.imported_symbols.contains(name)
    }

    /// Scan `source` for Test2 `use` statements and aggregate their effects.
    pub fn from_source(source: &str) -> Self {
        let mut facts = Test2Facts::default();
        for stmt in use_statements(source) {
            let Some((module, args)) = parse_use_statement(&stmt) else {
                continue;
            };
            let Some(resolved) = resolve_import(&module, &args) else {
                continue;
            };
            facts.modules.push(module);
            for sym in resolved.symbols {
                facts.imported_symbols.insert(sym);
            }
            if let Some(pragmas) = resolved.pragmas {
                facts.strict |= pragmas.strict;
                facts.warnings |= pragmas.warnings;
            }
        }
        facts
    }
}

// ---------------------------------------------------------------------------
// Source scanning helpers.
// ---------------------------------------------------------------------------

/// Whether `s` is a plain Perl identifier (bareword), optionally quoted by the
/// caller before this check.
fn is_bareword(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether an import-option flag (`no_strict`, `no_warnings`, ...) appears in the
/// raw args, in either `-flag` or `-flag => 1` form.
fn args_contains_option(raw_args: &str, flag: &str) -> bool {
    let needle = format!("-{flag}");
    raw_args.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-')).any(|tok| {
        // Match the exact flag token, not a prefix (`-no_strict` must not match
        // a hypothetical `-no_strictness`).
        tok == needle
    })
}

/// Split raw import-argument text into classifiable atoms. Handles `qw//`
/// lists, quoted strings, and comma / fat-comma separated barewords.
fn tokenize_import_args(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let expanded = expand_qw(raw);
    // Normalize fat commas and commas to a single separator, then split.
    for piece in expanded.split([',']) {
        let piece = piece.replace("=>", " ");
        for tok in piece.split_whitespace() {
            let cleaned = strip_quotes(tok);
            if !cleaned.is_empty() {
                out.push(cleaned.to_string());
            }
        }
    }
    out
}

/// Expand every `qw/.../` (and `qw{}`, `qw()`, `qw[]`, `qw<>`) construct in
/// `raw` into a plain space-separated word list.
fn expand_qw(raw: &str) -> String {
    let mut out = String::new();
    let bytes = raw.as_bytes();
    let mut i = 0;
    // Invariant: `i` is always on a UTF-8 char boundary at the top of the loop
    // (we only ever advance past a whole char or past an ASCII delimiter byte),
    // so `raw[i..]` and the delimiter slices below never split a codepoint.
    // `qw`, its delimiters, and its closers are all ASCII, so the structural
    // scan uses byte predicates while non-ASCII content is copied char-wise.
    while i < bytes.len() {
        // Only treat `qw` as the quote-word operator on a word boundary, so
        // barewords like `qwerty` or `my_qw` are not misread as `qw`.
        let on_word_boundary =
            i == 0 || !matches!(bytes[i - 1], b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_');
        if on_word_boundary && bytes[i] == b'q' && bytes.get(i + 1) == Some(&b'w') {
            // Find the delimiter after optional whitespace (all ASCII).
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if let Some(&open) = bytes.get(j) {
                // A real delimiter is non-word, non-whitespace, and must be
                // ASCII: a non-ASCII byte here is a multi-byte lead/continuation
                // byte, and treating it as the delimiter would slice `raw`
                // mid-codepoint below and panic. (`qwords` would otherwise treat
                // `o` as the delimiter.)
                if open.is_ascii()
                    && !open.is_ascii_alphanumeric()
                    && open != b'_'
                    && !open.is_ascii_whitespace()
                {
                    let close = match open {
                        b'(' => b')',
                        b'{' => b'}',
                        b'[' => b']',
                        b'<' => b'>',
                        other => other,
                    };
                    if let Some(end_rel) = bytes[j + 1..].iter().position(|&b| b == close) {
                        // `j + 1` and `j + 1 + end_rel` sit on ASCII delimiter
                        // bytes, i.e. char boundaries, so slicing `raw` is safe.
                        let inner = &raw[j + 1..j + 1 + end_rel];
                        out.push(' ');
                        out.push_str(inner);
                        out.push(' ');
                        i = j + 1 + end_rel + 1;
                        continue;
                    }
                }
            }
        }
        // Copy the whole current char (handles multi-byte UTF-8 safely).
        let Some(ch) = raw[i..].chars().next() else { break };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Strip surrounding single or double quotes from a token.
fn strip_quotes(tok: &str) -> &str {
    let tok = tok.trim();
    let bytes = tok.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'\'' || first == b'"') && first == last {
            return &tok[1..tok.len() - 1];
        }
    }
    tok
}

/// Extract `use ...;` statements from Perl source, respecting quotes and `#`
/// comments so multi-line imports and commented-out lines are handled.
fn use_statements(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut in_comment = false;
    let mut escaped = false;
    for c in source.chars() {
        if in_comment {
            if c == '\n' {
                in_comment = false;
            }
            continue;
        }
        // A backslash inside a string escapes the next char, so an escaped
        // quote does not close the string (e.g. `use Foo "a\"b";`).
        if escaped {
            cur.push(c);
            escaped = false;
            continue;
        }
        match c {
            '\\' if in_single || in_double => {
                escaped = true;
                cur.push(c);
            }
            '#' if !in_single && !in_double => in_comment = true,
            '\'' if !in_double => {
                in_single = !in_single;
                cur.push(c);
            }
            '"' if !in_single => {
                in_double = !in_double;
                cur.push(c);
            }
            ';' if !in_single && !in_double => {
                let trimmed = cur.trim();
                if starts_with_keyword(trimmed, "use") {
                    out.push(trimmed.to_string());
                }
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    out
}

/// Whether `stmt` begins with the bareword `keyword` followed by whitespace.
fn starts_with_keyword(stmt: &str, keyword: &str) -> bool {
    stmt.strip_prefix(keyword).is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

/// Parse a `use Module ...` statement into `(module, raw_args)`.
fn parse_use_statement(stmt: &str) -> Option<(String, String)> {
    let rest = stmt.strip_prefix("use")?;
    let rest = rest.trim_start();
    // Read the module name: identifier chars and `::`.
    let module: String =
        rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':').collect();
    if module.is_empty() {
        return None;
    }
    let args = rest[module.len()..].trim().to_string();
    Some((module, args))
}

#[cfg(test)]
mod tests;

//! Single-line Perl import head parsing and literal require/import extraction.
//!
//! Parse a single source line that starts with `use` or `require` and return
//! the first import token with stable byte offsets.
//!
//! Also provides [`extract_require_import_symbols`], a text-level extractor
//! that recognises the literal `require Module; Module->import(...)` adjacency
//! pattern in multi-line source without requiring AST construction.

/// When a module is loaded relative to program execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadTiming {
    /// Module is loaded at compile time (e.g. `use`).
    CompileTime,
    /// Module is loaded at runtime (e.g. `require`).
    Runtime,
}

/// Whether the module's `import` method is called after loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportBehavior {
    /// The module's `import` method is called (as with `use`).
    CallsImport,
    /// No `import` call is made (as with `require`).
    NoImport,
}

/// Semantic description of a `use`/`require` dispatch form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchSemantics {
    /// When the module load happens.
    pub load_timing: LoadTiming,
    /// Whether `import` is called on the loaded module.
    pub import_behavior: ImportBehavior,
}

impl DispatchSemantics {
    /// A short human-readable description suitable for hover text.
    #[must_use]
    pub fn hover_description(&self) -> &'static str {
        match (self.load_timing, self.import_behavior) {
            (LoadTiming::CompileTime, ImportBehavior::CallsImport) => {
                "compile-time load; calls import()"
            }
            (LoadTiming::Runtime, ImportBehavior::NoImport) => "runtime load; no import() call",
            (LoadTiming::CompileTime, ImportBehavior::NoImport) => {
                "compile-time load; no import() call"
            }
            (LoadTiming::Runtime, ImportBehavior::CallsImport) => "runtime load; calls import()",
        }
    }
}

/// How a `use` statement spells its import list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportListForm {
    /// `use Module;`
    Default,
    /// `use Module ();`
    Empty,
    /// `use Module (...)`
    Explicit,
}

/// Distinguishes the two syntactic forms of `require`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequireForm {
    /// `require Module::Name` — bare module name.
    ModuleName,
    /// `require "path/to/file.pm"` or `require 'path/to/file.pm'` — quoted file path.
    FilePath,
}

/// Classifies the import statement form for a parsed line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleImportKind {
    /// `use Module::Name;`
    Use,
    /// `require Module::Name;` or `require "file.pm";`
    Require,
    /// `use parent ...`
    UseParent,
    /// `use base ...`
    UseBase,
}

impl ModuleImportKind {
    /// Returns the dispatch semantics for this import kind.
    #[must_use]
    pub fn dispatch_semantics(self) -> DispatchSemantics {
        match self {
            ModuleImportKind::Use | ModuleImportKind::UseParent | ModuleImportKind::UseBase => {
                DispatchSemantics {
                    load_timing: LoadTiming::CompileTime,
                    import_behavior: ImportBehavior::CallsImport,
                }
            }
            ModuleImportKind::Require => DispatchSemantics {
                load_timing: LoadTiming::Runtime,
                import_behavior: ImportBehavior::NoImport,
            },
        }
    }
}

/// Parsed leading import token from a `use`/`require` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleImportHead<'a> {
    /// Parsed statement kind.
    pub kind: ModuleImportKind,
    /// First token after `use` or `require` (quotes stripped for file-path forms).
    pub token: &'a str,
    /// Inclusive byte start offset of `token` in the full line.
    pub token_start: usize,
    /// Exclusive byte end offset of `token` in the full line.
    pub token_end: usize,
    /// For `require`, whether the argument was a quoted file path or a bare module name.
    /// Always `None` for `use` forms.
    require_form: Option<RequireForm>,
    /// For `use` statements, how the import list is spelled.
    pub import_list: Option<ImportListForm>,
}

/// Resolve a known export tag to its symbol list for a specific module.
///
/// The `tag` argument can be passed with or without a leading `:`.
/// Returns `None` when the module/tag pair is not in the built-in catalog.
#[must_use]
pub fn resolve_known_export_tag(module: &str, tag: &str) -> Option<&'static [&'static str]> {
    let normalized_tag = tag.strip_prefix(':').unwrap_or(tag);
    match (module, normalized_tag) {
        ("POSIX", "sys_wait_h") => Some(&["WIFEXITED", "WEXITSTATUS", "WIFSIGNALED", "WTERMSIG"]),
        ("POSIX", "fcntl_h") => Some(&["F_GETFL", "F_SETFL", "F_SETFD", "F_GETFD"]),
        ("POSIX", "termios_h") => Some(&["TCSANOW", "TCSADRAIN", "TCSAFLUSH", "B9600"]),
        ("File::Find", "find") => Some(&["find", "finddepth"]),
        ("Fcntl", "seek") => Some(&["SEEK_SET", "SEEK_CUR", "SEEK_END"]),
        ("Fcntl", "lock") => Some(&["LOCK_SH", "LOCK_EX", "LOCK_NB", "LOCK_UN"]),
        ("Encode", "fallback") => Some(&["FB_DEFAULT", "FB_CROAK", "FB_QUIET", "FB_WARN"]),
        _ => None,
    }
}

impl<'a> ModuleImportHead<'a> {
    /// Returns the [`RequireForm`] for `require` statements, or `None` for `use` forms.
    #[must_use]
    pub fn require_form(&self) -> Option<RequireForm> {
        self.require_form
    }

    /// Returns the module name for resolution purposes.
    ///
    /// For `require "Foo/Bar.pm"` (FilePath form with `.pm` extension), converts
    /// the file-path token to canonical module-name format (`Foo::Bar`).
    ///
    /// All other forms (`.pl`, extensionless, bare `ModuleName`, `use` statements)
    /// return the token unchanged. Does NOT mutate `token`, `token_start`, or
    /// `token_end` — the original offsets and raw token remain valid.
    #[must_use]
    pub fn token_as_module_name(&self) -> String {
        if self.require_form == Some(RequireForm::FilePath) && self.token.ends_with(".pm") {
            crate::path::module_path_to_name(self.token)
        } else {
            self.token.to_owned()
        }
    }
}

/// Parse the leading import token of a single Perl source line.
///
/// Returns [`None`] when the line does not start with `use` or `require`
/// (after leading whitespace) or when no token is present after the keyword.
#[must_use]
pub fn parse_module_import_head(line: &str) -> Option<ModuleImportHead<'_>> {
    if let Some((token, token_start, token_end)) = parse_statement_head(line, "use") {
        let kind = match token {
            "parent" => ModuleImportKind::UseParent,
            "base" => ModuleImportKind::UseBase,
            _ => ModuleImportKind::Use,
        };

        let import_list = match kind {
            ModuleImportKind::Use => Some(classify_use_import_list(&line[token_end..])),
            ModuleImportKind::UseParent | ModuleImportKind::UseBase => None,
            ModuleImportKind::Require => None,
        };

        return Some(ModuleImportHead {
            kind,
            token,
            token_start,
            token_end,
            require_form: None,
            import_list,
        });
    }

    if let Some(result) = parse_require_head(line) {
        return Some(result);
    }

    None
}

/// Parse a `require` statement, handling both bare module names and quoted file paths.
fn parse_require_head(line: &str) -> Option<ModuleImportHead<'_>> {
    let trimmed = line.trim_start();
    let leading = line.len().saturating_sub(trimmed.len());

    let rest = trimmed.strip_prefix("require")?;
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }

    let after_keyword = leading + "require".len();

    let rest_trimmed = rest.trim_start();
    let quote_offset = rest.len() - rest_trimmed.len();

    if let Some(quote_char) = rest_trimmed.chars().next().filter(|ch| *ch == '"' || *ch == '\'') {
        let quoted = &rest_trimmed[quote_char.len_utf8()..];
        let close_idx = quoted.find(quote_char)?;
        let inner = &quoted[..close_idx];

        let token_start = after_keyword + quote_offset + quote_char.len_utf8();
        let token_end = token_start + inner.len();
        return Some(ModuleImportHead {
            kind: ModuleImportKind::Require,
            token: inner,
            token_start,
            token_end,
            require_form: Some(RequireForm::FilePath),
            import_list: None,
        });
    }

    let (token, token_rel_start, token_rel_end) = first_token_with_range(rest)?;
    let token_start = after_keyword + token_rel_start;
    let token_end = after_keyword + token_rel_end;

    Some(ModuleImportHead {
        kind: ModuleImportKind::Require,
        token,
        token_start,
        token_end,
        require_form: Some(RequireForm::ModuleName),
        import_list: None,
    })
}

fn classify_use_import_list(rest: &str) -> ImportListForm {
    let trimmed = rest.trim_start();

    if trimmed.is_empty() || trimmed.starts_with(';') {
        return ImportListForm::Default;
    }

    if let Some(after_open) = trimmed.strip_prefix('(')
        && let Some(close_idx) = after_open.find(')')
        && after_open[..close_idx].trim().is_empty()
    {
        let after_close = after_open[close_idx + 1..].trim_start();
        if after_close.is_empty() || after_close.starts_with(';') || after_close.starts_with('#') {
            return ImportListForm::Empty;
        }
    }

    ImportListForm::Explicit
}

fn parse_statement_head<'a>(line: &'a str, keyword: &str) -> Option<(&'a str, usize, usize)> {
    let trimmed = line.trim_start();
    let leading = line.len().saturating_sub(trimmed.len());

    let rest = trimmed.strip_prefix(keyword)?;
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }

    let (token, token_rel_start, token_rel_end) = first_token_with_range(rest)?;
    let token_start = leading + keyword.len() + token_rel_start;
    let token_end = leading + keyword.len() + token_rel_end;

    Some((token, token_start, token_end))
}

fn first_token_with_range(input: &str) -> Option<(&str, usize, usize)> {
    let mut token_start = None;

    for (idx, ch) in input.char_indices() {
        match token_start {
            None => {
                if is_token_delimiter(ch) {
                    continue;
                }
                token_start = Some(idx);
            }
            Some(start) => {
                if is_token_delimiter(ch) {
                    if start == idx {
                        return None;
                    }
                    return Some((&input[start..idx], start, idx));
                }
            }
        }
    }

    if let Some(start) = token_start {
        if start < input.len() { Some((&input[start..], start, input.len())) } else { None }
    } else {
        None
    }
}

fn is_token_delimiter(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ';' | '(' | ')')
}

// ── Literal require/import extractor ────────────────────────────────────────

/// A single symbol extracted from a literal `require Module; Module->import(...)` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequireImportEntry {
    /// The fully qualified module name (e.g. `Foo::Bar`).
    pub module: String,
    /// The symbol name imported from the module.
    pub symbol: String,
    /// Byte offset of the `require` statement start in the source string.
    pub require_byte_offset: usize,
    /// Byte offset of the `Module->import(...)` statement start in the source string.
    pub import_byte_offset: usize,
}

/// Extract symbols from literal `require Module; Module->import(...)` patterns
/// found anywhere in `source`.
///
/// # Recognised patterns
///
/// - `require Module::Path;` followed on the same or a later nearby line by
///   `Module::Path->import(qw(a b c));` or another Perl `qw` delimiter
/// - `require Module::Path;` followed by
///   `Module::Path->import('a', 'b');`
/// - `require Module::Path;` followed by
///   `Module::Path->import("a", "b");`
///
/// Whitespace around `->`, `import`, and the call parentheses is tolerated
/// for literal receiver calls.
///
/// # Non-goals (not matched)
///
/// - `require $var;` (dynamic module name — variable)
/// - `Module->import(@list);` (dynamic argument list — array variable)
/// - `map { Module->import($_) } @syms;` (computed expressions)
/// - `$class->import('x');` (variable receiver)
///
/// The extractor is **text-level only** — it does not parse a full AST.
/// It works on whitespace-normalised lines and a small lookahead window.
#[must_use]
pub fn extract_require_import_symbols(source: &str) -> Vec<RequireImportEntry> {
    let mut entries = Vec::new();

    // Build a list of (trimmed_byte_offset, trimmed_line) pairs.
    let lines: Vec<(usize, &str)> = {
        let mut v = Vec::new();
        let mut offset = 0usize;
        for line in source.split('\n') {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let leading = line.len().saturating_sub(line.trim_start().len());
                v.push((offset + leading, trimmed));
            }
            offset += line.len() + 1; // +1 for the '\n' we split on
        }
        v
    };

    for (i, &(req_offset, req_line)) in lines.iter().enumerate() {
        // Match `require BarewordModule;`
        let parsed_require = match parse_literal_require_line(req_line) {
            Some(parsed_require) => parsed_require,
            None => continue,
        };
        let module = parsed_require.module;

        if collect_literal_import_entries(
            &mut entries,
            module,
            req_offset,
            req_offset + parsed_require.tail_start,
            parsed_require.tail,
        ) {
            continue;
        }

        // Scan the remaining lines within a reasonable window (same scope, adjacent).
        // We allow up to 5 blank-skipped lines between require and import to handle
        // common real-world spacing without false positives across unrelated statements.
        let window_end = (i + 1 + 5).min(lines.len());
        for &(imp_offset, imp_line) in &lines[i + 1..window_end] {
            if collect_literal_import_entries(
                &mut entries,
                module,
                req_offset,
                imp_offset,
                imp_line,
            ) {
                // Consumed this import statement — move to next require.
                break;
            }
            // If the line is a different require or a use, stop looking for a matching import.
            if is_statement_terminator(imp_line) {
                break;
            }
        }
    }

    entries
}

/// Parse a line of the form `require BarewordModule::Name;`.
///
/// Returns the module name string slice from `line`, or `None` if the line
/// does not match this exact pattern.
///
/// Rejects:
/// - `require $var;` (variable)
/// - `require "file.pm";` (quoted file path)
/// - `require 'file.pm';` (quoted file path)
struct ParsedLiteralRequire<'a> {
    module: &'a str,
    tail_start: usize,
    tail: &'a str,
}

fn parse_literal_require_line(line: &str) -> Option<ParsedLiteralRequire<'_>> {
    let rest = line.strip_prefix("require")?;
    // Must have whitespace after `require`.
    if !rest.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let leading_after_keyword = rest.len().saturating_sub(rest.trim_start().len());
    let rest = rest.trim_start();
    // Reject variables and quoted paths.
    if rest.starts_with('$') || rest.starts_with('"') || rest.starts_with('\'') {
        return None;
    }

    let module_end = rest.find(|c: char| c == ';' || c.is_whitespace()).unwrap_or(rest.len());
    let module = &rest[..module_end];
    if !is_valid_bareword_module_name(module) {
        return None;
    }

    let after_module = &rest[module_end..];
    let semicolon_offset = after_module.find(';')?;
    let tail_start = "require".len() + leading_after_keyword + module_end + semicolon_offset + 1;
    Some(ParsedLiteralRequire { module, tail_start, tail: &line[tail_start..] })
}

fn is_valid_bareword_module_name(module: &str) -> bool {
    if module.is_empty() {
        return false;
    }

    module.split("::").all(|part| {
        !part.is_empty()
            && part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

fn collect_literal_import_entries(
    entries: &mut Vec<RequireImportEntry>,
    module: &str,
    require_byte_offset: usize,
    import_byte_offset: usize,
    candidate: &str,
) -> bool {
    let leading = candidate.len().saturating_sub(candidate.trim_start().len());
    let candidate = candidate.trim_start();

    if let Some(symbols) = parse_literal_import_call(candidate, module) {
        for symbol in symbols {
            entries.push(RequireImportEntry {
                module: module.to_string(),
                symbol,
                require_byte_offset,
                import_byte_offset: import_byte_offset + leading,
            });
        }
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_as_module_name_keeps_non_pm_require_tokens() -> Result<(), String> {
        let bare = parse_module_import_head("require Local::Util;")
            .ok_or_else(|| "expected bare require head".to_string())?;
        assert_eq!(bare.token_as_module_name(), "Local::Util");

        let script = parse_module_import_head(r#"require "script.pl";"#)
            .ok_or_else(|| "expected quoted script require head".to_string())?;
        assert_eq!(script.token_as_module_name(), "script.pl");

        Ok(())
    }
}

/// Parse a line of the form `Module::Name->import(literal list);`.
///
/// Returns `Some(Vec<String>)` of symbol names when the line matches the
/// expected module name with only literal arguments (`qw(...)`, `'x'`, `"x"`).
/// Returns `None` when the line does not match or contains dynamic arguments.
fn parse_literal_import_call(line: &str, expected_module: &str) -> Option<Vec<String>> {
    let after_module = line.strip_prefix(expected_module)?.trim_start();
    let after_arrow = after_module.strip_prefix("->")?.trim_start();
    let after_method = after_arrow.strip_prefix("import")?.trim_start();
    let after_open = after_method.strip_prefix('(')?;

    // Find the matching close paren.
    let close_idx = after_open.rfind(')')?;
    let args_src = &after_open[..close_idx];

    // Reject dynamic arguments: arrays, scalars, map, grep.
    if args_src.contains('@') || args_src.contains('$') {
        return None;
    }

    let symbols = parse_literal_arg_list(args_src)?;
    Some(symbols)
}

/// Parse the interior of an `import(...)` argument list that contains only
/// literal strings and/or a `qw(...)` list.
///
/// Returns `None` when any argument looks dynamic or unparseable.
fn parse_literal_arg_list(args: &str) -> Option<Vec<String>> {
    let trimmed = args.trim();

    if trimmed.is_empty() {
        return Some(Vec::new());
    }

    if let Some(words) = parse_qw_arg_list(trimmed) {
        return Some(words);
    }

    // Comma-separated literal strings: 'a', "b", 'c'
    let mut symbols = Vec::new();
    for part in trimmed.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        // Single-quoted string.
        if let Some(inner) = p.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
            if inner.is_empty() {
                continue;
            }
            symbols.push(inner.to_string());
            continue;
        }
        // Double-quoted string.
        if let Some(inner) = p.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            if inner.is_empty() {
                continue;
            }
            symbols.push(inner.to_string());
            continue;
        }
        // Anything else is not a literal — bail out.
        return None;
    }

    Some(symbols)
}

fn parse_qw_arg_list(trimmed: &str) -> Option<Vec<String>> {
    let after_operator = trimmed.strip_prefix("qw")?;
    let delimiter = after_operator.chars().next()?;
    if delimiter.is_ascii_alphanumeric() || delimiter == '_' || delimiter.is_whitespace() {
        return None;
    }

    let closing = match delimiter {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        other => other,
    };

    let inner_start = "qw".len() + delimiter.len_utf8();
    let inner_end = trimmed.len().checked_sub(closing.len_utf8())?;
    if inner_start > inner_end || !trimmed.ends_with(closing) {
        return None;
    }

    let inner = &trimmed[inner_start..inner_end];
    Some(inner.split_whitespace().filter(|word| !word.is_empty()).map(str::to_string).collect())
}

/// Return true when `line` indicates a new statement boundary that should stop
/// the lookahead window for require-then-import matching.
///
/// We stop on `use`, another `require`, a `sub`, `package`, or `my` declaration
/// to avoid false positives across unrelated statement blocks.
fn is_statement_terminator(line: &str) -> bool {
    line.starts_with("use ")
        || line.starts_with("require ")
        || line.starts_with("sub ")
        || line.starts_with("package ")
        || line.starts_with("my ")
        || line.starts_with("our ")
        || line.starts_with("local ")
}

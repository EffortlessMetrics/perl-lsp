//! Single-line Perl import head parsing.
//!
//! This crate provides one narrow responsibility: parse a single source line
//! that starts with `use` or `require` and return the first import token with
//! stable byte offsets.

#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]

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
///
/// This is syntactic only: it distinguishes the default import form from an
/// explicitly empty list and from an explicit parenthesized import list.
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

impl<'a> ModuleImportHead<'a> {
    /// Returns the [`RequireForm`] for `require` statements, or `None` for `use` forms.
    #[must_use]
    pub fn require_form(&self) -> Option<RequireForm> {
        self.require_form
    }
}

/// Parse the leading import token of a single Perl source line.
///
/// Returns [`None`] when the line does not start with `use` or `require`
/// (after leading whitespace) or when no token is present after the keyword.
///
/// For `require "file.pm"` and `require 'file.pm'` forms, the surrounding
/// quotes are stripped and the inner path is returned as `token`.
///
/// # Examples
///
/// ```
/// use perl_module_import::{ModuleImportKind, parse_module_import_head};
///
/// let parsed = parse_module_import_head("use Foo::Bar;");
/// assert_eq!(parsed.map(|head| head.kind), Some(ModuleImportKind::Use));
/// assert_eq!(parsed.map(|head| head.token), Some("Foo::Bar"));
///
/// let parsed = parse_module_import_head("use parent 'Foo::Bar';");
/// assert_eq!(parsed.map(|head| head.kind), Some(ModuleImportKind::UseParent));
/// assert_eq!(parsed.map(|head| head.token), Some("parent"));
/// ```
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

    // Check for quoted file-path form: require "..." or require '...'
    let rest_trimmed = rest.trim_start();
    let quote_offset = rest.len() - rest_trimmed.len();

    if let Some(inner) = rest_trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"').or_else(|| s.split('"').next()))
        .or_else(|| {
            rest_trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\'').or_else(|| s.split('\'').next()))
        })
    {
        // Quoted form: token is the content inside the quotes, offsets point inside them
        let quote_char_len = 1usize; // single byte for ' or "
        let token_start = after_keyword + quote_offset + quote_char_len;
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

    // Bare module name form
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

    if let Some(after_open) = trimmed.strip_prefix('(') {
        if let Some(close_idx) = after_open.find(')') {
            if after_open[..close_idx].trim().is_empty() {
                let after_close = after_open[close_idx + 1..].trim_start();
                if after_close.is_empty()
                    || after_close.starts_with(';')
                    || after_close.starts_with('#')
                {
                    return ImportListForm::Empty;
                }
            }
        }

        return ImportListForm::Explicit;
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

#[cfg(test)]
mod tests {
    use super::{ModuleImportKind, parse_module_import_head};

    #[test]
    fn parses_use_statement_head() {
        let parsed = parse_module_import_head("use Foo::Bar;");

        assert!(parsed.is_some());
        if let Some(head) = parsed {
            assert_eq!(head.kind, ModuleImportKind::Use);
            assert_eq!(head.token, "Foo::Bar");
            assert_eq!(head.token_start, 4);
            assert_eq!(head.token_end, 12);
        }
    }

    #[test]
    fn parses_require_statement_head() {
        let parsed = parse_module_import_head("  require Foo::Bar;");

        assert!(parsed.is_some());
        if let Some(head) = parsed {
            assert_eq!(head.kind, ModuleImportKind::Require);
            assert_eq!(head.token, "Foo::Bar");
            assert_eq!(head.token_start, 10);
            assert_eq!(head.token_end, 18);
        }
    }

    #[test]
    fn classifies_parent_and_base_specializations() {
        let parent = parse_module_import_head("use parent qw(Foo::Bar);");
        let base = parse_module_import_head("use base 'Foo::Bar';");

        assert!(parent.is_some());
        if let Some(head) = parent {
            assert_eq!(head.kind, ModuleImportKind::UseParent);
            assert_eq!(head.token, "parent");
        }

        assert!(base.is_some());
        if let Some(head) = base {
            assert_eq!(head.kind, ModuleImportKind::UseBase);
            assert_eq!(head.token, "base");
        }
    }

    #[test]
    fn rejects_non_keyword_boundaries() {
        assert!(parse_module_import_head("user Foo::Bar;").is_none());
        assert!(parse_module_import_head("required Foo::Bar;").is_none());
    }

    #[test]
    fn rejects_missing_tokens() {
        assert!(parse_module_import_head("use ;").is_none());
        assert!(parse_module_import_head("require").is_none());
    }
}

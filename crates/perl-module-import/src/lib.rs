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

/// Parsed item from a `use Module ...` import list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseImportItem<'a> {
    /// A tag import item such as `:sys_wait_h`.
    Tag(&'a str),
    /// A plain symbol import item such as `WIFEXITED`.
    Symbol(&'a str),
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

/// Parse import items from a `use Module ...;` line.
///
/// Returns an empty vector for `use Module;` and `use Module ();`.
/// Returns `None` when the line does not parse as a `use` statement head.
#[must_use]
pub fn parse_use_import_items(line: &str) -> Option<Vec<UseImportItem<'_>>> {
    let head = parse_module_import_head(line)?;
    if head.kind == ModuleImportKind::Require {
        return None;
    }

    let mut rest = line[head.token_end..].trim_start();
    if rest.is_empty() || rest.starts_with(';') {
        return Some(Vec::new());
    }

    // Normalize the common `qw(...)` / `qw/.../` family into inner content.
    if let Some((inner, _consumed)) = parse_qw_literal(rest) {
        return Some(classify_import_tokens(inner.split_whitespace()));
    }

    // Parenthesized import list: use Module (...);
    if rest.starts_with('(')
        && let Some(close_idx) = rest.find(')')
    {
        rest = &rest[1..close_idx];
    }

    Some(classify_import_tokens(
        rest.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '(' | ')')),
    ))
}

/// Expand well-known import tags for common modules.
///
/// This is a static bootstrap map intended for editor resolution fallback.
#[must_use]
pub fn expand_known_import_tag(module: &str, tag: &str) -> &'static [&'static str] {
    match (module, tag) {
        ("POSIX", ":sys_wait_h") => &["WIFEXITED", "WEXITSTATUS", "WIFSIGNALED", "WTERMSIG"],
        ("POSIX", ":fcntl_h") => &["O_RDONLY", "O_WRONLY", "O_RDWR", "O_CREAT", "O_TRUNC"],
        ("POSIX", ":termios_h") => &["B9600", "CS8", "CLOCAL", "CREAD"],
        ("File::Find", ":find") => &["find", "finddepth"],
        ("Fcntl", ":seek") => &["SEEK_SET", "SEEK_CUR", "SEEK_END"],
        ("Fcntl", ":lock") => &["LOCK_SH", "LOCK_EX", "LOCK_NB", "LOCK_UN"],
        ("Encode", ":fallback") => &["FB_DEFAULT", "FB_CROAK", "FB_QUIET", "FB_PERLQQ"],
        _ => &[],
    }
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

fn parse_qw_literal(input: &str) -> Option<(&str, usize)> {
    let trimmed = input.trim_start();
    let ws = input.len().saturating_sub(trimmed.len());
    let body = trimmed.strip_prefix("qw")?;
    let mut chars = body.char_indices();
    let (delim_pos, open) = chars.next()?;
    if open.is_alphanumeric() || open.is_whitespace() {
        return None;
    }
    let open_abs = ws + 2 + delim_pos;
    let after_open_abs = open_abs + open.len_utf8();
    let close = matching_qw_delimiter(open);

    let search = &input[after_open_abs..];
    let close_rel = search.find(close)?;
    let close_abs = after_open_abs + close_rel;
    Some((&input[after_open_abs..close_abs], close_abs + close.len_utf8()))
}

fn matching_qw_delimiter(open: char) -> char {
    match open {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        _ => open,
    }
}

fn classify_import_tokens<'a>(tokens: impl Iterator<Item = &'a str>) -> Vec<UseImportItem<'a>> {
    let mut items = Vec::new();
    for token in tokens {
        let cleaned = strip_outer_quotes(token.trim());
        if cleaned.is_empty() || cleaned == "qw" {
            continue;
        }
        if cleaned.starts_with(':') {
            items.push(UseImportItem::Tag(cleaned));
        } else if !cleaned.starts_with('-') {
            items.push(UseImportItem::Symbol(cleaned));
        }
    }
    items
}

fn strip_outer_quotes(token: &str) -> &str {
    if token.len() >= 2 {
        let first = token.as_bytes()[0] as char;
        let last = token.as_bytes()[token.len() - 1] as char;
        if (first == '\'' && last == '\'') || (first == '"' && last == '"') {
            return &token[1..token.len() - 1];
        }
    }
    token
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
    use super::{
        ModuleImportKind, UseImportItem, expand_known_import_tag, parse_module_import_head,
        parse_use_import_items,
    };

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

    #[test]
    fn parses_tag_imports_from_qw_list() {
        let items = parse_use_import_items("use POSIX qw(:sys_wait_h :fcntl_h);");
        assert!(items.is_some());
        if let Some(items) = items {
            assert_eq!(
                items,
                vec![UseImportItem::Tag(":sys_wait_h"), UseImportItem::Tag(":fcntl_h")]
            );
        }
    }

    #[test]
    fn parses_mixed_tags_and_symbols() {
        let items = parse_use_import_items("use Encode qw(:fallback iso_8859_1);");
        assert!(items.is_some());
        if let Some(items) = items {
            assert_eq!(
                items,
                vec![UseImportItem::Tag(":fallback"), UseImportItem::Symbol("iso_8859_1")]
            );
        }
    }

    #[test]
    fn expands_common_known_tags() {
        let posix = expand_known_import_tag("POSIX", ":sys_wait_h");
        assert!(posix.contains(&"WIFEXITED"));
        assert!(posix.contains(&"WEXITSTATUS"));

        let fcntl = expand_known_import_tag("Fcntl", ":seek");
        assert_eq!(fcntl, &["SEEK_SET", "SEEK_CUR", "SEEK_END"]);
    }
}

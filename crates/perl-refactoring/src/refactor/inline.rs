//! Subroutine inlining for Perl code.
//!
//! Provides text-based subroutine inlining that replaces a call site with
//! the function's body after substituting formal parameters with the actual
//! arguments from the call.
//!
//! # Limitations
//!
//! This is a text-pattern implementation. It does not build a full AST and
//! therefore relies on heuristics for:
//! - Parameter extraction (assumes `my ($a, $b, …) = @_;` pattern)
//! - Return detection (counts `return` keywords)
//! - Recursion detection (looks for the sub name inside the body)
//! - Side-effect detection (looks for `print`, `warn`, `die`, `open`, `close`,
//!   `write`, `read`, `seek`, `sysread`, `syswrite`)
//!
//! Functions that do not follow these conventions may not be inlined correctly.
//! The safe defaults are to **reject** when uncertain (recursion, large body,
//! multiple returns) and to **warn** when side effects are detected.

use std::collections::HashMap;

/// Maximum number of body lines before the inliner rejects the function.
const MAX_BODY_LINES: usize = 50;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error type returned by subroutine inlining operations.
#[derive(Debug, Clone)]
pub enum InlineError {
    /// The target subroutine was not found in the provided source.
    SubNotFound {
        /// Name of the subroutine that was searched for.
        name: String,
    },
    /// The subroutine calls itself (direct recursion) and cannot be inlined.
    Recursive {
        /// Name of the recursive subroutine.
        name: String,
    },
    /// The subroutine body has too many lines to inline safely.
    TooLarge {
        /// Name of the subroutine.
        name: String,
        /// Actual line count of the body.
        line_count: usize,
    },
    /// The subroutine has more than one `return` statement, which requires
    /// control-flow restructuring beyond simple text substitution.
    MultipleReturns {
        /// Name of the subroutine.
        name: String,
        /// Number of `return` statements found.
        count: usize,
    },
    /// The call site expression could not be parsed (wrong argument count, etc.).
    CallSiteParseFailed {
        /// Diagnostic message.
        message: String,
    },
}

impl std::fmt::Display for InlineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InlineError::SubNotFound { name } => {
                write!(f, "subroutine '{}' not found in source", name)
            }
            InlineError::Recursive { name } => {
                write!(f, "cannot inline recursive subroutine '{}'", name)
            }
            InlineError::TooLarge { name, line_count } => {
                write!(
                    f,
                    "subroutine '{}' is too large to inline ({} lines, max {})",
                    name, line_count, MAX_BODY_LINES
                )
            }
            InlineError::MultipleReturns { name, count } => {
                write!(
                    f,
                    "subroutine '{}' has {} return points; only single-return subs can be inlined",
                    name, count
                )
            }
            InlineError::CallSiteParseFailed { message } => {
                write!(f, "failed to parse call site: {}", message)
            }
        }
    }
}

impl std::error::Error for InlineError {}

// ---------------------------------------------------------------------------
// Analysis result
// ---------------------------------------------------------------------------

/// The result of analysing a subroutine's inlineability.
#[derive(Debug, Clone)]
pub enum InlineAbility {
    /// The subroutine can be inlined.
    Ok {
        /// Formal parameter names (without sigils) in declaration order.
        params: Vec<String>,
        /// The body text, stripped of the parameter-extraction line.
        body: String,
        /// Whether the body contains operations with observable side effects.
        has_side_effects: bool,
    },
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse whether a named subroutine can be inlined.
///
/// Returns `Ok(InlineAbility::Ok { … })` when safe to inline, or an
/// [`InlineError`] when the subroutine must not be inlined.
pub fn analyze_sub_for_inlining(
    source: &str,
    sub_name: &str,
) -> Result<InlineAbility, InlineError> {
    let parsed = parse_sub_definition(source, sub_name)
        .ok_or_else(|| InlineError::SubNotFound { name: sub_name.to_string() })?;

    // Recursion check
    if body_calls_self(&parsed.body, sub_name) {
        return Err(InlineError::Recursive { name: sub_name.to_string() });
    }

    // Size check
    let body_line_count = parsed.body.lines().count();
    if body_line_count > MAX_BODY_LINES {
        return Err(InlineError::TooLarge {
            name: sub_name.to_string(),
            line_count: body_line_count,
        });
    }

    // Multiple-return check
    let return_count = count_return_statements(&parsed.body);
    if return_count > 1 {
        return Err(InlineError::MultipleReturns {
            name: sub_name.to_string(),
            count: return_count,
        });
    }

    let side_effects = has_side_effects(&parsed.body);

    Ok(InlineAbility::Ok {
        params: parsed.params,
        body: parsed.body,
        has_side_effects: side_effects,
    })
}

/// Text-based Perl subroutine inliner.
///
/// Create one instance per source file and call [`inline_call`] (or its
/// variants) to produce the inlined expression text.
pub struct SubInliner {
    source: String,
}

impl SubInliner {
    /// Create a new inliner from Perl source text.
    pub fn new(source: &str) -> Self {
        Self { source: source.to_string() }
    }

    /// Inline a single call to `sub_name`.
    ///
    /// `call_expr` is the full call expression string, e.g. `"add(3, 4)"`.
    ///
    /// Returns the replacement text (the inlined expression), or an
    /// [`InlineError`] if the subroutine cannot be inlined.
    pub fn inline_call(&self, sub_name: &str, call_expr: &str) -> Result<String, InlineError> {
        let (inlined, _warnings) = self.inline_call_inner(sub_name, call_expr, &[])?;
        Ok(inlined)
    }

    /// Like [`inline_call`] but also returns any warnings (e.g. side effects).
    pub fn inline_call_with_warnings(
        &self,
        sub_name: &str,
        call_expr: &str,
    ) -> Result<(String, Vec<String>), InlineError> {
        self.inline_call_inner(sub_name, call_expr, &[])
    }

    /// Like [`inline_call`] but accepts a list of variable names that already
    /// exist in the outer scope, so collisions can be detected and renamed.
    pub fn inline_call_with_outer_vars(
        &self,
        sub_name: &str,
        call_expr: &str,
        outer_vars: &[String],
    ) -> Result<String, InlineError> {
        let (inlined, _warnings) = self.inline_call_inner(sub_name, call_expr, outer_vars)?;
        Ok(inlined)
    }

    // ------------------------------------------------------------------
    // Internal
    // ------------------------------------------------------------------

    fn inline_call_inner(
        &self,
        sub_name: &str,
        call_expr: &str,
        outer_vars: &[String],
    ) -> Result<(String, Vec<String>), InlineError> {
        let ability = analyze_sub_for_inlining(&self.source, sub_name)?;
        let InlineAbility::Ok { params, body, has_side_effects } = ability;

        let mut warnings = Vec::new();
        if has_side_effects {
            warnings.push(format!(
                "subroutine '{}' contains side effects (print/warn/die/I/O); \
                 inlining preserves them but may change semantics",
                sub_name
            ));
        }

        // Extract arguments from call expression
        let args = extract_call_args(call_expr, sub_name)?;

        // Build substitution map: param_name -> arg_text
        let mut sub_map: HashMap<String, String> = HashMap::new();
        for (i, param) in params.iter().enumerate() {
            let arg = args.get(i).cloned().unwrap_or_default();
            sub_map.insert(param.clone(), arg);
        }

        // Rename local variables to avoid outer-scope collisions
        let body = rename_collisions(&body, outer_vars);

        // Substitute parameters in body
        let substituted = substitute_params(&body, &sub_map);

        // Extract the return expression from the body
        let expr = extract_return_expr(&substituted);

        Ok((expr, warnings))
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parsed representation of a subroutine definition extracted from source.
struct ParsedSub {
    /// Formal parameter names (sigils stripped).
    params: Vec<String>,
    /// Body text with the parameter line removed.
    body: String,
}

/// Extract a subroutine definition from source text.
///
/// Recognises the pattern:
/// ```text
/// sub NAME {
///     …body…
/// }
/// ```
///
/// Returns `None` if the pattern is not found.
fn parse_sub_definition(source: &str, sub_name: &str) -> Option<ParsedSub> {
    let start = find_sub_start(source, sub_name)?;

    // Find the matching closing brace
    let body_start = source[start..].find('{').map(|i| start + i + 1)?;
    let body_raw = extract_balanced_braces(source, body_start)?;

    // Extract parameter line: "my ($a, $b) = @_;"
    let (params, body_without_params) = extract_params_line(&body_raw);

    Some(ParsedSub { params, body: body_without_params })
}

/// Find the byte offset of `sub NAME` followed by `{` in `source`.
fn find_sub_start(source: &str, sub_name: &str) -> Option<usize> {
    let mut pos = 0;
    while pos < source.len() {
        let rest = &source[pos..];
        if let Some(idx) = rest.find("sub ") {
            let after_sub = &rest[idx + 4..];
            let trimmed = after_sub.trim_start();
            if let Some(after_name) = trimmed.strip_prefix(sub_name) {
                // Verify it's a word boundary (not "sub foobar" when looking for "foo")
                let boundary_ok =
                    after_name.chars().next().is_none_or(|c| !c.is_alphanumeric() && c != '_');
                if boundary_ok && after_name.trim_start().starts_with('{') {
                    return Some(pos + idx);
                }
            }
            pos += idx + 4;
        } else {
            break;
        }
    }
    None
}

/// Extract the text between a matching pair of braces starting at `open_pos`
/// (the position AFTER the opening `{`).
fn extract_balanced_braces(source: &str, open_pos: usize) -> Option<String> {
    let mut depth = 1usize;
    let chars: Vec<char> = source[open_pos..].chars().collect();
    let mut end = 0;
    let mut found = false;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    found = true;
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    if !found {
        return None;
    }
    Some(chars[..end].iter().collect())
}

/// Parse out the Perl parameter-extraction line `my ($a, $b) = @_;` from the
/// top of the body, returning (params, remaining_body).
///
/// If no such line is found, returns ([], original_body).
fn extract_params_line(body: &str) -> (Vec<String>, String) {
    for (i, line) in body.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("my (") && trimmed.contains("= @_") {
            let params = parse_param_names(trimmed);
            let remaining: String = body
                .lines()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, l)| l)
                .collect::<Vec<_>>()
                .join("\n");
            return (params, remaining);
        }
    }
    (vec![], body.to_string())
}

/// Extract parameter names from `my ($a, $b) = @_;`, returning bare names
/// without sigils.
fn parse_param_names(line: &str) -> Vec<String> {
    let open = match line.find('(') {
        Some(i) => i,
        None => return vec![],
    };
    let close = match line.rfind(')') {
        Some(i) => i,
        None => return vec![],
    };
    if close <= open {
        return vec![];
    }
    let inner = &line[open + 1..close];
    inner
        .split(',')
        .map(|s| s.trim().trim_start_matches(['$', '@', '%']).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Body analysis helpers
// ---------------------------------------------------------------------------

/// Count occurrences of the `return` keyword as a standalone token in `body`.
///
/// Skips occurrences inside single- or double-quoted string literals so that
/// `my $msg = "will return a value";` is not counted as a return statement.
fn count_return_statements(body: &str) -> usize {
    let mut count = 0usize;
    let mut pos = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let bytes = body.as_bytes();

    while pos < body.len() {
        let b = bytes[pos];

        // Track string context — handle backslash escapes
        match b {
            b'\\' if in_single_quote || in_double_quote => {
                // Skip escaped character
                pos += 2;
                continue;
            }
            b'\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                pos += 1;
                continue;
            }
            b'"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                pos += 1;
                continue;
            }
            _ => {}
        }

        // Only count `return` tokens outside string literals
        if !in_single_quote && !in_double_quote {
            let rest = &body[pos..];
            if rest.starts_with("return") {
                // Check character before
                let before_ok = if pos > 0 {
                    let prev = bytes[pos - 1];
                    !prev.is_ascii_alphanumeric() && prev != b'_'
                } else {
                    true
                };
                // Check character after
                let after_pos = pos + 6;
                let after_ok = if after_pos < body.len() {
                    let next = bytes[after_pos];
                    !next.is_ascii_alphanumeric() && next != b'_'
                } else {
                    true
                };
                if before_ok && after_ok {
                    count += 1;
                }
                pos += 6;
                continue;
            }
        }

        pos += body[pos..].chars().next().map_or(1, |c| c.len_utf8());
    }
    count
}

/// Check whether the body contains observable side-effect operations.
fn has_side_effects(body: &str) -> bool {
    const SIDE_EFFECT_KEYWORDS: &[&str] = &[
        "print ", "warn ", "die ", "open ", "close ", "read ", "write ", "seek ", "sysread",
        "syswrite", "printf", "say ",
    ];
    SIDE_EFFECT_KEYWORDS.iter().any(|kw| body.contains(kw))
}

/// Check whether the body calls itself (direct recursion).
///
/// Skips occurrences of `sub_name(` that appear inside string literals to
/// avoid false-positive recursion detection when the sub name is merely
/// mentioned in a string (e.g. `my $msg = "add(1,2) adds two numbers"`).
fn body_calls_self(body: &str, sub_name: &str) -> bool {
    let call_pattern = format!("{}(", sub_name);
    let bytes = body.as_bytes();
    let mut pos = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while pos < body.len() {
        let b = bytes[pos];
        match b {
            b'\\' if in_single_quote || in_double_quote => {
                pos += 2;
                continue;
            }
            b'\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                pos += 1;
                continue;
            }
            b'"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                pos += 1;
                continue;
            }
            _ => {}
        }
        if !in_single_quote && !in_double_quote && body[pos..].starts_with(&call_pattern) {
            return true;
        }
        pos += body[pos..].chars().next().map_or(1, |c| c.len_utf8());
    }
    false
}

// ---------------------------------------------------------------------------
// Argument extraction
// ---------------------------------------------------------------------------

/// Extract the argument list from a call expression like `foo(1, 2, "bar")`.
fn extract_call_args(call_expr: &str, sub_name: &str) -> Result<Vec<String>, InlineError> {
    let sub_pos = call_expr.find(sub_name).ok_or_else(|| InlineError::CallSiteParseFailed {
        message: format!("call expression does not contain sub name '{}'", sub_name),
    })?;

    let after_name_pos = sub_pos + sub_name.len();
    let rest = call_expr[after_name_pos..].trim_start();
    if !rest.starts_with('(') {
        // Bare call with no parens — no arguments
        return Ok(vec![]);
    }

    // Find '(' absolute position
    let paren_offset = call_expr[after_name_pos..].find('(').unwrap_or(0);
    let open_abs = after_name_pos + paren_offset;

    let close_abs = find_matching_paren(call_expr, open_abs).ok_or_else(|| {
        InlineError::CallSiteParseFailed {
            message: "unmatched parenthesis in call expression".to_string(),
        }
    })?;

    let args_str = &call_expr[open_abs + 1..close_abs];
    if args_str.trim().is_empty() {
        return Ok(vec![]);
    }

    Ok(split_args(args_str))
}

/// Find the matching `)` for the `(` at byte position `open` in `s`.
fn find_matching_paren(s: &str, open: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(open) != Some(&b'(') {
        return None;
    }
    let mut depth = 0usize;
    for (i, &b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Split a comma-separated argument string, respecting nested parens and quotes.
fn split_args(args_str: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let chars: Vec<char> = args_str.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if in_double_quote || in_single_quote => {
                current.push(c);
                i += 1;
                if i < chars.len() {
                    current.push(chars[i]);
                }
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(c);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(c);
            }
            '(' | '[' | '{' if !in_single_quote && !in_double_quote => {
                depth += 1;
                current.push(c);
            }
            ')' | ']' | '}' if !in_single_quote && !in_double_quote => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if depth == 0 && !in_single_quote && !in_double_quote => {
                result.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(c),
        }
        i += 1;
    }

    if !current.trim().is_empty() {
        result.push(current.trim().to_string());
    }

    result
}

// ---------------------------------------------------------------------------
// Body transformation
// ---------------------------------------------------------------------------

/// Replace occurrences of `$param_name` in `body` with the corresponding
/// argument text.
///
/// Uses word-boundary-aware replacement to avoid corrupting longer variable
/// names that share a prefix with a parameter (e.g. replacing `$price` must
/// not corrupt `$price_adjusted`).  Sorted by descending name length so that
/// longer names are never shadowed by shorter prefix matches.
fn substitute_params(body: &str, sub_map: &HashMap<String, String>) -> String {
    let mut result = body.to_string();
    let mut pairs: Vec<(&String, &String)> = sub_map.iter().collect();
    pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (param, arg) in pairs {
        let var = format!("${}", param);
        result = replace_whole_var(&result, &var, arg);
    }
    result
}

/// Rename local variable declarations in `body` that collide with names in
/// `outer_vars`, appending `_inlined` to the bare name.
fn rename_collisions(body: &str, outer_vars: &[String]) -> String {
    let mut result = body.to_string();
    for outer in outer_vars {
        let bare = outer.trim_start_matches(['$', '@', '%']);
        let my_decl = format!("my ${}", bare);
        if result.contains(&my_decl) {
            let renamed_bare = format!("{}_inlined", bare);
            let renamed_decl = format!("my ${}", renamed_bare);
            // Replace the declaration first — use word-boundary-aware replacement so
            // that "my $x" does not corrupt "my $x_count" when the outer var is "$x".
            result = replace_whole_var(&result, &my_decl, &renamed_decl);
            // Then replace all uses of $bare that are not the new $bare_inlined
            // We do this by replacing "$bare" with "$bare_inlined" across the body,
            // but we already renamed the declaration above so the decl is safe.
            let var = format!("${}", bare);
            let renamed_var = format!("${}", renamed_bare);
            // Only replace if not already part of a longer name
            result = replace_whole_var(&result, &var, &renamed_var);
        }
    }
    result
}

/// Replace occurrences of `var` in `text` that are complete variable
/// references (not a prefix of a longer variable name).
fn replace_whole_var(text: &str, var: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut pos = 0;
    while pos < text.len() {
        if text[pos..].starts_with(var) {
            let after = pos + var.len();
            let next_is_alphanum =
                text[after..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '_');
            if !next_is_alphanum {
                result.push_str(replacement);
                pos = after;
                continue;
            }
        }
        let c = text[pos..].chars().next().unwrap_or('\0');
        result.push(c);
        pos += c.len_utf8();
    }
    result
}

/// Extract the expression value from a body containing a single `return`.
///
/// Returns `(expr)` for `return expr;`, or the trimmed body if no `return`.
fn extract_return_expr(body: &str) -> String {
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("return ") {
            let expr = trimmed.trim_start_matches("return ").trim_end_matches(';').trim();
            return format!("({})", expr);
        }
    }
    body.trim().to_string()
}

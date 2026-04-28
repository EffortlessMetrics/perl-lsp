//! Perl regex validation and analysis
//!
//! This module provides tools to validate Perl regular expressions
//! and detect potential security or performance issues like catastrophic backtracking.

use thiserror::Error;

/// Error type for Perl regex validation failures.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum RegexError {
    /// Syntax error at a specific byte offset in the regex pattern.
    #[error("{message} at offset {offset}")]
    Syntax {
        /// Human-readable description of the syntax issue.
        message: String,
        /// Byte offset where the error was detected.
        offset: usize,
    },
}

impl RegexError {
    /// Create a new syntax error with a message and byte offset.
    pub fn syntax(message: impl Into<String>, offset: usize) -> Self {
        RegexError::Syntax { message: message.into(), offset }
    }
}

/// Validator for Perl regular expressions to prevent security and performance issues
pub struct RegexValidator {
    max_nesting: usize,
    max_unicode_properties: usize,
}

impl Default for RegexValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexValidator {
    /// Create a new validator with default safety limits
    pub fn new() -> Self {
        Self {
            // Default limits from issue #461
            max_nesting: 10,
            // Limit from issue #460
            max_unicode_properties: 50,
        }
    }

    /// Validate a regex pattern for potential performance or security risks
    pub fn validate(&self, pattern: &str, start_pos: usize) -> Result<(), RegexError> {
        self.check_complexity(pattern, start_pos)
    }

    /// Check if the pattern contains embedded code constructs (?{...}) or (??{...})
    pub fn detects_code_execution(&self, pattern: &str) -> bool {
        let bytes = pattern.as_bytes();
        let mut i = 0;
        let len = bytes.len();
        while i < len {
            let ch = bytes[i];
            if ch == b'\\' {
                i += 2; // skip escaped
                continue;
            }
            if ch == b'[' {
                // Skip character class content so literals like [(?{] are not
                // misclassified as embedded code execution.
                i += 1;
                while i < len {
                    let class_ch = bytes[i];
                    if class_ch == b'\\' {
                        i += 2; // skip escaped char inside class
                    } else if class_ch == b']' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
            if ch == b'(' {
                if i + 1 < len && bytes[i + 1] == b'?' {
                    i += 2; // consume '(' and '?'
                    // Check for { or ?{
                    if i < len {
                        if bytes[i] == b'{' {
                            return true; // (?{
                        } else if bytes[i] == b'?' {
                            if i + 1 < len && bytes[i + 1] == b'{' {
                                return true; // (??{
                            }
                        }
                    }
                    continue;
                }
            }
            i += 1;
        }
        false
    }

    /// Check for nested quantifiers that can cause catastrophic backtracking
    /// e.g. (a+)+, (a*)*, (a?)*
    pub fn detect_nested_quantifiers(&self, pattern: &str) -> bool {
        // This is a heuristic check for nested quantifiers
        // It looks for a quantifier character following a group that ends with a quantifier
        // e.g. ")+" in "...)+"
        // Real implementation would need a full regex parser, but this heuristic
        // covers common cases like (a+)+

        let bytes = pattern.as_bytes();
        let mut i = 0;
        let len = bytes.len();
        let mut group_stack = Vec::new();

        // Track the last significant character index and its type
        // Type: 0=other, 1=quantifier, 2=group_end
        let mut last_type = 0;

        while i < len {
            let ch = bytes[i];
            match ch {
                b'\\' => {
                    i += 2; // skip escaped
                    last_type = 0;
                    continue;
                }
                b'(' => {
                    // Check if non-capturing or other special group
                    if i + 1 < len && bytes[i + 1] == b'?' {
                        i += 2; // consume '(' and '?'
                        // Skip group-type specifier so it doesn't reach the
                        // quantifier match arm (mirrors check_complexity logic)
                        if i < len
                            && matches!(
                                bytes[i],
                                b':' | b'=' | b'!' | b'<' | b'>' | b'|' | b'P' | b'#'
                            )
                        {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                    group_stack.push(false); // false = no quantifier inside yet
                    last_type = 0;
                    continue;
                }
                b')' => {
                    if let Some(has_quantifier) = group_stack.pop() {
                        if has_quantifier {
                            last_type = 2; // group end with internal quantifier
                        } else {
                            last_type = 0;
                        }
                    }
                }
                b'+' | b'*' | b'?' | b'{' => {
                    // If we just closed a group that had a quantifier inside,
                    // and now we see another quantifier, that's a nested quantifier!
                    if last_type == 2 {
                        // Check if it's really a quantifier or literal {
                        if ch == b'{' {
                            // Only count as quantifier if it looks like {n} or {n,m}.
                            let mut peek_i = i + 1;
                            if Self::is_brace_quantifier(bytes, &mut peek_i) {
                                return true;
                            } else {
                                // Important fix: If it's not a brace quantifier, do NOT
                                // advance i using peek_i. It's just a literal '{'
                                last_type = 0;
                                i += 1;
                                continue;
                            }
                        } else {
                            return true;
                        }
                    }

                    // Mark current group as having a quantifier
                    if let Some(last) = group_stack.last_mut() {
                        *last = true;
                    }
                    last_type = 1;
                }
                _ => {
                    last_type = 0;
                }
            }
            i += 1;
        }
        false
    }

    fn is_brace_quantifier(bytes: &[u8], i: &mut usize) -> bool {
        // Require at least one digit after '{'
        let mut has_digit = false;
        let mut has_comma = false;
        let len = bytes.len();

        while *i < len {
            let ch = bytes[*i];
            *i += 1;
            if ch.is_ascii_digit() {
                has_digit = true;
            } else if ch == b',' && !has_comma {
                has_comma = true;
            } else if ch == b'}' && has_digit {
                return true;
            } else {
                break;
            }
        }

        false // Should have returned true at '}' if valid
    }

    fn check_complexity(&self, pattern: &str, start_pos: usize) -> Result<(), RegexError> {
        // NOTE: Nested quantifier detection (detect_nested_quantifiers) is intentionally
        // NOT called here. The heuristic produces too many false positives on valid Perl
        // patterns such as (?:/\.)+, (\w+)*, (?:pattern)+. Callers that want an advisory
        // check can invoke detect_nested_quantifiers() directly and surface the result
        // as a non-fatal diagnostic.

        let bytes = pattern.as_bytes();
        let mut i = 0;
        let len = bytes.len();

        // Stack stores the type of the current group
        let mut stack: Vec<GroupType> = Vec::new();
        let mut unicode_property_count = 0;

        while i < len {
            let ch = bytes[i];
            match ch {
                b'\\' => {
                    // Check for escaped character
                    if i + 1 < len {
                        let next_char = bytes[i + 1];
                        match next_char {
                            b'p' | b'P' => {
                                // Unicode property start \p or \P
                                // We consume the 'p'/'P'
                                i += 2;

                                // Check if it's followed by {
                                if i < len && bytes[i] == b'{' {
                                    unicode_property_count += 1;
                                    if unicode_property_count > self.max_unicode_properties {
                                        return Err(RegexError::syntax(
                                            "Too many Unicode properties in regex (max 50)",
                                            start_pos + i - 2, // approximate original idx
                                        ));
                                    }
                                }
                                continue;
                            }
                            _ => {
                                // Just skip other escaped chars
                                i += 2;
                                continue;
                            }
                        }
                    }
                }
                b'[' => {
                    // Need to skip character classes
                    i += 1;
                    while i < len {
                        if bytes[i] == b'\\' {
                            i += 2;
                        } else if bytes[i] == b']' {
                            break;
                        } else {
                            i += 1;
                        }
                    }
                }
                b'(' => {
                    let mut group_type = GroupType::Normal;

                    // Check for extension syntax (?...)
                    if i + 1 < len && bytes[i + 1] == b'?' {
                        i += 2; // consume '(' and '?'

                        // Check for < (lookbehind or named capture)
                        if i < len && bytes[i] == b'<' {
                            i += 1; // consume <

                            // Check for = or ! (lookbehind)
                            if i < len && (bytes[i] == b'=' || bytes[i] == b'!') {
                                i += 1; // consume = or !
                                group_type = GroupType::Lookbehind;
                            }
                            // Otherwise it's likely a named capture (?<name>...) or condition (?<...)
                            // which we treat as a normal group
                        } else if i < len && bytes[i] == b'|' {
                            i += 1; // consume |
                            group_type = GroupType::BranchReset { branch_count: 1 };
                        }
                    } else {
                        i += 1;
                    }

                    match group_type {
                        GroupType::Lookbehind => {
                            // Calculate current lookbehind depth
                            let lookbehind_depth =
                                stack.iter().filter(|g| matches!(g, GroupType::Lookbehind)).count();
                            if lookbehind_depth >= self.max_nesting {
                                return Err(RegexError::syntax(
                                    "Regex lookbehind nesting too deep",
                                    start_pos + i - 1, // rough idx
                                ));
                            }
                        }
                        GroupType::BranchReset { .. } => {
                            // Calculate current branch reset nesting
                            let reset_depth = stack
                                .iter()
                                .filter(|g| matches!(g, GroupType::BranchReset { .. }))
                                .count();
                            if reset_depth >= self.max_nesting {
                                // Use same nesting limit for now
                                return Err(RegexError::syntax(
                                    "Regex branch reset nesting too deep",
                                    start_pos + i - 1,
                                ));
                            }
                        }
                        _ => {}
                    }
                    stack.push(group_type);
                    continue;
                }
                b'|' => {
                    // Check if we are in a branch reset group
                    if let Some(GroupType::BranchReset { branch_count }) = stack.last_mut() {
                        *branch_count += 1;
                        if *branch_count > 50 {
                            // Max 50 branches
                            return Err(RegexError::syntax(
                                "Too many branches in branch reset group (max 50)",
                                start_pos + i,
                            ));
                        }
                    }
                }
                b')' => {
                    // Pop group from stack
                    stack.pop();
                }
                _ => {}
            }
            i += 1;
        }

        Ok(())
    }
}

enum GroupType {
    Normal,
    Lookbehind,
    BranchReset { branch_count: usize },
}

/// A named capture group extracted from a regex pattern.

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureGroup {
    /// The capture group name from `(?<name>...)`.
    pub name: String,
    /// One-based capture index (counting all capturing groups left to right).
    pub index: usize,
    /// The sub-pattern inside the capture group.
    pub pattern: String,
}

/// Analysis utilities for Perl regex patterns: capture extraction and hover text.
pub struct RegexAnalyzer;

impl RegexAnalyzer {
    /// Extract all named capture groups from a Perl regex pattern.
    ///
    /// Scans the pattern for `(?<name>...)` groups and returns them in left-to-right
    /// order. Non-capturing groups (`(?:...)`), lookaheads, and lookbehinds do not
    /// increment the capture index. Escaped parentheses (`\(`) are skipped.
    ///
    /// # Example
    /// ```
    /// use perl_regex::RegexAnalyzer;
    /// let caps = RegexAnalyzer::extract_named_captures("(?<year>\\d{4})-(?<month>\\d{2})");
    /// assert_eq!(caps.len(), 2);
    /// assert_eq!(caps[0].name, "year");
    /// assert_eq!(caps[0].index, 1);
    /// ```
    pub fn extract_named_captures(pattern: &str) -> Vec<CaptureGroup> {
        let mut result = Vec::new();
        let mut capture_index = 0usize;
        let bytes = pattern.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Skip escaped characters.
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }

            // Skip character classes [...] entirely.
            if bytes[i] == b'[' {
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == b']' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }

            if bytes[i] == b'(' {
                i += 1;

                // Determine the group kind.
                if i < len && bytes[i] == b'?' {
                    i += 1; // consume '?'

                    if i < len && bytes[i] == b'<' {
                        i += 1; // consume '<'

                        // Lookbehind: (?<= or (?<!  — not a capture.
                        if i < len && (bytes[i] == b'=' || bytes[i] == b'!') {
                            i += 1;
                            continue;
                        }

                        if let Some((name, next_pos)) =
                            parse_named_capture_name_from(bytes, i, b'>')
                        {
                            capture_index += 1;
                            i = next_pos;

                            // Collect the sub-pattern up to the matching ')'.
                            let pattern_start = i;
                            let mut depth = 1usize;
                            while i < len && depth > 0 {
                                if bytes[i] == b'\\' {
                                    i += 2;
                                    continue;
                                }
                                if bytes[i] == b'[' {
                                    i += 1;
                                    while i < len {
                                        if bytes[i] == b'\\' {
                                            i += 2;
                                        } else if bytes[i] == b']' {
                                            i += 1;
                                            break;
                                        } else {
                                            i += 1;
                                        }
                                    }
                                    continue;
                                }
                                if bytes[i] == b'(' {
                                    depth += 1;
                                } else if bytes[i] == b')' {
                                    depth -= 1;
                                }
                                i += 1;
                            }
                            // The ')' was consumed above; sub-pattern ends before it.
                            let sub: String = if i > 0 && pattern_start < i - 1 {
                                // Since we parsed byte by byte matching ASCII mostly,
                                // the slice boundaries should be valid UTF-8.
                                // If not, String::from_utf8_lossy covers it safely.
                                String::from_utf8_lossy(&bytes[pattern_start..i - 1]).into_owned()
                            } else {
                                String::new()
                            };

                            result.push(CaptureGroup { name, index: capture_index, pattern: sub });
                            continue;
                        }
                    } else if i < len && bytes[i] == b'\'' {
                        if let Some((name, next_pos)) =
                            parse_named_capture_name(bytes, i, b'\'', b'\'')
                        {
                            capture_index += 1;
                            i = next_pos;

                            // Collect the sub-pattern up to the matching ')'.
                            let pattern_start = i;
                            let mut depth = 1usize;
                            while i < len && depth > 0 {
                                if bytes[i] == b'\\' {
                                    i += 2;
                                    continue;
                                }
                                if bytes[i] == b'[' {
                                    i += 1;
                                    while i < len {
                                        if bytes[i] == b'\\' {
                                            i += 2;
                                        } else if bytes[i] == b']' {
                                            i += 1;
                                            break;
                                        } else {
                                            i += 1;
                                        }
                                    }
                                    continue;
                                }
                                if bytes[i] == b'(' {
                                    depth += 1;
                                } else if bytes[i] == b')' {
                                    depth -= 1;
                                }
                                i += 1;
                            }
                            // The ')' was consumed above; sub-pattern ends before it.
                            let sub: String = if i > 0 && pattern_start < i - 1 {
                                String::from_utf8_lossy(&bytes[pattern_start..i - 1]).into_owned()
                            } else {
                                String::new()
                            };

                            result.push(CaptureGroup { name, index: capture_index, pattern: sub });
                            continue;
                        }
                    } else if i < len
                        && matches!(bytes[i], b':' | b'=' | b'!' | b'>' | b'|' | b'P' | b'#')
                    {
                        // Non-capturing group: (?:...), (?=...), (?!...), (?|...), etc.
                        // Does not increment capture_index; just move on (fall through to
                        // normal scanning — the loop will handle nested parens naturally).
                        continue;
                    }
                    // Any other (?...) — treat as non-capturing for index purposes.
                    continue;
                }

                // Plain capturing group `(...)`.
                capture_index += 1;
                continue;
            }

            i += 1;
        }

        result
    }

    /// Generate hover text for a Perl regex pattern and its modifiers.
    ///
    /// Summarises the named capture groups and explains the meaning of each
    /// modifier flag (`i`, `m`, `s`, `x`, `g`).
    ///
    /// # Example
    /// ```
    /// use perl_regex::RegexAnalyzer;
    /// let text = RegexAnalyzer::hover_text_for_regex("(?<id>\\d+)", "i");
    /// assert!(text.contains("id"));
    /// assert!(text.contains("case"));
    /// ```
    pub fn hover_text_for_regex(pattern: &str, modifiers: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        if !pattern.is_empty() {
            parts.push(format!("Regex: `{pattern}`"));
        }

        // Named captures section.
        let captures = Self::extract_named_captures(pattern);
        if !captures.is_empty() {
            parts.push("Named captures:".to_string());
            for cap in &captures {
                parts.push(format!(
                    "  ${{{name}}} (capture {index}): `{pat}`",
                    name = cap.name,
                    index = cap.index,
                    pat = cap.pattern,
                ));
            }
        }

        // Modifier explanations.
        let modifier_notes: Vec<&str> = modifiers
            .chars()
            .filter_map(|m| match m {
                'i' => Some("case-insensitive matching"),
                'm' => Some("multiline mode: ^ and $ match line boundaries"),
                's' => Some("single-line mode: dot matches newline"),
                'x' => Some("extended mode: whitespace and comments allowed"),
                'g' => Some("global: match all occurrences"),
                _ => None,
            })
            .collect();

        if !modifier_notes.is_empty() {
            parts.push("Modifiers:".to_string());
            for note in modifier_notes {
                parts.push(format!("  {note}"));
            }
        }

        parts.join("\n")
    }
}

fn parse_named_capture_name(
    bytes: &[u8],
    pos: usize,
    open_delim: u8,
    close_delim: u8,
) -> Option<(String, usize)> {
    if pos >= bytes.len() || bytes[pos] != open_delim {
        return None;
    }

    let mut i = pos + 1;
    let name_start = i;
    while i < bytes.len() && bytes[i] != close_delim {
        i += 1;
    }

    if i == name_start || i >= bytes.len() {
        return None;
    }

    let name = String::from_utf8_lossy(&bytes[name_start..i]).into_owned();
    Some((name, i + 1))
}

fn parse_named_capture_name_from(
    bytes: &[u8],
    start: usize,
    close_delim: u8,
) -> Option<(String, usize)> {
    if start >= bytes.len() {
        return None;
    }

    let mut i = start;
    while i < bytes.len() && bytes[i] != close_delim {
        i += 1;
    }

    if i == start || i >= bytes.len() {
        return None;
    }

    let name = String::from_utf8_lossy(&bytes[start..i]).into_owned();
    Some((name, i + 1))
}

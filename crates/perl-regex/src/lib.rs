//! Perl regex validation and analysis
//!
//! This module provides tools to validate Perl regular expressions
//! and detect potential security or performance issues like catastrophic backtracking.

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum RegexError {
    #[error("{message} at offset {offset}")]
    Syntax {
        message: String,
        offset: usize,
    },
}

impl RegexError {
    pub fn syntax(message: impl Into<String>, offset: usize) -> Self {
        RegexError::Syntax {
            message: message.into(),
            offset,
        }
    }
}

/// Validator for Perl regular expressions to prevent security and performance issues
pub struct RegexValidator {
    max_nesting: usize,
    max_unicode_properties: usize,
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
        let mut chars = pattern.char_indices().peekable();
        while let Some((_, ch)) = chars.next() {
            if ch == '\\' {
                chars.next(); // skip escaped
                continue;
            }
            if ch == '(' {
                if let Some((_, '?')) = chars.peek() {
                    chars.next(); // consume ?
                    // Check for { or ?{
                    if let Some((_, next)) = chars.peek() {
                        if *next == '{' {
                            return true; // (?{
                        } else if *next == '?' {
                            chars.next(); // consume second ?
                            if let Some((_, '{')) = chars.peek() {
                                return true; // (??{
                            }
                        }
                    }
                }
            }
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

        let mut chars = pattern.char_indices().peekable();
        let mut group_stack = Vec::new();

        // Track the last significant character index and its type
        // Type: 0=other, 1=quantifier, 2=group_end
        let mut last_type = 0;

        while let Some((_, ch)) = chars.next() {
            match ch {
                '\\' => {
                    chars.next(); // skip escaped
                    last_type = 0;
                }
                '(' => {
                    // Check if non-capturing or other special group
                    if let Some((_, '?')) = chars.peek() {
                        // Special group, might be safe or not
                        // For now we just track it as a group start
                    }
                    group_stack.push(false); // false = no quantifier inside yet
                    last_type = 0;
                }
                ')' => {
                    if let Some(has_quantifier) = group_stack.pop() {
                        if has_quantifier {
                            last_type = 2; // group end with internal quantifier
                        } else {
                            last_type = 0;
                        }
                    }
                }
                '+' | '*' | '?' | '{' => {
                    // If we just closed a group that had a quantifier inside,
                    // and now we see another quantifier, that's a nested quantifier!
                    if last_type == 2 {
                        // Check if it's really a quantifier or literal {
                        if ch == '{' {
                            // Only count as quantifier if it looks like {n} or {n,m}
                            // peek ahead... (simplified for now)
                            return true; // Assume { is quantifier for safety heuristic
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
        }
        false
    }

    fn check_complexity(&self, pattern: &str, start_pos: usize) -> Result<(), RegexError> {
        if self.detect_nested_quantifiers(pattern) {
            return Err(RegexError::syntax(
                "Potential catastrophic backtracking detected (nested quantifiers)",
                start_pos,
            ));
        }

        let mut chars = pattern.char_indices().peekable();
        // Stack stores the type of the current group
        let mut stack: Vec<GroupType> = Vec::new();
        let mut unicode_property_count = 0;

        while let Some((idx, ch)) = chars.next() {
            match ch {
                '\\' => {
                    // Check for escaped character
                    if let Some((_, next_char)) = chars.peek() {
                        match next_char {
                            'p' | 'P' => {
                                // Unicode property start \p or \P
                                // We consume the 'p'/'P'
                                chars.next();

                                // Check if it's followed by {
                                if let Some((_, '{')) = chars.peek() {
                                    unicode_property_count += 1;
                                    if unicode_property_count > self.max_unicode_properties {
                                        return Err(RegexError::syntax(
                                            "Too many Unicode properties in regex (max 50)",
                                            start_pos + idx,
                                        ));
                                    }
                                }
                            }
                            _ => {
                                // Just skip other escaped chars
                                chars.next();
                            }
                        }
                    }
                }
                '(' => {
                    let mut group_type = GroupType::Normal;

                    // Check for extension syntax (?...)
                    if let Some((_, '?')) = chars.peek() {
                        chars.next(); // consume ?

                        // Check for < (lookbehind or named capture)
                        if let Some((_, '<')) = chars.peek() {
                            chars.next(); // consume <

                            // Check for = or ! (lookbehind)
                            if matches!(chars.peek(), Some((_, '=')) | Some((_, '!'))) {
                                chars.next(); // consume = or !
                                group_type = GroupType::Lookbehind;
                            }
                            // Otherwise it's likely a named capture (?<name>...) or condition (?<...)
                            // which we treat as a normal group
                        } else if let Some((_, '|')) = chars.peek() {
                            chars.next(); // consume |
                            group_type = GroupType::BranchReset { branch_count: 1 };
                        }
                    }

                    match group_type {
                        GroupType::Lookbehind => {
                            // Calculate current lookbehind depth
                            let lookbehind_depth =
                                stack.iter().filter(|g| matches!(g, GroupType::Lookbehind)).count();
                            if lookbehind_depth >= self.max_nesting {
                                return Err(RegexError::syntax(
                                    "Regex lookbehind nesting too deep",
                                    start_pos + idx,
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
                                    start_pos + idx,
                                ));
                            }
                        }
                        _ => {}
                    }
                    stack.push(group_type);
                }
                '|' => {
                    // Check if we are in a branch reset group
                    if let Some(GroupType::BranchReset { branch_count }) = stack.last_mut() {
                        *branch_count += 1;
                        if *branch_count > 50 {
                            // Max 50 branches
                            return Err(RegexError::syntax(
                                "Too many branches in branch reset group (max 50)",
                                start_pos + idx,
                            ));
                        }
                    }
                }
                ')' => {
                    stack.pop();
                }
                '[' => {
                    // Skip character class [ ... ]
                    // Need to handle escaping inside []
                    while let Some((_, c)) = chars.next() {
                        if c == '\\' {
                            chars.next();
                        } else if c == ']' {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

enum GroupType {
    Normal,
    Lookbehind,
    BranchReset { branch_count: usize },
}

impl Default for RegexValidator {
    fn default() -> Self {
        Self::new()
    }
}

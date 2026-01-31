//! Expression safety validation
//!
//! This module provides the core validation logic for detecting dangerous
//! operations in Perl expressions during debug evaluation.

use crate::patterns::{ASSIGNMENT_OPERATORS, dangerous_ops_re, regex_mutation_re};

/// Error type for unsafe expression detection
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    /// Expression contains a dangerous operation
    #[error(
        "Safe evaluation mode: potentially mutating operation '{0}' not allowed (use allowSideEffects: true)"
    )]
    DangerousOperation(String),

    /// Expression contains an assignment operator
    #[error(
        "Safe evaluation mode: assignment operator '{0}' not allowed (use allowSideEffects: true)"
    )]
    AssignmentOperator(String),

    /// Expression contains increment/decrement operators
    #[error(
        "Safe evaluation mode: increment/decrement operators not allowed (use allowSideEffects: true)"
    )]
    IncrementDecrement,

    /// Expression contains backticks (shell execution)
    #[error(
        "Safe evaluation mode: backticks (shell execution) not allowed (use allowSideEffects: true)"
    )]
    Backticks,

    /// Expression contains a regex mutation operator (s///, tr///, y///)
    #[error(
        "Safe evaluation mode: regex mutation operator '{0}' not allowed (use allowSideEffects: true)"
    )]
    RegexMutation(String),

    /// Expression contains newlines (potential command injection)
    #[error("Expression cannot contain newlines")]
    ContainsNewlines,
}

/// Result type for expression validation
pub type ValidationResult = Result<(), ValidationError>;

/// Safe expression evaluator
///
/// Validates that expressions are safe for evaluation during debugging,
/// blocking operations that could mutate state or have side effects.
#[derive(Debug, Clone, Default)]
pub struct SafeEvaluator {
    // Future: could add configuration options here
}

impl SafeEvaluator {
    /// Create a new safe evaluator
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate that an expression is safe for evaluation
    ///
    /// # Arguments
    ///
    /// * `expression` - The Perl expression to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if the expression is safe, or an error describing why it's unsafe.
    pub fn validate(&self, expression: &str) -> ValidationResult {
        // Check for newlines (command injection vector)
        if expression.contains('\n') || expression.contains('\r') {
            return Err(ValidationError::ContainsNewlines);
        }

        // Check for backticks (shell execution)
        if expression.contains('`') {
            return Err(ValidationError::Backticks);
        }

        // Check for assignment operators
        for op in ASSIGNMENT_OPERATORS {
            if expression.contains(op) {
                return Err(ValidationError::AssignmentOperator(op.to_string()));
            }
        }

        // Check for increment/decrement operators
        if expression.contains("++") || expression.contains("--") {
            return Err(ValidationError::IncrementDecrement);
        }

        // Check for dangerous operations using regex
        self.check_dangerous_operations(expression)?;

        // Check for regex mutation operators
        self.check_regex_mutation(expression)?;

        Ok(())
    }

    /// Check for dangerous operations in the expression
    fn check_dangerous_operations(&self, expression: &str) -> ValidationResult {
        let re = dangerous_ops_re();

        for mat in re.find_iter(expression) {
            let op = mat.as_str();
            let start = mat.start();
            let end = mat.end();

            // Allow harmless occurrences in single-quoted literals
            if is_in_single_quotes(expression, start) {
                continue;
            }

            // Allow sigil-prefixed identifiers ($print, @say, %exit, *printf)
            if is_sigil_prefixed_identifier(expression, start) {
                continue;
            }

            // Allow ${print} (simple scalar braced variable form)
            if is_simple_braced_scalar_var(expression, start, end) {
                continue;
            }

            // Allow package-qualified names unless it's CORE::
            if is_package_qualified_not_core(expression, start) {
                continue;
            }

            // Block: either bare op or CORE:: qualified
            return Err(ValidationError::DangerousOperation(op.to_string()));
        }

        Ok(())
    }

    /// Check for regex mutation operators (s///, tr///, y///)
    fn check_regex_mutation(&self, expression: &str) -> ValidationResult {
        let re = regex_mutation_re();

        if let Some(mat) = re.find(expression) {
            let op = mat.as_str();
            let start = mat.start();

            // Allow sigil-prefixed identifiers ($s, $tr, $y)
            if is_sigil_prefixed_identifier(expression, start) {
                return Ok(());
            }

            // Allow escape sequences like \s, \y
            if is_escape_sequence(expression, start) {
                return Ok(());
            }

            return Err(ValidationError::RegexMutation(op.trim().to_string()));
        }

        Ok(())
    }
}

/// Check if a position in a string is inside single quotes
fn is_in_single_quotes(s: &str, idx: usize) -> bool {
    let mut in_sq = false;
    let mut escaped = false;

    for (i, ch) in s.char_indices() {
        if i >= idx {
            break;
        }
        if in_sq {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_sq = false;
            }
        } else if ch == '\'' {
            in_sq = true;
        }
    }

    in_sq
}

/// Check if a match is preceded by CORE:: (which means it IS dangerous)
fn is_core_qualified(s: &str, op_start: usize) -> bool {
    let s_bytes = s.as_bytes();
    // Check for GLOBAL prefix first
    if op_start >= 8 && &s_bytes[op_start - 8..op_start] == b"GLOBAL::" {
        // If GLOBAL, require CORE::GLOBAL::op
        return op_start >= 14 && &s_bytes[op_start - 14..op_start - 8] == b"CORE::";
    }

    // Check for regular CORE:: prefix
    op_start >= 6 && &s_bytes[op_start - 6..op_start] == b"CORE::"
}

/// Check if the match is a sigil-prefixed identifier ($print, @say, %exit, *dump)
fn is_sigil_prefixed_identifier(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();
    if op_start == 0 {
        return false;
    }

    // Must be preceded by a sigil
    if !matches!(bytes[op_start - 1], b'$' | b'@' | b'%' | b'*') {
        return false;
    }

    // Security: Check it's not being used for code execution (&$sub or ->$method)
    let mut i = op_start - 1;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }

    if i > 0 {
        let prev = bytes[i - 1];

        // &$sub is a code dereference (dangerous)
        if prev == b'&' {
            return false;
        }

        // ->$method is a method call (potentially dangerous)
        if prev == b'>' && i > 1 && bytes[i - 2] == b'-' {
            return false;
        }

        // Handle braced dereference &{ $sub }
        if prev == b'{' {
            i -= 1;
            while i > 0 && bytes[i - 1].is_ascii_whitespace() {
                i -= 1;
            }
            if i > 0 && bytes[i - 1] == b'&' {
                return false;
            }
        }
    }

    true
}

/// Check if the match is a simple braced scalar variable ${print}
fn is_simple_braced_scalar_var(s: &str, op_start: usize, op_end: usize) -> bool {
    let bytes = s.as_bytes();

    // Scan left for `${` (allow whitespace between)
    let mut i = op_start;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    if i < 1 || bytes[i - 1] != b'{' {
        return false;
    }
    i -= 1;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    if i < 1 || bytes[i - 1] != b'$' {
        return false;
    }

    // Scan right for `}` (allow whitespace between)
    let mut j = op_end;
    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    j < bytes.len() && bytes[j] == b'}'
}

/// Check if the match is package-qualified (Foo::print) but not CORE::
fn is_package_qualified_not_core(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();
    if op_start < 2 || bytes[op_start - 1] != b':' || bytes[op_start - 2] != b':' {
        return false;
    }
    // It's qualified, but we need to check it's not CORE::
    !is_core_qualified(s, op_start)
}

/// Check if the match is an escape sequence (preceded by backslash)
fn is_escape_sequence(s: &str, match_start: usize) -> bool {
    if match_start == 0 {
        return false;
    }
    s.as_bytes()[match_start - 1] == b'\\'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_expressions() {
        let evaluator = SafeEvaluator::new();

        // Simple arithmetic
        assert!(evaluator.validate("$x + $y").is_ok());
        assert!(evaluator.validate("$hash{key}").is_ok());
        assert!(evaluator.validate("$array[0]").is_ok());
        assert!(evaluator.validate("length($str)").is_ok());

        // Package-qualified (not CORE)
        assert!(evaluator.validate("Foo::print").is_ok());
        assert!(evaluator.validate("My::Module::system").is_ok());
    }

    #[test]
    fn test_dangerous_operations() {
        let evaluator = SafeEvaluator::new();

        // Code execution
        assert!(evaluator.validate("eval('code')").is_err());
        assert!(evaluator.validate("system('ls')").is_err());
        assert!(evaluator.validate("exec('/bin/sh')").is_err());

        // I/O
        assert!(evaluator.validate("print 'hello'").is_err());
        assert!(evaluator.validate("open(FH, '<', 'file')").is_err());
    }

    #[test]
    fn test_sigil_prefixed_identifiers() {
        let evaluator = SafeEvaluator::new();

        // These should be allowed (they're variable names, not operations)
        assert!(evaluator.validate("$print").is_ok());
        assert!(evaluator.validate("@say").is_ok());
        assert!(evaluator.validate("%exit").is_ok());
        assert!(evaluator.validate("$system_name").is_ok());
    }

    #[test]
    fn test_braced_variables() {
        let evaluator = SafeEvaluator::new();

        // ${print} is a variable, should be allowed
        assert!(evaluator.validate("${print}").is_ok());
    }

    #[test]
    fn test_assignment_operators() {
        let evaluator = SafeEvaluator::new();

        assert!(evaluator.validate("$x = 1").is_err());
        assert!(evaluator.validate("$x += 1").is_err());
        assert!(evaluator.validate("$x .= 'str'").is_err());
    }

    #[test]
    fn test_increment_decrement() {
        let evaluator = SafeEvaluator::new();

        assert!(evaluator.validate("$x++").is_err());
        assert!(evaluator.validate("++$x").is_err());
        assert!(evaluator.validate("$x--").is_err());
    }

    #[test]
    fn test_backticks() {
        let evaluator = SafeEvaluator::new();

        assert!(evaluator.validate("`ls -la`").is_err());
    }

    #[test]
    fn test_newlines() {
        let evaluator = SafeEvaluator::new();

        assert!(evaluator.validate("1\nprint 'hacked'").is_err());
        assert!(evaluator.validate("1\rprint 'hacked'").is_err());
    }

    #[test]
    fn test_regex_mutation() {
        let evaluator = SafeEvaluator::new();

        assert!(evaluator.validate("s/foo/bar/").is_err());
        assert!(evaluator.validate("tr/a-z/A-Z/").is_err());
        assert!(evaluator.validate("y/abc/xyz/").is_err());
    }

    #[test]
    fn test_escape_sequences_allowed() {
        let evaluator = SafeEvaluator::new();

        // \s in a regex match pattern should be allowed (it's not s///)
        // However, our simple regex catches it - this is a known limitation
        // The validator allows escape sequences like \s
        assert!(evaluator.validate("/\\s+/").is_ok());
    }

    #[test]
    fn test_single_quoted_strings() {
        let evaluator = SafeEvaluator::new();

        // Ops inside single quotes should be allowed (they're literal strings)
        assert!(evaluator.validate("'print this'").is_ok());
        assert!(evaluator.validate("'system call'").is_ok());
    }
}

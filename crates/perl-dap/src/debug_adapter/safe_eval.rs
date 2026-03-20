//! Safe expression evaluation validation.
//!
//! Validates that expressions sent to the Perl debugger for evaluation
//! do not contain dangerous operations (mutations, I/O, system calls).

use super::*;

/// Check if a position in a string is inside single quotes
/// (conservative: only tracks single-quoted string literals)
pub(super) fn is_in_single_quotes(s: &str, idx: usize) -> bool {
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

/// Check if the match is CORE:: or CORE::GLOBAL:: qualified (must block these)
pub(super) fn is_core_qualified(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();

    // Must have :: immediately before op
    if op_start < 2 || bytes[op_start - 1] != b':' || bytes[op_start - 2] != b':' {
        return false;
    }

    // Extract the identifier right before that ::
    let end = op_start - 2;
    let mut start = end;
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            start -= 1;
        } else {
            break;
        }
    }
    let seg = &s[start..end];
    if seg == "CORE" {
        return true;
    }
    if seg != "GLOBAL" {
        return false;
    }

    // If GLOBAL, require CORE::GLOBAL::op
    if start < 2 || bytes[start - 1] != b':' || bytes[start - 2] != b':' {
        return false;
    }
    let end2 = start - 2;
    let mut start2 = end2;
    while start2 > 0 {
        let b = bytes[start2 - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            start2 -= 1;
        } else {
            break;
        }
    }
    &s[start2..end2] == "CORE"
}

/// Check if the match is a sigil-prefixed identifier ($print, @say, %exit, *dump)
/// BUT NOT if it's a dereference call (&$print) or method call (->$print)
pub(super) fn is_sigil_prefixed_identifier(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();
    if op_start == 0 {
        return false;
    }

    // Must be preceded by a sigil
    if !matches!(bytes[op_start - 1], b'$' | b'@' | b'%' | b'*') {
        return false;
    }

    // Security: If it's a sigil, we must ensure it's not being used in a way
    // that triggers execution (like &$sub or ->$method).
    // We scan backwards from the sigil (op_start - 1) skipping whitespace.
    let mut i = op_start - 1;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }

    if i > 0 {
        let prev = bytes[i - 1];

        // Block dereference execution (&$sub)
        if prev == b'&' {
            return false;
        }

        // Block method call (->$method)
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
/// Does NOT skip ${print()} or ${print + 1}
pub(super) fn is_simple_braced_scalar_var(s: &str, op_start: usize, op_end: usize) -> bool {
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
pub(super) fn is_package_qualified_not_core(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();
    if op_start < 2 || bytes[op_start - 1] != b':' || bytes[op_start - 2] != b':' {
        return false;
    }
    // It's qualified, but we need to check it's not CORE::
    !is_core_qualified(s, op_start)
}

/// Validate that an expression is safe for evaluation (non-mutating)
///
/// AC10.2: Safe evaluation mode validates expressions don't have side effects
///
/// This function uses a pre-compiled regex for performance and includes
/// context-aware filtering to reduce false positives for:
/// - Sigil-prefixed identifiers ($print, @say, %exit)
/// - Simple braced scalar variables ${print}
/// - Package-qualified names (Foo::print) unless CORE::
/// - Single-quoted string literals ('print')
///
/// Note: Method calls ($obj->print) are intentionally NOT exempted because
/// dangerous operations remain dangerous regardless of invocation syntax.
pub(super) fn validate_safe_expression(expression: &str) -> Option<String> {
    // Check for assignment operators using regex to properly handle multi-char ops
    // This avoids false positives for comparison operators (e.g., == contains =)
    if let Some(re) = assignment_ops_re() {
        for mat in re.find_iter(expression) {
            let op = mat.as_str();
            let start = mat.start();

            // Allow harmless occurrences in single-quoted literals
            if is_in_single_quotes(expression, start) {
                continue;
            }

            // Check if it's strictly an assignment operator
            match op {
                "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "**=" | ".=" | "&=" | "|=" | "^="
                | "<<=" | ">>=" | "&&=" | "||=" | "//=" | "x=" => {
                    return Some(format!(
                        "Safe evaluation mode: assignment operator '{}' not allowed (use allowSideEffects: true)",
                        op
                    ));
                }
                _ => {}
            }
        }
    }

    // Check for dynamic subroutine calls &{...}
    // This blocks tricks like &{"sys"."tem"}("ls")
    if let Some(re) = deref_re() {
        if re.is_match(expression) {
            return Some(
                "Safe evaluation mode: dynamic subroutine calls (&{...}) not allowed (use allowSideEffects: true)"
                    .to_string(),
            );
        }
    }

    // Check for glob operations <*...> (anywhere in expression)
    // This blocks filesystem access via globs
    if let Some(re) = glob_re() {
        if re.is_match(expression) {
            return Some(
                "Safe evaluation mode: glob operations (<*...>) not allowed (use allowSideEffects: true)"
                    .to_string(),
            );
        }
    }

    // Check for file handle reads <$fh> or globs at start of expression
    // This blocks state changes via reads like <STDIN> or <$fh>
    if expression.trim().starts_with('<') {
        return Some(
            "Safe evaluation mode: file handle reads (<...>) and globs not allowed (use allowSideEffects: true)"
                .to_string(),
        );
    }

    // Check for mutating operations using pre-compiled regex
    if let Some(re) = dangerous_ops_re() {
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
            return Some(format!(
                "Safe evaluation mode: potentially mutating operation '{}' not allowed (use allowSideEffects: true)",
                op
            ));
        }
    }

    // Check for regex mutation operators (s///, tr///, y///)
    // Handled separately to avoid false positives with escape sequences like \s in /\s+/
    if let Some(re) = regex_mutation_re() {
        if let Some(mat) = re.find(expression) {
            let op = mat.as_str();
            let start = mat.start();

            // Allow sigil-prefixed identifiers ($s, $tr, $y)
            if is_sigil_prefixed_identifier(expression, start) {
                // It's a variable, allow it
            } else if is_escape_sequence(expression, start) {
                // It's an escape sequence like \s or \y, allow it
            } else {
                return Some(format!(
                    "Safe evaluation mode: regex mutation operator '{}' not allowed (use allowSideEffects: true)",
                    op.trim()
                ));
            }
        }
    }

    // Check for increment/decrement operators
    if expression.contains("++") || expression.contains("--") {
        return Some(
            "Safe evaluation mode: increment/decrement operators not allowed (use allowSideEffects: true)"
                .to_string(),
        );
    }

    // Check for backticks (shell execution)
    if expression.contains('`') {
        return Some(
            "Safe evaluation mode: backticks (shell execution) not allowed (use allowSideEffects: true)"
                .to_string(),
        );
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for safe_eval false-positive filtering
    #[test]
    fn safe_eval_allows_identifiers_named_like_ops() {
        // These should NOT be blocked - they're identifiers, not builtins
        let allowed = [
            "$print",           // scalar variable
            "@say",             // array variable
            "%exit",            // hash variable
            "*printf",          // glob
            "${print}",         // braced scalar variable
            "${ print }",       // braced with spaces
            "'print'",          // single-quoted string
            "Foo::print",       // package-qualified
            "My::Module::exit", // deeply qualified
        ];

        for expr in allowed {
            let err = validate_safe_expression(expr);
            assert!(err.is_none(), "unexpected block for {expr:?}: {err:?}");
        }
    }

    #[test]
    fn safe_eval_still_blocks_real_ops() {
        // These MUST be blocked - they're actual dangerous operations
        let blocked = [
            "print",
            "print $x",
            "say 'hello'",
            "exit",
            "exit 0",
            "eval '$x'",
            "eval { }",
            "system 'ls'",
            "exec '/bin/sh'",
            "fork",
            "kill 9, $$",
            "CORE::print $x",
            "CORE::GLOBAL::exit",
            "$obj->print",
            "$obj->system('ls')",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn test_safe_eval_mutating_regex_ops() {
        let blocked = [
            "$x =~ s/a/b/",
            "s/a/b/",
            "$x =~ tr/a/b/",
            "tr/a/b/",
            "y/a/b/",
            "$x =~ y/a/b/", // Bound y/// form
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn test_safe_eval_allows_regex_literals_with_escape_sequences() {
        // These should NOT be blocked - they're regex patterns or identifiers, not mutations
        // Note: Patterns using =~ are blocked by the assignment check (pre-existing behavior)
        // so we test patterns without =~ here
        let allowed = [
            r#"/\s+/"#,    // \s in regex literal (no binding operator)
            r#"/string/"#, // match containing 's'
            r#"/tricky/"#, // match containing 'tr'
            r#"/yay/"#,    // match containing 'y'
            r#"$s"#,       // variable named $s
            r#"$tr"#,      // variable named $tr
            r#"$y"#,       // variable named $y
            r#"qr/\s+/"#,  // compiled regex with \s
        ];

        for expr in allowed {
            let err = validate_safe_expression(expr);
            assert!(err.is_none(), "unexpected block for {expr:?}: {err:?}");
        }
    }

    #[test]
    fn safe_eval_blocks_new_dangerous_ops() {
        // Verify the extended deny-list works
        let blocked = [
            "eval '$code'",
            "kill 9, $pid",
            "exit 1",
            "dump",
            "fork",
            "chroot '/tmp'",
            "print STDERR 'x'",
            "say 'hello'",
            "printf '%s', $x",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn safe_eval_blocks_extended_ops_v2() {
        // Verify the even more extended deny-list works (glob, readline, IPC, etc.)
        let blocked = [
            "glob '*'",
            "readline $fh",
            "ioctl $fh, 1, 1",
            "srand",
            "dbmopen %h, 'file', 0666",
            "shmget $key, 10, 0666",
            "select $r, $w, $e, 0",
            "shutdown $socket, 2",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn safe_eval_blocks_mutation_and_resource_ops() {
        // Verify newly added mutation and resource management operations are blocked
        let blocked = [
            "bless $ref, 'Class'",
            "reset 'a-z'",
            "umask 0022",
            "binmode $fh",
            "opendir $dh, '.'",
            "closedir $dh",
            "seek $fh, 0, 0",
            "sysseek $fh, 0, 0",
            "setpgrp",
            "setpriority 0, 0, 10",
            "formline",
            "write",
            "lock $ref",
            "pipe $r, $w",
            "socketpair $r, $w, 1, 1, 1",
            "setsockopt $s, 1, 1, 1",
            "utime 1, 1, 'file'",
            "readdir $dh",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn test_safe_eval_blocks_dereference_execution() {
        // These are variables (safe to access)
        let allowed = ["$system", "@exec", "%fork"];

        for expr in allowed {
            let err = validate_safe_expression(expr);
            assert!(err.is_none(), "unexpected block for {expr:?}: {err:?}");
        }

        // These are dereference calls (NOT safe)
        // &$system calls the sub ref in $system
        // ->$system calls the method named in $system
        let blocked = [
            "&$system",
            "& $system",
            "&{$system}", // Braced form
            "$obj->$system",
            "$obj-> $system",
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }

    #[test]
    fn test_safe_eval_bypass_prevention() {
        // These patterns attempt to bypass safe evaluation checks
        let bypasses = [
            "&{'sys'.'tem'}('ls')", // Dynamic function name via concatenation
            "& { 'sys' . 'tem' }",  // Dynamic function name with spaces
            "<*.txt>",              // Glob operator for filesystem access
            "CORE::print",          // Explicitly blocked by dangerous ops regex
        ];

        for expr in bypasses {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "Expression '{}' should be blocked but was allowed", expr);
        }
    }

    #[test]
    fn test_safe_eval_assignment_ops_precision() {
        // These are comparison/binding operators (SAFE) but were previously blocked
        // because they contain '='
        let allowed = [
            "$a == $b",
            "$a != $b",
            "$a <= $b",
            "$a >= $b",
            "$a <=> $b",
            "$a =~ /regex/",
            "$a !~ /regex/",
            "$a ~~ $b", // Smart match
            // Logical ops
            "$a && $b",
            "$a || $b",
            "$a // $b",
            // Bitwise ops
            "$a & $b",
            "$a | $b",
            "$a ^ $b",
            "$a << $b",
            "$a >> $b",
            // Range
            "1..10",
        ];

        for expr in allowed {
            let err = validate_safe_expression(expr);
            assert!(err.is_none(), "unexpected block for {expr:?}: {err:?}");
        }

        // These are strict assignment operators (UNSAFE) and MUST be blocked
        let blocked = [
            "$a = 1",
            "$a += 1",
            "$a -= 1",
            "$a *= 1",
            "$a /= 1",
            "$a %= 1",
            "$a **= 1",
            "$a .= 's'",
            "$a &= 1",
            "$a |= 1",
            "$a ^= 1",
            "$a <<= 1",
            "$a >>= 1",
            "$a &&= 1",
            "$a ||= 1",
            "$a //= 1",
            "$a x= 3", // Repetition assignment
        ];

        for expr in blocked {
            let err = validate_safe_expression(expr);
            assert!(err.is_some(), "expected block for {expr:?}");
        }
    }
}

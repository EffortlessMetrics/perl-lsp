//! Shared debugger parsing and validation helpers for `perl-dap`.
//!
//! This microcrate extracts debugger-facing text parsing and safe-expression
//! validation so the main DAP adapter can stay focused on protocol/session flow.

use regex::Regex;
use std::sync::OnceLock;

static CONTEXT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static PROMPT_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static STACK_FRAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static ERROR_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static EXCEPTION_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static WARNING_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static DANGEROUS_OPS_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static REGEX_MUTATION_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static ASSIGNMENT_OPS_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static DEREF_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static GLOB_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static SET_VARIABLE_NAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();
static FUNCTION_BREAKPOINT_NAME_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebuggerContext {
    pub func: Option<String>,
    pub file: Option<String>,
    pub line: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackFrameMatch {
    pub func: String,
    pub file: String,
    pub line: i32,
}

fn context_re() -> Option<&'static Regex> {
    CONTEXT_RE
        .get_or_init(|| {
            Regex::new(r"^(?:(?P<func>[A-Za-z_][\w:]*+?)::(?:\((?P<file>[^:)]+):(?P<line>\d+)\):?|__ANON__)|main::(?:\()?(?P<file2>[^:)\s]+)(?:\))?:(?P<line2>\d+):?)")
        })
        .as_ref()
        .ok()
}

fn prompt_re() -> Option<&'static Regex> {
    PROMPT_RE.get_or_init(|| Regex::new(r"^\s*DB<?\d*>?\s*$")).as_ref().ok()
}

fn stack_frame_re() -> Option<&'static Regex> {
    STACK_FRAME_RE
        .get_or_init(|| {
            Regex::new(r"^\s*#?\s*(?P<frame>\d+)?\s+(?P<func>[A-Za-z_][\w:]*+?)(?:\s+called)?\s+at\s+(?P<file>[^\s]+)\s+line\s+(?P<line>\d+)")
        })
        .as_ref()
        .ok()
}

fn error_re() -> Option<&'static Regex> {
    ERROR_RE
        .get_or_init(|| {
            Regex::new(r"^(?:.*?\s+at\s+(?P<file>[^\s]+)\s+line\s+(?P<line>\d+)|Syntax error|Can't locate|Global symbol).*$")
        })
        .as_ref()
        .ok()
}

fn exception_re() -> Option<&'static Regex> {
    EXCEPTION_RE
        .get_or_init(|| {
            Regex::new(r"(?i)\b(?:died|uncaught exception|panic)\b|^\s*at\s+\S+?\s+line\s+\d+\.?$")
        })
        .as_ref()
        .ok()
}

fn warning_re() -> Option<&'static Regex> {
    WARNING_RE
        .get_or_init(|| {
            Regex::new(
                r"(?i)\b(?:warn(?:ing)?|carp|cluck)\b.*\bat\s+\S+?\s+line\s+\d+|^.+\bat\s+\S+?\s+line\s+\d+\.?\s*$",
            )
        })
        .as_ref()
        .ok()
}

fn dangerous_ops_re() -> Option<&'static Regex> {
    DANGEROUS_OPS_RE
        .get_or_init(|| {
            let ops = [
                "push",
                "pop",
                "shift",
                "unshift",
                "splice",
                "delete",
                "undef",
                "srand",
                "bless",
                "each",
                "keys",
                "values",
                "reset",
                "system",
                "exec",
                "fork",
                "exit",
                "dump",
                "kill",
                "alarm",
                "sleep",
                "wait",
                "waitpid",
                "setpgrp",
                "setpriority",
                "umask",
                "lock",
                "qx",
                "readpipe",
                "syscall",
                "open",
                "close",
                "print",
                "say",
                "printf",
                "sysread",
                "syswrite",
                "glob",
                "readline",
                "eof",
                "ioctl",
                "fcntl",
                "flock",
                "select",
                "dbmopen",
                "dbmclose",
                "binmode",
                "opendir",
                "closedir",
                "readdir",
                "rewinddir",
                "seekdir",
                "telldir",
                "seek",
                "sysseek",
                "formline",
                "write",
                "pipe",
                "socketpair",
                "mkdir",
                "rmdir",
                "unlink",
                "rename",
                "chdir",
                "chmod",
                "chown",
                "chroot",
                "truncate",
                "symlink",
                "link",
                "utime",
                "lstat",
                "stat",
                "readlink",
                "eval",
                "require",
                "do",
                "tie",
                "untie",
                "socket",
                "connect",
                "bind",
                "listen",
                "accept",
                "send",
                "recv",
                "shutdown",
                "setsockopt",
                "msgctl",
                "msgget",
                "msgrcv",
                "msgsnd",
                "semctl",
                "semget",
                "semop",
                "shmctl",
                "shmget",
                "shmread",
                "shmwrite",
            ];
            let pattern = format!(r"\b(?P<op>{})\b", ops.join("|"));
            Regex::new(&pattern)
        })
        .as_ref()
        .ok()
}

fn regex_mutation_re() -> Option<&'static Regex> {
    REGEX_MUTATION_RE.get_or_init(|| Regex::new(r"\b(?:s|tr|y)[^\w\s]")).as_ref().ok()
}

fn assignment_ops_re() -> Option<&'static Regex> {
    ASSIGNMENT_OPS_RE.get_or_init(|| Regex::new(r"([!~^&|+\-*/%=<>]+)")).as_ref().ok()
}

fn deref_re() -> Option<&'static Regex> {
    DEREF_RE.get_or_init(|| Regex::new(r"&[\s]*\{")).as_ref().ok()
}

fn glob_re() -> Option<&'static Regex> {
    GLOB_RE.get_or_init(|| Regex::new(r"<\*[^>]*>")).as_ref().ok()
}

fn set_variable_name_re() -> Option<&'static Regex> {
    SET_VARIABLE_NAME_RE
        .get_or_init(|| {
            Regex::new(
                r#"^(?:[\$@%][A-Za-z_][A-Za-z0-9_:]*|[\$@%][\^!_?\-\.\$\[\]\/|;,:~=+<>'"]|\$\d+)$"#,
            )
        })
        .as_ref()
        .ok()
}

fn function_breakpoint_name_re() -> Option<&'static Regex> {
    FUNCTION_BREAKPOINT_NAME_RE
        .get_or_init(|| Regex::new(r"^(?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*$"))
        .as_ref()
        .ok()
}

pub fn is_debugger_prompt(text: &str) -> bool {
    prompt_re().is_some_and(|re| re.is_match(text))
}

pub fn parse_context_line(text: &str) -> Option<DebuggerContext> {
    let caps = context_re()?.captures(text)?;
    let func = caps.name("func").map(|m| m.as_str().to_string());
    let file = caps.name("file").or_else(|| caps.name("file2")).map(|m| m.as_str().to_string());
    let line = caps
        .name("line")
        .or_else(|| caps.name("line2"))
        .and_then(|m| m.as_str().parse::<i32>().ok());
    Some(DebuggerContext { func, file, line })
}

pub fn parse_stack_frame_line(text: &str) -> Option<StackFrameMatch> {
    let caps = stack_frame_re()?.captures(text)?;
    Some(StackFrameMatch {
        func: caps.name("func")?.as_str().to_string(),
        file: caps.name("file")?.as_str().to_string(),
        line: caps.name("line")?.as_str().parse::<i32>().ok()?,
    })
}

pub fn parse_error_location(text: &str) -> Option<(String, i32)> {
    let caps = error_re()?.captures(text)?;
    let file = caps.name("file")?.as_str().to_string();
    let line = caps.name("line")?.as_str().parse::<i32>().ok()?;
    Some((file, line))
}

pub fn is_exception_line(text: &str) -> bool {
    exception_re().is_some_and(|re| re.is_match(text))
}

pub fn is_warning_line(text: &str) -> bool {
    warning_re().is_some_and(|re| re.is_match(text))
}

pub fn is_valid_set_variable_name(name: &str) -> bool {
    set_variable_name_re().is_some_and(|re| re.is_match(name))
}

pub fn is_valid_function_breakpoint_name(name: &str) -> bool {
    function_breakpoint_name_re().is_some_and(|re| re.is_match(name))
}

fn is_escape_sequence(s: &str, match_start: usize) -> bool {
    if match_start == 0 {
        return false;
    }
    s.as_bytes()[match_start - 1] == b'\\'
}

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

fn is_core_qualified(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();

    if op_start < 2 || bytes[op_start - 1] != b':' || bytes[op_start - 2] != b':' {
        return false;
    }

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

fn is_sigil_prefixed_identifier(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();
    if op_start == 0 {
        return false;
    }

    if !matches!(bytes[op_start - 1], b'$' | b'@' | b'%' | b'*') {
        return false;
    }

    let mut i = op_start - 1;
    while i > 0 && bytes[i - 1].is_ascii_whitespace() {
        i -= 1;
    }

    if i > 0 {
        let prev = bytes[i - 1];

        if prev == b'&' {
            return false;
        }

        if prev == b'>' && i > 1 && bytes[i - 2] == b'-' {
            return false;
        }

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

fn is_simple_braced_scalar_var(s: &str, op_start: usize, op_end: usize) -> bool {
    let bytes = s.as_bytes();

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

    let mut j = op_end;
    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    j < bytes.len() && bytes[j] == b'}'
}

fn is_package_qualified_not_core(s: &str, op_start: usize) -> bool {
    let bytes = s.as_bytes();
    if op_start < 2 || bytes[op_start - 1] != b':' || bytes[op_start - 2] != b':' {
        return false;
    }
    !is_core_qualified(s, op_start)
}

pub fn validate_safe_expression(expression: &str) -> Option<String> {
    if let Some(re) = assignment_ops_re() {
        for mat in re.find_iter(expression) {
            let op = mat.as_str();
            let start = mat.start();

            if is_in_single_quotes(expression, start) {
                continue;
            }

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

    if let Some(re) = deref_re() {
        if re.is_match(expression) {
            return Some(
                "Safe evaluation mode: dynamic subroutine calls (&{...}) not allowed (use allowSideEffects: true)"
                    .to_string(),
            );
        }
    }

    if let Some(re) = glob_re() {
        if re.is_match(expression) {
            return Some(
                "Safe evaluation mode: glob operations (<*...>) not allowed (use allowSideEffects: true)"
                    .to_string(),
            );
        }
    }

    if expression.trim().starts_with('<') {
        return Some(
            "Safe evaluation mode: file handle reads (<...>) and globs not allowed (use allowSideEffects: true)"
                .to_string(),
        );
    }

    if let Some(re) = dangerous_ops_re() {
        for mat in re.find_iter(expression) {
            let op = mat.as_str();
            let start = mat.start();
            let end = mat.end();

            if is_in_single_quotes(expression, start) {
                continue;
            }

            if is_sigil_prefixed_identifier(expression, start) {
                continue;
            }

            if is_simple_braced_scalar_var(expression, start, end) {
                continue;
            }

            if is_package_qualified_not_core(expression, start) {
                continue;
            }

            return Some(format!(
                "Safe evaluation mode: potentially mutating operation '{}' not allowed (use allowSideEffects: true)",
                op
            ));
        }
    }

    if let Some(re) = regex_mutation_re() {
        if let Some(mat) = re.find(expression) {
            let op = mat.as_str();
            let start = mat.start();

            if is_sigil_prefixed_identifier(expression, start) {
            } else if is_escape_sequence(expression, start) {
            } else {
                return Some(format!(
                    "Safe evaluation mode: regex mutation operator '{}' not allowed (use allowSideEffects: true)",
                    op.trim()
                ));
            }
        }
    }

    if expression.contains("++") || expression.contains("--") {
        return Some(
            "Safe evaluation mode: increment/decrement operators not allowed (use allowSideEffects: true)"
                .to_string(),
        );
    }

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

    #[test]
    fn parses_context_line() {
        let parsed = parse_context_line("main::(/tmp/test.pl):42:");
        assert_eq!(
            parsed,
            Some(DebuggerContext {
                func: None,
                file: Some("/tmp/test.pl".to_string()),
                line: Some(42),
            })
        );
    }

    #[test]
    fn parses_stack_frame_line() {
        let parsed = parse_stack_frame_line("# 1 Foo::bar called at /tmp/test.pl line 15");
        assert_eq!(
            parsed,
            Some(StackFrameMatch {
                func: "Foo::bar".to_string(),
                file: "/tmp/test.pl".to_string(),
                line: 15,
            })
        );
    }

    #[test]
    fn rejects_mutating_expression() {
        let err = validate_safe_expression("system('ls')");
        assert!(err.is_some());
    }

    #[test]
    fn allows_simple_scalar_reads() {
        let err = validate_safe_expression("$value + 1");
        assert!(err.is_none());
    }
}

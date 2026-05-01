use regex::Regex;
use std::collections::VecDeque;

use super::regexes::*;

pub(super) const DEBUG_SESSION_TERMINATE_WAIT_MS: u64 = 250;
pub(super) const DEBUGGER_QUERY_WAIT_MS: u64 = 75;
pub(super) const DEBUGGER_FRAME_POLL_MS: u64 = 10;

pub(super) const RECENT_OUTPUT_MAX_LINES: usize = 2048;
const MAX_DEBUGGER_IDENTIFIER_LEN: usize = 512;

#[derive(Debug, Clone)]
pub(super) struct RecentOutputLine {
    pub(super) id: u64,
    pub(super) raw: String,
    pub(super) normalized: String,
}

#[derive(Debug)]
pub(super) struct RecentOutputBuffer {
    pub(super) lines: VecDeque<RecentOutputLine>,
    pub(super) next_line_id: u64,
}

impl RecentOutputBuffer {
    pub(super) fn new() -> Self {
        Self { lines: VecDeque::with_capacity(RECENT_OUTPUT_MAX_LINES), next_line_id: 1 }
    }
}

pub(super) fn context_re() -> Option<&'static Regex> {
    CONTEXT_RE
        .get_or_init(|| {
            // Match Perl debugger context lines of the form:
            //   Func::Name(/path/to/file.pl:42):
            //   main::(/path/to/file.pl:42):
            //   main::(C:\Windows\path\file.pl:42):
            //
            // File paths may contain `:` on Windows (drive letter prefix such as `C:\`).
            // The path-capturing groups allow a `:` only when it is immediately followed
            // by a non-digit, non-space character — matching `C:\path` but not the `:42`
            // line-number separator.
            Regex::new(r"^(?:(?P<func>[A-Za-z_][\w:]*+?)::(?:\((?P<file>[^:)\s]+(?::[^:)\d\s][^:)\s]*)*):(?P<line>\d+)\):?|__ANON__)|main::(?:\()?(?P<file2>[^:)\s]+(?::[^:)\d\s][^:)\s]*)*)(?:\))?:(?P<line2>\d+):?)")
        })
        .as_ref()
        .ok()
}

pub(super) fn prompt_re() -> Option<&'static Regex> {
    PROMPT_RE.get_or_init(|| Regex::new(r"^\s*DB<?\d*>?\s*$")).as_ref().ok()
}

pub(super) fn stack_frame_re() -> Option<&'static Regex> {
    STACK_FRAME_RE
        .get_or_init(|| {
            Regex::new(r"^\s*#?\s*(?P<frame>\d+)?\s+(?P<func>[A-Za-z_][\w:]*+?)(?:\s+called)?\s+at\s+(?P<file>.+?)\s+line\s+(?P<line>\d+)")
        })
        .as_ref()
        .ok()
}

#[allow(dead_code)] // Reserved for future variable parsing enhancements
pub(super) fn variable_re() -> Option<&'static Regex> {
    VARIABLE_RE
        .get_or_init(|| Regex::new(r"^\s*(?P<name>[\$\@\%][\w:]+)\s*=\s*(?P<value>.*?)$"))
        .as_ref()
        .ok()
}

pub(super) fn error_re() -> Option<&'static Regex> {
    ERROR_RE
        .get_or_init(|| {
            Regex::new(r"^(?:.*?\s+at\s+(?P<file>[^\s]+)\s+line\s+(?P<line>\d+)|Syntax error|Can't locate|Global symbol).*$")
        })
        .as_ref()
        .ok()
}

pub(super) fn exception_re() -> Option<&'static Regex> {
    EXCEPTION_RE
        .get_or_init(|| {
            // Perl `die` often emits two lines:
            //  - message text
            //  - `at /path/file.pl line N.`
            Regex::new(r"(?i)\b(?:died|uncaught exception|panic)\b|^\s*at\s+\S+?\s+line\s+\d+\.?$")
        })
        .as_ref()
        .ok()
}

pub(super) fn warning_re() -> Option<&'static Regex> {
    WARNING_RE
        .get_or_init(|| {
            // Perl `warn`, `Carp::carp`, and `Carp::cluck` emit warning messages.
            // Common patterns:
            //  - "Something went wrong at script.pl line 42."
            //  - "Use of uninitialized value..."
            //  - Explicit warn/carp/cluck output
            Regex::new(
                r"(?i)\b(?:warn(?:ing)?|carp|cluck)\b.*\bat\s+\S+?\s+line\s+\d+|^.+\bat\s+\S+?\s+line\s+\d+\.?\s*$",
            )
        })
        .as_ref()
        .ok()
}

pub(super) fn dangerous_ops_re() -> Option<&'static Regex> {
    DANGEROUS_OPS_RE
        .get_or_init(|| {
            // Dangerous operations that can mutate state, perform I/O, or execute code
            // Categories:
            //   - State mutation: push, pop, shift, unshift, splice, delete, undef, srand
            //   - Process control: system, exec, fork, exit, dump, kill, alarm, sleep, wait, waitpid
            //   - I/O: qx, readpipe, syscall, open, close, print, say, printf, sysread, syswrite, glob, readline, ioctl, fcntl, flock, select, dbmopen, dbmclose
            //   - Filesystem: mkdir, rmdir, unlink, rename, chdir, chmod, chown, chroot, truncate, symlink, link
            //   - Code loading: eval, require, do (file)
            //   - Tie/untie: can execute arbitrary code via FETCH/STORE
            //   - Network: socket, connect, bind, listen, accept, send, recv, shutdown
            //   - IPC: msg*, sem*, shm*
            // Note: s/tr/y regex mutation operators handled separately via regex_mutation_re()
            let ops = [
                // State mutation
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
                "reset", // Process control
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
                "lock", // I/O
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
                "socketpair", // Filesystem
                "mkdir",
                "rmdir",
                "unlink",
                "rename",
                "chdir",
                "chmod",
                "chown",
                "chroot",
                "truncate",
                "utime",
                "symlink",
                "link", // Code loading/execution
                "eval",
                "require",
                "do", // Tie mechanism (can execute arbitrary code)
                "tie",
                "untie", // Network
                "socket",
                "connect",
                "bind",
                "listen",
                "accept",
                "send",
                "recv",
                "shutdown",
                "setsockopt",
                // IPC
                "msgget",
                "msgsnd",
                "msgrcv",
                "msgctl",
                "semget",
                "semop",
                "semctl",
                "shmget",
                "shmat",
                "shmdt",
                "shmctl",
            ];
            // Build pattern: \b(op1|op2|...)\b
            let pattern = format!(r"\b(?:{})\b", ops.join("|"));
            Regex::new(&pattern)
        })
        .as_ref()
        .ok()
}

/// Regex to match mutating regex operators (s///, tr///, y///)
/// Matches s, tr, y followed by a delimiter character
pub(super) fn regex_mutation_re() -> Option<&'static Regex> {
    REGEX_MUTATION_RE
        .get_or_init(|| {
            // Match s, tr, y followed by a delimiter character (not alphanumeric/underscore/whitespace)
            // Common delimiters: / # | ! { [ ( ' "
            // Note: We filter out escape sequences like \s manually after matching
            Regex::new(r"\b(?:s|tr|y)[^\w\s]")
        })
        .as_ref()
        .ok()
}

/// Regex to match potential assignment operators (any sequence of operator chars)
pub(super) fn assignment_ops_re() -> Option<&'static Regex> {
    ASSIGNMENT_OPS_RE
        .get_or_init(|| {
            // Match any sequence of operator characters to tokenize operators
            Regex::new(r"([!~^&|+\-*/%=<>]+)")
        })
        .as_ref()
        .ok()
}

/// Regex to match dynamic subroutine dereferencing: &{...}
pub(super) fn deref_re() -> Option<&'static Regex> {
    DEREF_RE.get_or_init(|| Regex::new(r"&[\s]*\{")).as_ref().ok()
}

/// Regex to match glob operations: <*...>
pub(super) fn glob_re() -> Option<&'static Regex> {
    GLOB_RE.get_or_init(|| Regex::new(r"<\*[^>]*>")).as_ref().ok()
}

/// Regex for matching ANSI escape sequences in debugger output.
pub(super) fn ansi_escape_re() -> Option<&'static Regex> {
    ANSI_ESCAPE_RE.get_or_init(|| Regex::new(r"\x1B\[[0-9;]*[A-Za-z]")).as_ref().ok()
}

/// Regex for validating setVariable variable names to avoid debugger command injection.
pub(super) fn set_variable_name_re() -> Option<&'static Regex> {
    SET_VARIABLE_NAME_RE
        .get_or_init(|| {
            Regex::new(r"^[\$\@\%](?:[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*|\d+|_)$")
        })
        .as_ref()
        .ok()
}

/// Validate DAP setVariable names (e.g. `$x`, `%ENV`, `$Package::value`) for safe passthrough.
pub(super) fn is_valid_set_variable_name(name: &str) -> bool {
    name.len() <= MAX_DEBUGGER_IDENTIFIER_LEN
        && set_variable_name_re().is_some_and(|re| re.is_match(name))
}

pub(super) fn function_breakpoint_name_re() -> Option<&'static Regex> {
    FUNCTION_BREAKPOINT_NAME_RE
        .get_or_init(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*$"))
        .as_ref()
        .ok()
}

pub(super) fn is_valid_function_breakpoint_name(name: &str) -> bool {
    name.len() <= MAX_DEBUGGER_IDENTIFIER_LEN
        && function_breakpoint_name_re().is_some_and(|re| re.is_match(name))
}

pub(super) fn inc_re() -> Option<&'static Regex> {
    INC_RE.get_or_init(|| Regex::new(r"'([^']+)'\s*=>\s*'([^']+)'")).as_ref().ok()
}

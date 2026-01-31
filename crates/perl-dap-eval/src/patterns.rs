//! Dangerous operation patterns
//!
//! This module defines the patterns used to detect dangerous Perl operations
//! that should be blocked during safe expression evaluation.

use regex::Regex;
use std::sync::OnceLock;

/// List of dangerous Perl operations that can mutate state, perform I/O, or execute code
///
/// Categories:
/// - State mutation: push, pop, shift, unshift, splice, delete, undef, srand
/// - Process control: system, exec, fork, exit, dump, kill, alarm, sleep, wait, waitpid
/// - I/O: qx, readpipe, syscall, open, close, print, say, printf, etc.
/// - Filesystem: mkdir, rmdir, unlink, rename, chdir, chmod, chown, chroot, truncate
/// - Code loading: eval, require, do (file)
/// - Tie mechanism: can execute arbitrary code via FETCH/STORE
/// - Network: socket, connect, bind, listen, accept, send, recv, shutdown
/// - IPC: msg*, sem*, shm*
pub const DANGEROUS_OPERATIONS: &[&str] = &[
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
    "reset",
    // Process control
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
    // I/O
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
    // Filesystem
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
    "link",
    // Code loading/execution
    "eval",
    "require",
    "do",
    // Tie mechanism (can execute arbitrary code)
    "tie",
    "untie",
    // Network
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

/// Assignment operators that indicate mutation
pub const ASSIGNMENT_OPERATORS: &[&str] = &[
    "=", "+=", "-=", "*=", "/=", "%=", "**=", ".=", "&=", "|=", "^=", "<<=", ">>=", "&&=", "||=",
    "//=",
];

/// Compiled regex for dangerous operations
///
/// Pattern matches word boundaries around operation names.
pub fn dangerous_ops_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let pattern = format!(r"\b(?:{})\b", DANGEROUS_OPERATIONS.join("|"));
        #[allow(clippy::expect_used)]
        Regex::new(&pattern).expect("Dangerous ops regex compilation failed")
    })
}

/// Compiled regex for regex mutation operators (s///, tr///, y///)
///
/// Matches s, tr, y followed by a delimiter character (not alphanumeric/underscore/whitespace).
pub fn regex_mutation_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Match s, tr, y followed by a delimiter character (not alphanumeric/underscore/whitespace)
        #[allow(clippy::expect_used)]
        Regex::new(r"\b(?:s|tr|y)[^\w\s]").expect("Mutation regex compilation failed")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_ops_regex() {
        let re = dangerous_ops_re();

        // Should match dangerous ops
        assert!(re.is_match("system('ls')"));
        assert!(re.is_match("eval($code)"));
        assert!(re.is_match("print 'hello'"));

        // Should NOT match as standalone (would need full validator for context)
        // The regex just does raw matching - context is handled by validator
    }

    #[test]
    fn test_regex_mutation_regex() {
        let re = regex_mutation_re();

        // Should match s///, tr///, y///
        assert!(re.is_match("s/foo/bar/"));
        assert!(re.is_match("tr/a-z/A-Z/"));
        assert!(re.is_match("y/abc/xyz/"));
    }
}

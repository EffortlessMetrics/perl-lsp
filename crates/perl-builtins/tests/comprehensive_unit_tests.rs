//! Comprehensive unit tests for the `perl-builtins` crate.
//!
//! Covers:
//! - `builtin_signatures_phf` module: BUILTIN_SIGS, BUILTIN_FULL_SIGS,
//!   `get_param_names`, `is_builtin`, `builtin_count`
//! - `builtin_signatures` module: `create_builtin_signatures`, `BuiltinSignature`

use perl_builtins::builtin_signatures::create_builtin_signatures;
use perl_builtins::builtin_signatures_phf::{
    BUILTIN_FULL_SIGS, BUILTIN_SIGS, builtin_count, get_param_names, is_builtin,
};
use perl_tdd_support::must_some;

// ============================================================
// builtin_signatures_phf — BUILTIN_SIGS static map
// ============================================================

#[test]
fn phf_map_is_nonempty() -> Result<(), String> {
    if BUILTIN_SIGS.is_empty() {
        return Err("BUILTIN_SIGS should not be empty".into());
    }
    Ok(())
}

#[test]
fn builtin_count_matches_map_len() -> Result<(), String> {
    let count = builtin_count();
    if count != BUILTIN_SIGS.len() {
        return Err(format!(
            "builtin_count()={count} but BUILTIN_SIGS.len()={}",
            BUILTIN_SIGS.len()
        ));
    }
    Ok(())
}

// ---- is_builtin positive cases ----

#[test]
fn is_builtin_io_functions() -> Result<(), String> {
    for name in &[
        "print", "printf", "say", "open", "sysopen", "close", "read", "readpipe", "sysread",
        "write", "syswrite", "binmode", "seek", "tell", "truncate", "eof", "fileno", "flock",
        "fcntl", "ioctl", "getc", "readline", "select", "sysseek",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_string_functions() -> Result<(), String> {
    for name in &[
        "chomp",
        "chop",
        "chr",
        "crypt",
        "fc",
        "index",
        "lc",
        "lcfirst",
        "length",
        "ord",
        "pack",
        "reverse",
        "rindex",
        "sprintf",
        "substr",
        "uc",
        "ucfirst",
        "unpack",
        "quotemeta",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_array_functions() -> Result<(), String> {
    for name in
        &["push", "pop", "shift", "unshift", "splice", "grep", "map", "sort", "join", "split"]
    {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_hash_functions() -> Result<(), String> {
    for name in &["each", "keys", "values", "delete", "exists"] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_math_functions() -> Result<(), String> {
    for name in
        &["abs", "atan2", "cos", "exp", "hex", "int", "log", "oct", "rand", "sin", "sqrt", "srand"]
    {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_file_dir_functions() -> Result<(), String> {
    for name in &[
        "chdir",
        "chroot",
        "chmod",
        "chown",
        "link",
        "lstat",
        "mkdir",
        "opendir",
        "readdir",
        "readlink",
        "glob",
        "rename",
        "rmdir",
        "stat",
        "symlink",
        "umask",
        "unlink",
        "utime",
        "closedir",
        "rewinddir",
        "seekdir",
        "telldir",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_process_functions() -> Result<(), String> {
    for name in &[
        "alarm",
        "exec",
        "fork",
        "getpgrp",
        "getppid",
        "getpriority",
        "kill",
        "pipe",
        "setpgrp",
        "setpriority",
        "syscall",
        "sleep",
        "system",
        "times",
        "wait",
        "waitpid",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_time_functions() -> Result<(), String> {
    for name in &["gmtime", "localtime", "time"] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_network_functions() -> Result<(), String> {
    for name in &[
        "accept",
        "bind",
        "connect",
        "getpeername",
        "getsockname",
        "getsockopt",
        "listen",
        "recv",
        "send",
        "setsockopt",
        "shutdown",
        "socket",
        "socketpair",
        "sockatmark",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_sysinfo_functions() -> Result<(), String> {
    for name in &[
        "gethostbyaddr",
        "gethostbyname",
        "gethostent",
        "getnetbyaddr",
        "getnetbyname",
        "getnetent",
        "getprotobyname",
        "getprotobynumber",
        "getprotoent",
        "getservbyname",
        "getservbyport",
        "getservent",
        "sethostent",
        "setnetent",
        "setprotoent",
        "setservent",
        "endhostent",
        "endnetent",
        "endprotoent",
        "endservent",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_user_group_functions() -> Result<(), String> {
    for name in &[
        "getgrent",
        "getgrgid",
        "getgrnam",
        "getlogin",
        "getuid",
        "geteuid",
        "getgid",
        "getegid",
        "getgroups",
        "setuid",
        "seteuid",
        "setgid",
        "setegid",
        "setgroups",
        "getpwent",
        "getpwnam",
        "getpwuid",
        "setgrent",
        "setpwent",
        "endgrent",
        "endpwent",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_ipc_functions() -> Result<(), String> {
    for name in &[
        "msgctl", "msgget", "msgrcv", "msgsnd", "semctl", "semget", "semop", "shmctl", "shmget",
        "shmread", "shmwrite",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_database_functions() -> Result<(), String> {
    for name in &["dbmclose", "dbmopen", "tie", "tied", "untie"] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_misc_functions() -> Result<(), String> {
    for name in &[
        "bless",
        "caller",
        "die",
        "do",
        "eval",
        "exit",
        "goto",
        "last",
        "next",
        "redo",
        "ref",
        "require",
        "return",
        "scalar",
        "undef",
        "wantarray",
        "warn",
        "defined",
        "dump",
        "formline",
        "local",
        "my",
        "our",
        "state",
        "reset",
        "study",
        "pos",
        "use",
        "vec",
        "lock",
        "prototype",
    ] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_file_test_operators() -> Result<(), String> {
    for op in &[
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ] {
        if !is_builtin(op) {
            return Err(format!("File test operator {op} should be a builtin"));
        }
    }
    Ok(())
}

// ---- is_builtin negative cases ----

#[test]
fn is_builtin_rejects_nonexistent_names() -> Result<(), String> {
    for name in &[
        "",
        "not_a_builtin",
        "foobar",
        "PRINT",
        "Print",
        "Printf",
        " print",
        "print ",
        "my_function",
        "Perl::function",
    ] {
        if is_builtin(name) {
            return Err(format!("{name:?} should NOT be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_case_sensitive() -> Result<(), String> {
    // Perl builtins are lowercase (except file test ops)
    if is_builtin("PRINT") || is_builtin("Open") || is_builtin("CHOMP") {
        return Err("Builtins lookup should be case-sensitive".into());
    }
    Ok(())
}

// ============================================================
// builtin_signatures_phf — get_param_names
// ============================================================

#[test]
fn get_param_names_returns_correct_params_for_print() -> Result<(), String> {
    let params = get_param_names("print");
    if params != ["FILEHANDLE", "LIST"] {
        return Err(format!("Unexpected params for print: {params:?}"));
    }
    Ok(())
}

#[test]
fn get_param_names_returns_correct_params_for_open() -> Result<(), String> {
    let params = get_param_names("open");
    if params != ["FILEHANDLE", "MODE", "FILENAME"] {
        return Err(format!("Unexpected params for open: {params:?}"));
    }
    Ok(())
}

#[test]
fn get_param_names_returns_correct_params_for_substr() -> Result<(), String> {
    let params = get_param_names("substr");
    if params != ["EXPR", "OFFSET", "LENGTH", "REPLACEMENT"] {
        return Err(format!("Unexpected params for substr: {params:?}"));
    }
    Ok(())
}

#[test]
fn get_param_names_returns_correct_params_for_splice() -> Result<(), String> {
    let params = get_param_names("splice");
    if params != ["ARRAY", "OFFSET", "LENGTH", "LIST"] {
        return Err(format!("Unexpected params for splice: {params:?}"));
    }
    Ok(())
}

#[test]
fn get_param_names_zero_param_builtins() -> Result<(), String> {
    for name in &["fork", "getppid", "times", "wait", "time", "wantarray"] {
        let params = get_param_names(name);
        if !params.is_empty() {
            return Err(format!("{name} should have 0 params but got {params:?}"));
        }
    }
    Ok(())
}

#[test]
fn get_param_names_returns_empty_for_unknown() -> Result<(), String> {
    let params = get_param_names("not_a_builtin");
    if !params.is_empty() {
        return Err(format!("Expected empty params for unknown, got {params:?}"));
    }
    Ok(())
}

#[test]
fn get_param_names_returns_empty_for_empty_string() -> Result<(), String> {
    let params = get_param_names("");
    if !params.is_empty() {
        return Err(format!("Expected empty params for empty string, got {params:?}"));
    }
    Ok(())
}

#[test]
fn get_param_names_file_test_operator() -> Result<(), String> {
    let params = get_param_names("-e");
    if params != ["FILE"] {
        return Err(format!("Unexpected params for -e: {params:?}"));
    }
    Ok(())
}

#[test]
fn get_param_names_multi_param_functions() -> Result<(), String> {
    // Verify various param counts
    let cases: &[(&str, usize)] =
        &[("close", 1), ("crypt", 2), ("index", 3), ("read", 4), ("socketpair", 5)];
    for &(name, expected_count) in cases {
        let params = get_param_names(name);
        if params.len() != expected_count {
            return Err(format!(
                "{name}: expected {expected_count} params, got {} ({params:?})",
                params.len()
            ));
        }
    }
    Ok(())
}

// ============================================================
// builtin_signatures_phf — BUILTIN_FULL_SIGS
// ============================================================

#[test]
fn full_sigs_contains_expected_entries() -> Result<(), String> {
    for name in &["print", "printf", "say", "open", "close", "substr", "splice", "split"] {
        if BUILTIN_FULL_SIGS.get(name).is_none() {
            return Err(format!("BUILTIN_FULL_SIGS missing entry for {name}"));
        }
    }
    Ok(())
}

#[test]
fn full_sigs_print_has_multiple_variants() -> Result<(), String> {
    let sigs = must_some(BUILTIN_FULL_SIGS.get("print"));
    if sigs.len() < 2 {
        return Err(format!("print should have multiple full signatures, got {}", sigs.len()));
    }
    Ok(())
}

#[test]
fn full_sigs_substr_has_multiple_variants() -> Result<(), String> {
    let sigs = must_some(BUILTIN_FULL_SIGS.get("substr"));
    if sigs.len() < 3 {
        return Err(format!("substr should have at least 3 full signatures, got {}", sigs.len()));
    }
    Ok(())
}

#[test]
fn full_sigs_splice_has_four_variants() -> Result<(), String> {
    let sigs = must_some(BUILTIN_FULL_SIGS.get("splice"));
    if sigs.len() != 4 {
        return Err(format!("splice should have 4 full signatures, got {}", sigs.len()));
    }
    Ok(())
}

#[test]
fn full_sigs_split_has_four_variants() -> Result<(), String> {
    let sigs = must_some(BUILTIN_FULL_SIGS.get("split"));
    if sigs.len() != 4 {
        return Err(format!("split should have 4 full signatures, got {}", sigs.len()));
    }
    Ok(())
}

#[test]
fn full_sigs_entries_contain_function_name() -> Result<(), String> {
    for (name, sigs) in BUILTIN_FULL_SIGS.entries() {
        for sig in *sigs {
            if !sig.starts_with(name) {
                return Err(format!(
                    "Full sig {sig:?} for {name} should start with the function name"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn full_sigs_unknown_returns_none() -> Result<(), String> {
    if BUILTIN_FULL_SIGS.get("not_a_function").is_some() {
        return Err("BUILTIN_FULL_SIGS should return None for unknown".into());
    }
    Ok(())
}

// ============================================================
// builtin_signatures_phf — BUILTIN_SIGS data integrity
// ============================================================

#[test]
fn all_sigs_entries_have_nonempty_key() -> Result<(), String> {
    for (name, _) in BUILTIN_SIGS.entries() {
        if name.is_empty() {
            return Err("BUILTIN_SIGS contains an empty key".into());
        }
    }
    Ok(())
}

#[test]
fn all_param_names_are_nonempty_and_uppercase() -> Result<(), String> {
    for (name, params) in BUILTIN_SIGS.entries() {
        for param in *params {
            if param.is_empty() {
                return Err(format!("{name} has an empty param name"));
            }
            if *param != param.to_uppercase() {
                return Err(format!("{name} param {param} is not uppercase"));
            }
        }
    }
    Ok(())
}

#[test]
fn no_duplicate_params_within_entry() -> Result<(), String> {
    for (name, params) in BUILTIN_SIGS.entries() {
        let mut seen = std::collections::HashSet::new();
        for param in *params {
            if !seen.insert(*param) {
                return Err(format!("{name} has duplicate param {param}"));
            }
        }
    }
    Ok(())
}

// ============================================================
// builtin_signatures — create_builtin_signatures()
// ============================================================

#[test]
fn create_builtin_signatures_returns_nonempty_map() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if sigs.is_empty() {
        return Err("create_builtin_signatures should not be empty".into());
    }
    Ok(())
}

#[test]
fn create_builtin_signatures_is_cached() -> Result<(), String> {
    let sigs1 = create_builtin_signatures();
    let sigs2 = create_builtin_signatures();
    // OnceLock means both calls return the same reference
    let ptr1 = sigs1 as *const _;
    let ptr2 = sigs2 as *const _;
    if ptr1 != ptr2 {
        return Err("create_builtin_signatures should return the same cached reference".into());
    }
    Ok(())
}

#[test]
fn signatures_contains_io_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &[
        "print",
        "printf",
        "say",
        "open",
        "XSLoader::load",
        "DynaLoader::bootstrap",
        "bootstrap",
        "sysopen",
        "close",
        "read",
        "readline",
        "readpipe",
        "sysread",
        "write",
        "syswrite",
        "seek",
        "tell",
        "eof",
    ] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing IO function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_string_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &[
        "chomp",
        "chop",
        "chr",
        "ord",
        "hex",
        "oct",
        "length",
        "substr",
        "index",
        "rindex",
        "sprintf",
        "lc",
        "lcfirst",
        "uc",
        "ucfirst",
        "quotemeta",
        "split",
        "join",
        "reverse",
    ] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing string function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_array_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["push", "pop", "shift", "unshift", "splice", "map", "grep", "sort"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing array function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_hash_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["each", "keys", "values", "exists", "delete"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing hash function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_math_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["abs", "atan2", "cos", "sin", "exp", "log", "sqrt", "int", "rand", "srand"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing math function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_process_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["system", "exec", "fork", "wait", "waitpid", "kill", "getpid", "getppid"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing process function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_time_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["time", "localtime", "gmtime", "sleep", "alarm"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing time function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_control_flow() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["die", "warn", "exit", "return", "next", "last", "redo", "goto"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing control flow function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_type_ref_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["ref", "bless", "defined", "undef", "scalar", "wantarray"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing type/ref function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_module_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["require", "use", "no", "import", "unimport", "package"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing module function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_eval_do() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["eval", "do", "caller"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing eval/do function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_tie_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["tie", "tied", "untie"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing tie function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_socket_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &[
        "socket",
        "bind",
        "listen",
        "accept",
        "connect",
        "shutdown",
        "send",
        "recv",
        "getsockopt",
        "setsockopt",
        "socketpair",
        "sockatmark",
        "getpeername",
        "getsockname",
    ] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing socket function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_io_control() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["pipe", "fcntl", "ioctl", "flock", "select", "getc", "binmode", "fileno"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing IO control function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_network_info() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &[
        "gethostbyname",
        "gethostbyaddr",
        "getnetbyname",
        "getnetbyaddr",
        "getprotobyname",
        "getprotobynumber",
        "getservbyname",
        "getservbyport",
        "gethostent",
        "getnetent",
        "getprotoent",
        "getservent",
        "sethostent",
        "setnetent",
        "setprotoent",
        "setservent",
        "endhostent",
        "endnetent",
        "endprotoent",
        "endservent",
    ] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing network info function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_user_group() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &[
        "getpwnam",
        "getpwuid",
        "getpwent",
        "setpwent",
        "endpwent",
        "getgrnam",
        "getgrgid",
        "getgrent",
        "setgrent",
        "endgrent",
        "getlogin",
        "getuid",
        "geteuid",
        "getgid",
        "getegid",
        "getgroups",
        "setuid",
        "seteuid",
        "setgid",
        "setegid",
        "setgroups",
    ] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing user/group function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_directory_functions() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &["opendir", "readdir", "closedir", "rewinddir", "telldir", "seekdir"] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing directory function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_file_operations() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &[
        "chdir", "chroot", "chmod", "chown", "link", "symlink", "readlink", "rename", "unlink",
        "mkdir", "rmdir", "stat", "lstat",
    ] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing file operation function: {name}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_file_test_operators() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for op in &[
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ] {
        if !sigs.contains_key(op) {
            return Err(format!("Missing file test operator: {op}"));
        }
    }
    Ok(())
}

#[test]
fn signatures_contains_miscellaneous() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in &[
        "pack",
        "unpack",
        "study",
        "pos",
        "reset",
        "formline",
        "format",
        "dump",
        "dbmopen",
        "dbmclose",
        "vec",
        "prototype",
        "lock",
        "umask",
        "truncate",
        "glob",
        "setpgrp",
        "getpgrp",
        "syscall",
        "times",
        "getpriority",
        "setpriority",
    ] {
        if !sigs.contains_key(name) {
            return Err(format!("Missing misc function: {name}"));
        }
    }
    Ok(())
}

// ============================================================
// BuiltinSignature struct — field verification
// ============================================================

#[test]
fn signature_print_has_multiple_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let print_sig = must_some(sigs.get("print"));
    if print_sig.signatures.len() < 2 {
        return Err(format!(
            "print should have multiple variants, got {}",
            print_sig.signatures.len()
        ));
    }
    Ok(())
}

#[test]
fn signature_print_documentation_nonempty() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let print_sig = must_some(sigs.get("print"));
    if print_sig.documentation.is_empty() {
        return Err("print documentation should not be empty".into());
    }
    Ok(())
}

#[test]
fn signature_fork_has_single_variant() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let fork_sig = must_some(sigs.get("fork"));
    if fork_sig.signatures.len() != 1 {
        return Err(format!("fork should have 1 variant, got {}", fork_sig.signatures.len()));
    }
    Ok(())
}

#[test]
fn signature_open_has_three_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let open_sig = must_some(sigs.get("open"));
    if open_sig.signatures.len() != 3 {
        return Err(format!("open should have 3 variants, got {}", open_sig.signatures.len()));
    }
    Ok(())
}

#[test]
fn all_signatures_have_nonempty_docs() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for (name, sig) in sigs.iter() {
        if sig.documentation.is_empty() {
            return Err(format!("{name} has empty documentation"));
        }
    }
    Ok(())
}

#[test]
fn all_signatures_have_at_least_one_variant() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for (name, sig) in sigs.iter() {
        if sig.signatures.is_empty() {
            return Err(format!("{name} has no signature variants"));
        }
    }
    Ok(())
}

#[test]
fn all_signature_variants_are_nonempty() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for (name, sig) in sigs.iter() {
        for variant in &sig.signatures {
            if variant.is_empty() {
                return Err(format!("{name} has an empty signature variant"));
            }
        }
    }
    Ok(())
}

#[test]
fn signature_variants_contain_function_name() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for (name, sig) in sigs.iter() {
        for variant in &sig.signatures {
            // File test operators like "-e FILE" - the name is "-e"
            // Regular functions like "print LIST" - name is "print"
            if !variant.starts_with(name) {
                return Err(format!(
                    "Signature variant {variant:?} for {name} should start with function name"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn file_test_operators_have_file_test_documentation() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for op in &["-e", "-f", "-d", "-r", "-w"] {
        let sig = must_some(sigs.get(op));
        if sig.documentation.is_empty() {
            return Err(format!("{op} should have documentation"));
        }
    }
    Ok(())
}

#[test]
fn file_test_operators_each_have_two_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for op in &[
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ] {
        let sig = must_some(sigs.get(op));
        if sig.signatures.len() != 2 {
            return Err(format!(
                "{op} should have 2 variants (with and without FILE), got {}",
                sig.signatures.len()
            ));
        }
    }
    Ok(())
}

// ============================================================
// Cross-module consistency checks
// ============================================================

#[test]
fn phf_and_hashmap_share_common_core_builtins() -> Result<(), String> {
    let hashmap_sigs = create_builtin_signatures();
    // A representative subset that must exist in both
    let core_builtins = [
        "print", "open", "close", "chomp", "push", "pop", "keys", "values", "die", "warn", "eval",
        "ref", "bless", "substr", "split", "join", "map", "grep", "sort", "stat",
    ];
    for name in &core_builtins {
        if !is_builtin(name) {
            return Err(format!("{name} missing from PHF map"));
        }
        if !hashmap_sigs.contains_key(name) {
            return Err(format!("{name} missing from HashMap signatures"));
        }
    }
    Ok(())
}

#[test]
fn xs_bootstrap_signatures_exist_in_both_stores() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for name in ["XSLoader::load", "DynaLoader::bootstrap", "bootstrap"] {
        let sig = must_some(sigs.get(name));
        if sig.documentation.is_empty() {
            return Err(format!("{name} should have documentation"));
        }
        if sig.signatures.is_empty() {
            return Err(format!("{name} should have at least one signature"));
        }
        if !is_builtin(name) {
            return Err(format!("{name} should be present in the PHF builtin map"));
        }
        if get_param_names(name).is_empty() {
            return Err(format!("{name} should expose parameter names"));
        }
    }
    Ok(())
}

#[test]
fn full_sigs_entries_exist_in_phf_map() -> Result<(), String> {
    for (name, _) in BUILTIN_FULL_SIGS.entries() {
        if !is_builtin(name) {
            return Err(format!(
                "BUILTIN_FULL_SIGS has {name} but it is missing from BUILTIN_SIGS"
            ));
        }
    }
    Ok(())
}

// ============================================================
// Edge cases and boundary conditions
// ============================================================

#[test]
fn builtin_count_is_reasonable() -> Result<(), String> {
    let count = builtin_count();
    // Perl has ~200+ builtins; we should have a substantial number
    if count < 100 {
        return Err(format!("builtin_count()={count}, expected at least 100"));
    }
    if count > 500 {
        return Err(format!("builtin_count()={count}, unexpectedly large"));
    }
    Ok(())
}

#[test]
fn get_param_names_idempotent() -> Result<(), String> {
    let p1 = get_param_names("print");
    let p2 = get_param_names("print");
    if p1 != p2 {
        return Err("get_param_names should return consistent results".into());
    }
    Ok(())
}

#[test]
fn create_builtin_signatures_idempotent() -> Result<(), String> {
    let s1 = create_builtin_signatures();
    let s2 = create_builtin_signatures();
    if s1.len() != s2.len() {
        return Err("create_builtin_signatures should return consistent results".into());
    }
    Ok(())
}

#[test]
fn specific_signature_details_use() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let use_sig = must_some(sigs.get("use"));
    if use_sig.signatures.len() < 4 {
        return Err(format!(
            "use should have at least 4 variants, got {}",
            use_sig.signatures.len()
        ));
    }
    Ok(())
}

#[test]
fn specific_signature_details_no() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let no_sig = must_some(sigs.get("no"));
    if no_sig.signatures.len() < 4 {
        return Err(format!("no should have at least 4 variants, got {}", no_sig.signatures.len()));
    }
    Ok(())
}

#[test]
fn specific_signature_splice_variants_ordered() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let splice_sig = must_some(sigs.get("splice"));
    // Most specific variant should come first
    let first = splice_sig.signatures.first().copied();
    if first != Some("splice ARRAY, OFFSET, LENGTH, LIST") {
        return Err(format!("splice first variant should be most specific, got {first:?}"));
    }
    Ok(())
}

#[test]
fn specific_phf_params_atan2() -> Result<(), String> {
    let params = get_param_names("atan2");
    if params != ["Y", "X"] {
        return Err(format!("atan2 params should be [Y, X], got {params:?}"));
    }
    Ok(())
}

#[test]
fn specific_phf_params_socketpair() -> Result<(), String> {
    let params = get_param_names("socketpair");
    if params != ["SOCKET1", "SOCKET2", "DOMAIN", "TYPE", "PROTOCOL"] {
        return Err(format!("socketpair params wrong: {params:?}"));
    }
    Ok(())
}

#[test]
fn specific_phf_params_bless() -> Result<(), String> {
    let params = get_param_names("bless");
    if params != ["REF", "CLASSNAME"] {
        return Err(format!("bless params should be [REF, CLASSNAME], got {params:?}"));
    }
    Ok(())
}

#[test]
fn specific_phf_params_use() -> Result<(), String> {
    let params = get_param_names("use");
    if params != ["MODULE", "VERSION", "LIST"] {
        return Err(format!("use params should be [MODULE, VERSION, LIST], got {params:?}"));
    }
    Ok(())
}

#[test]
fn goto_has_three_signature_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let goto_sig = must_some(sigs.get("goto"));
    if goto_sig.signatures.len() != 3 {
        return Err(format!("goto should have 3 variants, got {}", goto_sig.signatures.len()));
    }
    Ok(())
}

#[test]
fn select_has_three_signature_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let select_sig = must_some(sigs.get("select"));
    if select_sig.signatures.len() != 3 {
        return Err(format!("select should have 3 variants, got {}", select_sig.signatures.len()));
    }
    Ok(())
}

#[test]
fn require_has_four_signature_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    let req_sig = must_some(sigs.get("require"));
    if req_sig.signatures.len() != 4 {
        return Err(format!("require should have 4 variants, got {}", req_sig.signatures.len()));
    }
    Ok(())
}

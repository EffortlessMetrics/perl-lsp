//! Comprehensive unit tests for the `perl-builtins-phf` crate.
//!
//! Covers:
//! - `BUILTIN_SIGS` static PHF map: all entries, parameter types, categories
//! - `BUILTIN_FULL_SIGS` static PHF map: variant counts, content, ordering
//! - `get_param_names`: correctness, edge cases, unknown inputs
//! - `is_builtin`: positive, negative, case sensitivity
//! - `builtin_count`: consistency, bounds
//! - Data integrity: no empty keys, uppercase params, no duplicates
//! - Cross-map consistency: BUILTIN_FULL_SIGS subset of BUILTIN_SIGS

use perl_lexer::builtins::phf_lookup::{
    BUILTIN_FULL_SIGS, BUILTIN_SIGS, builtin_count, get_param_names, is_builtin,
};

// ============================================================
// builtin_count and BUILTIN_SIGS basics
// ============================================================

#[test]
fn builtin_count_matches_map_len() -> Result<(), String> {
    let count = builtin_count();
    if count != BUILTIN_SIGS.len() {
        return Err(format!(
            "builtin_count()={count} != BUILTIN_SIGS.len()={}",
            BUILTIN_SIGS.len()
        ));
    }
    Ok(())
}

#[test]
fn builtin_count_exceeds_150() -> Result<(), String> {
    let count = builtin_count();
    if count < 150 {
        return Err(format!("Expected at least 150 builtins, got {count}"));
    }
    Ok(())
}

#[test]
fn builtin_count_under_500() -> Result<(), String> {
    let count = builtin_count();
    if count > 500 {
        return Err(format!("Unexpectedly high builtin count: {count}"));
    }
    Ok(())
}

#[test]
fn builtin_count_is_stable_across_calls() -> Result<(), String> {
    let c1 = builtin_count();
    let c2 = builtin_count();
    if c1 != c2 {
        return Err(format!("builtin_count not stable: {c1} vs {c2}"));
    }
    Ok(())
}

#[test]
fn builtin_sigs_is_not_empty() -> Result<(), String> {
    if BUILTIN_SIGS.is_empty() {
        return Err("BUILTIN_SIGS should not be empty".into());
    }
    Ok(())
}

#[test]
fn builtin_sigs_iteration_count_matches_len() -> Result<(), String> {
    let iter_count = BUILTIN_SIGS.entries().count();
    if iter_count != BUILTIN_SIGS.len() {
        return Err(format!(
            "entries().count()={iter_count} != len()={}",
            BUILTIN_SIGS.len()
        ));
    }
    Ok(())
}

#[test]
fn full_sigs_is_subset_of_builtin_sigs() -> Result<(), String> {
    if BUILTIN_FULL_SIGS.len() >= builtin_count() {
        return Err(format!(
            "BUILTIN_FULL_SIGS.len()={} should be < builtin_count()={}",
            BUILTIN_FULL_SIGS.len(),
            builtin_count()
        ));
    }
    Ok(())
}

#[test]
fn full_sigs_has_at_least_40_entries() -> Result<(), String> {
    if BUILTIN_FULL_SIGS.len() < 40 {
        return Err(format!(
            "BUILTIN_FULL_SIGS should have at least 40 entries, got {}",
            BUILTIN_FULL_SIGS.len()
        ));
    }
    Ok(())
}

// ============================================================
// is_builtin — I/O functions
// ============================================================

#[test]
fn is_builtin_io_functions() -> Result<(), String> {
    let io = [
        "print", "printf", "say", "open", "sysopen", "close", "read", "readpipe", "sysread",
        "write", "syswrite", "binmode", "seek", "tell", "truncate", "eof", "fileno", "flock",
        "fcntl", "ioctl", "getc", "readline", "select", "sysseek",
    ];
    for name in &io {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (I/O)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — string functions
// ============================================================

#[test]
fn is_builtin_string_functions() -> Result<(), String> {
    let strings = [
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
    ];
    for name in &strings {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (string)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — array functions
// ============================================================

#[test]
fn is_builtin_array_functions() -> Result<(), String> {
    let arrays = [
        "push", "pop", "shift", "unshift", "splice", "grep", "map", "sort", "join", "split",
    ];
    for name in &arrays {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (array)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — hash functions
// ============================================================

#[test]
fn is_builtin_hash_functions() -> Result<(), String> {
    let hashes = ["each", "keys", "values", "delete", "exists"];
    for name in &hashes {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (hash)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — math functions
// ============================================================

#[test]
fn is_builtin_math_functions() -> Result<(), String> {
    let math = [
        "abs", "atan2", "cos", "exp", "hex", "int", "log", "oct", "rand", "sin", "sqrt", "srand",
    ];
    for name in &math {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (math)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — file/directory functions
// ============================================================

#[test]
fn is_builtin_file_dir_functions() -> Result<(), String> {
    let file_dir = [
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
    ];
    for name in &file_dir {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (file/dir)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — file test operators (all 27)
// ============================================================

#[test]
fn is_builtin_all_27_file_test_operators() -> Result<(), String> {
    let ops = [
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ];
    let mut count = 0;
    for op in &ops {
        if !is_builtin(op) {
            return Err(format!("File test operator {op} should be a builtin"));
        }
        count += 1;
    }
    if count != 27 {
        return Err(format!("Expected 27 file test operators, counted {count}"));
    }
    Ok(())
}

// ============================================================
// is_builtin — process functions
// ============================================================

#[test]
fn is_builtin_process_functions() -> Result<(), String> {
    let procs = [
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
    ];
    for name in &procs {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (process)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — time functions
// ============================================================

#[test]
fn is_builtin_time_functions() -> Result<(), String> {
    for name in &["gmtime", "localtime", "time"] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (time)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — network/socket functions
// ============================================================

#[test]
fn is_builtin_network_functions() -> Result<(), String> {
    let network = [
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
    ];
    for name in &network {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (network)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — system info functions
// ============================================================

#[test]
fn is_builtin_sysinfo_functions() -> Result<(), String> {
    let sysinfo = [
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
    ];
    for name in &sysinfo {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (sysinfo)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — user/group functions
// ============================================================

#[test]
fn is_builtin_user_group_functions() -> Result<(), String> {
    let user_group = [
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
    ];
    for name in &user_group {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (user/group)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — IPC functions
// ============================================================

#[test]
fn is_builtin_ipc_functions() -> Result<(), String> {
    let ipc = [
        "msgctl", "msgget", "msgrcv", "msgsnd", "semctl", "semget", "semop", "shmctl", "shmget",
        "shmread", "shmwrite",
    ];
    for name in &ipc {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (IPC)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — database/tie functions
// ============================================================

#[test]
fn is_builtin_database_functions() -> Result<(), String> {
    for name in &["dbmclose", "dbmopen", "tie", "tied", "untie"] {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (database)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — miscellaneous functions
// ============================================================

#[test]
fn is_builtin_misc_functions() -> Result<(), String> {
    let misc = [
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
    ];
    for name in &misc {
        if !is_builtin(name) {
            return Err(format!("{name} should be a builtin (misc)"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — negative cases (non-existent functions)
// ============================================================

#[test]
fn is_builtin_rejects_nonexistent_names() -> Result<(), String> {
    let invalid = [
        "",
        "not_a_builtin",
        "foobar",
        "my_function",
        "Perl::function",
        "CORE::print",
        "main::open",
    ];
    for name in &invalid {
        if is_builtin(name) {
            return Err(format!("{name:?} should NOT be a builtin"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — case sensitivity
// ============================================================

#[test]
fn is_builtin_case_sensitive_uppercase_rejected() -> Result<(), String> {
    let uppercase = [
        "PRINT", "OPEN", "CHOMP", "PUSH", "SORT", "DIE", "EVAL", "FORK",
    ];
    for name in &uppercase {
        if is_builtin(name) {
            return Err(format!("{name} (uppercase) should NOT be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_case_sensitive_titlecase_rejected() -> Result<(), String> {
    let titlecase = [
        "Print", "Open", "Chomp", "Push", "Sort", "Die", "Eval", "Fork",
    ];
    for name in &titlecase {
        if is_builtin(name) {
            return Err(format!("{name} (titlecase) should NOT be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_case_sensitive_mixed_rejected() -> Result<(), String> {
    let mixed = ["pRint", "oPen", "cHomp", "pUsH"];
    for name in &mixed {
        if is_builtin(name) {
            return Err(format!("{name} (mixed case) should NOT be a builtin"));
        }
    }
    Ok(())
}

// ============================================================
// is_builtin — whitespace and special characters
// ============================================================

#[test]
fn is_builtin_rejects_whitespace_variants() -> Result<(), String> {
    let ws = [
        " print", "print ", " print ", "\tprint", "print\n", "\nprint",
    ];
    for name in &ws {
        if is_builtin(name) {
            return Err(format!("{name:?} (whitespace) should NOT be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_rejects_partial_names() -> Result<(), String> {
    let partials = ["pri", "prin", "ope", "chom", "splic", "sub", "sor"];
    for name in &partials {
        if is_builtin(name) {
            return Err(format!("{name:?} (partial) should NOT be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_rejects_sigil_prefixed() -> Result<(), String> {
    let sigiled = ["$print", "@push", "%keys", "&sort", "*open"];
    for name in &sigiled {
        if is_builtin(name) {
            return Err(format!("{name:?} (sigil-prefixed) should NOT be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_rejects_numeric_strings() -> Result<(), String> {
    let numerics = ["0", "1", "42", "-1", "3.14", "100"];
    for name in &numerics {
        if is_builtin(name) {
            return Err(format!("{name:?} (numeric) should NOT be a builtin"));
        }
    }
    Ok(())
}

#[test]
fn is_builtin_rejects_special_chars() -> Result<(), String> {
    let specials = ["!", "@", "#", "$", "%", "^", "&", "*", "(", ")", "{}"];
    for name in &specials {
        if is_builtin(name) {
            return Err(format!("{name:?} (special) should NOT be a builtin"));
        }
    }
    Ok(())
}

// ============================================================
// get_param_names — I/O function parameters
// ============================================================

#[test]
fn params_print() -> Result<(), String> {
    let p = get_param_names("print");
    if p != ["FILEHANDLE", "LIST"] {
        return Err(format!("print params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_printf() -> Result<(), String> {
    let p = get_param_names("printf");
    if p != ["FILEHANDLE", "FORMAT", "LIST"] {
        return Err(format!("printf params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_say() -> Result<(), String> {
    let p = get_param_names("say");
    if p != ["FILEHANDLE", "LIST"] {
        return Err(format!("say params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_open() -> Result<(), String> {
    let p = get_param_names("open");
    if p != ["FILEHANDLE", "MODE", "FILENAME"] {
        return Err(format!("open params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sysopen() -> Result<(), String> {
    let p = get_param_names("sysopen");
    if p != ["FILEHANDLE", "FILENAME", "MODE", "PERMS"] {
        return Err(format!("sysopen params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_close() -> Result<(), String> {
    let p = get_param_names("close");
    if p != ["FILEHANDLE"] {
        return Err(format!("close params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_read() -> Result<(), String> {
    let p = get_param_names("read");
    if p != ["FILEHANDLE", "SCALAR", "LENGTH", "OFFSET"] {
        return Err(format!("read params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_readpipe() -> Result<(), String> {
    let p = get_param_names("readpipe");
    if p != ["EXPR"] {
        return Err(format!("readpipe params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sysread() -> Result<(), String> {
    let p = get_param_names("sysread");
    if p != ["FILEHANDLE", "SCALAR", "LENGTH", "OFFSET"] {
        return Err(format!("sysread params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_write() -> Result<(), String> {
    let p = get_param_names("write");
    if p != ["FILEHANDLE"] {
        return Err(format!("write params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_syswrite() -> Result<(), String> {
    let p = get_param_names("syswrite");
    if p != ["FILEHANDLE", "SCALAR", "LENGTH", "OFFSET"] {
        return Err(format!("syswrite params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_binmode() -> Result<(), String> {
    let p = get_param_names("binmode");
    if p != ["FILEHANDLE", "LAYER"] {
        return Err(format!("binmode params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_seek() -> Result<(), String> {
    let p = get_param_names("seek");
    if p != ["FILEHANDLE", "POSITION", "WHENCE"] {
        return Err(format!("seek params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_tell() -> Result<(), String> {
    let p = get_param_names("tell");
    if p != ["FILEHANDLE"] {
        return Err(format!("tell params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_truncate() -> Result<(), String> {
    let p = get_param_names("truncate");
    if p != ["FILEHANDLE", "LENGTH"] {
        return Err(format!("truncate params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_eof() -> Result<(), String> {
    let p = get_param_names("eof");
    if p != ["FILEHANDLE"] {
        return Err(format!("eof params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_fileno() -> Result<(), String> {
    let p = get_param_names("fileno");
    if p != ["FILEHANDLE"] {
        return Err(format!("fileno params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_flock() -> Result<(), String> {
    let p = get_param_names("flock");
    if p != ["FILEHANDLE", "OPERATION"] {
        return Err(format!("flock params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_fcntl() -> Result<(), String> {
    let p = get_param_names("fcntl");
    if p != ["FILEHANDLE", "FUNCTION", "SCALAR"] {
        return Err(format!("fcntl params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_ioctl() -> Result<(), String> {
    let p = get_param_names("ioctl");
    if p != ["FILEHANDLE", "FUNCTION", "SCALAR"] {
        return Err(format!("ioctl params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getc() -> Result<(), String> {
    let p = get_param_names("getc");
    if p != ["FILEHANDLE"] {
        return Err(format!("getc params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_readline() -> Result<(), String> {
    let p = get_param_names("readline");
    if p != ["FILEHANDLE"] {
        return Err(format!("readline params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_select() -> Result<(), String> {
    let p = get_param_names("select");
    if p != ["FILEHANDLE"] {
        return Err(format!("select params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sysseek() -> Result<(), String> {
    let p = get_param_names("sysseek");
    if p != ["FILEHANDLE", "POSITION", "WHENCE"] {
        return Err(format!("sysseek params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — string function parameters
// ============================================================

#[test]
fn params_chomp() -> Result<(), String> {
    let p = get_param_names("chomp");
    if p != ["VARIABLE"] {
        return Err(format!("chomp params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_chop() -> Result<(), String> {
    let p = get_param_names("chop");
    if p != ["VARIABLE"] {
        return Err(format!("chop params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_chr() -> Result<(), String> {
    let p = get_param_names("chr");
    if p != ["NUMBER"] {
        return Err(format!("chr params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_crypt() -> Result<(), String> {
    let p = get_param_names("crypt");
    if p != ["PLAINTEXT", "SALT"] {
        return Err(format!("crypt params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_fc() -> Result<(), String> {
    let p = get_param_names("fc");
    if p != ["EXPR"] {
        return Err(format!("fc params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_index() -> Result<(), String> {
    let p = get_param_names("index");
    if p != ["STR", "SUBSTR", "POSITION"] {
        return Err(format!("index params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_lc() -> Result<(), String> {
    let p = get_param_names("lc");
    if p != ["EXPR"] {
        return Err(format!("lc params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_lcfirst() -> Result<(), String> {
    let p = get_param_names("lcfirst");
    if p != ["EXPR"] {
        return Err(format!("lcfirst params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_length() -> Result<(), String> {
    let p = get_param_names("length");
    if p != ["EXPR"] {
        return Err(format!("length params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_ord() -> Result<(), String> {
    let p = get_param_names("ord");
    if p != ["EXPR"] {
        return Err(format!("ord params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_pack() -> Result<(), String> {
    let p = get_param_names("pack");
    if p != ["TEMPLATE", "LIST"] {
        return Err(format!("pack params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_reverse() -> Result<(), String> {
    let p = get_param_names("reverse");
    if p != ["LIST"] {
        return Err(format!("reverse params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_rindex() -> Result<(), String> {
    let p = get_param_names("rindex");
    if p != ["STR", "SUBSTR", "POSITION"] {
        return Err(format!("rindex params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sprintf() -> Result<(), String> {
    let p = get_param_names("sprintf");
    if p != ["FORMAT", "LIST"] {
        return Err(format!("sprintf params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_substr() -> Result<(), String> {
    let p = get_param_names("substr");
    if p != ["EXPR", "OFFSET", "LENGTH", "REPLACEMENT"] {
        return Err(format!("substr params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_uc() -> Result<(), String> {
    let p = get_param_names("uc");
    if p != ["EXPR"] {
        return Err(format!("uc params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_ucfirst() -> Result<(), String> {
    let p = get_param_names("ucfirst");
    if p != ["EXPR"] {
        return Err(format!("ucfirst params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_unpack() -> Result<(), String> {
    let p = get_param_names("unpack");
    if p != ["TEMPLATE", "EXPR"] {
        return Err(format!("unpack params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_quotemeta() -> Result<(), String> {
    let p = get_param_names("quotemeta");
    if p != ["EXPR"] {
        return Err(format!("quotemeta params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — array function parameters
// ============================================================

#[test]
fn params_push() -> Result<(), String> {
    let p = get_param_names("push");
    if p != ["ARRAY", "LIST"] {
        return Err(format!("push params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_pop() -> Result<(), String> {
    let p = get_param_names("pop");
    if p != ["ARRAY"] {
        return Err(format!("pop params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_shift() -> Result<(), String> {
    let p = get_param_names("shift");
    if p != ["ARRAY"] {
        return Err(format!("shift params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_unshift() -> Result<(), String> {
    let p = get_param_names("unshift");
    if p != ["ARRAY", "LIST"] {
        return Err(format!("unshift params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_splice() -> Result<(), String> {
    let p = get_param_names("splice");
    if p != ["ARRAY", "OFFSET", "LENGTH", "LIST"] {
        return Err(format!("splice params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_grep() -> Result<(), String> {
    let p = get_param_names("grep");
    if p != ["BLOCK", "LIST"] {
        return Err(format!("grep params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_map() -> Result<(), String> {
    let p = get_param_names("map");
    if p != ["BLOCK", "LIST"] {
        return Err(format!("map params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sort() -> Result<(), String> {
    let p = get_param_names("sort");
    if p != ["BLOCK", "LIST"] {
        return Err(format!("sort params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_join() -> Result<(), String> {
    let p = get_param_names("join");
    if p != ["EXPR", "LIST"] {
        return Err(format!("join params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_split() -> Result<(), String> {
    let p = get_param_names("split");
    if p != ["PATTERN", "EXPR", "LIMIT"] {
        return Err(format!("split params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — hash function parameters
// ============================================================

#[test]
fn params_each() -> Result<(), String> {
    let p = get_param_names("each");
    if p != ["HASH"] {
        return Err(format!("each params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_keys() -> Result<(), String> {
    let p = get_param_names("keys");
    if p != ["HASH"] {
        return Err(format!("keys params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_values() -> Result<(), String> {
    let p = get_param_names("values");
    if p != ["HASH"] {
        return Err(format!("values params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_delete() -> Result<(), String> {
    let p = get_param_names("delete");
    if p != ["EXPR"] {
        return Err(format!("delete params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_exists() -> Result<(), String> {
    let p = get_param_names("exists");
    if p != ["EXPR"] {
        return Err(format!("exists params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — math function parameters
// ============================================================

#[test]
fn params_abs() -> Result<(), String> {
    let p = get_param_names("abs");
    if p != ["VALUE"] {
        return Err(format!("abs params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_atan2() -> Result<(), String> {
    let p = get_param_names("atan2");
    if p != ["Y", "X"] {
        return Err(format!("atan2 params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_cos() -> Result<(), String> {
    let p = get_param_names("cos");
    if p != ["EXPR"] {
        return Err(format!("cos params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_exp() -> Result<(), String> {
    let p = get_param_names("exp");
    if p != ["EXPR"] {
        return Err(format!("exp params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_hex() -> Result<(), String> {
    let p = get_param_names("hex");
    if p != ["EXPR"] {
        return Err(format!("hex params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_int() -> Result<(), String> {
    let p = get_param_names("int");
    if p != ["EXPR"] {
        return Err(format!("int params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_log() -> Result<(), String> {
    let p = get_param_names("log");
    if p != ["EXPR"] {
        return Err(format!("log params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_oct() -> Result<(), String> {
    let p = get_param_names("oct");
    if p != ["EXPR"] {
        return Err(format!("oct params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_rand() -> Result<(), String> {
    let p = get_param_names("rand");
    if p != ["EXPR"] {
        return Err(format!("rand params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sin() -> Result<(), String> {
    let p = get_param_names("sin");
    if p != ["EXPR"] {
        return Err(format!("sin params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sqrt() -> Result<(), String> {
    let p = get_param_names("sqrt");
    if p != ["EXPR"] {
        return Err(format!("sqrt params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_srand() -> Result<(), String> {
    let p = get_param_names("srand");
    if p != ["EXPR"] {
        return Err(format!("srand params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — file/directory function parameters
// ============================================================

#[test]
fn params_chdir() -> Result<(), String> {
    let p = get_param_names("chdir");
    if p != ["EXPR"] {
        return Err(format!("chdir params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_chroot() -> Result<(), String> {
    let p = get_param_names("chroot");
    if p != ["FILENAME"] {
        return Err(format!("chroot params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_chmod() -> Result<(), String> {
    let p = get_param_names("chmod");
    if p != ["MODE", "LIST"] {
        return Err(format!("chmod params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_chown() -> Result<(), String> {
    let p = get_param_names("chown");
    if p != ["UID", "GID", "LIST"] {
        return Err(format!("chown params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_link() -> Result<(), String> {
    let p = get_param_names("link");
    if p != ["OLDFILE", "NEWFILE"] {
        return Err(format!("link params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_lstat() -> Result<(), String> {
    let p = get_param_names("lstat");
    if p != ["FILEHANDLE"] {
        return Err(format!("lstat params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_mkdir() -> Result<(), String> {
    let p = get_param_names("mkdir");
    if p != ["FILENAME", "MODE"] {
        return Err(format!("mkdir params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_opendir() -> Result<(), String> {
    let p = get_param_names("opendir");
    if p != ["DIRHANDLE", "EXPR"] {
        return Err(format!("opendir params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_readdir() -> Result<(), String> {
    let p = get_param_names("readdir");
    if p != ["DIRHANDLE"] {
        return Err(format!("readdir params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_readlink() -> Result<(), String> {
    let p = get_param_names("readlink");
    if p != ["EXPR"] {
        return Err(format!("readlink params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_glob() -> Result<(), String> {
    let p = get_param_names("glob");
    if p != ["EXPR"] {
        return Err(format!("glob params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_rename() -> Result<(), String> {
    let p = get_param_names("rename");
    if p != ["OLDNAME", "NEWNAME"] {
        return Err(format!("rename params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_rmdir() -> Result<(), String> {
    let p = get_param_names("rmdir");
    if p != ["FILENAME"] {
        return Err(format!("rmdir params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_stat() -> Result<(), String> {
    let p = get_param_names("stat");
    if p != ["FILEHANDLE"] {
        return Err(format!("stat params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_symlink() -> Result<(), String> {
    let p = get_param_names("symlink");
    if p != ["OLDFILE", "NEWFILE"] {
        return Err(format!("symlink params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_umask() -> Result<(), String> {
    let p = get_param_names("umask");
    if p != ["EXPR"] {
        return Err(format!("umask params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_unlink() -> Result<(), String> {
    let p = get_param_names("unlink");
    if p != ["LIST"] {
        return Err(format!("unlink params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_utime() -> Result<(), String> {
    let p = get_param_names("utime");
    if p != ["ATIME", "MTIME", "LIST"] {
        return Err(format!("utime params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_closedir() -> Result<(), String> {
    let p = get_param_names("closedir");
    if p != ["DIRHANDLE"] {
        return Err(format!("closedir params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_rewinddir() -> Result<(), String> {
    let p = get_param_names("rewinddir");
    if p != ["DIRHANDLE"] {
        return Err(format!("rewinddir params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_seekdir() -> Result<(), String> {
    let p = get_param_names("seekdir");
    if p != ["DIRHANDLE", "POS"] {
        return Err(format!("seekdir params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_telldir() -> Result<(), String> {
    let p = get_param_names("telldir");
    if p != ["DIRHANDLE"] {
        return Err(format!("telldir params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — file test operators (all return ["FILE"])
// ============================================================

#[test]
fn params_all_file_test_operators_return_file() -> Result<(), String> {
    let ops = [
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ];
    for op in &ops {
        let p = get_param_names(op);
        if p.len() != 1 || p[0] != "FILE" {
            return Err(format!("{op} should have params [\"FILE\"], got {p:?}"));
        }
    }
    Ok(())
}

// ============================================================
// get_param_names — process function parameters
// ============================================================

#[test]
fn params_alarm() -> Result<(), String> {
    let p = get_param_names("alarm");
    if p != ["SECONDS"] {
        return Err(format!("alarm params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_exec() -> Result<(), String> {
    let p = get_param_names("exec");
    if p != ["PROGRAM", "LIST"] {
        return Err(format!("exec params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_fork() -> Result<(), String> {
    let p = get_param_names("fork");
    if !p.is_empty() {
        return Err(format!("fork should have 0 params, got {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getpgrp() -> Result<(), String> {
    let p = get_param_names("getpgrp");
    if p != ["PID"] {
        return Err(format!("getpgrp params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getppid() -> Result<(), String> {
    let p = get_param_names("getppid");
    if !p.is_empty() {
        return Err(format!("getppid should have 0 params, got {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getpriority() -> Result<(), String> {
    let p = get_param_names("getpriority");
    if p != ["WHICH", "WHO"] {
        return Err(format!("getpriority params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_kill() -> Result<(), String> {
    let p = get_param_names("kill");
    if p != ["SIGNAL", "LIST"] {
        return Err(format!("kill params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_pipe() -> Result<(), String> {
    let p = get_param_names("pipe");
    if p != ["READHANDLE", "WRITEHANDLE"] {
        return Err(format!("pipe params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_setpgrp() -> Result<(), String> {
    let p = get_param_names("setpgrp");
    if p != ["PID", "PGRP"] {
        return Err(format!("setpgrp params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_setpriority() -> Result<(), String> {
    let p = get_param_names("setpriority");
    if p != ["WHICH", "WHO", "PRIORITY"] {
        return Err(format!("setpriority params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_syscall() -> Result<(), String> {
    let p = get_param_names("syscall");
    if p != ["NUMBER", "LIST"] {
        return Err(format!("syscall params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sleep() -> Result<(), String> {
    let p = get_param_names("sleep");
    if p != ["EXPR"] {
        return Err(format!("sleep params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_system() -> Result<(), String> {
    let p = get_param_names("system");
    if p != ["PROGRAM", "LIST"] {
        return Err(format!("system params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_times() -> Result<(), String> {
    let p = get_param_names("times");
    if !p.is_empty() {
        return Err(format!("times should have 0 params, got {p:?}"));
    }
    Ok(())
}

#[test]
fn params_wait() -> Result<(), String> {
    let p = get_param_names("wait");
    if !p.is_empty() {
        return Err(format!("wait should have 0 params, got {p:?}"));
    }
    Ok(())
}

#[test]
fn params_waitpid() -> Result<(), String> {
    let p = get_param_names("waitpid");
    if p != ["PID", "FLAGS"] {
        return Err(format!("waitpid params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — time function parameters
// ============================================================

#[test]
fn params_gmtime() -> Result<(), String> {
    let p = get_param_names("gmtime");
    if p != ["EXPR"] {
        return Err(format!("gmtime params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_localtime() -> Result<(), String> {
    let p = get_param_names("localtime");
    if p != ["EXPR"] {
        return Err(format!("localtime params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_time() -> Result<(), String> {
    let p = get_param_names("time");
    if !p.is_empty() {
        return Err(format!("time should have 0 params, got {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — network function parameters
// ============================================================

#[test]
fn params_accept() -> Result<(), String> {
    let p = get_param_names("accept");
    if p != ["NEWSOCKET", "GENERICSOCKET"] {
        return Err(format!("accept params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_bind() -> Result<(), String> {
    let p = get_param_names("bind");
    if p != ["SOCKET", "NAME"] {
        return Err(format!("bind params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_connect() -> Result<(), String> {
    let p = get_param_names("connect");
    if p != ["SOCKET", "NAME"] {
        return Err(format!("connect params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getpeername() -> Result<(), String> {
    let p = get_param_names("getpeername");
    if p != ["SOCKET"] {
        return Err(format!("getpeername params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getsockname() -> Result<(), String> {
    let p = get_param_names("getsockname");
    if p != ["SOCKET"] {
        return Err(format!("getsockname params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getsockopt() -> Result<(), String> {
    let p = get_param_names("getsockopt");
    if p != ["SOCKET", "LEVEL", "OPTNAME"] {
        return Err(format!("getsockopt params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_listen() -> Result<(), String> {
    let p = get_param_names("listen");
    if p != ["SOCKET", "QUEUESIZE"] {
        return Err(format!("listen params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_recv() -> Result<(), String> {
    let p = get_param_names("recv");
    if p != ["SOCKET", "SCALAR", "LENGTH", "FLAGS"] {
        return Err(format!("recv params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_send() -> Result<(), String> {
    let p = get_param_names("send");
    if p != ["SOCKET", "MSG", "FLAGS", "TO"] {
        return Err(format!("send params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_setsockopt() -> Result<(), String> {
    let p = get_param_names("setsockopt");
    if p != ["SOCKET", "LEVEL", "OPTNAME", "OPTVAL"] {
        return Err(format!("setsockopt params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_shutdown() -> Result<(), String> {
    let p = get_param_names("shutdown");
    if p != ["SOCKET", "HOW"] {
        return Err(format!("shutdown params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_socket() -> Result<(), String> {
    let p = get_param_names("socket");
    if p != ["SOCKET", "DOMAIN", "TYPE", "PROTOCOL"] {
        return Err(format!("socket params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_socketpair() -> Result<(), String> {
    let p = get_param_names("socketpair");
    if p != ["SOCKET1", "SOCKET2", "DOMAIN", "TYPE", "PROTOCOL"] {
        return Err(format!("socketpair params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sockatmark() -> Result<(), String> {
    let p = get_param_names("sockatmark");
    if p != ["SOCKET"] {
        return Err(format!("sockatmark params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — system info functions (zero-param and with params)
// ============================================================

#[test]
fn params_gethostbyaddr() -> Result<(), String> {
    let p = get_param_names("gethostbyaddr");
    if p != ["ADDR", "ADDRTYPE"] {
        return Err(format!("gethostbyaddr params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_gethostbyname() -> Result<(), String> {
    let p = get_param_names("gethostbyname");
    if p != ["NAME"] {
        return Err(format!("gethostbyname params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_sysinfo_zero_param() -> Result<(), String> {
    let zero_param = [
        "gethostent",
        "getnetent",
        "getprotoent",
        "getservent",
        "endhostent",
        "endnetent",
        "endprotoent",
        "endservent",
    ];
    for name in &zero_param {
        let p = get_param_names(name);
        if !p.is_empty() {
            return Err(format!("{name} should have 0 params, got {p:?}"));
        }
    }
    Ok(())
}

#[test]
fn params_sysinfo_stayopen_funcs() -> Result<(), String> {
    let stayopen = ["sethostent", "setnetent", "setprotoent", "setservent"];
    for name in &stayopen {
        let p = get_param_names(name);
        if p != ["STAYOPEN"] {
            return Err(format!("{name} should have [\"STAYOPEN\"], got {p:?}"));
        }
    }
    Ok(())
}

#[test]
fn params_getnetbyaddr() -> Result<(), String> {
    let p = get_param_names("getnetbyaddr");
    if p != ["ADDR", "ADDRTYPE"] {
        return Err(format!("getnetbyaddr params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getnetbyname() -> Result<(), String> {
    let p = get_param_names("getnetbyname");
    if p != ["NAME"] {
        return Err(format!("getnetbyname params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getprotobyname() -> Result<(), String> {
    let p = get_param_names("getprotobyname");
    if p != ["NAME"] {
        return Err(format!("getprotobyname params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getprotobynumber() -> Result<(), String> {
    let p = get_param_names("getprotobynumber");
    if p != ["NUMBER"] {
        return Err(format!("getprotobynumber params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getservbyname() -> Result<(), String> {
    let p = get_param_names("getservbyname");
    if p != ["NAME", "PROTO"] {
        return Err(format!("getservbyname params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getservbyport() -> Result<(), String> {
    let p = get_param_names("getservbyport");
    if p != ["PORT", "PROTO"] {
        return Err(format!("getservbyport params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — user/group function parameters
// ============================================================

#[test]
fn params_user_group_zero_param() -> Result<(), String> {
    let zero_param = [
        "getgrent",
        "getlogin",
        "getuid",
        "geteuid",
        "getgid",
        "getegid",
        "getgroups",
        "getpwent",
        "setgrent",
        "setpwent",
        "endgrent",
        "endpwent",
    ];
    for name in &zero_param {
        let p = get_param_names(name);
        if !p.is_empty() {
            return Err(format!("{name} should have 0 params, got {p:?}"));
        }
    }
    Ok(())
}

#[test]
fn params_getgrgid() -> Result<(), String> {
    let p = get_param_names("getgrgid");
    if p != ["GID"] {
        return Err(format!("getgrgid params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getgrnam() -> Result<(), String> {
    let p = get_param_names("getgrnam");
    if p != ["NAME"] {
        return Err(format!("getgrnam params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_setuid() -> Result<(), String> {
    let p = get_param_names("setuid");
    if p != ["UID"] {
        return Err(format!("setuid params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_seteuid() -> Result<(), String> {
    let p = get_param_names("seteuid");
    if p != ["UID"] {
        return Err(format!("seteuid params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_setgid() -> Result<(), String> {
    let p = get_param_names("setgid");
    if p != ["GID"] {
        return Err(format!("setgid params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_setegid() -> Result<(), String> {
    let p = get_param_names("setegid");
    if p != ["GID"] {
        return Err(format!("setegid params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_setgroups() -> Result<(), String> {
    let p = get_param_names("setgroups");
    if p != ["LIST"] {
        return Err(format!("setgroups params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getpwnam() -> Result<(), String> {
    let p = get_param_names("getpwnam");
    if p != ["NAME"] {
        return Err(format!("getpwnam params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_getpwuid() -> Result<(), String> {
    let p = get_param_names("getpwuid");
    if p != ["UID"] {
        return Err(format!("getpwuid params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — IPC function parameters
// ============================================================

#[test]
fn params_msgctl() -> Result<(), String> {
    let p = get_param_names("msgctl");
    if p != ["ID", "CMD", "ARG"] {
        return Err(format!("msgctl params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_msgget() -> Result<(), String> {
    let p = get_param_names("msgget");
    if p != ["KEY", "FLAGS"] {
        return Err(format!("msgget params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_msgrcv() -> Result<(), String> {
    let p = get_param_names("msgrcv");
    if p != ["ID", "VAR", "SIZE", "TYPE", "FLAGS"] {
        return Err(format!("msgrcv params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_msgsnd() -> Result<(), String> {
    let p = get_param_names("msgsnd");
    if p != ["ID", "MSG", "FLAGS"] {
        return Err(format!("msgsnd params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_semctl() -> Result<(), String> {
    let p = get_param_names("semctl");
    if p != ["ID", "SEMNUM", "CMD", "ARG"] {
        return Err(format!("semctl params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_semget() -> Result<(), String> {
    let p = get_param_names("semget");
    if p != ["KEY", "NSEMS", "FLAGS"] {
        return Err(format!("semget params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_semop() -> Result<(), String> {
    let p = get_param_names("semop");
    if p != ["ID", "OPSTRING"] {
        return Err(format!("semop params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_shmctl() -> Result<(), String> {
    let p = get_param_names("shmctl");
    if p != ["ID", "CMD", "ARG"] {
        return Err(format!("shmctl params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_shmget() -> Result<(), String> {
    let p = get_param_names("shmget");
    if p != ["KEY", "SIZE", "FLAGS"] {
        return Err(format!("shmget params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_shmread() -> Result<(), String> {
    let p = get_param_names("shmread");
    if p != ["ID", "VAR", "POS", "SIZE"] {
        return Err(format!("shmread params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_shmwrite() -> Result<(), String> {
    let p = get_param_names("shmwrite");
    if p != ["ID", "STRING", "POS", "SIZE"] {
        return Err(format!("shmwrite params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — database/tie function parameters
// ============================================================

#[test]
fn params_dbmclose() -> Result<(), String> {
    let p = get_param_names("dbmclose");
    if p != ["HASH"] {
        return Err(format!("dbmclose params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_dbmopen() -> Result<(), String> {
    let p = get_param_names("dbmopen");
    if p != ["HASH", "DBNAME", "MODE"] {
        return Err(format!("dbmopen params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_tie() -> Result<(), String> {
    let p = get_param_names("tie");
    if p != ["VARIABLE", "CLASSNAME", "LIST"] {
        return Err(format!("tie params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_tied() -> Result<(), String> {
    let p = get_param_names("tied");
    if p != ["VARIABLE"] {
        return Err(format!("tied params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_untie() -> Result<(), String> {
    let p = get_param_names("untie");
    if p != ["VARIABLE"] {
        return Err(format!("untie params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — miscellaneous function parameters
// ============================================================

#[test]
fn params_bless() -> Result<(), String> {
    let p = get_param_names("bless");
    if p != ["REF", "CLASSNAME"] {
        return Err(format!("bless params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_caller() -> Result<(), String> {
    let p = get_param_names("caller");
    if p != ["EXPR"] {
        return Err(format!("caller params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_die() -> Result<(), String> {
    let p = get_param_names("die");
    if p != ["LIST"] {
        return Err(format!("die params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_do() -> Result<(), String> {
    let p = get_param_names("do");
    if p != ["BLOCK"] {
        return Err(format!("do params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_eval() -> Result<(), String> {
    let p = get_param_names("eval");
    if p != ["EXPR"] {
        return Err(format!("eval params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_exit() -> Result<(), String> {
    let p = get_param_names("exit");
    if p != ["EXPR"] {
        return Err(format!("exit params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_goto() -> Result<(), String> {
    let p = get_param_names("goto");
    if p != ["LABEL"] {
        return Err(format!("goto params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_last() -> Result<(), String> {
    let p = get_param_names("last");
    if p != ["LABEL"] {
        return Err(format!("last params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_next() -> Result<(), String> {
    let p = get_param_names("next");
    if p != ["LABEL"] {
        return Err(format!("next params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_redo() -> Result<(), String> {
    let p = get_param_names("redo");
    if p != ["LABEL"] {
        return Err(format!("redo params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_ref() -> Result<(), String> {
    let p = get_param_names("ref");
    if p != ["EXPR"] {
        return Err(format!("ref params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_require() -> Result<(), String> {
    let p = get_param_names("require");
    if p != ["VERSION"] {
        return Err(format!("require params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_return() -> Result<(), String> {
    let p = get_param_names("return");
    if p != ["LIST"] {
        return Err(format!("return params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_scalar() -> Result<(), String> {
    let p = get_param_names("scalar");
    if p != ["EXPR"] {
        return Err(format!("scalar params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_undef() -> Result<(), String> {
    let p = get_param_names("undef");
    if p != ["EXPR"] {
        return Err(format!("undef params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_wantarray() -> Result<(), String> {
    let p = get_param_names("wantarray");
    if !p.is_empty() {
        return Err(format!("wantarray should have 0 params, got {p:?}"));
    }
    Ok(())
}

#[test]
fn params_warn() -> Result<(), String> {
    let p = get_param_names("warn");
    if p != ["LIST"] {
        return Err(format!("warn params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_defined() -> Result<(), String> {
    let p = get_param_names("defined");
    if p != ["EXPR"] {
        return Err(format!("defined params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_dump() -> Result<(), String> {
    let p = get_param_names("dump");
    if p != ["LABEL"] {
        return Err(format!("dump params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_formline() -> Result<(), String> {
    let p = get_param_names("formline");
    if p != ["PICTURE", "LIST"] {
        return Err(format!("formline params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_local() -> Result<(), String> {
    let p = get_param_names("local");
    if p != ["EXPR"] {
        return Err(format!("local params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_my() -> Result<(), String> {
    let p = get_param_names("my");
    if p != ["VARLIST"] {
        return Err(format!("my params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_our() -> Result<(), String> {
    let p = get_param_names("our");
    if p != ["VARLIST"] {
        return Err(format!("our params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_state() -> Result<(), String> {
    let p = get_param_names("state");
    if p != ["VARLIST"] {
        return Err(format!("state params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_reset() -> Result<(), String> {
    let p = get_param_names("reset");
    if p != ["EXPR"] {
        return Err(format!("reset params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_study() -> Result<(), String> {
    let p = get_param_names("study");
    if p != ["SCALAR"] {
        return Err(format!("study params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_pos() -> Result<(), String> {
    let p = get_param_names("pos");
    if p != ["SCALAR"] {
        return Err(format!("pos params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_use() -> Result<(), String> {
    let p = get_param_names("use");
    if p != ["MODULE", "VERSION", "LIST"] {
        return Err(format!("use params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_vec() -> Result<(), String> {
    let p = get_param_names("vec");
    if p != ["EXPR", "OFFSET", "BITS"] {
        return Err(format!("vec params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_lock() -> Result<(), String> {
    let p = get_param_names("lock");
    if p != ["THING"] {
        return Err(format!("lock params: {p:?}"));
    }
    Ok(())
}

#[test]
fn params_prototype() -> Result<(), String> {
    let p = get_param_names("prototype");
    if p != ["FUNCTION"] {
        return Err(format!("prototype params: {p:?}"));
    }
    Ok(())
}

// ============================================================
// get_param_names — unknown/edge cases
// ============================================================

#[test]
fn params_unknown_returns_empty() -> Result<(), String> {
    let p = get_param_names("not_a_builtin");
    if !p.is_empty() {
        return Err(format!("Expected empty for unknown, got {p:?}"));
    }
    Ok(())
}

#[test]
fn params_empty_string_returns_empty() -> Result<(), String> {
    let p = get_param_names("");
    if !p.is_empty() {
        return Err(format!("Expected empty for empty string, got {p:?}"));
    }
    Ok(())
}

#[test]
fn params_whitespace_returns_empty() -> Result<(), String> {
    let p = get_param_names(" ");
    if !p.is_empty() {
        return Err(format!("Expected empty for whitespace, got {p:?}"));
    }
    Ok(())
}

#[test]
fn params_newline_returns_empty() -> Result<(), String> {
    let p = get_param_names("\n");
    if !p.is_empty() {
        return Err(format!("Expected empty for newline, got {p:?}"));
    }
    Ok(())
}

#[test]
fn get_param_names_is_idempotent() -> Result<(), String> {
    let p1 = get_param_names("print");
    let p2 = get_param_names("print");
    if p1 != p2 {
        return Err("get_param_names should be idempotent".into());
    }
    Ok(())
}

// ============================================================
// get_param_names — parameter count verification
// ============================================================

#[test]
fn param_count_0_params() -> Result<(), String> {
    let zero = [
        "fork",
        "getppid",
        "times",
        "wait",
        "time",
        "wantarray",
        "gethostent",
        "getnetent",
        "getprotoent",
        "getservent",
        "endhostent",
        "endnetent",
        "endprotoent",
        "endservent",
        "getgrent",
        "getlogin",
        "getuid",
        "geteuid",
        "getgid",
        "getegid",
        "getgroups",
        "getpwent",
        "setgrent",
        "setpwent",
        "endgrent",
        "endpwent",
    ];
    for name in &zero {
        if !get_param_names(name).is_empty() {
            return Err(format!("{name} should have 0 params"));
        }
    }
    Ok(())
}

#[test]
fn param_count_1_param() -> Result<(), String> {
    let one = [
        "close",
        "tell",
        "eof",
        "fileno",
        "getc",
        "readline",
        "select",
        "write",
        "readpipe",
        "chomp",
        "chop",
        "chr",
        "fc",
        "lc",
        "lcfirst",
        "length",
        "ord",
        "reverse",
        "uc",
        "ucfirst",
        "quotemeta",
        "pop",
        "shift",
        "each",
        "keys",
        "values",
        "delete",
        "exists",
        "abs",
        "cos",
        "exp",
        "hex",
        "int",
        "log",
        "oct",
        "rand",
        "sin",
        "sqrt",
        "srand",
        "chdir",
        "chroot",
        "readlink",
        "glob",
        "rmdir",
        "umask",
        "readdir",
        "closedir",
        "rewinddir",
        "telldir",
        "alarm",
        "sleep",
        "getpgrp",
        "gmtime",
        "localtime",
        "getpeername",
        "getsockname",
        "sockatmark",
        "gethostbyname",
        "getnetbyname",
        "getprotobyname",
        "getprotobynumber",
        "getgrgid",
        "getgrnam",
        "getpwnam",
        "getpwuid",
        "dbmclose",
        "tied",
        "untie",
        "caller",
        "eval",
        "exit",
        "goto",
        "last",
        "next",
        "redo",
        "ref",
        "require",
        "scalar",
        "undef",
        "defined",
        "dump",
        "local",
        "reset",
        "study",
        "pos",
        "lock",
        "prototype",
    ];
    for name in &one {
        let p = get_param_names(name);
        if p.len() != 1 {
            return Err(format!(
                "{name} should have 1 param, got {} ({p:?})",
                p.len()
            ));
        }
    }
    Ok(())
}

#[test]
fn param_count_2_params() -> Result<(), String> {
    let two = [
        "print",
        "say",
        "binmode",
        "truncate",
        "flock",
        "crypt",
        "pack",
        "sprintf",
        "unpack",
        "push",
        "unshift",
        "grep",
        "map",
        "sort",
        "join",
        "chmod",
        "link",
        "symlink",
        "rename",
        "opendir",
        "seekdir",
        "mkdir",
        "exec",
        "system",
        "kill",
        "pipe",
        "setpgrp",
        "waitpid",
        "syscall",
        "bind",
        "connect",
        "listen",
        "shutdown",
        "gethostbyaddr",
        "getnetbyaddr",
        "getservbyname",
        "getservbyport",
        "formline",
        "atan2",
        "semop",
        "msgget",
        "bless",
        "getpriority",
    ];
    for name in &two {
        let p = get_param_names(name);
        if p.len() != 2 {
            return Err(format!(
                "{name} should have 2 params, got {} ({p:?})",
                p.len()
            ));
        }
    }
    Ok(())
}

#[test]
fn param_count_3_params() -> Result<(), String> {
    let three = [
        "printf",
        "open",
        "seek",
        "sysseek",
        "fcntl",
        "ioctl",
        "index",
        "rindex",
        "split",
        "chown",
        "utime",
        "setpriority",
        "getsockopt",
        "dbmopen",
        "tie",
        "msgctl",
        "shmctl",
        "semget",
        "msgsnd",
        "use",
        "vec",
    ];
    for name in &three {
        let p = get_param_names(name);
        if p.len() != 3 {
            return Err(format!(
                "{name} should have 3 params, got {} ({p:?})",
                p.len()
            ));
        }
    }
    Ok(())
}

#[test]
fn param_count_4_params() -> Result<(), String> {
    let four = [
        "sysopen",
        "read",
        "sysread",
        "syswrite",
        "substr",
        "splice",
        "recv",
        "send",
        "setsockopt",
        "socket",
        "shmread",
        "shmwrite",
        "semctl",
    ];
    for name in &four {
        let p = get_param_names(name);
        if p.len() != 4 {
            return Err(format!(
                "{name} should have 4 params, got {} ({p:?})",
                p.len()
            ));
        }
    }
    Ok(())
}

#[test]
fn param_count_5_params() -> Result<(), String> {
    let five = ["socketpair", "msgrcv"];
    for name in &five {
        let p = get_param_names(name);
        if p.len() != 5 {
            return Err(format!(
                "{name} should have 5 params, got {} ({p:?})",
                p.len()
            ));
        }
    }
    Ok(())
}

// ============================================================
// Data integrity — all keys non-empty, all params uppercase
// ============================================================

#[test]
fn all_keys_are_nonempty() -> Result<(), String> {
    for (key, _) in BUILTIN_SIGS.entries() {
        if key.is_empty() {
            return Err("Found empty key in BUILTIN_SIGS".into());
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

#[test]
fn all_param_names_are_ascii() -> Result<(), String> {
    for (name, params) in BUILTIN_SIGS.entries() {
        for param in *params {
            if !param.is_ascii() {
                return Err(format!("{name} param {param} is not ASCII"));
            }
        }
    }
    Ok(())
}

// ============================================================
// BUILTIN_FULL_SIGS — variant content and ordering
// ============================================================

#[test]
fn full_sigs_all_entries_exist_in_builtin_sigs() -> Result<(), String> {
    for (name, _) in BUILTIN_FULL_SIGS.entries() {
        if !is_builtin(name) {
            return Err(format!(
                "BUILTIN_FULL_SIGS has {name} but it is missing from BUILTIN_SIGS"
            ));
        }
    }
    Ok(())
}

#[test]
fn full_sigs_all_variants_start_with_function_name() -> Result<(), String> {
    for (name, sigs) in BUILTIN_FULL_SIGS.entries() {
        for sig in *sigs {
            let Some(rest) = sig.strip_prefix(name) else {
                return Err(format!(
                    "Full sig {sig:?} for {name} should start with the function name"
                ));
            };
            if let Some(next) = rest.chars().next() {
                if next.is_ascii_alphanumeric() || next == '_' {
                    return Err(format!(
                        "Full sig {sig:?} for {name} should keep the builtin name as a token boundary"
                    ));
                }
            }
        }
    }
    Ok(())
}

#[test]
fn full_sigs_all_variants_are_nonempty() -> Result<(), String> {
    for (name, sigs) in BUILTIN_FULL_SIGS.entries() {
        for sig in *sigs {
            if sig.is_empty() {
                return Err(format!("{name} has an empty full signature variant"));
            }
        }
    }
    Ok(())
}

#[test]
fn full_sigs_all_variants_are_ascii() -> Result<(), String> {
    for (name, sigs) in BUILTIN_FULL_SIGS.entries() {
        for sig in *sigs {
            if !sig.is_ascii() {
                return Err(format!(
                    "{name} full sig variant contains non-ASCII: {sig:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn full_sigs_print_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("print") {
        Some(sigs) => {
            if sigs.len() != 4 {
                return Err(format!("print should have 4 full sigs, got {}", sigs.len()));
            }
            if sigs[0] != "print FILEHANDLE LIST" {
                return Err(format!("print first variant wrong: {}", sigs[0]));
            }
        }
        None => return Err("print missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_open_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("open") {
        Some(sigs) => {
            if sigs.len() != 3 {
                return Err(format!("open should have 3 full sigs, got {}", sigs.len()));
            }
            if sigs[0] != "open FILEHANDLE, MODE, FILENAME" {
                return Err(format!("open first variant wrong: {}", sigs[0]));
            }
        }
        None => return Err("open missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_substr_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("substr") {
        Some(sigs) => {
            if sigs.len() != 3 {
                return Err(format!(
                    "substr should have 3 full sigs, got {}",
                    sigs.len()
                ));
            }
        }
        None => return Err("substr missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_splice_has_four_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("splice") {
        Some(sigs) => {
            if sigs.len() != 4 {
                return Err(format!(
                    "splice should have 4 full sigs, got {}",
                    sigs.len()
                ));
            }
        }
        None => return Err("splice missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_split_has_four_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("split") {
        Some(sigs) => {
            if sigs.len() != 4 {
                return Err(format!("split should have 4 full sigs, got {}", sigs.len()));
            }
        }
        None => return Err("split missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_system_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("system") {
        Some(sigs) => {
            if sigs.len() != 2 {
                return Err(format!(
                    "system should have 2 full sigs, got {}",
                    sigs.len()
                ));
            }
            if sigs[0] != "system PROGRAM, LIST" {
                return Err(format!("system first variant wrong: {}", sigs[0]));
            }
        }
        None => return Err("system missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_close_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("close") {
        Some(sigs) => {
            if sigs.len() != 2 {
                return Err(format!("close should have 2 full sigs, got {}", sigs.len()));
            }
        }
        None => return Err("close missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_say_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("say") {
        Some(sigs) => {
            if sigs.len() != 4 {
                return Err(format!("say should have 4 full sigs, got {}", sigs.len()));
            }
        }
        None => return Err("say missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_map_has_two_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("map") {
        Some(sigs) => {
            if sigs.len() != 2 {
                return Err(format!("map should have 2 full sigs, got {}", sigs.len()));
            }
            if sigs[0] != "map BLOCK LIST" {
                return Err(format!("map first variant wrong: {}", sigs[0]));
            }
        }
        None => return Err("map missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_grep_has_two_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("grep") {
        Some(sigs) => {
            if sigs.len() != 2 {
                return Err(format!("grep should have 2 full sigs, got {}", sigs.len()));
            }
            if sigs[0] != "grep BLOCK LIST" {
                return Err(format!("grep first variant wrong: {}", sigs[0]));
            }
        }
        None => return Err("grep missing from BUILTIN_FULL_SIGS".into()),
    }
    Ok(())
}

#[test]
fn full_sigs_sort_has_three_variants() -> Result<(), String> {
    match BUILTIN_FULL_SIGS.get("sort") {
        Some(sigs) => {
            if sigs.len() != 3 {
                return Err(format!("sort should have 3 full sigs, got {}", sigs.len()));
            }
        }
        None => return Err("sort missing from BUILTIN_FULL_SIGS".into()),
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

#[test]
fn full_sigs_case_sensitive() -> Result<(), String> {
    if BUILTIN_FULL_SIGS.get("Print").is_some() {
        return Err("BUILTIN_FULL_SIGS should be case-sensitive".into());
    }
    if BUILTIN_FULL_SIGS.get("PRINT").is_some() {
        return Err("BUILTIN_FULL_SIGS should be case-sensitive".into());
    }
    Ok(())
}

// ============================================================
// Semantic patterns — first param conventions
// ============================================================

#[test]
fn io_functions_have_filehandle_first() -> Result<(), String> {
    let io = [
        "print", "printf", "say", "open", "close", "read", "sysread", "write", "syswrite",
        "binmode", "seek", "tell", "truncate", "eof", "fileno", "flock", "fcntl", "ioctl", "getc",
        "readline", "select", "sysseek",
    ];
    for name in &io {
        let p = get_param_names(name);
        match p.first() {
            Some(&"FILEHANDLE") => {}
            other => {
                return Err(format!(
                    "{name} first param should be FILEHANDLE, got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn directory_functions_have_dirhandle_first() -> Result<(), String> {
    let dirs = [
        "opendir",
        "readdir",
        "closedir",
        "rewinddir",
        "seekdir",
        "telldir",
    ];
    for name in &dirs {
        let p = get_param_names(name);
        match p.first() {
            Some(&"DIRHANDLE") => {}
            other => {
                return Err(format!(
                    "{name} first param should be DIRHANDLE, got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn socket_functions_have_socket_first() -> Result<(), String> {
    let socks = [
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
        "sockatmark",
    ];
    for name in &socks {
        let p = get_param_names(name);
        match p.first() {
            Some(first) if first.contains("SOCKET") => {}
            other => {
                return Err(format!(
                    "{name} first param should contain SOCKET, got {other:?}"
                ));
            }
        }
    }
    Ok(())
}

// ============================================================
// index/rindex symmetry
// ============================================================

#[test]
fn index_and_rindex_have_same_params() -> Result<(), String> {
    let ip = get_param_names("index");
    let rp = get_param_names("rindex");
    if ip != rp {
        return Err(format!("index={ip:?} vs rindex={rp:?}"));
    }
    Ok(())
}

// ============================================================
// PHF get consistency with iteration
// ============================================================

#[test]
fn get_returns_same_as_iteration() -> Result<(), String> {
    for (key, params) in BUILTIN_SIGS.entries() {
        match BUILTIN_SIGS.get(key) {
            Some(looked_up) => {
                if *looked_up != *params {
                    return Err(format!("Mismatch for {key}: iteration vs get"));
                }
            }
            None => return Err(format!("get({key}) returned None but key exists")),
        }
    }
    Ok(())
}

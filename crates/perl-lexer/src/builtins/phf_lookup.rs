//! Consolidated built-in function signatures for Perl using perfect hash
//!
//! This module provides a single source of truth for all Perl built-in function signatures
//! Used by both InlayHints and SignatureHelp providers

use phf::phf_map;

/// Built-in function signatures as a perfect hash map
/// Each entry maps function name to parameter names
pub static BUILTIN_SIGS: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    // ===== I/O Functions =====
    "print" => &["FILEHANDLE", "LIST"],
    "printf" => &["FILEHANDLE", "FORMAT", "LIST"],
    "say" => &["FILEHANDLE", "LIST"],
    "open" => &["FILEHANDLE", "MODE", "FILENAME"],
    "XSLoader::load" => &["MODULE", "VERSION"],
    "DynaLoader::bootstrap" => &["MODULE", "VERSION"],
    "bootstrap" => &["MODULE", "VERSION"],
    "sysopen" => &["FILEHANDLE", "FILENAME", "MODE", "PERMS"],
    "close" => &["FILEHANDLE"],
    "read" => &["FILEHANDLE", "SCALAR", "LENGTH", "OFFSET"],
    "readpipe" => &["EXPR"],
    "sysread" => &["FILEHANDLE", "SCALAR", "LENGTH", "OFFSET"],
    "write" => &["FILEHANDLE"],
    "syswrite" => &["FILEHANDLE", "SCALAR", "LENGTH", "OFFSET"],
    "binmode" => &["FILEHANDLE", "LAYER"],
    "seek" => &["FILEHANDLE", "POSITION", "WHENCE"],
    "tell" => &["FILEHANDLE"],
    "truncate" => &["FILEHANDLE", "LENGTH"],
    "eof" => &["FILEHANDLE"],
    "fileno" => &["FILEHANDLE"],
    "flock" => &["FILEHANDLE", "OPERATION"],
    "fcntl" => &["FILEHANDLE", "FUNCTION", "SCALAR"],
    "ioctl" => &["FILEHANDLE", "FUNCTION", "SCALAR"],
    "getc" => &["FILEHANDLE"],
    "readline" => &["FILEHANDLE"],
    "select" => &["FILEHANDLE"],
    "sysseek" => &["FILEHANDLE", "POSITION", "WHENCE"],

    // ===== String Functions =====
    "chomp" => &["VARIABLE"],
    "chop" => &["VARIABLE"],
    "chr" => &["NUMBER"],
    "crypt" => &["PLAINTEXT", "SALT"],
    "fc" => &["EXPR"],
    "index" => &["STR", "SUBSTR", "POSITION"],
    "lc" => &["EXPR"],
    "lcfirst" => &["EXPR"],
    "length" => &["EXPR"],
    "ord" => &["EXPR"],
    "pack" => &["TEMPLATE", "LIST"],
    "reverse" => &["LIST"],
    "rindex" => &["STR", "SUBSTR", "POSITION"],
    "sprintf" => &["FORMAT", "LIST"],
    "substr" => &["EXPR", "OFFSET", "LENGTH", "REPLACEMENT"],
    "uc" => &["EXPR"],
    "ucfirst" => &["EXPR"],
    "unpack" => &["TEMPLATE", "EXPR"],
    "quotemeta" => &["EXPR"],

    // ===== Array Functions =====
    "push" => &["ARRAY", "LIST"],
    "pop" => &["ARRAY"],
    "shift" => &["ARRAY"],
    "unshift" => &["ARRAY", "LIST"],
    "splice" => &["ARRAY", "OFFSET", "LENGTH", "LIST"],
    "grep" => &["BLOCK", "LIST"],
    "map" => &["BLOCK", "LIST"],
    "sort" => &["BLOCK", "LIST"],
    "join" => &["EXPR", "LIST"],
    "split" => &["PATTERN", "EXPR", "LIMIT"],

    // ===== Hash Functions =====
    "each" => &["HASH"],
    "keys" => &["HASH"],
    "values" => &["HASH"],
    "delete" => &["EXPR"],
    "exists" => &["EXPR"],

    // ===== Math Functions =====
    "abs" => &["VALUE"],
    "atan2" => &["Y", "X"],
    "cos" => &["EXPR"],
    "exp" => &["EXPR"],
    "hex" => &["EXPR"],
    "int" => &["EXPR"],
    "log" => &["EXPR"],
    "oct" => &["EXPR"],
    "rand" => &["EXPR"],
    "sin" => &["EXPR"],
    "sqrt" => &["EXPR"],
    "srand" => &["EXPR"],

    // ===== File/Directory Functions =====
    "chdir" => &["EXPR"],
    "chroot" => &["FILENAME"],
    "chmod" => &["MODE", "LIST"],
    "chown" => &["UID", "GID", "LIST"],
    "link" => &["OLDFILE", "NEWFILE"],
    "lstat" => &["FILEHANDLE"],
    "mkdir" => &["FILENAME", "MODE"],
    "opendir" => &["DIRHANDLE", "EXPR"],
    "readdir" => &["DIRHANDLE"],
    "readlink" => &["EXPR"],
    "glob" => &["EXPR"],
    "rename" => &["OLDNAME", "NEWNAME"],
    "rmdir" => &["FILENAME"],
    "stat" => &["FILEHANDLE"],
    "symlink" => &["OLDFILE", "NEWFILE"],
    "umask" => &["EXPR"],
    "unlink" => &["LIST"],
    "utime" => &["ATIME", "MTIME", "LIST"],
    "closedir" => &["DIRHANDLE"],
    "rewinddir" => &["DIRHANDLE"],
    "seekdir" => &["DIRHANDLE", "POS"],
    "telldir" => &["DIRHANDLE"],

    // ===== File Test Operators =====
    "-e" => &["FILE"],
    "-f" => &["FILE"],
    "-d" => &["FILE"],
    "-r" => &["FILE"],
    "-w" => &["FILE"],
    "-x" => &["FILE"],
    "-o" => &["FILE"],
    "-R" => &["FILE"],
    "-W" => &["FILE"],
    "-X" => &["FILE"],
    "-O" => &["FILE"],
    "-z" => &["FILE"],
    "-s" => &["FILE"],
    "-l" => &["FILE"],
    "-p" => &["FILE"],
    "-S" => &["FILE"],
    "-b" => &["FILE"],
    "-c" => &["FILE"],
    "-t" => &["FILE"],
    "-u" => &["FILE"],
    "-g" => &["FILE"],
    "-k" => &["FILE"],
    "-T" => &["FILE"],
    "-B" => &["FILE"],
    "-M" => &["FILE"],
    "-A" => &["FILE"],
    "-C" => &["FILE"],

    // ===== Process Functions =====
    "alarm" => &["SECONDS"],
    "exec" => &["PROGRAM", "LIST"],
    "fork" => &[],
    "getpgrp" => &["PID"],
    "getppid" => &[],
    "getpriority" => &["WHICH", "WHO"],
    "kill" => &["SIGNAL", "LIST"],
    "pipe" => &["READHANDLE", "WRITEHANDLE"],
    "setpgrp" => &["PID", "PGRP"],
    "setpriority" => &["WHICH", "WHO", "PRIORITY"],
    "syscall" => &["NUMBER", "LIST"],
    "sleep" => &["EXPR"],
    "system" => &["PROGRAM", "LIST"],
    "times" => &[],
    "wait" => &[],
    "waitpid" => &["PID", "FLAGS"],

    // ===== Time Functions =====
    "gmtime" => &["EXPR"],
    "localtime" => &["EXPR"],
    "time" => &[],

    // ===== Network Functions =====
    "accept" => &["NEWSOCKET", "GENERICSOCKET"],
    "bind" => &["SOCKET", "NAME"],
    "connect" => &["SOCKET", "NAME"],
    "getpeername" => &["SOCKET"],
    "getsockname" => &["SOCKET"],
    "getsockopt" => &["SOCKET", "LEVEL", "OPTNAME"],
    "listen" => &["SOCKET", "QUEUESIZE"],
    "recv" => &["SOCKET", "SCALAR", "LENGTH", "FLAGS"],
    "send" => &["SOCKET", "MSG", "FLAGS", "TO"],
    "setsockopt" => &["SOCKET", "LEVEL", "OPTNAME", "OPTVAL"],
    "shutdown" => &["SOCKET", "HOW"],
    "socket" => &["SOCKET", "DOMAIN", "TYPE", "PROTOCOL"],
    "socketpair" => &["SOCKET1", "SOCKET2", "DOMAIN", "TYPE", "PROTOCOL"],
    "sockatmark" => &["SOCKET"],

    // ===== System Info Functions =====
    "gethostbyaddr" => &["ADDR", "ADDRTYPE"],
    "gethostbyname" => &["NAME"],
    "gethostent" => &[],
    "getnetbyaddr" => &["ADDR", "ADDRTYPE"],
    "getnetbyname" => &["NAME"],
    "getnetent" => &[],
    "getprotobyname" => &["NAME"],
    "getprotobynumber" => &["NUMBER"],
    "getprotoent" => &[],
    "getservbyname" => &["NAME", "PROTO"],
    "getservbyport" => &["PORT", "PROTO"],
    "getservent" => &[],
    "sethostent" => &["STAYOPEN"],
    "setnetent" => &["STAYOPEN"],
    "setprotoent" => &["STAYOPEN"],
    "setservent" => &["STAYOPEN"],
    "endhostent" => &[],
    "endnetent" => &[],
    "endprotoent" => &[],
    "endservent" => &[],

    // ===== User/Group Functions =====
    "getgrent" => &[],
    "getgrgid" => &["GID"],
    "getgrnam" => &["NAME"],
    "getlogin" => &[],
    "getuid" => &[],
    "geteuid" => &[],
    "getgid" => &[],
    "getegid" => &[],
    "getgroups" => &[],
    "setuid" => &["UID"],
    "seteuid" => &["UID"],
    "setgid" => &["GID"],
    "setegid" => &["GID"],
    "setgroups" => &["LIST"],
    "getpwent" => &[],
    "getpwnam" => &["NAME"],
    "getpwuid" => &["UID"],
    "setgrent" => &[],
    "setpwent" => &[],
    "endgrent" => &[],
    "endpwent" => &[],

    // ===== IPC Functions =====
    "msgctl" => &["ID", "CMD", "ARG"],
    "msgget" => &["KEY", "FLAGS"],
    "msgrcv" => &["ID", "VAR", "SIZE", "TYPE", "FLAGS"],
    "msgsnd" => &["ID", "MSG", "FLAGS"],
    "semctl" => &["ID", "SEMNUM", "CMD", "ARG"],
    "semget" => &["KEY", "NSEMS", "FLAGS"],
    "semop" => &["ID", "OPSTRING"],
    "shmctl" => &["ID", "CMD", "ARG"],
    "shmget" => &["KEY", "SIZE", "FLAGS"],
    "shmread" => &["ID", "VAR", "POS", "SIZE"],
    "shmwrite" => &["ID", "STRING", "POS", "SIZE"],

    // ===== Database Functions =====
    "dbmclose" => &["HASH"],
    "dbmopen" => &["HASH", "DBNAME", "MODE"],
    "tie" => &["VARIABLE", "CLASSNAME", "LIST"],
    "tied" => &["VARIABLE"],
    "untie" => &["VARIABLE"],

    // ===== Miscellaneous Functions =====
    "bless" => &["REF", "CLASSNAME"],
    "caller" => &["EXPR"],
    "die" => &["LIST"],
    "do" => &["BLOCK"],
    "eval" => &["EXPR"],
    "exit" => &["EXPR"],
    "goto" => &["LABEL"],
    "last" => &["LABEL"],
    "next" => &["LABEL"],
    "redo" => &["LABEL"],
    "ref" => &["EXPR"],
    "require" => &["VERSION"],
    "return" => &["LIST"],
    "scalar" => &["EXPR"],
    "undef" => &["EXPR"],
    "wantarray" => &[],
    "warn" => &["LIST"],
    "defined" => &["EXPR"],
    "dump" => &["LABEL"],
    "formline" => &["PICTURE", "LIST"],
    "local" => &["EXPR"],
    "my" => &["VARLIST"],
    "our" => &["VARLIST"],
    "state" => &["VARLIST"],
    "reset" => &["EXPR"],
    "study" => &["SCALAR"],
    "pos" => &["SCALAR"],
    "use" => &["MODULE", "VERSION", "LIST"],
    "vec" => &["EXPR", "OFFSET", "BITS"],
    "lock" => &["THING"],
    "prototype" => &["FUNCTION"],

    // ===== UTF-8 Encoding Functions =====
    "utf8::encode" => &["SCALAR"],
    "utf8::decode" => &["SCALAR"],
    "utf8::is_utf8" => &["SCALAR"],
    "utf8::valid" => &["SCALAR"],
    "utf8::upgrade" => &["SCALAR"],
    "utf8::downgrade" => &["SCALAR", "FAIL_OK"],
    "utf8::native_to_unicode" => &["CODEPOINT"],
    "utf8::unicode_to_native" => &["CODEPOINT"],
};

/// Full signatures for documentation (used by signature help)
pub static BUILTIN_FULL_SIGS: phf::Map<&'static str, &'static [&'static str]> = phf_map! {
    "print" => &["print FILEHANDLE LIST", "print FILEHANDLE", "print LIST", "print"],
    "printf" => &["printf FILEHANDLE FORMAT, LIST", "printf FORMAT, LIST"],
    "say" => &["say FILEHANDLE LIST", "say FILEHANDLE", "say LIST", "say"],
    "open" => &["open FILEHANDLE, MODE, FILENAME", "open FILEHANDLE, EXPR", "open FILEHANDLE"],
    "XSLoader::load" => &["XSLoader::load MODULE, VERSION"],
    "DynaLoader::bootstrap" => &["DynaLoader::bootstrap MODULE, VERSION"],
    "bootstrap" => &["bootstrap MODULE, VERSION"],
    "close" => &["close FILEHANDLE", "close"],
    "substr" => &["substr EXPR, OFFSET, LENGTH, REPLACEMENT", "substr EXPR, OFFSET, LENGTH", "substr EXPR, OFFSET"],
    "splice" => &["splice ARRAY, OFFSET, LENGTH, LIST", "splice ARRAY, OFFSET, LENGTH", "splice ARRAY, OFFSET", "splice ARRAY"],
    "split" => &["split PATTERN, EXPR, LIMIT", "split PATTERN, EXPR", "split PATTERN", "split"],
    "push" => &["push ARRAY, LIST"],
    "pop" => &["pop ARRAY", "pop"],
    "shift" => &["shift ARRAY", "shift"],
    "unshift" => &["unshift ARRAY, LIST"],
    "map" => &["map BLOCK LIST", "map EXPR, LIST"],
    "grep" => &["grep BLOCK LIST", "grep EXPR, LIST"],
    "sort" => &["sort BLOCK LIST", "sort SUBNAME LIST", "sort LIST"],
    "join" => &["join EXPR, LIST"],
    "reverse" => &["reverse LIST"],
    "chomp" => &["chomp VARIABLE", "chomp LIST", "chomp"],
    "chop" => &["chop VARIABLE", "chop"],
    "index" => &["index STR, SUBSTR, POSITION", "index STR, SUBSTR"],
    "rindex" => &["rindex STR, SUBSTR, POSITION", "rindex STR, SUBSTR"],
    "sprintf" => &["sprintf FORMAT, LIST"],
    "die" => &["die LIST"],
    "warn" => &["warn LIST"],
    "eval" => &["eval EXPR", "eval BLOCK"],
    "each" => &["each HASH", "each ARRAY"],
    "keys" => &["keys HASH", "keys ARRAY"],
    "values" => &["values HASH", "values ARRAY"],
    "delete" => &["delete EXPR"],
    "exists" => &["exists EXPR"],
    "defined" => &["defined EXPR"],
    "ref" => &["ref EXPR"],
    "bless" => &["bless REF, CLASSNAME", "bless REF"],
    "read" => &["read FILEHANDLE, SCALAR, LENGTH, OFFSET", "read FILEHANDLE, SCALAR, LENGTH"],
    "seek" => &["seek FILEHANDLE, POSITION, WHENCE"],
    "tell" => &["tell FILEHANDLE", "tell"],
    "eof" => &["eof FILEHANDLE", "eof"],
    "binmode" => &["binmode FILEHANDLE, LAYER", "binmode FILEHANDLE"],
    "chmod" => &["chmod MODE, LIST"],
    "chown" => &["chown UID, GID, LIST"],
    "mkdir" => &["mkdir FILENAME, MODE", "mkdir FILENAME", "mkdir"],
    "rmdir" => &["rmdir FILENAME", "rmdir"],
    "unlink" => &["unlink LIST"],
    "rename" => &["rename OLDNAME, NEWNAME"],
    "stat" => &["stat FILEHANDLE", "stat EXPR"],
    "opendir" => &["opendir DIRHANDLE, EXPR"],
    "readdir" => &["readdir DIRHANDLE"],
    "closedir" => &["closedir DIRHANDLE"],
    "system" => &["system PROGRAM, LIST", "system PROGRAM"],
    "exec" => &["exec PROGRAM, LIST", "exec PROGRAM"],

    // ===== UTF-8 Encoding Functions =====
    "utf8::encode" => &["utf8::encode SCALAR"],
    "utf8::decode" => &["utf8::decode SCALAR"],
    "utf8::is_utf8" => &["utf8::is_utf8 SCALAR"],
    "utf8::valid" => &["utf8::valid SCALAR"],
    "utf8::upgrade" => &["utf8::upgrade SCALAR"],
    "utf8::downgrade" => &["utf8::downgrade SCALAR, FAIL_OK", "utf8::downgrade SCALAR"],
    "utf8::native_to_unicode" => &["utf8::native_to_unicode CODEPOINT"],
    "utf8::unicode_to_native" => &["utf8::unicode_to_native CODEPOINT"],
};

/// Get parameter names for a builtin function
pub fn get_param_names(function_name: &str) -> &'static [&'static str] {
    BUILTIN_SIGS.get(function_name).copied().unwrap_or(&[])
}

/// Check if a function is a builtin
pub fn is_builtin(function_name: &str) -> bool {
    BUILTIN_SIGS.contains_key(function_name)
}

/// Get the number of builtins
pub fn builtin_count() -> usize {
    BUILTIN_SIGS.len()
}

#[cfg(test)]
mod tests {
    use super::{BUILTIN_FULL_SIGS, builtin_count, get_param_names, is_builtin};

    #[test]
    fn exposes_common_builtins() {
        assert!(is_builtin("print"));
        assert!(!is_builtin("not_a_builtin"));
    }

    #[test]
    fn returns_open_param_names() {
        assert_eq!(get_param_names("open"), ["FILEHANDLE", "MODE", "FILENAME"]);
        assert!(builtin_count() > 150);
    }

    #[test]
    fn full_sigs_cover_target_builtins() {
        let targets =
            ["open", "print", "push", "pop", "splice", "map", "grep", "sort", "join", "split"];
        for name in &targets {
            assert!(
                BUILTIN_FULL_SIGS.contains_key(name),
                "BUILTIN_FULL_SIGS should contain '{}'",
                name
            );
        }
    }

    #[test]
    fn full_sigs_have_nonempty_variants() {
        let targets =
            ["open", "print", "push", "pop", "splice", "map", "grep", "sort", "join", "split"];
        for name in &targets {
            let sigs = BUILTIN_FULL_SIGS.get(name);
            assert!(sigs.is_some(), "BUILTIN_FULL_SIGS should have entry for '{}'", name);
            let sigs = sigs.copied().unwrap_or(&[]);
            assert!(!sigs.is_empty(), "'{}' should have at least one full signature", name);
        }
    }

    #[test]
    fn full_sigs_push_has_correct_variants() {
        let sigs = BUILTIN_FULL_SIGS.get("push").copied().unwrap_or(&[]);
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0], "push ARRAY, LIST");
    }

    #[test]
    fn full_sigs_pop_has_correct_variants() {
        let sigs = BUILTIN_FULL_SIGS.get("pop").copied().unwrap_or(&[]);
        assert_eq!(sigs.len(), 2);
        assert_eq!(sigs[0], "pop ARRAY");
        assert_eq!(sigs[1], "pop");
    }

    #[test]
    fn full_sigs_map_has_correct_variants() {
        let sigs = BUILTIN_FULL_SIGS.get("map").copied().unwrap_or(&[]);
        assert_eq!(sigs.len(), 2);
        assert_eq!(sigs[0], "map BLOCK LIST");
        assert_eq!(sigs[1], "map EXPR, LIST");
    }

    #[test]
    fn full_sigs_grep_has_correct_variants() {
        let sigs = BUILTIN_FULL_SIGS.get("grep").copied().unwrap_or(&[]);
        assert_eq!(sigs.len(), 2);
        assert_eq!(sigs[0], "grep BLOCK LIST");
        assert_eq!(sigs[1], "grep EXPR, LIST");
    }

    #[test]
    fn full_sigs_sort_has_correct_variants() {
        let sigs = BUILTIN_FULL_SIGS.get("sort").copied().unwrap_or(&[]);
        assert_eq!(sigs.len(), 3);
        assert_eq!(sigs[0], "sort BLOCK LIST");
        assert_eq!(sigs[1], "sort SUBNAME LIST");
        assert_eq!(sigs[2], "sort LIST");
    }

    #[test]
    fn full_sigs_join_has_correct_variants() {
        let sigs = BUILTIN_FULL_SIGS.get("join").copied().unwrap_or(&[]);
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0], "join EXPR, LIST");
    }

    #[test]
    fn full_sigs_splice_has_four_variants() {
        let sigs = BUILTIN_FULL_SIGS.get("splice").copied().unwrap_or(&[]);
        assert_eq!(sigs.len(), 4);
    }

    #[test]
    fn full_sigs_split_has_four_variants() {
        let sigs = BUILTIN_FULL_SIGS.get("split").copied().unwrap_or(&[]);
        assert_eq!(sigs.len(), 4);
    }

    #[test]
    fn full_sigs_additional_builtins() {
        // Verify we also added some commonly needed extras
        let extras = [
            "chomp", "chop", "index", "rindex", "die", "warn", "eval", "each", "keys", "values",
            "bless", "defined",
        ];
        for name in &extras {
            assert!(
                BUILTIN_FULL_SIGS.contains_key(name),
                "BUILTIN_FULL_SIGS should contain '{}'",
                name
            );
        }
    }

    #[test]
    fn utf8_namespace_functions_are_builtins() {
        // utf8::encode/decode and friends are core Perl functions from the
        // `utf8::` namespace. They must be recognised as builtins so that
        // completion, hover, and signature help can surface documentation
        // for them. See GitHub issue #3371.
        let utf8_fns = [
            "utf8::encode",
            "utf8::decode",
            "utf8::is_utf8",
            "utf8::valid",
            "utf8::upgrade",
            "utf8::downgrade",
            "utf8::native_to_unicode",
            "utf8::unicode_to_native",
        ];
        for name in &utf8_fns {
            assert!(is_builtin(name), "is_builtin should recognise '{}'", name);
            assert!(
                !get_param_names(name).is_empty(),
                "get_param_names should return params for '{}'",
                name
            );
        }
    }

    #[test]
    fn utf8_encode_decode_take_scalar_parameter() {
        // utf8::encode/decode mutate a single scalar in place.
        assert_eq!(get_param_names("utf8::encode"), ["SCALAR"]);
        assert_eq!(get_param_names("utf8::decode"), ["SCALAR"]);
        assert_eq!(get_param_names("utf8::is_utf8"), ["SCALAR"]);
        assert_eq!(get_param_names("utf8::valid"), ["SCALAR"]);
        assert_eq!(get_param_names("utf8::upgrade"), ["SCALAR"]);
    }

    #[test]
    fn utf8_downgrade_has_optional_fail_ok_parameter() {
        // utf8::downgrade accepts an optional FAIL_OK second argument that
        // changes whether bad input croaks or returns a false value.
        assert_eq!(get_param_names("utf8::downgrade"), ["SCALAR", "FAIL_OK"]);
    }

    #[test]
    fn utf8_codepoint_converters_take_codepoint() {
        // The native/unicode converters take a code point number, not a scalar.
        assert_eq!(get_param_names("utf8::native_to_unicode"), ["CODEPOINT"]);
        assert_eq!(get_param_names("utf8::unicode_to_native"), ["CODEPOINT"]);
    }

    #[test]
    fn utf8_full_sigs_are_populated() {
        // Full signatures drive signature help and the multi-variant form
        // shown for utf8::downgrade.
        let expected = [
            ("utf8::encode", 1),
            ("utf8::decode", 1),
            ("utf8::is_utf8", 1),
            ("utf8::valid", 1),
            ("utf8::upgrade", 1),
            ("utf8::downgrade", 2),
            ("utf8::native_to_unicode", 1),
            ("utf8::unicode_to_native", 1),
        ];
        for (name, expected_len) in &expected {
            let sigs = BUILTIN_FULL_SIGS.get(name).copied().unwrap_or(&[]);
            assert_eq!(
                sigs.len(),
                *expected_len,
                "'{}' should have {} signature variant(s)",
                name,
                expected_len
            );
            assert!(
                sigs.iter().all(|s| s.starts_with(name)),
                "'{}' full signatures should start with the function name",
                name
            );
        }
    }

    #[test]
    fn utf8_downgrade_signatures_cover_both_forms() {
        // Two-argument and one-argument forms must both be present.
        let sigs = BUILTIN_FULL_SIGS.get("utf8::downgrade").copied().unwrap_or(&[]);
        assert!(sigs.iter().any(|s| s.contains("FAIL_OK")), "two-arg form missing: {:?}", sigs);
        assert!(sigs.contains(&"utf8::downgrade SCALAR"), "one-arg form missing: {:?}", sigs);
    }
}

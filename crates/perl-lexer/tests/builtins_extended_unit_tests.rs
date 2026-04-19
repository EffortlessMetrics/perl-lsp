//! Extended unit tests for the `perl-builtins` crate.
//!
//! Complements `comprehensive_unit_tests.rs` with additional coverage for:
//! - PHF map iteration and entry consistency
//! - Parameter name semantics (FILEHANDLE, SCALAR, LIST patterns)
#![allow(clippy::panic)]
//! - Cross-module alignment between PHF and HashMap signatures
//! - Signature variant ordering (most-specific-first convention)
//! - Edge cases around whitespace, unicode, and boundary inputs
//! - Documentation content quality checks

use perl_lexer::builtins::builtin_signatures::create_builtin_signatures;
use perl_lexer::builtins::phf_lookup::{
    BUILTIN_FULL_SIGS, BUILTIN_SIGS, builtin_count, get_param_names, is_builtin,
};

// ============================================================
// PHF map iteration and entry-level checks
// ============================================================

#[test]
fn phf_map_entries_all_have_static_lifetime_keys() {
    // Every key in the PHF map should be a non-empty &'static str.
    for (key, _) in BUILTIN_SIGS.entries() {
        if key.is_empty() {
            panic!("Found empty key in BUILTIN_SIGS");
        }
    }
}

#[test]
fn phf_map_entry_count_matches_iteration_count() {
    let iter_count = BUILTIN_SIGS.entries().count();
    let len = BUILTIN_SIGS.len();
    if iter_count != len {
        panic!("entries().count()={iter_count} but len()={len}");
    }
}

#[test]
fn phf_map_get_returns_same_as_iteration_for_known_keys() -> Result<(), String> {
    for (key, params) in BUILTIN_SIGS.entries() {
        match BUILTIN_SIGS.get(key) {
            Some(looked_up) => {
                if *looked_up != *params {
                    return Err(format!("Mismatch for {key}: iteration vs get"));
                }
            }
            None => {
                return Err(format!(
                    "get({key}) returned None but key exists in entries"
                ));
            }
        }
    }
    Ok(())
}

// ============================================================
// Parameter name semantic patterns
// ============================================================

#[test]
fn io_functions_have_filehandle_as_first_param() -> Result<(), String> {
    let io_funcs = [
        "print", "printf", "say", "open", "close", "read", "sysread", "write", "syswrite",
        "binmode", "seek", "tell", "truncate", "eof", "fileno", "flock", "fcntl", "ioctl", "getc",
        "readline", "select", "sysseek",
    ];
    for name in &io_funcs {
        let params = get_param_names(name);
        if let Some(first) = params.first() {
            if *first != "FILEHANDLE" {
                return Err(format!(
                    "{name} first param should be FILEHANDLE, got {first}"
                ));
            }
        } else {
            return Err(format!("{name} should have at least one param"));
        }
    }
    Ok(())
}

#[test]
fn directory_functions_have_dirhandle_param() -> Result<(), String> {
    let dir_funcs = [
        "opendir",
        "readdir",
        "closedir",
        "rewinddir",
        "seekdir",
        "telldir",
    ];
    for name in &dir_funcs {
        let params = get_param_names(name);
        if let Some(first) = params.first() {
            if *first != "DIRHANDLE" {
                return Err(format!(
                    "{name} first param should be DIRHANDLE, got {first}"
                ));
            }
        } else {
            return Err(format!("{name} should have at least one param"));
        }
    }
    Ok(())
}

#[test]
fn socket_functions_have_socket_param() -> Result<(), String> {
    let socket_funcs = [
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
    for name in &socket_funcs {
        let params = get_param_names(name);
        if let Some(first) = params.first() {
            if !first.contains("SOCKET") {
                return Err(format!(
                    "{name} first param should contain SOCKET, got {first}"
                ));
            }
        } else {
            return Err(format!("{name} should have at least one param"));
        }
    }
    Ok(())
}

#[test]
fn all_file_test_operators_have_single_file_param() -> Result<(), String> {
    let ops = [
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ];
    for op in &ops {
        let params = get_param_names(op);
        if params.len() != 1 {
            return Err(format!(
                "{op} should have exactly 1 param, got {}",
                params.len()
            ));
        }
        if params[0] != "FILE" {
            return Err(format!("{op} param should be FILE, got {}", params[0]));
        }
    }
    Ok(())
}

// ============================================================
// Zero-parameter builtins exhaustive check
// ============================================================

#[test]
fn all_zero_param_builtins_return_empty_slice() -> Result<(), String> {
    let zero_param = [
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
    for name in &zero_param {
        let params = get_param_names(name);
        if !params.is_empty() {
            return Err(format!("{name} should have 0 params, got {params:?}"));
        }
    }
    Ok(())
}

// ============================================================
// Cross-module alignment: PHF entries present in HashMap
// ============================================================

#[test]
fn every_phf_io_entry_exists_in_hashmap() -> Result<(), String> {
    let hashmap_sigs = create_builtin_signatures();
    let io_funcs = [
        "print", "printf", "say", "open", "close", "read", "readline", "readpipe", "sysread",
        "write", "syswrite", "seek", "tell", "eof",
    ];
    for name in &io_funcs {
        if !hashmap_sigs.contains_key(name) {
            return Err(format!("{name} in PHF but missing from HashMap signatures"));
        }
    }
    Ok(())
}

#[test]
fn every_phf_math_entry_exists_in_hashmap() -> Result<(), String> {
    let hashmap_sigs = create_builtin_signatures();
    let math_funcs = [
        "abs", "atan2", "cos", "exp", "hex", "int", "log", "oct", "rand", "sin", "sqrt", "srand",
    ];
    for name in &math_funcs {
        if !hashmap_sigs.contains_key(name) {
            return Err(format!("{name} in PHF but missing from HashMap signatures"));
        }
    }
    Ok(())
}

#[test]
fn every_phf_file_test_exists_in_hashmap() -> Result<(), String> {
    let hashmap_sigs = create_builtin_signatures();
    let ops = [
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ];
    for op in &ops {
        if !hashmap_sigs.contains_key(op) {
            return Err(format!("{op} in PHF but missing from HashMap signatures"));
        }
    }
    Ok(())
}

// ============================================================
// Signature variant ordering checks
// ============================================================

#[test]
fn full_sigs_print_starts_with_most_specific() -> Result<(), String> {
    if let Some(sigs) = BUILTIN_FULL_SIGS.get("print") {
        if let Some(first) = sigs.first() {
            if !first.contains("FILEHANDLE") || !first.contains("LIST") {
                return Err(format!(
                    "print first full sig should be the most specific (FILEHANDLE LIST), got {first}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn full_sigs_open_starts_with_three_arg_form() -> Result<(), String> {
    if let Some(sigs) = BUILTIN_FULL_SIGS.get("open") {
        if let Some(first) = sigs.first() {
            if !first.contains("MODE") || !first.contains("FILENAME") {
                return Err(format!(
                    "open first full sig should be 3-arg form, got {first}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn full_sigs_split_starts_with_three_arg_form() -> Result<(), String> {
    if let Some(sigs) = BUILTIN_FULL_SIGS.get("split") {
        if let Some(first) = sigs.first() {
            if !first.contains("LIMIT") {
                return Err(format!(
                    "split first full sig should include LIMIT, got {first}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn hashmap_splice_variants_ordered_most_specific_first() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(splice_sig) = sigs.get("splice") {
        let variant_lengths: Vec<usize> = splice_sig.signatures.iter().map(|s| s.len()).collect();
        // Each successive variant should be shorter or equal (most specific first)
        for window in variant_lengths.windows(2) {
            if window[0] < window[1] {
                return Err(format!(
                    "splice variants not ordered most-specific-first: lengths {variant_lengths:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn hashmap_open_variants_ordered_most_specific_first() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(open_sig) = sigs.get("open") {
        let variant_lengths: Vec<usize> = open_sig.signatures.iter().map(|s| s.len()).collect();
        for window in variant_lengths.windows(2) {
            if window[0] < window[1] {
                return Err(format!(
                    "open variants not ordered most-specific-first: lengths {variant_lengths:?}"
                ));
            }
        }
    }
    Ok(())
}

// ============================================================
// Edge cases and boundary inputs
// ============================================================

#[test]
fn is_builtin_rejects_whitespace_variations() {
    let whitespace_names = [
        " print", "print ", " print ", "\tprint", "print\n", "\nprint",
    ];
    for name in &whitespace_names {
        if is_builtin(name) {
            panic!("{name:?} with whitespace should not be a builtin");
        }
    }
}

#[test]
fn is_builtin_rejects_partial_names() {
    let partials = ["pri", "prin", "ope", "chom", "splic"];
    for name in &partials {
        if is_builtin(name) {
            panic!("{name:?} (partial) should not be a builtin");
        }
    }
}

#[test]
fn is_builtin_rejects_qualified_names() {
    let qualified = ["CORE::print", "main::open", "Perl::chomp"];
    for name in &qualified {
        if is_builtin(name) {
            panic!("{name:?} (qualified) should not be a builtin");
        }
    }
}

#[test]
fn is_builtin_rejects_sigil_prefixed_names() {
    let sigiled = ["$print", "@push", "%keys", "&sort", "*open"];
    for name in &sigiled {
        if is_builtin(name) {
            panic!("{name:?} (sigil-prefixed) should not be a builtin");
        }
    }
}

#[test]
fn get_param_names_returns_empty_for_whitespace_input() {
    let params = get_param_names(" ");
    if !params.is_empty() {
        panic!("whitespace input should return empty params");
    }
}

#[test]
fn get_param_names_returns_empty_for_newline_input() {
    let params = get_param_names("\n");
    if !params.is_empty() {
        panic!("newline input should return empty params");
    }
}

#[test]
fn is_builtin_rejects_numeric_strings() {
    let numerics = ["0", "1", "42", "-1", "3.14"];
    for name in &numerics {
        if is_builtin(name) {
            panic!("{name:?} (numeric) should not be a builtin");
        }
    }
}

#[test]
fn is_builtin_rejects_special_chars() {
    let specials = ["!", "@", "#", "$", "%", "^", "&", "*", "(", ")", "{}"];
    for name in &specials {
        if is_builtin(name) {
            panic!("{name:?} (special char) should not be a builtin");
        }
    }
}

// ============================================================
// Documentation quality checks on HashMap signatures
// ============================================================

#[test]
fn all_hashmap_docs_are_ascii_printable() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for (name, sig) in sigs.iter() {
        if !sig.documentation.is_ascii() {
            return Err(format!(
                "{name} documentation contains non-ASCII characters"
            ));
        }
    }
    Ok(())
}

#[test]
fn all_hashmap_docs_do_not_end_with_period() -> Result<(), String> {
    // Convention check: documentation strings are short descriptions without trailing period
    let sigs = create_builtin_signatures();
    for (name, sig) in sigs.iter() {
        if sig.documentation.ends_with('.') {
            return Err(format!(
                "{name} documentation ends with period (convention: no trailing period)"
            ));
        }
    }
    Ok(())
}

#[test]
fn all_hashmap_signature_variants_are_ascii() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    for (name, sig) in sigs.iter() {
        for variant in &sig.signatures {
            if !variant.is_ascii() {
                return Err(format!("{name} variant {variant:?} contains non-ASCII"));
            }
        }
    }
    Ok(())
}

#[test]
fn hashmap_signature_count_is_substantial() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    // The HashMap should have a substantial number of entries
    if sigs.len() < 80 {
        return Err(format!(
            "create_builtin_signatures has {} entries, expected at least 80",
            sigs.len()
        ));
    }
    Ok(())
}

// ============================================================
// Specific parameter verification for various categories
// ============================================================

#[test]
fn phf_params_crypt_has_plaintext_salt() -> Result<(), String> {
    let params = get_param_names("crypt");
    if params != ["PLAINTEXT", "SALT"] {
        return Err(format!(
            "crypt params should be [PLAINTEXT, SALT], got {params:?}"
        ));
    }
    Ok(())
}

#[test]
fn phf_params_chmod_has_mode_list() -> Result<(), String> {
    let params = get_param_names("chmod");
    if params != ["MODE", "LIST"] {
        return Err(format!(
            "chmod params should be [MODE, LIST], got {params:?}"
        ));
    }
    Ok(())
}

#[test]
fn phf_params_chown_has_uid_gid_list() -> Result<(), String> {
    let params = get_param_names("chown");
    if params != ["UID", "GID", "LIST"] {
        return Err(format!(
            "chown params should be [UID, GID, LIST], got {params:?}"
        ));
    }
    Ok(())
}

#[test]
fn phf_params_kill_has_signal_list() -> Result<(), String> {
    let params = get_param_names("kill");
    if params != ["SIGNAL", "LIST"] {
        return Err(format!(
            "kill params should be [SIGNAL, LIST], got {params:?}"
        ));
    }
    Ok(())
}

#[test]
fn phf_params_waitpid_has_pid_flags() -> Result<(), String> {
    let params = get_param_names("waitpid");
    if params != ["PID", "FLAGS"] {
        return Err(format!(
            "waitpid params should be [PID, FLAGS], got {params:?}"
        ));
    }
    Ok(())
}

#[test]
fn phf_params_recv_has_four_params() -> Result<(), String> {
    let params = get_param_names("recv");
    if params != ["SOCKET", "SCALAR", "LENGTH", "FLAGS"] {
        return Err(format!("recv params wrong: {params:?}"));
    }
    Ok(())
}

#[test]
fn phf_params_send_has_four_params() -> Result<(), String> {
    let params = get_param_names("send");
    if params != ["SOCKET", "MSG", "FLAGS", "TO"] {
        return Err(format!("send params wrong: {params:?}"));
    }
    Ok(())
}

#[test]
fn phf_params_split_has_pattern_expr_limit() -> Result<(), String> {
    let params = get_param_names("split");
    if params != ["PATTERN", "EXPR", "LIMIT"] {
        return Err(format!(
            "split params should be [PATTERN, EXPR, LIMIT], got {params:?}"
        ));
    }
    Ok(())
}

#[test]
fn phf_params_index_has_str_substr_position() -> Result<(), String> {
    let params = get_param_names("index");
    if params != ["STR", "SUBSTR", "POSITION"] {
        return Err(format!(
            "index params should be [STR, SUBSTR, POSITION], got {params:?}"
        ));
    }
    Ok(())
}

#[test]
fn phf_params_rindex_matches_index_params() -> Result<(), String> {
    let index_params = get_param_names("index");
    let rindex_params = get_param_names("rindex");
    if index_params != rindex_params {
        return Err(format!(
            "index and rindex should have same params: index={index_params:?}, rindex={rindex_params:?}"
        ));
    }
    Ok(())
}

// ============================================================
// IPC function parameter checks
// ============================================================

#[test]
fn phf_params_msgctl_has_id_cmd_arg() -> Result<(), String> {
    let params = get_param_names("msgctl");
    if params != ["ID", "CMD", "ARG"] {
        return Err(format!("msgctl params wrong: {params:?}"));
    }
    Ok(())
}

#[test]
fn phf_params_semget_has_key_nsems_flags() -> Result<(), String> {
    let params = get_param_names("semget");
    if params != ["KEY", "NSEMS", "FLAGS"] {
        return Err(format!("semget params wrong: {params:?}"));
    }
    Ok(())
}

#[test]
fn phf_params_shmread_has_four_params() -> Result<(), String> {
    let params = get_param_names("shmread");
    if params != ["ID", "VAR", "POS", "SIZE"] {
        return Err(format!("shmread params wrong: {params:?}"));
    }
    Ok(())
}

// ============================================================
// Database function parameter checks
// ============================================================

#[test]
fn phf_params_dbmopen_has_hash_dbname_mode() -> Result<(), String> {
    let params = get_param_names("dbmopen");
    if params != ["HASH", "DBNAME", "MODE"] {
        return Err(format!("dbmopen params wrong: {params:?}"));
    }
    Ok(())
}

#[test]
fn phf_params_tie_has_variable_classname_list() -> Result<(), String> {
    let params = get_param_names("tie");
    if params != ["VARIABLE", "CLASSNAME", "LIST"] {
        return Err(format!("tie params wrong: {params:?}"));
    }
    Ok(())
}

// ============================================================
// HashMap signature content checks for specific functions
// ============================================================

#[test]
fn hashmap_chomp_has_three_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(sig) = sigs.get("chomp") {
        if sig.signatures.len() != 3 {
            return Err(format!(
                "chomp should have 3 variants, got {}",
                sig.signatures.len()
            ));
        }
    } else {
        return Err("chomp missing from HashMap".into());
    }
    Ok(())
}

#[test]
fn hashmap_bless_has_two_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(sig) = sigs.get("bless") {
        if sig.signatures.len() != 2 {
            return Err(format!(
                "bless should have 2 variants, got {}",
                sig.signatures.len()
            ));
        }
    } else {
        return Err("bless missing from HashMap".into());
    }
    Ok(())
}

#[test]
fn hashmap_eval_has_two_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(sig) = sigs.get("eval") {
        if sig.signatures.len() != 2 {
            return Err(format!(
                "eval should have 2 variants, got {}",
                sig.signatures.len()
            ));
        }
    } else {
        return Err("eval missing from HashMap".into());
    }
    Ok(())
}

#[test]
fn hashmap_grep_has_two_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(sig) = sigs.get("grep") {
        if sig.signatures.len() != 2 {
            return Err(format!(
                "grep should have 2 variants, got {}",
                sig.signatures.len()
            ));
        }
    } else {
        return Err("grep missing from HashMap".into());
    }
    Ok(())
}

#[test]
fn hashmap_map_has_two_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(sig) = sigs.get("map") {
        if sig.signatures.len() != 2 {
            return Err(format!(
                "map should have 2 variants, got {}",
                sig.signatures.len()
            ));
        }
    } else {
        return Err("map missing from HashMap".into());
    }
    Ok(())
}

#[test]
fn hashmap_sort_has_three_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(sig) = sigs.get("sort") {
        if sig.signatures.len() != 3 {
            return Err(format!(
                "sort should have 3 variants, got {}",
                sig.signatures.len()
            ));
        }
    } else {
        return Err("sort missing from HashMap".into());
    }
    Ok(())
}

#[test]
fn hashmap_split_has_four_variants() -> Result<(), String> {
    let sigs = create_builtin_signatures();
    if let Some(sig) = sigs.get("split") {
        if sig.signatures.len() != 4 {
            return Err(format!(
                "split should have 4 variants, got {}",
                sig.signatures.len()
            ));
        }
    } else {
        return Err("split missing from HashMap".into());
    }
    Ok(())
}

// ============================================================
// Consistency: functions with single-arg defaulting to $_
// ============================================================

#[test]
fn single_expr_functions_in_both_modules() -> Result<(), String> {
    let hashmap_sigs = create_builtin_signatures();
    // Functions that take a single EXPR and default to $_
    let defaulting_funcs = [
        "chomp", "chop", "chr", "lc", "lcfirst", "length", "ord", "uc", "ucfirst",
    ];
    for name in &defaulting_funcs {
        if !is_builtin(name) {
            return Err(format!("{name} missing from PHF"));
        }
        if !hashmap_sigs.contains_key(name) {
            return Err(format!("{name} missing from HashMap"));
        }
    }
    Ok(())
}

// ============================================================
// Full sigs variant text content checks
// ============================================================

#[test]
fn full_sigs_close_has_two_variants() -> Result<(), String> {
    if let Some(sigs) = BUILTIN_FULL_SIGS.get("close") {
        if sigs.len() != 2 {
            return Err(format!("close should have 2 full sigs, got {}", sigs.len()));
        }
    } else {
        return Err("close missing from BUILTIN_FULL_SIGS".into());
    }
    Ok(())
}

#[test]
fn full_sigs_printf_has_two_variants() -> Result<(), String> {
    if let Some(sigs) = BUILTIN_FULL_SIGS.get("printf") {
        if sigs.len() != 2 {
            return Err(format!(
                "printf should have 2 full sigs, got {}",
                sigs.len()
            ));
        }
    } else {
        return Err("printf missing from BUILTIN_FULL_SIGS".into());
    }
    Ok(())
}

#[test]
fn full_sigs_say_has_four_variants() -> Result<(), String> {
    if let Some(sigs) = BUILTIN_FULL_SIGS.get("say") {
        if sigs.len() != 4 {
            return Err(format!("say should have 4 full sigs, got {}", sigs.len()));
        }
    } else {
        return Err("say missing from BUILTIN_FULL_SIGS".into());
    }
    Ok(())
}

// ============================================================
// Builtin count boundary checks
// ============================================================

#[test]
fn builtin_count_includes_file_test_operators() {
    // There are 27 file test operators in the PHF map
    let file_test_count = [
        "-e", "-f", "-d", "-r", "-w", "-x", "-o", "-R", "-W", "-X", "-O", "-z", "-s", "-l", "-p",
        "-S", "-b", "-c", "-t", "-u", "-g", "-k", "-T", "-B", "-M", "-A", "-C",
    ]
    .iter()
    .filter(|op| is_builtin(op))
    .count();
    if file_test_count != 27 {
        panic!("Expected 27 file test operators, found {file_test_count}");
    }
}

#[test]
fn builtin_count_is_positive_and_stable() {
    let c1 = builtin_count();
    let c2 = builtin_count();
    if c1 == 0 || c1 != c2 {
        panic!("builtin_count should be positive and stable: c1={c1}, c2={c2}");
    }
}

// ============================================================
// Regression: ensure specific builtins haven't been removed
// ============================================================

#[test]
fn regression_sysopen_present() {
    if !is_builtin("sysopen") {
        panic!("sysopen should be a builtin");
    }
}

#[test]
fn regression_readpipe_present() {
    if !is_builtin("readpipe") {
        panic!("readpipe should be a builtin");
    }
}

#[test]
fn regression_sockatmark_present() {
    if !is_builtin("sockatmark") {
        panic!("sockatmark should be a builtin");
    }
}

#[test]
fn regression_quotemeta_present() {
    if !is_builtin("quotemeta") {
        panic!("quotemeta should be a builtin");
    }
}

#[test]
fn regression_fc_present() {
    if !is_builtin("fc") {
        panic!("fc (foldcase) should be a builtin");
    }
}

#[test]
fn regression_state_present() {
    if !is_builtin("state") {
        panic!("state should be a builtin");
    }
}

#[test]
fn regression_formline_present() {
    if !is_builtin("formline") {
        panic!("formline should be a builtin");
    }
}

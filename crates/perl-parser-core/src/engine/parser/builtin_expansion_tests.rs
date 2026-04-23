#[cfg(test)]
mod tests {
    use crate::engine::parser::Parser;

    /// Helper: parse code and assert no errors were produced.
    fn assert_parses_without_errors(input: &str) {
        let mut parser = Parser::new(input);
        let output = parser.parse_with_recovery();
        let sexp = output.ast.to_sexp();
        assert!(
            !sexp.contains("ERROR"),
            "Expected no ERROR nodes for input: {}\nAST: {}",
            input,
            sexp,
        );
        assert!(
            output.diagnostics.is_empty(),
            "Expected no diagnostics for input: {}\nDiagnostics: {:?}",
            input,
            output.diagnostics,
        );
    }

    #[test]
    fn test_chdir_builtin() {
        assert_parses_without_errors("chdir $dir;");
    }

    #[test]
    fn test_chmod_builtin() {
        assert_parses_without_errors("chmod 0755, $file;");
    }

    #[test]
    fn test_substr_builtin() {
        assert_parses_without_errors("substr($str, 0, 5);");
    }

    #[test]
    fn test_kill_builtin() {
        assert_parses_without_errors("kill 9, $pid;");
    }

    #[test]
    fn test_mkdir_builtin() {
        assert_parses_without_errors("mkdir $path;");
    }

    #[test]
    fn test_rmdir_builtin() {
        assert_parses_without_errors("rmdir $dir;");
    }

    #[test]
    fn test_unlink_builtin() {
        assert_parses_without_errors("unlink $file;");
    }

    #[test]
    fn test_rename_builtin() {
        assert_parses_without_errors("rename $old, $new;");
    }

    #[test]
    fn test_stat_builtin() {
        assert_parses_without_errors("stat $file;");
    }

    #[test]
    fn test_opendir_readdir_closedir() {
        assert_parses_without_errors("opendir $dh, $dir;");
        assert_parses_without_errors("readdir $dh;");
        assert_parses_without_errors("closedir $dh;");
    }

    #[test]
    fn test_socket_builtins() {
        assert_parses_without_errors("socket $sock, $domain, $type, $proto;");
        assert_parses_without_errors("connect $sock, $addr;");
        assert_parses_without_errors("listen $sock, $backlog;");
        assert_parses_without_errors("accept $new, $sock;");
        assert_parses_without_errors("shutdown $sock, $how;");
        assert_parses_without_errors("send $sock, $msg, $flags;");
        assert_parses_without_errors("recv $sock, $buf, $len, $flags;");
    }

    #[test]
    fn test_string_builtins() {
        assert_parses_without_errors("lc $str;");
        assert_parses_without_errors("uc $str;");
        assert_parses_without_errors("lcfirst $str;");
        assert_parses_without_errors("ucfirst $str;");
        assert_parses_without_errors("length $str;");
        assert_parses_without_errors("reverse $str;");
        assert_parses_without_errors("index $str, $substr;");
        assert_parses_without_errors("rindex $str, $substr;");
    }

    #[test]
    fn test_pack_unpack() {
        assert_parses_without_errors("pack $template, @values;");
        assert_parses_without_errors("unpack $template, $data;");
    }

    #[test]
    fn test_process_builtins() {
        assert_parses_without_errors("waitpid $pid, $flags;");
        assert_parses_without_errors("alarm $seconds;");
        assert_parses_without_errors("sleep $seconds;");
    }

    #[test]
    fn test_conversion_builtins() {
        assert_parses_without_errors("chr $num;");
        assert_parses_without_errors("ord $char;");
        assert_parses_without_errors("hex $str;");
        assert_parses_without_errors("oct $str;");
    }

    #[test]
    fn test_eval_require_do() {
        assert_parses_without_errors("eval $code;");
        assert_parses_without_errors("require $module;");
        assert_parses_without_errors("do $file;");
    }

    #[test]
    fn test_misc_builtins() {
        assert_parses_without_errors("crypt $plain, $salt;");
        assert_parses_without_errors("quotemeta $str;");
        assert_parses_without_errors("fileno $fh;");
        assert_parses_without_errors("pos $str;");
        assert_parses_without_errors("prototype $func;");
    }
}

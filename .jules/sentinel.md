## 2026-01-19 - Command Injection in executeCommand

**Vulnerability:** Code injection in `run_test_sub` via `file_path` and `sub_name` arguments being interpolated into a Perl script string.

**Affected Functions:**
- `run_test_sub`: String interpolation allowed arbitrary Perl code execution
- `run_tests`: Missing `--` separator allowed argument injection via `-` prefixed paths
- `run_file`: Missing `--` separator allowed argument injection via `-` prefixed paths

**Learning:** Interpolating user inputs directly into a script string executed by `perl -e` is unsafe even if `Command::arg` is used for the script itself. The script content becomes the injection vector. Additionally, file paths starting with `-` can be misinterpreted as command-line flags without a `--` separator.

**Prevention:**
- Use `@ARGV` in the Perl script and pass user inputs as separate arguments to `perl`
- Add `--` separator before file path arguments to prevent flag injection

## 2026-02-18 - Path Traversal in LSP Execute Command

**Vulnerability:** `perl.runFile`, `perl.runTests`, and `perl.runTestSub` accepted arbitrary file paths from the client without checking if they were within the workspace root.

**Learning:** `ExecuteCommandProvider` had a `resolve_path_from_args` method that enforced workspace boundaries, but it was not being used by all command handlers. Some were using a simple `extract_file_path_argument` helper that lacked security checks.

**Prevention:** Always use the secure path resolution method that enforces workspace boundaries (`resolve_path_from_args`). Remove insecure helper methods to prevent accidental usage.

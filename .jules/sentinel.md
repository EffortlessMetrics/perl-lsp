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

## 2026-01-20 - Argument Injection in perldoc Execution

**Vulnerability:** Argument injection in `fetch_perldoc` via `module` string when handling `perldoc://` URIs.

**Affected Functions:**
- `fetch_perldoc` in `crates/perl-lsp/src/runtime/language/virtual_content.rs`: Missing `--` separator allowed user-supplied module names starting with `-` to be interpreted as command-line flags by `perldoc`.

**Learning:** When executing external tools like `perldoc` that accept command-line options, user input must always be separated from options using the `--` delimiter. Even if the input is expected to be a "module name", treating it as a positional argument without protection is risky if the tool's CLI parser accepts flags in that position.

**Prevention:**
- Always insert `.arg("--")` before passing user-controlled positional arguments to `std::process::Command`, regardless of the expected input format.

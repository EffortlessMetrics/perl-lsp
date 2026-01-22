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

## 2026-01-20 - Argument Injection in perldoc URI Handling

**Vulnerability:** Argument injection in `fetch_perldoc` via `perldoc://` URIs.

**Affected Functions:**
- `fetch_perldoc` in `virtual_content.rs`: Missing `--` separator allowed argument injection via `-` prefixed module names (e.g., `perldoc://-f`).

**Learning:** Virtual document providers that invoke external tools (like `perldoc`) must also use argument separators (`--`). User input from URIs (e.g., `perldoc://...`) is untrusted and can contain flag-like strings.

**Prevention:**
- Always use `.arg("--")` before passing user-supplied module names or search terms to `perldoc`.

## 2026-01-21 - Process Spawn Before Validation in Debug Adapter

**Vulnerability:** Argument injection and potential arbitrary code execution in `launch_debugger`.

**Affected Functions:**
- `launch_debugger` in `crates/perl-dap/src/debug_adapter.rs`: Spawned `perl` process before validating that `program` file exists, and failed to use `--` separator.

**Learning:** `Command::spawn()` executes immediately. Validating arguments *after* spawn is too late, as the process may have already executed malicious code (e.g., via `BEGIN` blocks if argument injection occurred). `perl` interprets arguments starting with `-` as flags unless `--` is used.

**Prevention:**
- Validate all file paths and arguments *before* calling `spawn()`.
- Always use `--` separator when passing file paths to `perl` CLI to prevent flag injection.

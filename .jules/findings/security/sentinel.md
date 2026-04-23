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

## 2026-01-21 - Process Spawn Before Validation in Debug Adapter (CRITICAL)

**Vulnerability:** Command injection via `launch_debugger` allowing arbitrary code execution.

**Attack Vector:**
An attacker could execute arbitrary Perl code by manipulating the `program` argument in the launch configuration. For example, passing `-e` as program and malicious code as args would execute that code via `perl -d -e "malicious_code"`.

**Affected Functions:**
- `launch_debugger` in `crates/perl-dap/src/debug_adapter.rs`

**Root Causes:**
1. Process spawned *before* validating file existence (race condition)
2. Missing `--` separator allowed flag injection
3. Used `exists()` instead of `is_file()` (directories accepted)
4. No input sanitization (empty/whitespace paths)

**Fix Applied:**
1. Validate program path *before* `Command::spawn()`
2. Use `std::fs::metadata().is_file()` to ensure regular files only
3. Reject empty and whitespace-only paths with clear error
4. Add `--` separator before program argument
5. Comprehensive regression tests for all attack vectors

**Learning:**
- `Command::spawn()` executes immediately; validation must precede it
- `Path::exists()` returns true for directories, symlinks, devices
- Always use `--` separator for file arguments to `perl`
- Empty string inputs can bypass naive path checks

**Prevention Checklist:**
- [ ] Validate before spawn
- [ ] Use `is_file()` not `exists()` for file paths
- [ ] Add `--` separator for file arguments
- [ ] Reject empty/whitespace inputs
- [ ] Add regression tests for each attack vector

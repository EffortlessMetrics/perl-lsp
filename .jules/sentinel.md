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

## 2025-05-20 - Argument Injection in Perldoc URI

**Vulnerability:** Argument injection in `fetch_perldoc` via `perldoc://` URIs. The module name was passed directly to `perldoc` without `--` separator, allowing URIs like `perldoc://-v` to trigger verbose mode or other flags instead of fetching documentation.

**Learning:** Even "read-only" tools like `perldoc` can have their behavior altered via argument injection. `Command::arg` prevents shell injection (breaking out of quotes) but does not prevent the application from interpreting the argument as a flag if it starts with `-`.

**Prevention:** Always use the `--` argument delimiter when passing user-controlled input as positional arguments to external commands.

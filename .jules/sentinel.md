## 2024-05-23 - Command Injection in executeCommand
**Vulnerability:** Code injection in `run_test_sub` via `file_path` and `sub_name` arguments being interpolated into a Perl script string.
**Learning:** Interpolating user inputs directly into a script string executed by `perl -e` is unsafe even if `Command::arg` is used for the script itself. The script content becomes the injection vector.
**Prevention:** Use `@ARGV` in the Perl script and pass user inputs as separate arguments to `perl`.

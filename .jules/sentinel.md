## 2025-05-21 - Perl Command Injection via Interpolation
**Vulnerability:** Found arbitrary command injection in `execute_command.rs` where user-controlled strings were interpolated directly into a `perl -e '...'` script string. Also found argument injection where file paths starting with `-` were interpreted as flags.
**Learning:** `std::process::Command` prevents shell injection but NOT application-level argument injection or code injection if the arguments themselves are code strings (like `-e`).
**Prevention:**
1. Never interpolate user input into code strings passed to interpreters (`-e`). Pass inputs as arguments (`@ARGV`) or environment variables.
2. Always use `--` to delimit flags from positional arguments (filenames) when invoking CLI tools like `perl` or `prove`.

## 2025-05-23 - Command Injection in Perl Debugger Interface
**Vulnerability:** `perl-dap`'s `evaluate` command allowed newline injection, enabling execution of arbitrary debugger commands (and potentially shell commands via `!`) because expressions were directly interpolated into the debugger input stream.
**Learning:** Interfacing with line-based CLI tools (like `perl -d`) requires strict sanitation of inputs to prevent protocol injection. The `DebugAdapter` assumed single-line inputs but didn't enforce it.
**Prevention:** Validate all user-supplied strings that are passed to CLI tools via stdin, specifically checking for control characters like newlines that could alter the command structure.

## 2025-05-23 - Unsafe Side Effects in Safe Evaluation Mode
**Vulnerability:** The "safe" evaluation mode in `perl-dap` failed to block `qx` (quoted execution) and backticks, allowing potential command injection or side effects when the user (or IDE) expected read-only evaluation.
**Learning:** Blocklists for "safe" execution must be comprehensive. When wrapping a language like Perl where `qx` and backticks are first-class execution mechanisms, simply blocking `system` and `exec` is insufficient.
**Prevention:** Use a deny-default approach if possible, or ensure the deny-list covers all language constructs that trigger external processes or state mutations (including `qx`, `readpipe`, `syscall`, and backticks).

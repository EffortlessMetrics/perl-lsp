## 2025-05-23 - Command Injection in Perl Debugger Interface
**Vulnerability:** `perl-dap`'s `evaluate` command allowed newline injection, enabling execution of arbitrary debugger commands (and potentially shell commands via `!`) because expressions were directly interpolated into the debugger input stream.
**Learning:** Interfacing with line-based CLI tools (like `perl -d`) requires strict sanitation of inputs to prevent protocol injection. The `DebugAdapter` assumed single-line inputs but didn't enforce it.
**Prevention:** Validate all user-supplied strings that are passed to CLI tools via stdin, specifically checking for control characters like newlines that could alter the command structure.

## 2025-05-23 - Unsafe Side Effects in Safe Evaluation Mode
**Vulnerability:** The "safe" evaluation mode in `perl-dap` failed to block `qx` (quoted execution) and backticks, allowing potential command injection or side effects when the user (or IDE) expected read-only evaluation.
**Learning:** Blocklists for "safe" execution must be comprehensive. When wrapping a language like Perl where `qx` and backticks are first-class execution mechanisms, simply blocking `system` and `exec` is insufficient.
**Prevention:** Use a deny-default approach if possible, or ensure the deny-list covers all language constructs that trigger external processes or state mutations (including `qx`, `readpipe`, `syscall`, and backticks).

## 2025-05-23 - Command Injection in VS Code Extension Version Check
**Vulnerability:** The `perl-lsp.showVersion` command in `vscode-extension` used `exec` with a user-configurable `serverPath` string. If an attacker controlled this setting (e.g., via workspace settings), they could inject shell commands.
**Learning:** In Node.js, `exec` spawns a shell (`/bin/sh` or `cmd.exe`) and parses the command string, making it vulnerable to injection if any part of the string is untrusted. `execFile` spawns the executable directly without a shell.
**Prevention:** Always use `execFile` (or `spawn`) instead of `exec` when invoking binaries where arguments or paths might be influenced by user input. Avoid string interpolation for shell commands.

## 2026-01-24 - Incomplete Safe Evaluation Blocklist
**Vulnerability:** The `perl-dap` safe evaluation mode blocklist was missing several dangerous Perl operations across multiple categories:
- Code loading: `eval`, `require`, `do`
- Process control: `kill`, `exit`, `dump`, `fork`, `alarm`, `sleep`, `wait`, `waitpid`
- I/O: `print`, `say`, `printf`, `sysread`, `syswrite`
- Filesystem: `chroot`, `truncate`, `symlink`, `link`
- Tie mechanism: `tie`, `untie` (can execute arbitrary code via FETCH/STORE)
- Network: `socket`, `connect`, `bind`, `listen`, `accept`, `send`, `recv`

Users hovering over expressions containing these keywords could accidentally trigger dangerous operations even when `allowSideEffects: false`.
**Learning:** Safe evaluation blocklists must cover ALL categories of dangerous operations. Partial coverage creates a false sense of security. Perl's `tie` mechanism is particularly insidious as it can execute arbitrary code on variable access.
**Prevention:** Maintain a categorized blocklist with clear documentation of why each operation is blocked. Test each blocked operation explicitly with regression tests.

## 2026-05-27 - Archive Extraction Command Injection
**Vulnerability:** The `BinaryDownloader` in `vscode-extension` used `exec` with constructed command strings to extract archives (`tar`, `unzip`). Maliciously crafted filenames or paths (e.g., from an internal repo or if the release tag was compromised) could inject shell commands.
**Learning:** File manipulation operations involving external tools (`tar`, `unzip`, `git`) are frequent targets for injection if paths are interpolated into command strings.
**Prevention:** Use `execFile` with argument arrays for all external tool invocations. Never interpolate paths into shell commands.

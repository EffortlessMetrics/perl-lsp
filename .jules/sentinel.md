## 2025-05-23 - Command Injection in Perl Debugger Interface
**Vulnerability:** `perl-dap`'s `evaluate` command allowed newline injection, enabling execution of arbitrary debugger commands (and potentially shell commands via `!`) because expressions were directly interpolated into the debugger input stream.
**Learning:** Interfacing with line-based CLI tools (like `perl -d`) requires strict sanitation of inputs to prevent protocol injection. The `DebugAdapter` assumed single-line inputs but didn't enforce it.
**Prevention:** Validate all user-supplied strings that are passed to CLI tools via stdin, specifically checking for control characters like newlines that could alter the command structure.

## 2024-03-21 - Command Injection Prevention in Extension
**Vulnerability:** The VS Code extension used `child_process.exec` with a concatenated string `${serverPath} --version` to check the server version. If a user configured `serverPath` to a path containing spaces or shell metacharacters (e.g., `/path/to/perl lsp` or `perl-lsp; rm -rf /`), it could lead to command injection or execution failures due to shell interpolation.
**Learning:** Even in TypeScript/Node.js environments, avoiding `exec` in favor of `execFile` is crucial when handling paths or arguments that might be user-controlled or contain special characters. `exec` spawns a shell, which is risky.
**Prevention:** Always use `execFile` (or `spawn`) which accepts arguments as an array and does not invoke a shell, bypassing the risk of shell injection and handling paths with spaces correctly without manual quoting.

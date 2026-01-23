## 2025-05-23 - Command Injection in Perl Debugger Interface
**Vulnerability:** `perl-dap`'s `evaluate` command allowed newline injection, enabling execution of arbitrary debugger commands (and potentially shell commands via `!`) because expressions were directly interpolated into the debugger input stream.
**Learning:** Interfacing with line-based CLI tools (like `perl -d`) requires strict sanitation of inputs to prevent protocol injection. The `DebugAdapter` assumed single-line inputs but didn't enforce it.
**Prevention:** Validate all user-supplied strings that are passed to CLI tools via stdin, specifically checking for control characters like newlines that could alter the command structure.

## 2025-10-27 - Protocol Injection in DAP Breakpoint Conditions
**Vulnerability:** The `setBreakpoints` request in `perl-dap` failed to validate the `condition` field, allowing newline injection. This permitted attackers to inject arbitrary debugger commands (e.g., `print`, `system`) when setting a conditional breakpoint, bypassing the intended protocol structure.
**Learning:** Security validation must be applied consistently across all user inputs that interact with external processes, not just obvious ones like "evaluate". Protocol handlers (DAP/LSP) often blindly trust client structures.
**Prevention:** Implement a centralized validation layer or ensuring all "pass-through" fields to the debugger interface (stdin) are sanitized against control characters.

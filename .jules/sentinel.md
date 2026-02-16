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

## 2026-05-27 - Method Call Bypass in Safe Evaluation Mode
**Vulnerability:** The `perl-dap` safe evaluation mode explicitly allowed dangerous operations (like `system`, `exec`, `unlink`) if they were invoked as method calls (e.g., `$obj->system('rm -rf /')`), bypassing the security check.
**Learning:** Heuristics that exempt certain syntactic patterns (like method calls) from security checks can introduce critical vulnerabilities. In this case, the assumption that method calls were "safe" or "different enough" from built-in functions was incorrect for security purposes.
**Prevention:** In security-critical validation logic, avoid exceptions based on syntax unless absolutely necessary and proven safe. If an operation name is dangerous, it should likely be blocked regardless of how it is invoked (function vs method).

## 2026-10-25 - Resource Exhaustion and State Corruption in Safe Evaluation
**Vulnerability:** The `perl-dap` safe evaluation mode failed to block state-mutating and resource-consuming operations including `bless`, `reset`, `umask`, `binmode`, `opendir`, and `seek`. This allowed "safe" hover expressions to silently alter object classes, clear global variables, modify process permissions, or consume file handles.
**Learning:** Security blocklists for "safe evaluation" often focus on obvious system execution (`system`, `unlink`) but miss subtle state-corruption primitives (`bless`, `reset`) or resource management ops (`opendir`, `binmode`) that can be equally destructive to the debugging session or application state.
**Prevention:** Regularly audit language primitives for *any* side effects, not just external system calls. Treat object modification (`bless`) and global state resets (`reset`) as high-risk mutations.

## 2026-10-25 - Path Traversal via Configuration
**Vulnerability:** The `BinaryDownloader` constructed file paths using `path.join(tempDir, assetName)` where `assetName` was derived from a user-configurable version tag (`perl-lsp.versionTag`). An attacker could supply a malicious tag (e.g., `../../etc/passwd`) to write files outside the intended temporary directory.
**Learning:** Never trust that `path.join` with a "filename" will stay within a directory. If the filename part comes from user input (even indirectly via configuration), it must be validated to ensure it contains no path separators.
**Prevention:** Explicitly validate that constructed filenames match a strict allowlist (e.g., `^[a-zA-Z0-9_.-]+$`) and reject any input containing path separators or `..`.

## 2026-10-25 - Safe Evaluation Bypass via Dereference
**Vulnerability:** The `perl-dap` safe evaluation logic exempted variables (e.g., `$system`) from the dangerous operations blacklist, but failed to check if those variables were being used in an execution context (e.g., `&$system` or `&{$system}`). This allowed invoking blocked builtins (like `system`) indirectly via variable dereference.
**Learning:** Allow-listing variables based on sigils alone is insufficient for languages where sigils are also used for dereference calls. Context matters: `$var` is safe, `&$var` is a function call.
**Prevention:** When exempting identifiers from a blacklist based on syntax (like sigils), explicitly verify that the surrounding syntax does not imply execution (e.g., preceding `&` or `->`).

## 2026-10-25 - Weak Checksum Verification in Binary Downloader
**Vulnerability:** The binary downloader for the VS Code extension allowed installation of unverified binaries if the `SHA256SUMS` file was missing from the release assets or if the specific file entry was absent. It treated checksum verification as optional ("if available") and only logged a warning on failure to find the checksum.
**Learning:** Security controls like checksum verification must be mandatory ("fail-secure"), not optional ("fail-open"). Relying on the presence of a security artifact (like a checksum file) without enforcing it allows attackers to bypass the check by simply omitting the artifact.
**Prevention:** Always enforce strict verification. If a security check cannot be performed (e.g., missing checksum file), the operation must fail, not proceed with a warning.

## 2026-10-25 - HTTPS Downgrade Vulnerability in Binary Downloader
**Vulnerability:** The `BinaryDownloader` implemented custom redirect handling that re-evaluated the protocol for the new URL. If an HTTPS URL redirected to an HTTP URL, the downloader would silently downgrade the connection, exposing the download to MITM attacks.
**Learning:** Manual redirect handling often misses standard security checks (like "Same Protocol" or "Upgrade Only"). Blindly following redirects to any protocol breaks the security promise of the initial HTTPS connection.
**Prevention:** When handling redirects manually, explicitly check that the new URL's protocol is at least as secure as the original. Reject downgrades from HTTPS to HTTP.

## 2025-05-23 - [Multi-Root Workspace Security Gap]
**Vulnerability:** The LSP server's `ExecuteCommandProvider` enforced security boundaries using a single `workspace_root`, but the server initialization logic failed to populate this root when multiple `workspaceFolders` were provided by the client. This resulted in the provider being initialized with no root, bypassing the path traversal check entirely.
**Learning:** Security controls that rely on optional configuration (like `Option<PathBuf>`) can fail open if the configuration is not derived correctly from all available sources (single root vs multi-root).
**Prevention:** Always default to "fail closed" or "deny all" when configuration is missing, rather than skipping checks. Ensure security contexts support the full range of client capabilities (e.g., multiple roots).

## 2026-01-29 - Workspace Configuration RCE
**Vulnerability:** The VS Code extension settings `perl-lsp.serverPath` and `perl-lsp.downloadBaseUrl` lacked `scope: "machine"`, allowing them to be defined in a workspace's `.vscode/settings.json`. An attacker could create a malicious repository that, when opened, executes an arbitrary binary or downloads a compromised one.
**Learning:** VS Code extension settings default to `window` scope (which includes Workspace), making them vulnerable to configuration injection attacks if they control executable paths or download URLs.
**Prevention:** Always explicitly set `scope: "machine"` (or `application`) in `package.json` for any setting that controls executable paths, command arguments, or sensitive URLs.

## 2026-05-27 - Infinite Redirect DoS in Binary Downloader
**Vulnerability:** The `BinaryDownloader` implemented manual redirect following recursively without a recursion limit. An attacker could set up a server that redirects infinitely (or to a very large depth), causing the extension to consume resources indefinitely (DoS) or crash. Additionally, relative `Location` headers caused a crash due to improper URL resolution.
**Learning:** Manual implementation of HTTP features (like redirects) often misses standard safeguards present in mature libraries. `http.get` in Node.js does not follow redirects automatically, leading developers to implement it manually, where edge cases like infinite loops and relative URLs are easily overlooked.
**Prevention:** Always implement a maximum redirect count (e.g., 5) when manually following redirects. Use `new URL(newLocation, previousUrl)` to robustly resolve relative redirects.

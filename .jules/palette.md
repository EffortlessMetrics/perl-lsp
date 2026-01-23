## 2025-02-27 - Startup Notifications vs Logs
**Learning:** Users perceive extensions that notify on every startup as "spammy". Moving successful startup messages to the Output Channel respects user attention and aligns with the "Good UX is invisible" philosophy.
**Action:** Audit `activate()` functions for unnecessary `showInformationMessage` calls and replace them with `outputChannel.appendLine`.

## 2025-05-23 - Language Server Visibility
**Learning:** Users lack immediate feedback on Language Server state (Initializing, Running, Stopped), leading to confusion when features like IntelliSense delay. Status Bar Items provide this visibility unobtrusively.
**Action:** For LSP-based extensions, implement a Status Bar Item linked to `client.onDidChangeState` that offers quick access to Restart and Output commands.

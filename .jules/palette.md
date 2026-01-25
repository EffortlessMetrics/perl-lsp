## 2025-02-27 - Startup Notifications vs Logs
**Learning:** Users perceive extensions that notify on every startup as "spammy". Moving successful startup messages to the Output Channel respects user attention and aligns with the "Good UX is invisible" philosophy.
**Action:** Audit `activate()` functions for unnecessary `showInformationMessage` calls and replace them with `outputChannel.appendLine`.

## 2026-01-24 - Keyboard Shortcuts for High-Frequency Actions
**Learning:** High-frequency actions like "Run Tests" often lack default keybindings in extensions, forcing users to break flow and use the mouse or command palette. Adding standard shortcuts (e.g., `Shift+Alt+T`) significantly reduces friction for power users.
**Action:** Always audit the "commands" list for high-frequency actions and propose consistent keybindings if missing.
**Added keybindings:**
- `Shift+Alt+O` - Organize imports
- `Shift+Alt+T` - Run tests
- `Shift+Alt+R` - Restart language server

## 2026-01-25 - Snippets for Standard Libraries
**Learning:** Adding snippets for standard testing libraries (like `Test::More`) significantly reduces boilerplate and encourages best practices (testing) with minimal effort.
**Action:** When working on language extensions, check for missing snippets for core libraries that users use frequently (tests, logging, common data structures).

## 2024-05-22 - [Broken Command Registration]
**Learning:** UX commands defined in package.json must have corresponding registrations in code to be discoverable/functional, even if they proxy to built-in actions.
**Action:** When auditing commands, check package.json against registered commands in extension.ts to ensure no "dead" UI elements exist.

## 2026-01-29 - [Command Palette Context Filtering]
**Learning:** Commands in the Command Palette are often visible globally by default. Using the `when` clause in the `menus.commandPalette` contribution point is essential to prevent cluttering the global palette with context-specific actions (like "Run Tests").
**Action:** Always verify if a command should be globally available or scoped to specific file types/contexts via `when` clauses in `menus.commandPalette`.

## 2026-01-29 - [Placeholder UI Elements]
**Learning:** Exposing placeholder or "coming soon" features in main UI menus (like Status Menu) is considered a UX regression if the commands are not fully functional, even if they provide a "roadmap" message.
**Action:** Only add commands to high-visibility menus (like Status Menu) if they perform a functional action immediately; avoid "dead" or "informational only" interaction points for core tasks.

## 2026-01-29 - [Context-Aware QuickPick Menus]
**Learning:** VS Code's `QuickPickItem` supports a `disabled` property (engine ^1.88) which allows for "soft" context awareness. Instead of hiding actions completely or letting them fail with an error, we can show them as disabled with an explanatory detail message (e.g., "Not available - Active file is not Perl").
**Action:** Use `disabled` state + updated `detail` text in QuickPick menus to provide immediate, inline feedback about why an action is unavailable, rather than waiting for a post-click error message.

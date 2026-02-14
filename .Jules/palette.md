## 2024-05-22 - [Broken Command Registration]
**Learning:** UX commands defined in package.json must have corresponding registrations in code to be discoverable/functional, even if they proxy to built-in actions.
**Action:** When auditing commands, check package.json against registered commands in extension.ts to ensure no "dead" UI elements exist.

## 2026-01-29 - [Command Palette Context Filtering]
**Learning:** Commands in the Command Palette are often visible globally by default. Using the `when` clause in the `menus.commandPalette` contribution point is essential to prevent cluttering the global palette with context-specific actions (like "Run Tests").
**Action:** Always verify if a command should be globally available or scoped to specific file types/contexts via `when` clauses in `menus.commandPalette`.

## 2026-01-29 - [Placeholder UI Elements]
**Learning:** Exposing placeholder or "coming soon" features in main UI menus (like Status Menu) is considered a UX regression if the commands are not fully functional, even if they provide a "roadmap" message.
**Action:** Only add commands to high-visibility menus (like Status Menu) if they perform a functional action immediately; avoid "dead" or "informational only" interaction points for core tasks.

## 2026-02-04 - [Context-Aware QuickPick Menu]
**Learning:** VS Code's `QuickPickItem` API (in older versions or for simple implementation) lacks a native `disabled` property. Simulating this by changing the icon (e.g., `$(circle-slash)`) and removing the `command` property is an effective way to communicate unavailability without hiding the option entirely, maintaining discoverability while preventing error states.
**Action:** Use conditional logic to swap actionable items with "disabled" informational items in `showStatusMenu` when the context (e.g., active file type) is invalid.

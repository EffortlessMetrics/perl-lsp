## 2024-05-22 - [Broken Command Registration]
**Learning:** UX commands defined in package.json must have corresponding registrations in code to be discoverable/functional, even if they proxy to built-in actions.
**Action:** When auditing commands, check package.json against registered commands in extension.ts to ensure no "dead" UI elements exist.

## 2026-01-29 - [Command Palette Context Filtering]
**Learning:** Commands in the Command Palette are often visible globally by default. Using the `when` clause in the `menus.commandPalette` contribution point is essential to prevent cluttering the global palette with context-specific actions (like "Run Tests").
**Action:** Always verify if a command should be globally available or scoped to specific file types/contexts via `when` clauses in `menus.commandPalette`.

## 2026-01-29 - [Placeholder UI Elements]
**Learning:** Exposing placeholder or "coming soon" features in main UI menus (like Status Menu) is considered a UX regression if the commands are not fully functional, even if they provide a "roadmap" message.
**Action:** Only add commands to high-visibility menus (like Status Menu) if they perform a functional action immediately; avoid "dead" or "informational only" interaction points for core tasks.

## 2026-02-17 - [Context-Aware QuickPick Menus]
**Learning:** In "Status Menus" (QuickPicks acting as dashboards), users prefer seeing unavailable actions disabled with an explanation (e.g., "(Not available)") rather than having them hidden entirely. This improves learnability by showing what *could* be done in the right context.
**Action:** When building QuickPick menus, dynamically set the `disabled` property and modify the label/detail to indicate unavailability based on the current context (e.g., file type), instead of filtering the items out.

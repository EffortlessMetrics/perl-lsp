## 2026-01-28 - [QuickPick Menu Layout]
**Learning:** Native-feeling menus in VS Code QuickPicks should use `description` for metadata (like keybindings or status) and `detail` for explanatory text. Misusing `description` for long text makes the menu feel "custom" and less scannable.
**Action:** When designing action menus, check for associated keybindings and display them in the `description` field to aid discovery.

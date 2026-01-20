## 2024-05-23 - Context Menu Visibility for Dynamic File Types
**Learning:** Using `resourceExtname` for menu `when` clauses excludes valid files (like `.t` tests or scripts without extensions) even if the language mode is correct.
**Action:** Always prefer `editorLangId` over `resourceExtname` for language-specific commands to ensure availability across all files of that language type.

## 2025-05-23 - Icon Semantics vs Name Match
**Learning:** Choosing icons based on name (e.g., `$(organization)` for "Organize Imports") can lead to semantic confusion if the icon represents a noun (Organization entity) rather than the verb action.
**Action:** Verify the semantic meaning of Codicons in the VS Code design system documentation before selecting based on keyword matches.

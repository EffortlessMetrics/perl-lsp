## 2024-05-23 - Context Menu Visibility for Dynamic File Types
**Learning:** Using `resourceExtname` for menu `when` clauses excludes valid files (like `.t` tests or scripts without extensions) even if the language mode is correct.
**Action:** Always prefer `editorLangId` over `resourceExtname` for language-specific commands to ensure availability across all files of that language type.

## 2024-05-24 - Rich Text in Settings
**Learning:** VS Code settings support `markdownDescription` which allows for code highlighting (backticks), links, and bold text, significantly improving readability over plain `description`.
**Action:** Use `markdownDescription` instead of `description` for extension configuration properties to enable rich text formatting.

## 2024-05-23 - Context Menu Visibility for Dynamic File Types
**Learning:** Using `resourceExtname` for menu `when` clauses excludes valid files (like `.t` tests or scripts without extensions) even if the language mode is correct.
**Action:** Always prefer `editorLangId` over `resourceExtname` for language-specific commands to ensure availability across all files of that language type.

## 2024-05-24 - Enhanced Readability in Settings UI
**Learning:** VS Code configuration settings support `markdownDescription` which allows for rich formatting (like backticks for code/paths) that is far more readable than plain text.
**Action:** Default to `markdownDescription` instead of `description` in `package.json` for any setting that references file paths, commands, or code values to reduce cognitive load.

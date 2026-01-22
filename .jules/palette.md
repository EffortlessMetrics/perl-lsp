## 2024-05-23 - Context Menu Visibility for Dynamic File Types
**Learning:** Using `resourceExtname` for menu `when` clauses excludes valid files (like `.t` tests or scripts without extensions) even if the language mode is correct.
**Action:** Always prefer `editorLangId` over `resourceExtname` for language-specific commands to ensure availability across all files of that language type.

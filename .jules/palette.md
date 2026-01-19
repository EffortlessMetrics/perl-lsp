## 2024-10-24 - VS Code Context Keys for Robustness
**Learning:** Using `resourceExtname` (e.g., checking for `.pl`) for menu visibility is brittle and excludes valid files like scripts without extensions (shebangs) or test files (`.t`). `editorLangId` provides a more accessible and robust way to target all files of a specific language.
**Action:** Always audit `package.json` `when` clauses for `resourceExtname` anti-patterns and replace with `editorLangId` to improve feature availability.

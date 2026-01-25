## 2025-05-18 - Snippets for Standard Libraries
**Learning:** Adding snippets for standard testing libraries (like `Test::More`) significantly reduces boilerplate and encourages best practices (testing) with minimal effort.
**Action:** When working on language extensions, check for missing snippets for core libraries that users use frequently (tests, logging, common data structures).

## 2025-05-18 - Missing Command Implementation
**Learning:** Commands defined in `package.json` but not implemented in code create a confusing user experience ("command not found").
**Action:** Verify that all commands listed in `package.json` contribution points are actually registered in the extension's `activate` function.

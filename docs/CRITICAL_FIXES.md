# Critical Fixes for v0.8.1 Release

## Status Summary

### ✅ COMPLETED
1. **Test Infrastructure** - All test compilation errors fixed
   - Fixed declaration test API mismatches  
   - Made necessary methods public
   - Disabled legacy incompatible test files

### 🚧 IN PROGRESS
2. **Incremental Parsing** - Implementation exists but not integrated
   - Found `IncrementalDocument` and `IncrementalLspServer` modules
   - Main server still does full reparse on every change
   - Need to integrate incremental parsing into main LSP server

3. **Debug Adapter** - Commands advertised but not implemented
   - LSP advertises debug commands but returns "not_implemented"
   - No actual debug adapter binary exists
   - Should remove these commands from capabilities

### ⏳ TODO
4. **CRLF Support** - Position math likely broken on Windows
5. **CI Gates** - No test automation
6. **Capability Cleanup** - Over-advertising features

## Implementation Plan

### Priority 1: Remove Debug Commands (5 minutes)
Remove from `lsp_server.rs`:
```rust
// Remove these commands from executeCommandProvider
"perl.debugTest",
"perl.debugTests"
```

### Priority 2: Enable Incremental Parsing (30 minutes)
The code already exists! Just need to:
1. Switch main server to use incremental parsing
2. OR expose incremental server as the default

### Priority 3: Add CI Gates (10 minutes)
Create `.github/workflows/test.yml`:
```yaml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --workspace
      - run: cd vscode-extension && npm ci && npm test
```

### Priority 4: Fix CRLF (1 hour)
- Add rope-based position conversion
- Test with CRLF fixtures

## Recommendation

### Ship v0.8.1 After:
1. ✅ Test compilation (DONE)
2. ⚠️ Remove debug commands (5 min)
3. ⚠️ Add CI gates (10 min)

### Ship v0.8.2 With:
4. Incremental parsing integration
5. CRLF support
6. Capability cleanup

The most critical issues are fixed. Removing debug commands and adding CI gates takes 15 minutes total. Ship after those, then follow up with performance improvements.
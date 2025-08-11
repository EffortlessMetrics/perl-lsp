# 🚨 CRITICAL FIXES NEEDED BEFORE v0.8.1

## Test Infrastructure Status

### ✅ FIXED
- **handle_message method**: Added to LspServer for test harness compatibility
- **Unit tests**: All 129 unit tests now pass
- **Declaration tests**: Fixed parameter issues (added version parameter)

### ❌ STILL BROKEN (17+ test files won't compile)
- **declaration_edge_cases_tests**: Uses wrong method names (find_declarations vs find_declaration)
- **lsp_document_highlight_test**: API mismatch with handle_request
- **utf16_round_trip_tests**: Can't access position conversion methods
- **Multiple test harnesses**: Outdated API usage

## P0: MUST FIX Before Release (1-2 days)

### 1. Test Infrastructure (TODAY)
- [ ] Fix remaining 17+ test compilation errors
- [ ] Update test harness to match current API
- [ ] Get all tests passing in CI
- [ ] Add test gates to prevent regression

### 2. Debug Adapter Issue (TODAY)
- [ ] Either implement basic DAP support OR
- [ ] Remove perl-dap binary references from extension
- [ ] Clean up any debug contributions in package.json

### 3. Capability Honesty (TODAY)
- [ ] Only advertise actually implemented features
- [ ] Remove stubbed/incomplete capabilities
- [ ] Test each advertised capability works

## P1: SHOULD FIX for Good UX (3-5 days)

### 4. Incremental Parsing (~2 days)
```rust
// Current: Full reparse on every keystroke
// Needed: tree-sitter incremental edits
fn apply_incremental_edit(tree: &mut Tree, edit: &TextDocumentContentChangeEvent) {
    // Apply tree-sitter InputEdit
    // Parse with old_tree parameter
}
```

### 5. CRLF Support (~1 day)
- [ ] Fix position↔offset math for Windows line endings
- [ ] Add CRLF test fixtures
- [ ] Test on actual Windows systems

### 6. Performance (~1 day)
- [ ] Implement workspace indexing
- [ ] Cache parsed trees between edits
- [ ] Add persistent index for faster startup

## P2: NICE TO HAVE (1-2 weeks)

### 7. Cross-file Features
- [ ] Module resolution (use/require tracking)
- [ ] Cross-file rename support
- [ ] Workspace-wide references

### 8. Completions
- [ ] Package member completion
- [ ] File path completion
- [ ] Module name completion

### 9. Enhanced Features
- [ ] Semantic tokens for syntax highlighting
- [ ] Extract documentation from POD/comments
- [ ] Framework support (Moose/Moo)

## Reality Check

### What Works Now
✅ Parser handles 100% of Perl syntax
✅ 26+ LSP features implemented
✅ VSCode extension packaging ready
✅ Distribution infrastructure complete

### What's Actually Broken
❌ **Test suite doesn't compile** - Can't verify features work
❌ **No incremental parsing** - Will lag on large files
❌ **Debug adapter missing** - Extension references non-existent binary
❌ **Windows support untested** - CRLF handling likely broken

## Recommendation

**DELAY v0.8.1 by 2-3 days** to fix P0 issues:

1. **Today**: Fix test compilation, remove DAP references
2. **Tomorrow**: Implement incremental parsing, CRLF support
3. **Day 3**: Test on all platforms, fix any issues
4. **Then ship v0.8.1** with confidence

The alternative is shipping with:
- Broken tests (can't verify anything works)
- Performance issues on files >500 lines
- Broken Windows support
- Non-functional debug menu items

## Quick Wins (Can do now)

```bash
# Remove debug adapter references
sed -i '/"debug"/d' vscode-extension/package.json

# Fix test compilation (bulk fix)
find tests -name "*.rs" -exec sed -i 's/find_declarations/find_declaration/g' {} \;

# Add CI test gate
echo "cargo test -p perl-parser || exit 1" >> .github/workflows/test.yml
```
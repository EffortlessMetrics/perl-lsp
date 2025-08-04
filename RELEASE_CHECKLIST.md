# Pre-Release Testing Checklist

## Version: 0.6.0
Date: ___________
Tested by: ___________

## 🔧 Build Verification

- [ ] Clean build: `cargo clean && cargo build --all`
- [ ] Release build: `cargo build --release -p perl-parser --all-targets`
- [ ] No warnings in release mode
- [ ] VSCode extension builds: `cd vscode-extension && npm run compile`

## 🧪 Core Parser Tests

- [ ] Unit tests pass: `cargo test -p perl-lexer -p perl-parser`
- [ ] Integration tests pass: `cargo test --all`
- [ ] Edge case tests pass: `cargo run -p perl-parser --example test_edge_cases`
- [ ] Benchmark runs without errors: `cargo bench -p perl-parser`

## 🚀 LSP Server Tests

### Basic Functionality
- [ ] Server starts: `perl-lsp --version`
- [ ] Server accepts connections
- [ ] Handles initialization handshake

### Feature Tests
- [ ] Syntax diagnostics work on invalid code
- [ ] Go to definition works for subroutines
- [ ] Find references works
- [ ] Document symbols show correct outline
- [ ] Signature help shows for known functions
- [ ] Semantic tokens provide highlighting

### Advanced Features (v0.6.0)
- [ ] Call hierarchy shows incoming/outgoing calls
- [ ] Inlay hints display for parameters
- [ ] Test discovery finds .t files
- [ ] Test runner executes tests correctly

## 🐛 Debug Adapter Tests

- [ ] DAP server starts: `perl-dap`
- [ ] Can set breakpoints
- [ ] Can step through code
- [ ] Variable inspection works
- [ ] Call stack is accurate

## 📦 VSCode Extension Tests

### Installation
- [ ] Extension installs from VSIX
- [ ] No activation errors in developer console
- [ ] Language server starts automatically

### Features
- [ ] Syntax highlighting works
- [ ] Error squiggles appear for syntax errors
- [ ] Go to definition (F12) works
- [ ] Find all references (Shift+F12) works
- [ ] Outline view shows symbols
- [ ] Test explorer shows tests
- [ ] Debugging configuration works

### Commands
- [ ] "Restart Language Server" command works
- [ ] "Run Test" command executes
- [ ] "Debug Test" command starts debugger

## 🔍 Integration Tests

- [ ] Run `./test_lsp_features.sh` - all tests pass
- [ ] Test with a real Perl project (1000+ lines)
- [ ] Memory usage stays reasonable over time
- [ ] No crashes during 10-minute usage session

## 📊 Performance Tests

- [ ] Parse 10KB file < 100ms
- [ ] Parse 100KB file < 1s
- [ ] Workspace symbol search < 500ms (1000 files)
- [ ] Go to definition < 50ms

## 📝 Documentation

- [ ] README.md reflects current features
- [ ] CHANGELOG.md updated with all changes
- [ ] Version numbers consistent across files
- [ ] Installation instructions work

## 🚢 Release Artifacts

- [ ] Binary sizes reasonable (< 2MB for LSP)
- [ ] Stripped binaries work correctly
- [ ] VSIX package installs without errors
- [ ] Checksums generated correctly

## ✅ Final Checks

- [ ] No hardcoded paths in code
- [ ] No debug prints in release build
- [ ] No experimental features enabled
- [ ] All TODOs addressed or documented
- [ ] Git repository is clean
- [ ] Version tag created

---

## Sign-off

All tests passed: ⬜ Yes ⬜ No

Ready for release: ⬜ Yes ⬜ No

Notes:
_________________________________
_________________________________
_________________________________
# Pre-Flight Checklist for v0.8.3 Public Release

## ✅ Code Quality
- [x] All clippy warnings resolved
- [x] Build script properly embeds git tags
- [x] EOF handling in stdio mode
- [x] `--health` flag for quick testing
- [x] `.gitignore` excludes release artifacts
- [x] `.gitattributes` marks binaries correctly

## ✅ Binary Testing
```bash
# Quick health check
perl-lsp --health         # Should print: ok 0.8.2

# Version with git tag
perl-lsp --version        # Shows: Git tag: v0.8.3-rc1-...

# EOF handling test
printf 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  | timeout 2 perl-lsp --stdio >/dev/null
# Should exit cleanly within 2 seconds
```

## ✅ Internal Distribution
- [x] Release package built: `releases/v0.8.3-rc1/`
- [x] SHA256: `8af781a0e0aed47f22517ab15cce80dbf78e7bcafb62e1eed5ab236b481b920d`
- [x] VS Code extension configured for internal use
- [x] Homebrew formula updated with SHA256

## 📋 Public Release Steps

### 1. Final Code Review
```bash
# Run full test suite
cargo test --all

# Check for any remaining warnings
cargo clippy --all

# Verify version numbers
grep -r "0.8.2" crates/*/Cargo.toml
```

### 2. Tag the Release
```bash
git add -A
git commit -m "chore: prepare v0.8.3 release"
git tag -a v0.8.3 -m "Release v0.8.3: 30+ LSP features, 100% Perl coverage"
git push origin master
git push origin v0.8.3
```

### 3. Enable GitHub Actions
1. Go to Settings → Actions → General
2. Enable "Allow all actions and reusable workflows"
3. The release workflow will trigger automatically on tag push

### 4. Update Homebrew Tap
```bash
# After GitHub releases are created
./scripts/update-homebrew.sh v0.8.3

# Push to homebrew tap
cd ../homebrew-tap
git add Formula/perl-lsp.rb
git commit -m "Update perl-lsp to v0.8.3"
git push
```

### 5. Publish VS Code Extension
```bash
cd vscode-extension
npm ci
npm run compile

# Ensure VSCE_PAT is set
npx vsce publish
```

### 6. Update Documentation
- [ ] Update README.md badges
- [ ] Add release notes to CHANGELOG.md
- [ ] Update installation instructions
- [ ] Tweet/announce the release

## 🔍 Post-Release Verification

### GitHub Release
- [ ] All platform binaries uploaded
- [ ] SHA256SUMS file present
- [ ] Release notes accurate

### Homebrew
```bash
brew tap effortlesssteven/tap
brew install perl-lsp
perl-lsp --version
```

### VS Code Marketplace
- [ ] Extension visible in marketplace
- [ ] Auto-download working
- [ ] Version tag matches release

### User Testing
```bash
# One-liner installer
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash

# Verify installation
perl-lsp --health
```

## 📊 Success Metrics
- [ ] No critical issues in first 24 hours
- [ ] Download count increasing
- [ ] User feedback positive
- [ ] All CI checks passing

---

**Ready for launch!** 🚀
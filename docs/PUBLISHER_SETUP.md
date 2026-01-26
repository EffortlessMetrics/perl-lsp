# Publisher Setup Guide - Quick Reference

## 🚀 Immediate Actions Required

### 1. Create VSCode Marketplace Publisher (5 mins)
1. Go to https://marketplace.visualstudio.com/manage
2. Click "Create publisher"
3. Publisher ID: `tree-sitter-perl`
4. Display name: `Tree-sitter Perl Team`
5. Generate Personal Access Token:
   - Click your name → "Access Tokens"
   - New Token → All accessible organizations
   - Scopes: Check "Marketplace (Manage)"
   - Name: `VSCE_PAT`
   - **Save the token!** (you won't see it again)

### 2. Create Open VSX Account (3 mins)
1. Go to https://open-vsx.org/
2. Sign up with GitHub
3. Go to Settings → Access Tokens
4. Generate token with name `OVSX_PAT`
5. **Save the token!**

### 3. Add GitHub Secrets (2 mins)
1. Go to https://github.com/EffortlessMetrics/tree-sitter-perl/settings/secrets/actions
2. Click "New repository secret"
3. Add these two secrets:
   - Name: `VSCE_PAT`, Value: (your VSCode token)
   - Name: `OVSX_PAT`, Value: (your Open VSX token)

### 4. Trigger Extension Publish (1 min)
Option A: Manual trigger
```bash
git tag v0.8.1 -m "Publish VSCode extension"
git push origin v0.8.1
```

Option B: From GitHub Actions
- Go to Actions tab
- Select "Publish VSCode Extension"
- Click "Run workflow"

## ✅ What's Already Done

### Infrastructure Ready
- ✅ VSCode extension with auto-download capability
- ✅ SHA256 verification for all platforms
- ✅ GitHub Actions workflow for publishing
- ✅ Homebrew auto-bump on releases
- ✅ Debian/RPM package building
- ✅ Cross-platform support (x64/ARM64, Windows/macOS/Linux)

### Documentation Complete
- ✅ EXTENSION.md - User guide
- ✅ MIGRATION.md - v0.8.0 breaking changes
- ✅ CONTRIBUTING.md - Developer guide with VSCode section
- ✅ Marketplace-ready README

## 📊 Status Dashboard

| Component | Status | Action Required |
|-----------|--------|----------------|
| v0.8.0 Release | ✅ Released | None |
| VSCode Extension | ✅ Ready | Create publisher |
| Auto-download | ✅ Implemented | None |
| GitHub Workflows | ✅ Created | Add secrets |
| Homebrew | ✅ Auto-bump ready | None |
| Deb/RPM | ✅ Workflow ready | None |
| Documentation | ✅ Complete | None |

## 🎯 Next Steps After Publishing

1. **Monitor First Release**
   - Check GitHub Actions for build status
   - Verify packages uploaded to release
   - Test extension installation from marketplace

2. **Announce Release**
   - Reddit: r/perl, r/vscode
   - Twitter/X: Tag @vscode
   - Perl forums/mailing lists

3. **Future Improvements**
   - Add screenshots to marketplace (1280x800 PNG)
   - Create video tutorial
   - Set up telemetry for usage stats
   - Add more code snippets

## 📝 Notes

- Extension will auto-publish on every tag push after secrets are added
- Homebrew formula will auto-update via PR after releases
- Linux packages (deb/rpm) build automatically on releases
- All platforms supported: Windows/macOS/Linux, x64/ARM64

---

**Ready to ship to users!** Just need those two tokens and we're live. 🚀
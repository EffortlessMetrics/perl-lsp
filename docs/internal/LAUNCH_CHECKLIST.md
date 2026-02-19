# 🚀 VSCode Extension Launch Checklist

## Pre-Launch Verification ✅

### Package Ready
- [x] Version bumped to 0.8.1
- [x] VSIX package built: `perl-lsp-0.8.1.vsix` (927 KB)
- [x] LICENSE file included
- [x] Icon added (icon.png)
- [x] All metadata updated

### Code Quality
- [x] TypeScript compiles without errors
- [x] Auto-download tested with .tar.xz support
- [x] SHA256 verification working
- [x] Multi-platform support verified

### Documentation
- [x] EXTENSION.md - User guide
- [x] MIGRATION.md - Breaking changes
- [x] RELEASE_NOTES_v0.8.1.md - Launch notes
- [x] SCREENSHOTS_PLAN.md - Visual guide
- [x] PUBLISHER_SETUP.md - Quick reference

## Launch Steps 🎯

### 1. Create Publisher Accounts (10 mins)

#### VSCode Marketplace
1. Go to: https://marketplace.visualstudio.com/manage
2. Click "Create publisher"
3. Publisher ID: `tree-sitter-perl`
4. Display name: `Tree-sitter Perl Team`
5. Generate PAT:
   - Name: `VSCE_PAT`
   - Organization: All accessible
   - Scopes: Marketplace (Publish)
   - Copy token immediately

#### Open VSX
1. Go to: https://open-vsx.org/
2. Sign up / Sign in
3. Go to Settings → Access Tokens
4. Create token:
   - Name: `OVSX_PAT`
   - Copy token immediately

### 2. Add GitHub Secrets (2 mins)

1. Go to: https://github.com/EffortlessMetrics/tree-sitter-perl/settings/secrets/actions
2. Click "New repository secret"
3. Add:
   - Name: `VSCE_PAT`, Value: [your VSCode PAT]
   - Name: `OVSX_PAT`, Value: [your Open VSX PAT]

### 3. Tag and Release (5 mins)

```bash
# Create and push tag
git tag -a v0.8.1 -m "Release v0.8.1: VSCode extension launch"
git push origin v0.8.1
```

This will trigger:
- cargo-dist binary builds
- Linux package generation (deb/rpm)
- Homebrew formula update
- VSCode/Open VSX publishing

### 4. Monitor Workflows

1. Check Actions: https://github.com/EffortlessMetrics/tree-sitter-perl/actions
2. Verify all workflows complete:
   - [ ] Release workflow (binaries)
   - [ ] Linux packages workflow
   - [ ] Extension publish workflow
   - [ ] Homebrew bump workflow

### 5. Post-Launch Verification

#### Extension Published
- [ ] VSCode Marketplace: Search "perl-lsp"
- [ ] Open VSX: Search "perl-lsp"
- [ ] Install from marketplace works
- [ ] Auto-download triggers on first use

#### Distribution Channels
- [ ] GitHub Release has all artifacts
- [ ] Homebrew formula updated
- [ ] Linux packages attached to release

### 6. Announce Launch 📢

#### GitHub Release
- Edit v0.8.1 release
- Add release notes from RELEASE_NOTES_v0.8.1.md
- Mark as latest release

#### Social Media Template
```
🎉 Perl LSP v0.8.1 is now on VSCode Marketplace!

✨ 26+ IDE features for Perl development
🚀 Lightning-fast performance
📦 Auto-downloads on all platforms
🔧 Zero configuration required

Install: ext install tree-sitter-perl.perl-lsp

Details: https://github.com/EffortlessMetrics/tree-sitter-perl
```

## Troubleshooting

### If publish fails
1. Check token permissions
2. Verify publisher ID matches package.json
3. Run locally: `vsce publish -p $VSCE_PAT`

### If auto-download fails
1. Check GitHub release artifacts
2. Verify naming pattern matches
3. Test with different platforms

### If Homebrew doesn't update
1. Manually trigger workflow
2. Or update formula manually
3. Create PR to homebrew-core

## Success Metrics 🎯

Track after 24 hours:
- [ ] Extension install count
- [ ] GitHub stars increase
- [ ] User feedback/issues
- [ ] Performance reports

## Next Steps

After successful launch:
1. Create marketplace screenshots
2. Record demo video
3. Write blog post
4. Submit to Perl Weekly
5. Announce on Reddit/r/perl

---

**You're ready to launch! 🚀**

Just complete the publisher account setup and run:
```bash
git tag -a v0.8.1 -m "Release v0.8.1: VSCode extension launch"
git push origin v0.8.1
```
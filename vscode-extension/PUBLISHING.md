# Publishing the Perl Language Server Extension

This guide is the release runbook for publishing `vscode-extension/` to the VS Code Marketplace.

## One-Time Setup

1. Install Node.js 20+.
2. Install dependencies:
   ```bash
   cd vscode-extension
   npm ci
   ```
3. Create a VS Code Marketplace publisher and PAT.
4. Authenticate:
   ```bash
   npx vsce login effortlesssteven
   ```

## Release Checklist (Marketplace Launch)

- [ ] `package.json` version is correct for release.
- [ ] `CHANGELOG.md` has release notes for the new version.
- [ ] `README.md` is up-to-date with feature set and install details.
- [ ] `icon.png` and marketplace metadata are present.
- [ ] `cargo build -p perl-lsp --release` succeeds in repo root.
- [ ] VSIX passes local packaging checks.

## Preflight + Package Commands

Run from `vscode-extension/`:

```bash
npm run marketplace:launch-check
```

This command:
1. Compiles TypeScript (`npm run compile`)
2. Verifies required marketplace files + metadata (`npm run marketplace:preflight`)
3. Builds a VSIX (`npm run package:vsix`)

## Manual Steps

```bash
# Compile only
npm run compile

# Metadata and asset checks only
npm run marketplace:preflight

# Build VSIX only
npm run package:vsix
```

## Publish

After preflight passes:

```bash
npx vsce publish
```

Optional version bump during publish:

```bash
npx vsce publish patch
npx vsce publish minor
npx vsce publish major
```

## Post-Publish Validation

1. Open Marketplace listing and verify:
   - Description, icon, and README rendering.
   - Version and changelog entry.
   - Install works in a fresh VS Code profile.
2. Tag repository release if needed.
3. Announce release notes.

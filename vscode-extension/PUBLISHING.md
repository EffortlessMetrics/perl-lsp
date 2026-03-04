# Publishing the Perl Language Server Extension

This guide is optimized for a **VS Code Marketplace launch** from the repository root.

## Prerequisites

1. Node.js + npm installed.
2. Rust toolchain installed.
3. `@vscode/vsce` available (already in `devDependencies`).
4. A Marketplace publisher and PAT with publish rights.

## 0) One-time marketplace auth

```bash
cd vscode-extension
npx @vscode/vsce login effortlesssteven
```

## 1) Preflight checks (recommended before every release)

```bash
cd vscode-extension
npm ci
npm run marketplace:preflight
```

This preflight command performs:
- TypeScript compile
- Package content listing (`vsce ls`)
- VSIX packaging into `vscode-extension/dist/`

## 2) Build and smoke test the VSIX locally

```bash
cd vscode-extension
code --install-extension dist/perl-lsp-0.10.0.vsix --force
code test/sample.pl
```

Smoke-test checklist:
- [ ] Extension activates on Perl files
- [ ] `Perl: Restart Perl Language Server` command works
- [ ] Hover, completion, and go-to-definition respond
- [ ] Diagnostics appear for malformed Perl code
- [ ] Debug configuration snippets are available

## 3) Publish to VS Code Marketplace

```bash
cd vscode-extension
npx @vscode/vsce publish
```

For a specific version (already updated in `package.json`):

```bash
cd vscode-extension
npx @vscode/vsce publish 0.10.0
```

## 4) Post-publish verification

1. Confirm listing metadata, icon, and README render on Marketplace.
2. Confirm install/update path from a clean VS Code profile.
3. Confirm release notes in `vscode-extension/CHANGELOG.md` match the published version.
4. Update top-level docs if install instructions changed.

## Troubleshooting

### Authentication or permission issues

```bash
cd vscode-extension
npx @vscode/vsce verify-pat effortlesssteven
```

Then refresh PAT and re-run `vsce login` if needed.

### Unexpected files in the VSIX

```bash
cd vscode-extension
npm run package:list
```

Tune `.vscodeignore` and rerun preflight.

### Packaging fails during dependency scan

Use the existing script which already sets `--no-yarn` and relies on npm lockfile:

```bash
cd vscode-extension
npm run package
```

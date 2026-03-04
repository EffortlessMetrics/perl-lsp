# VS Code Marketplace Launch Playbook

This guide is the **release runbook** for shipping `vscode-extension/` to the Visual Studio Marketplace.

It focuses on launch readiness and accurate commands for the current extension (`perl-lsp`, publisher `effortlesssteven`).

## 1) One-time setup

### Required accounts and credentials

1. Visual Studio Marketplace publisher account (`effortlesssteven`)
2. Azure DevOps Personal Access Token (PAT) with Marketplace publish permissions
3. Optional: Open VSX token if publishing to Open VSX as well

### Tooling

```bash
# From vscode-extension/
npm ci
npx @vscode/vsce --version
```

Login once per machine:

```bash
npx @vscode/vsce login effortlesssteven
```

## 2) Pre-launch checklist

Complete this checklist before packaging/publishing:

- [ ] `package.json` version bumped (semantic version)
- [ ] `CHANGELOG.md` updated with release notes
- [ ] `README.md` accurate for currently shipped capabilities
- [ ] Extension metadata validated (`displayName`, `description`, `categories`, `keywords`, `icon`, `repository`, `bugs`, `homepage`, `license`)
- [ ] Any new settings/commands reflected in README docs
- [ ] No accidental local/dev files included in the VSIX (`.vscodeignore`)
- [ ] Extension compiles and packages successfully
- [ ] Manual smoke test done in VS Code with a Perl workspace

## 3) Build and validate

From `vscode-extension/`:

```bash
# Compile TypeScript
npm run compile

# Package extension (also runs vscode:prepublish)
npx @vscode/vsce package --no-dependencies
```

Quick validation of produced artifact:

```bash
# Install locally for smoke testing
code --install-extension perl-lsp-<version>.vsix

# Example
code --install-extension perl-lsp-0.10.0.vsix
```

Smoke test minimum bar:

- [ ] Extension activates on `.pl`/`.pm` files
- [ ] Language server starts and responds
- [ ] Hover/completion/definition work
- [ ] Diagnostics appear for syntax errors
- [ ] Restart command (`Perl: Restart Perl Language Server`) works

## 4) Publish to Visual Studio Marketplace

From `vscode-extension/`:

```bash
# Publish current package.json version
npx @vscode/vsce publish --no-dependencies
```

Or bump + publish in one command:

```bash
npx @vscode/vsce publish patch --no-dependencies
# or minor / major
```

## 5) Optional: publish to Open VSX

If maintaining Open VSX parity:

```bash
# Requires ovsx CLI and token setup
npx ovsx publish perl-lsp-<version>.vsix -p "$OVSX_PAT"
```

## 6) Post-publish verification

- [ ] Marketplace listing renders correctly (icon, README, links)
- [ ] Install from marketplace succeeds:
  ```bash
  code --install-extension effortlesssteven.perl-lsp
  ```
- [ ] Basic language features work after clean install
- [ ] Release notes/changelog links are correct
- [ ] Any release announcement links to marketplace page

## 7) Rollback / hotfix guidance

If a bad release ships:

1. Ship a fast patch version (`x.y.z+1`) with fix.
2. Update changelog with explicit regression note.
3. Post issue/update in repository to guide impacted users.

## 8) Common failure modes

### Authentication or publisher issues

- Re-run `npx @vscode/vsce login effortlesssteven`
- Regenerate PAT if expired/revoked

### Missing files in package

- Check `.vscodeignore`
- Re-run packaging and inspect VSIX file list output

### Compile failures

- Run `npm ci`
- Ensure local Node version is compatible with dependencies

### Command not found for `code`

- Install VS Code shell command (`Shell Command: Install 'code' command in PATH`)

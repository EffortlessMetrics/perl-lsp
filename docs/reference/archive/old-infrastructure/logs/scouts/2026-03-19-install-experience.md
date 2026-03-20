# Scout Report: End-to-End Install and First-Run Experience

**Date**: 2026-03-19
**Status**: Read-only audit
**Scope**: User journey from binary install through first-run and editor setup

---

## Executive Summary

The install surface was already broad and fairly mature:

1. release installers existed for Unix and Windows
2. the CLI exposed `--help`, `--health`, `--info`, and `--version`
3. VS Code could discover, validate, or auto-download the binary
4. docs existed for general installation plus editor-specific setup

The main gap was not absence of install paths. It was first-run clarity and
signal: the binary could prove itself, but normal startup stayed quiet unless
logging was enabled.

---

## 1. Binary Discovery Paths

### CLI / manual paths
- `cargo install perl-lsp`
- release archive installers via `install.sh` and `install.ps1`
- manual docs in `docs/how-to/INSTALLATION.md`

### VS Code discovery order
From `vscode-extension/src/extension.ts`:
1. `perl-lsp.serverPath`
2. bundled extension binary
3. `PATH`
4. auto-download fallback

That order is historically important because it encodes a trust policy, not
just convenience.

---

## 2. Binary Verification Surface

### `perl-lsp --help`
The help text was already comprehensive and included:
- `--stdio`
- `--socket`
- `--port`
- `--log`
- `--health`
- `--info`
- `--check`
- `--version`
- `--features-json`
- `--feature-profile`
- `--completion`

### `perl-lsp --version`
Exposes:
- package version
- git tag
- parser line

### `perl-lsp --health`
Expected output contract:
- starts with `ok`
- docs spell this out as `ok <version>`
- VS Code health check depends on that exact prefix

### `perl-lsp --info`
Acts as the richer verification path:
- build/version metadata
- executable path
- feature-profile and coverage context

---

## 3. First-Run UX Observations

### Strong points
- explicit health probe exists
- info output exists
- docs tell users to run both after install
- VS Code refuses to attach if `--health` fails

### Friction points
- plain `--stdio` startup is quiet without `--log`
- successful startup therefore has less human-visible confirmation than the
  health/info path
- users are expected to trust editor attach plus `--health`, not terminal noise

This is not a protocol bug. It is a first-run ergonomics tension.

---

## 4. Managed Download And Provenance

### Installer scripts
`install.sh` and `install.ps1` both:
- resolve a release tag
- fetch the platform-specific asset
- attempt checksum verification via `SHA256SUMS`
- install into a user-local path
- verify using `--version`

### VS Code downloader
`vscode-extension/src/downloader.ts` adds:
- GitHub release discovery
- optional internal download base URL
- checksum verification
- HTTPS-only protections for remote downloads

The install story therefore already carried provenance hooks; it was not merely
"download whatever works."

---

## 5. Documentation Coverage

At scout time, install/setup guidance was spread across:
- `docs/how-to/INSTALLATION.md`
- `docs/how-to/EDITOR_SETUP.md`
- `docs/EDITORS/NEOVIM_SETUP.md`
- `docs/EDITORS/HELIX_SETUP.md`

The docs were stronger than the raw “quick start” impression suggested. The
problem was discoverability and coherence across surfaces, not lack of content.

---

## 6. Main Takeaway

The install/first-run story was already part of launch readiness:
- installation had multiple supported paths
- binary verification was explicit
- editor discovery order was deliberate
- downloads had checksum/provenance steps
- first-run quietness remained the main UX caveat

---

## Note

This file was preserved as a local scout log after archaeology work absorbed
its useful findings into tracked historical docs.

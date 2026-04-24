# Windows Distribution Manifests

This directory holds the shared update logic for Windows package-manager
metadata.

## Source Of Truth

- `update-manifests.ps1` updates the repo-owned manifests for Scoop,
  Chocolatey, and winget from a release version and SHA256 checksum.
- `distribution/scoop/perl-lsp.json` is the Scoop manifest source.
- `distribution/chocolatey/` contains the Chocolatey package files.
- `distribution/winget/perl-lsp.yaml` is the repo-local winget manifest source.

## Release Flow

The release workflows download the Windows release asset, compute its SHA256,
and then call `update-manifests.ps1`.

- Scoop and Chocolatey workflows keep their existing upstream PR targets.
- Winget refreshes the repo-local manifest first. Submitting that manifest to
  `winget-pkgs` remains a manual follow-up until that external workflow is
  added.

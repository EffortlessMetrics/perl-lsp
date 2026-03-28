# Linux Packaging Scaffold

This directory holds the repo-owned packaging templates for Linux package managers.

## Scope

- `apt/` holds Debian/Ubuntu packaging metadata templates
- `dnf/` holds RPM packaging metadata templates
- `pacman/` holds Arch-style packaging metadata templates
- `package-metadata.toml` carries shared release metadata for all three

## What this slice does

- Keeps the package-manager metadata in-repo and reviewable
- Makes the packaging shape explicit before any external repo publishing work
- Avoids depending on Launchpad, COPR, AUR, or other approval-gated infrastructure

## What this slice does not do

- It does not publish packages to third-party package repositories
- It does not claim official distro acceptance
- It does not replace the existing tarball-based GitHub release assets
- The current templates are x86_64-first so they stay small and reviewable; the metadata file also names the aarch64 GNU asset for the later matrix expansion

## Template inputs

The templates use placeholder tokens that can be rendered by a later release job:

- `__RELEASE_VERSION__`
- `__DEB_ARCH__`
- `__RPM_ARCH__`
- `__PACMAN_ARCH__`
- `__DOWNLOAD_URL__`
- `__DOWNLOAD_SHA256__`

The shared metadata file documents the package name, description, homepage, and the release asset names that should feed those templates.

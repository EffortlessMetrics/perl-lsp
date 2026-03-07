# perl-path-normalize Roadmap

> **Note:** This is the component-specific roadmap for `perl-path-normalize`. For the project-wide roadmap, see [`docs/project/ROADMAP.md`](../../docs/project/ROADMAP.md).

## Purpose
Secure component-wise path normalization for workspace-relative paths.

## Current Status (v0.10.0)
- **Status:** Initial Public Alpha
- **Integration:** Shared by workspace-bound path security crates.

## Future Milestones

### v0.10.x Hardening
- Add additional platform-focused tests for path component behavior.
- Refine error messages for invalid relative path inputs.

### v0.15.0 Stability Contract
- Keep normalization API minimal and semver-stable.
- Document integration patterns for security-focused consumers.

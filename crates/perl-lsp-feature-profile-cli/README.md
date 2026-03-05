# perl-lsp-feature-profile-cli

CLI argument parsing helpers for Perl LSP feature profile selection.

This microcrate is intentionally small and focused:

- Parse user-provided `--feature-profile` tokens.
- Provide canonical profile labels and supported token lists.
- Emit structured errors for unsupported profile names.

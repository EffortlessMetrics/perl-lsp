# Firebase Studio Setup

Firebase Studio runs a VS Code-compatible editor on top of Open VSX, so the
`perl-lsp-rs` extension works there with the same `perllsp` server binary.

## Install the Extension

1. Open **Extensions** in Firebase Studio.
2. Search for **Perl Language Server (perl-lsp)**.
3. Install publisher **EffortlessMetrics** (`EffortlessMetrics.perl-lsp-rs`).

If Marketplace search is restricted in your workspace policy, install from the
Open VSX listing:

- <https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs>

## Verify Binary Download

The extension auto-downloads `perllsp` the first time a Perl file is opened.
After opening a `.pl` or `.pm` file:

1. Run **Perl: Show Output Channel** from the Command Palette.
2. Confirm logs include `Binary installed to:` or `Using existing binary:`.

## If Download Is Blocked

Some Firebase Studio environments block direct GitHub downloads. Use one of
these fallbacks:

- Set `perl-lsp.downloadBaseUrl` to an internal mirror that hosts release
  archives and `SHA256SUMS`.
- Or manually place `perllsp` in the workspace/container and set
  `perl-lsp.serverPath` to that absolute path.

See [VS Code setup](VS_CODE_SETUP.md) for full settings and troubleshooting
flow.

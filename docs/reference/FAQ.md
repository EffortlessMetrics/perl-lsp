# FAQ

## How do I install perl-lsp?
- crates.io: `cargo install perl-lsp`
- Releases: https://github.com/EffortlessMetrics/perl-lsp/releases
- Installer (Linux/macOS, best-effort):  
  `curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash`
- From source (development default):
  `cargo install --path crates/perl-lsp`

## Does the installer install perl-dap?
No. The installer installs `perl-lsp`. Build/run `perl-dap` from source when needed.

## Where is feature coverage tracked?
`features.toml` is canonical. Computed metrics live in `docs/project/CURRENT_STATUS.md`.

## Where do I report bugs?
Open an issue with a minimal repro (smallest Perl snippet + expected vs actual).

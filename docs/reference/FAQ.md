# FAQ

## How do I install perl-lsp?
- crates.io: `cargo install perl-lsp`
- Releases: https://github.com/EffortlessMetrics/perl-lsp/releases
- Installer (Linux/macOS, best-effort):  
  `curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash`
- From source (development default):
  `cargo install --path crates/perl-lsp`

After installation, run `perl-lsp --health`. A healthy install prints `ok <version>`.

## How do I confirm my editor is actually talking to perl-lsp?
Use the smallest possible check first:

1. Confirm the binary is on your `PATH` with `which perl-lsp`.
2. Run `perl-lsp --health`.
3. Open your editor's LSP log/output panel.
4. Open a `.pl`, `.pm`, or `.t` file and verify the client starts `perl-lsp --stdio`.

For editor-specific steps, see the setup guides in `docs/EDITORS/` and the troubleshooting guide at `docs/how-to/TROUBLESHOOTING.md`.

## Which editors are supported?
Any editor with a Language Server Protocol client can work, but the repository maintains dedicated setup docs for:

- VS Code
- Neovim
- Emacs
- Helix
- Sublime Text
- coc.nvim

Start with `docs/tutorials/GETTING_STARTED.md` for the shortest path to a working setup.

## Does perl-lsp require a Perl runtime?
Not for parsing or core LSP features. `perl-lsp` is a native Rust binary. Some optional workflows, external tools, or project-specific integrations may still depend on your local Perl toolchain.

## Does the installer install perl-dap?
No. The installer installs `perl-lsp`. Build or install `perl-dap` separately when you need debugger support.

## Where is feature coverage tracked?
`features.toml` is canonical. Computed metrics live in `docs/project/CURRENT_STATUS.md`.

## Where do configuration options live?
See `docs/reference/CONFIG.md` for the full schema, including workspace include paths, indexing limits, completion caps, and other LSP settings.

## Where do I report bugs?
Open an issue with a minimal repro: the smallest Perl snippet, the editor/client you used, your `perl-lsp` version, and the expected versus actual behavior.

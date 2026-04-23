# FAQ

## Installation

### How do I install perl-lsp?

- **VS Code (recommended)**: install the [Perl LSP extension](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs) — it downloads the server binary automatically.
- **Pre-built binary**: download from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases).
- **Installer script (Linux/macOS, best-effort)**:
  `curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash`
- **From source**:
  `cargo install --path crates/perllsp`

### Does perl-lsp require Perl to be installed?

No. perl-lsp is a self-contained Rust binary. It parses Perl using a native recursive-descent parser — no Perl runtime is needed for completions, hover, diagnostics, go-to-definition, or any other IDE feature. Perl is only needed if you use the integrated test runner (`prove`, `perl -T ...`) or Perl::Critic integration, because those features actually execute Perl code.

### Does the installer also install perl-dap?

No. The installer installs `perl-lsp`. Build or install `perl-dap` separately when you need debugging support:

```bash
cargo install --path crates/perl-dap
```

### Which platforms are supported?

Pre-built binaries are provided for:

| Platform | Architecture |
|----------|-------------|
| Linux    | x86_64, aarch64 |
| macOS    | x86_64, Apple Silicon (aarch64) |
| Windows  | x86_64 |

Building from source (Rust 1.92+) works on any Rust-supported platform.

---

## Perl Compatibility

### Which Perl versions does perl-lsp support?

The parser targets Perl 5.8 through 5.40. This includes:

- All core syntax from Perl 5.8+
- Modern features: `say`, `given`/`when`, `state`, `fc`
- Perl 5.36+ signatures (experimental)
- `use v5.38; class ...` object syntax (partial support)
- Moose, Moo, and common OO frameworks (detection-level support)

This claim is backed by a visible CI workflow: [Perl Version Matrix](https://github.com/EffortlessMetrics/perl-lsp/actions/workflows/perl-version-matrix.yml). It runs version-gated Perl syntax probes on every Perl minor from 5.8 through 5.40, and also runs a Rust smoke test on both edge versions (5.8 and 5.40).

If you encounter a Perl construct that fails to parse, [report it](https://github.com/EffortlessMetrics/perl-lsp/issues) with a minimal example.

### Does it support Perl 5.8?

Yes. The parser targets Perl 5.8 as the minimum and handles most idioms from that era. Very old-style tie/format/write-heavy code may have partial coverage; check `CURRENT_STATUS.md` for details.

---

## Editor Support

### Which editors work with perl-lsp?

Any editor with LSP client support works. Point it at `perllsp --stdio`:

- **VS Code** — native extension with auto-download, UI settings, and DAP debugging
- **Neovim** — via `nvim-lspconfig` (`perl_ls` server)
- **Emacs** — via `eglot` or `lsp-mode`
- **Helix** — via `languages.toml`
- **Zed** — via `settings.json` language server configuration
- **Sublime Text** — via the LSP package
- **Kate**, **Lapce**, **Kakoune** — any editor with a generic LSP client

See [EDITOR_SETUP.md](../how-to/EDITOR_SETUP.md) for editor-specific configuration.

### Can I use it without VS Code?

Yes. The VS Code extension is the easiest path, but `perllsp --stdio` is a plain LSP server that works with any compliant client. The extension is a convenience layer on top of the same binary.

### Does it support debugging (DAP)?

Yes. `perl-dap` implements the Debug Adapter Protocol. In VS Code, the extension integrates both LSP and DAP automatically. In other editors, run `perl-dap` as a separate DAP server and configure your editor's debugger client accordingly.

See the [DAP User Guide](../tutorials/DAP_USER_GUIDE.md) for setup instructions.

---

## Performance

### How fast is it?

- **Incremental parsing**: under 1ms per keystroke for typical files.
- **LSP response times**: under 50ms for completions and hover on warm cache.
- **Memory**: approximately 50MB base, growing with workspace size.

For performance tuning options (cache sizes, deadline budgets, file limits), see [PERFORMANCE_TUNING.md](../how-to/PERFORMANCE_TUNING.md).

### Are there workspace size limits?

By default, perl-lsp indexes up to 10,000 files and 500,000 total symbols. For large monorepos, increase these via LSP settings:

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 50000,
      "maxTotalSymbols": 2000000,
      "workspaceScanDeadlineMs": 120000
    }
  }
}
```

See [CONFIG.md](CONFIG.md) for the full limits reference.

### What if the server is slow on startup?

The initial workspace scan budget is 30 seconds by default. If your workspace has many files, increase `perl.limits.workspaceScanDeadlineMs`. You can also use `.perl-lspignore` to exclude directories that don't contain Perl source.

---

## Configuration

### Where do I configure perl-lsp?

Configuration depends on your editor:

- **VS Code**: `settings.json` under `perl-lsp.*` keys (extension settings) or `perl.*` keys (LSP workspace settings).
- **Neovim/Emacs/Helix**: pass the `perl.*` settings table in your LSP client configuration.
- **Project-level**: create a `.perl-lsp.toml` file in your project root for settings that apply regardless of editor.

See [CONFIG.md](CONFIG.md) for the full configuration reference.

### Where is feature coverage tracked?

`features.toml` is the canonical source. Computed metrics live in `docs/project/CURRENT_STATUS.md`.

---

## Bugs and Contributions

### How do I report a parser bug?

Open an issue at [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues) with:

1. The smallest Perl snippet that reproduces the problem.
2. What you expected to happen (e.g. "should parse without errors").
3. What actually happened (e.g. "shows diagnostic on line 3").

Parser issues are the highest-priority bug class; they are usually fixed within one or two development cycles.

### How do I report an LSP feature bug?

Same as above — include editor name, version, and the exact LSP operation that misbehaves (completion, hover, go-to-definition, etc.). Attach the LSP log if possible (`perl-lsp.trace.server: "verbose"` in VS Code).

### Is perl-lsp open source?

Yes. perl-lsp is dual-licensed under [MIT](../../LICENSE-MIT) and [Apache-2.0](../../LICENSE-APACHE). Contributions are welcome — see [CONTRIBUTING.md](../../CONTRIBUTING.md).

### What is the release cadence?

perl-lsp is in active development. Releases are cut when meaningful milestones
are ready, such as parser coverage gains, new LSP features, or release-surface
hardening. The workspace version on `main` can move ahead of the latest
published release during release prep, so check GitHub Releases for the
currently shipped public release.

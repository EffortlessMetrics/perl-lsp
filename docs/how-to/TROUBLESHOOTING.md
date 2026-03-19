# Troubleshooting Guide

Practical fixes for common perl-lsp issues, organized by symptom.

## Quick Diagnostics

Before diving into specific issues, run these checks:

```bash
# Verify binary exists and runs
which perl-lsp && perl-lsp --version

# Health check (returns "ok" when binary is functional)
perl-lsp --health

# Detailed build info (version, git tag, feature profile)
perl-lsp --info
```

In VS Code, open the Output panel: **View > Output**, then select **Perl Language Server** from the dropdown. Most errors are logged there.

---

## 1. Extension Not Starting

### Binary not found

**Symptom**: Error message "Perl Language Server (perl-lsp) not found" with options to install or open settings.

**Cause**: The extension searches for the binary in this order:
1. `perl-lsp.serverPath` setting (user-configured absolute path)
2. Bundled binary at `<extension>/bin/<platform>-<arch>/perl-lsp`
3. `perl-lsp` in your system PATH
4. Auto-download from GitHub releases (if `perl-lsp.autoDownload` is `true`)

**Fix**:

- **Auto-download** (default): Ensure you have internet access. The extension downloads the correct binary for your platform automatically. Check the Output panel for download errors.
- **Manual install**: Set the `perl-lsp.serverPath` setting to the absolute path of your binary:
  ```json
  { "perl-lsp.serverPath": "/usr/local/bin/perl-lsp" }
  ```
- **Build from source**:
  ```bash
  cargo install perl-lsp
  # Binary goes to ~/.cargo/bin/perl-lsp
  ```
- **Internal/air-gapped networks**: Set `perl-lsp.autoDownload` to `false` and either set `perl-lsp.serverPath` or set `perl-lsp.downloadBaseUrl` to your internal mirror URL hosting the release archives and SHA256SUMS file.

### Permission denied

**Symptom**: The binary exists but cannot execute.

**Cause**: Missing execute permission on the binary (Linux/macOS).

**Fix**:
```bash
chmod +x $(which perl-lsp)
# Or if using a custom path:
chmod +x /path/to/perl-lsp
```

The extension automatically sets `0o755` on bundled and downloaded binaries, but manually placed binaries may lack the execute bit.

### Wrong architecture / health check failed

**Symptom**: Error message "perl-lsp health check failed. The binary does not respond to --health. It may be corrupted or incompatible with your platform."

**Cause**: The binary was built for a different OS or CPU architecture (e.g., x86_64 binary on an ARM Mac), or the binary is corrupt.

**Fix**:
- Click **Reinstall** in the error dialog to re-download the correct binary for your platform.
- Or build from source for your platform:
  ```bash
  cargo install perl-lsp --force
  ```
- Verify the binary runs: `perl-lsp --health` should print a line starting with `ok`.

---

## 2. No Completions / No Diagnostics

### Check that the LSP is running

**Symptom**: No syntax errors highlighted, no completions, no hover information.

**Fix**: Look at the status bar (bottom-right of VS Code):
- `$(check) Perl LSP` — server is running normally.
- `$(sync~spin) Perl LSP` — server is starting, wait a moment.
- `$(error) Perl LSP` — server is stopped. Click the status bar item and select **Restart Server**, or run the command palette action **Perl: Restart Perl Language Server** (`Shift+Alt+R`).

### Check file type association

**Symptom**: LSP features work in `.pm` files but not in your file.

**Cause**: The extension activates only for files recognized as Perl. Supported extensions: `.pl`, `.pm`, `.pod`, `.t`, `.psgi`. Files with a `#!.*perl` shebang are also recognized.

**Fix**:
- Check the language mode in the VS Code status bar (bottom-right). It should say **Perl**.
- If it says something else, click it and select **Perl** from the list, or press `Ctrl+K M` and type "Perl".

### Check workspace folder is open

**Symptom**: Completions work for built-in functions but not for your project's modules.

**Cause**: perl-lsp indexes files within your workspace. If you opened a single file instead of a folder, workspace-level features (module resolution, go-to-definition across files) are limited.

**Fix**: Open the project folder: **File > Open Folder** and select your project root.

### Increase log verbosity

**Symptom**: Something is wrong but you cannot tell what.

**Fix**: Set `perl-lsp.trace.server` to `"verbose"` in your settings:
```json
{ "perl-lsp.trace.server": "verbose" }
```
Then check the Output panel (**Perl Language Server**). The trace level can be changed at runtime without restarting. Options: `"off"`, `"messages"`, `"verbose"`.

### Diagnostics explicitly disabled

**Symptom**: No red/yellow squiggles even on files with syntax errors.

**Fix**: Check that diagnostics are enabled:
```json
{ "perl-lsp.enableDiagnostics": true }
```

---

## 3. Slow Performance

### Large workspace

**Symptom**: Editor feels sluggish, high CPU usage after opening a project.

**Cause**: perl-lsp indexes all Perl files in your workspace. Projects with thousands of `.pl`/`.pm` files (or large `local/lib/perl5` directories) take longer to index.

**Fix**:
- Ensure your `.gitignore` excludes `local/`, `vendor/`, `node_modules/`, and other non-project directories. perl-lsp respects `.gitignore` for file discovery.
- Disable features you do not need:
  ```json
  {
    "perl-lsp.enableSemanticTokens": false
  }
  ```

### Many parse errors

**Symptom**: Diagnostics panel shows hundreds of errors, editor is slow.

**Cause**: Files with syntax the parser does not yet support cause cascading errors, which consumes CPU.

**Fix**:
- Verify your files parse correctly with Perl itself: `perl -c yourfile.pl`
- Use `perl-lsp --check yourfile.pl` to test parsing without the editor.
- File an issue for unsupported syntax patterns at https://github.com/EffortlessMetrics/perl-lsp/issues

### High memory

**Symptom**: perl-lsp process using excessive memory.

**Cause**: Very large workspaces with many indexed files.

**Fix**: Reduce the scope of your workspace. Open only the subdirectory you are working in rather than a monorepo root.

---

## 4. Formatting Not Working

### perltidy not installed

**Symptom**: Format Document does nothing or shows an error.

**Cause**: perl-lsp uses `perltidy` for formatting. It must be installed and available in PATH.

**Fix**:
```bash
cpanm Perl::Tidy
# or
cpan Perl::Tidy

# Verify installation
perltidy --version
```

Ensure formatting is enabled in settings:
```json
{ "perl-lsp.enableFormatting": true }
```

### Custom .perltidyrc not found

**Symptom**: Formatting uses default perltidy style, ignoring your project's rules.

**Cause**: perl-lsp searches for `.perltidyrc` in the workspace root and home directory. If your config is elsewhere, it will not be found.

**Fix**: Set the path explicitly:
```json
{ "perl-lsp.perltidyConfig": "/path/to/your/.perltidyrc" }
```

If left empty (default), perl-lsp searches the workspace and home directory automatically. The extension also watches for `.perltidyrc` changes and reloads.

### Format on save not working

**Symptom**: Files are not formatted when you save.

**Fix**: Enable format-on-save in perl-lsp settings (this is separate from VS Code's built-in `editor.formatOnSave`):
```json
{ "perl-lsp.formatOnSave": true }
```

---

## 5. Debug Adapter Not Connecting

### perl-dap not found

**Symptom**: Error "Perl Debug Adapter (perl-dap) not found" when starting a debug session.

**Cause**: The debug adapter binary (`perl-dap`) ships alongside `perl-lsp` in release archives. If you built from source or the download did not include it, the binary may be missing.

**Fix**: The extension searches for `perl-dap` in these locations (in order):
1. Auto-download directory (same location as the managed perl-lsp binary)
2. System PATH
3. `~/.cargo/bin/perl-dap`
4. `/usr/local/bin/perl-dap`

Install it:
```bash
cargo install --path crates/perl-dap
# or reinstall the extension binary (includes perl-dap):
```
In VS Code, run **Perl: Reinstall Server Binary** — this re-downloads both `perl-lsp` and `perl-dap`.

### Port conflict (TCP attach mode)

**Symptom**: Debug attach fails to connect.

**Cause**: Default attach port (13603) is in use by another process, or the Perl debugger is not listening.

**Fix**:
- Check if the port is in use: `lsof -i :13603` (Linux/macOS) or `netstat -an | findstr 13603` (Windows)
- Change the port in your `launch.json`:
  ```json
  {
    "type": "perl",
    "request": "attach",
    "host": "localhost",
    "port": 13604,
    "timeout": 5000
  }
  ```

### No launch.json configured

**Symptom**: Debug starts but immediately fails or debugs the wrong file.

**Fix**: If you have no `launch.json`, the extension auto-creates a configuration that launches the current file. For a proper setup, create `.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Launch Script",
      "program": "${workspaceFolder}/script.pl",
      "stopOnEntry": true,
      "includePaths": ["lib"]
    }
  ]
}
```

---

## 6. Module Not Found / Go-to-Definition Fails

### Include paths not configured

**Symptom**: "Go to Definition" cannot find modules in your project's `lib/` directory, or module names show as unresolved.

**Cause**: perl-lsp needs to know where your Perl modules live. By default, it searches `lib` and `local/lib/perl5` relative to the workspace root.

**Fix**: Add your project's library paths to the `perl-lsp.includePaths` setting:
```json
{
  "perl-lsp.includePaths": [
    "lib",
    "local/lib/perl5",
    "vendor/lib"
  ]
}
```

Paths are resolved relative to the workspace root. You can also use absolute paths.

### Local modules with `use lib`

**Symptom**: Code uses `use lib 'lib'` or `use lib "$FindBin::Bin/../lib"` but perl-lsp cannot find those modules.

**Cause**: perl-lsp does not execute Perl code, so it cannot evaluate `use lib` statements dynamically.

**Fix**: Mirror the `use lib` paths in your `perl-lsp.includePaths` setting. For example, if your code has `use lib 'lib'`, ensure `"lib"` is in your `includePaths` (it is by default).

### CPAN/system modules

**Symptom**: Go-to-definition works for project modules but not for CPAN modules like `Moose`, `DBI`, etc.

**Cause**: System-installed modules are outside the workspace directory tree.

**Fix**: Add your Perl's library paths:
```bash
# Find your @INC paths
perl -e 'print join("\n", @INC)'
```

Add the relevant paths to `perl-lsp.includePaths`:
```json
{
  "perl-lsp.includePaths": [
    "lib",
    "local/lib/perl5",
    "/usr/local/lib/perl5/site_perl/5.38"
  ]
}
```

---

## Getting Help

1. **Check the Output panel**: View > Output > Perl Language Server. Most issues are logged here.

2. **Enable verbose tracing**:
   ```json
   { "perl-lsp.trace.server": "verbose" }
   ```

3. **Run diagnostics from the command line**:
   ```bash
   perl-lsp --health        # Quick binary check
   perl-lsp --info          # Version, git tag, feature profile
   perl-lsp --check file.pl # Parse a file without the editor
   ```

4. **File an issue**: https://github.com/EffortlessMetrics/perl-lsp/issues
   Include:
   - `perl-lsp --info` output
   - OS and editor version
   - Relevant Output panel logs
   - Minimal code reproduction (if applicable)

## See Also

- [EDITOR_SETUP.md](EDITOR_SETUP.md) — Editor-specific configuration (VS Code, Neovim, Emacs, Helix)
- [INSTALLATION.md](INSTALLATION.md) — Installation guide
- [DEBUGGING.md](DEBUGGING.md) — Debug adapter setup and usage

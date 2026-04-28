# Troubleshooting Guide

Use this page when `perllsp` is installed but something still does not work:
the binary is not found, the server does not start, diagnostics are missing, or
the editor feels slow.

## Start With The Basics

```bash
perllsp --version
perllsp --health
perllsp --info
```

If those fail, fix the binary installation and `PATH` first. If they pass, the
problem is usually in editor integration, workspace roots, or a stale cache.

## The Server Will Not Start

1. Run the server in the foreground:

   ```bash
   perllsp --stdio
   ```

2. Turn on logging and read stderr:

   ```bash
   RUST_LOG=perl_lsp=debug perllsp --stdio
   ```

3. Check the editor's LSP log panel or buffer.

## The Editor Connects, But Nothing Happens

- Confirm the file type is Perl.
- Confirm the workspace root is the repository root, not a parent directory.
- Confirm the editor command really starts `perllsp --stdio`.

If the editor is using a helper extension or plugin, check its own logs too.

## Emacs Does Not Start `perllsp`

1. Confirm Emacs can find the binary:

   ```elisp
   M-: (executable-find "perllsp")
   ```

2. Confirm the file is in a Perl major mode:

   ```elisp
   M-: major-mode
   ```

   Expected: `perl-mode`, `cperl-mode`, or an installed `perl-ts-mode`.

3. Confirm the server works outside Emacs:

   ```bash
   perllsp --version
   perllsp --health
   perllsp --info
   perllsp --check path/to/file.pl
   ```

4. For Eglot, inspect:

   ```elisp
   M-: (eglot-managed-p)
   M-x eglot-events-buffer
   M-x eglot-stderr-buffer
   ```

5. For `lsp-mode`, inspect:

   ```elisp
   M-x lsp-describe-session
   M-x lsp-workspace-show-log
   ```

6. Do not test stdio mode with raw JSON. LSP stdio traffic requires
   `Content-Length` framing.

## Sublime Text Does Not Start `perllsp`

1. Confirm `perllsp` works outside Sublime:

   ```bash
   perllsp --version
   perllsp --health
   perllsp --info
   ```

2. Confirm the `LSP` package is installed.
3. Confirm `Preferences: LSP Server Configurations` contains:

   ```json
   {
     "perl-lsp": {
       "enabled": true,
       "command": ["perllsp", "--stdio"],
       "selector": "source.perl"
     }
   }
   ```

4. Run `Tools > Developer > Show Scope Name` in a Perl file and confirm the
   root scope matches the configured selector.
5. Run `LSP: Troubleshoot Server` and `LSP: Toggle Log Panel`.
6. If Sublime cannot find `perllsp`, use an absolute path in `command`.

### Trae does not start `perllsp`

1. Confirm the `EffortlessMetrics.perl-lsp-rs` extension is installed and enabled.
2. Confirm the active document language is Perl.
3. If using extension-managed downloads, confirm `perl-lsp.autoDownload` is `true`.
4. If using a manual binary, run:

   ```bash
   perllsp --version
   perllsp --health
   perllsp --info
   ```

5. If Trae cannot find the binary, use an absolute `perl-lsp.serverPath`.
6. Check the Perl LSP output/log panel and temporarily set `perl-lsp.trace.server` to `messages`.

## Diagnostics Or Completions Are Missing

- Re-check the install with `perllsp --health`.
- Make sure the file is inside the indexed workspace.
- Restart the editor after changing language-server settings.
- If the project is large, try a smaller workspace root first.

## The Server Feels Slow

- Close unrelated files and trim the workspace to the project root.
- Disable any editor-side preview features that trigger extra refreshes.
- Compare behavior with a fresh shell session so stale environment state does
   not hide the problem.

## Module Resolution Problems

- Confirm the module lives under the workspace or configured include paths.
- Open the project root that contains the module tree, not just a subdirectory.
- If you are using vendored or local libraries, make sure that editor config
   points at them explicitly.

## Formatting Or Code Actions Are Missing

- Verify the editor has the relevant capability enabled.
- Check whether the current file actually has a Perl mode or file type.
- Inspect the LSP log for capability negotiation or request errors.

## OpenCode does not start `perllsp`

1. Confirm `perllsp` works outside OpenCode:

   ```bash
   perllsp --version
   perllsp --health
   perllsp --info
   ```

2. Confirm the active file extension is listed in
   `opencode.json` under `lsp.perl-lsp.extensions`.

3. If OpenCode cannot find the binary, start OpenCode from the same shell where
   `command -v perllsp` succeeds, or use an absolute path in the `command`
   array.

4. If `perllsp --stdio` appears to hang when run manually, that is expected. Use
   `perllsp --health`, `perllsp --info`, or `perllsp --check path/to/file.pl`
   for manual checks.

5. Start OpenCode with debug logs:

   ```bash
   opencode --log-level DEBUG
   ```

6. Check OpenCode logs:

   - macOS/Linux: `~/.local/share/opencode/log/`
   - Windows: `%USERPROFILE%\.local\share\opencode\log`

7. For direct hover, definition, references, and symbol operations, enable the
   experimental LSP tool:

   ```bash
   OPENCODE_EXPERIMENTAL_LSP_TOOL=true opencode
   ```

   and set:

   ```json
   {
     "permission": {
       "lsp": "allow"
     }
   }
   ```

## Vim-specific Startup Checks

### Vim does not start `perllsp`

1. Confirm Vim can see the binary:

   ```vim
   :echo executable('perllsp')
   ```

2. Confirm the buffer filetype:

   ```vim
   :set filetype?
   ```

   It must be `perl`.

3. For `vim-lsp`, inspect:

   ```vim
   :LspStatus
   :LspDocumentDiagnostics
   ```

4. For `coc.nvim`, inspect:

   ```vim
   :CocInfo
   :CocOpenLog
   :CocCommand document.echoFiletype
   :CocCommand workspace.showOutput
   ```

5. Check the server outside Vim:

   ```bash
   perllsp --health
   perllsp --info
   perllsp --check path/to/file.pl
   ```

## DAP Or Debugging Issues

If you are debugging with `perl-dap`, check the DAP guide:
[DAP_USER_GUIDE.md](../tutorials/DAP_USER_GUIDE.md).

## When To Escalate

Report an issue when you can include:

- `perllsp --version`
- `perllsp --health`
- editor name and version
- the workspace layout
- the smallest code sample that reproduces the problem

Open issues at [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues).

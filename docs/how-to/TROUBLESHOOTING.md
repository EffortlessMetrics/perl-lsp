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

## VS Code Quick Checks

1. Confirm the extension can locate the binary:

   ```bash
   command -v perllsp
   perllsp --version
   perllsp --health
   perllsp --info
   ```

2. Open **Output** → **Perl Language Server** in VS Code and review startup logs.
3. If you rely on managed binaries, verify `"perl-lsp.autoDownload": true`.
4. If `perllsp --stdio` appears to hang in a terminal, that is expected; it is
   waiting for framed LSP JSON-RPC input.

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
- If you are using vendored or local libraries, make sure the editor config
  points at them explicitly.

## Formatting Or Code Actions Are Missing

- Verify the editor has the relevant capability enabled.
- Check whether the current file actually has a Perl mode or file type.
- Inspect the LSP log for capability negotiation or request errors.

## DAP Or Debugging Issues

If you are debugging with `perl-dap`, check the DAP guide:
[DAP_USER_GUIDE.md](../tutorials/DAP_USER_GUIDE.md).

## When To Escalate

Report an issue when you can include:

- `perllsp --version`
- `perllsp --health`
- the editor name and version
- the workspace layout
- the smallest code sample that reproduces the problem

Open issues at [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues).

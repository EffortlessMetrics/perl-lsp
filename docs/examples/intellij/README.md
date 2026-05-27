# IntelliJ IDEA Continuous Testing Examples

This directory contains importable snippets for JetBrains IDEs.

For LSP setup in JetBrains IDEs, use the upstream LSP4IJ `perl-lsp`
integration when available. See
[`../../EDITORS/INTELLIJ_IDEA_SETUP.md`](../../EDITORS/INTELLIJ_IDEA_SETUP.md).
The external-tool snippets here are only for test-running workflows.

## External Tool: repo test watch

`external-tools.xml` defines two external tools:

- `perl-lsp: just dev-watch-tests`
- `perl-lsp: nextest local-fast`

### Import steps

1. Open **Settings** → **Tools** → **External Tools**.
2. Click the gear icon and choose **Import**.
3. Select `external-tools.xml` from this directory.
4. Run the tools from **Tools** → **External Tools**.

## File Watcher fallback

If you prefer reruns on save, create a File Watcher with:

- **Program**: `just`
- **Arguments**: `dev-watch-tests`
- **Working directory**: `$ProjectFileDir$`

Use `cargo nextest run --profile local-fast --workspace` when you want a narrower loop.

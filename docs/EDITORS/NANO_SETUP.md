# GNU nano + perl-lsp Companion Workflow

`perllsp` is an LSP server, and GNU nano is not an LSP client. That means nano
cannot directly consume LSP requests/responses over stdio.

You can still improve your nano workflow by pairing editing with fast CLI checks
from `perllsp`.

## What works with nano

- Syntax and parse validation via `perllsp --check`
- Project health verification via `perllsp --health`
- Server/build metadata inspection via `perllsp --info`

## What does not work inside nano

- Live diagnostics while typing
- Hover, go-to-definition, references, rename
- Code actions and inlay hints

For those features, use an LSP-capable editor listed in
[Editor Setup](../how-to/EDITOR_SETUP.md).

## Quick start

1. Verify install:

   ```bash
   perllsp --version
   perllsp --health
   ```

2. Edit in nano as usual:

   ```bash
   nano lib/My/Module.pm
   ```

3. Run a check after saving:

   ```bash
   perllsp --check lib/My/Module.pm
   ```

4. Check multiple files:

   ```bash
   perllsp --check lib/My/Module.pm script/tool.pl t/module.t
   ```

## Optional shell helper

Add this helper to your shell profile for a shorter command:

```bash
perlcheck() {
  perllsp --check "$@"
}
```

Then run:

```bash
perlcheck lib/My/Module.pm
```

## Recommended path for full IDE features

If you want completions, navigation, and refactoring while typing, keep nano for
quick edits but add one LSP-capable editor (Neovim, Emacs, Helix, Sublime Text,
or VS Code) for deep code-intelligence workflows.

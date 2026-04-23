# GNU nano Setup Guide for perl-lsp

GNU nano does not include a built-in LSP client, so it cannot attach to
`perllsp --stdio` directly.

This guide gives you the best practical nano workflow today:

1. Keep Perl syntax highlighting in nano.
2. Run `perllsp` checks from the terminal for diagnostics.
3. Use a full LSP editor (VS Code, Neovim, Emacs, Helix, Sublime) when you need
   completions, go-to-definition, rename, and code actions.

## 1) Syntax highlighting in nano

Add this to `~/.nanorc`:

```nanorc
syntax "perl" "\\.(pl|pm|t|psgi)$"
color brightgreen "\<(sub|my|our|use|package|if|else|elsif|for|foreach|while|return)\>"
color brightblue "\$[A-Za-z_][A-Za-z0-9_]*"
color brightcyan "@[A-Za-z_][A-Za-z0-9_]*"
color brightmagenta "%[A-Za-z_][A-Za-z0-9_]*"
color yellow "#.*"
```

## 2) Run diagnostics with perllsp from terminal

From your project root:

```bash
perllsp --check path/to/file.pl
```

For whole-project validation:

```bash
perllsp --check-project .
```

## 3) Known limitations in nano

Because nano is not an LSP client, these `perllsp` features are unavailable
inside nano itself:

- live diagnostics while typing
- inline completions and snippets
- go-to-definition and find references
- rename symbol and code actions

If those workflows are important, keep nano for quick edits and use one of the
LSP-capable editors from the main setup guide for IDE features.

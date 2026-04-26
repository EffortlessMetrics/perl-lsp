# Vim Setup Guide for perl-lsp

This guide covers classic Vim setup (not Neovim). Use this when you want
`perllsp` features such as diagnostics, go-to-definition, references, and
rename directly in Vim.

## Prerequisites

- Vim 8.2+ (or newer)
- `perllsp` installed and available on your `PATH`
- one LSP client plugin:
  - [`vim-lsp`](https://github.com/prabirshrestha/vim-lsp), or
  - [`coc.nvim`](https://github.com/neoclide/coc.nvim)

Quick verification:

```bash
perllsp --version
perllsp --health
```

## Option A: vim-lsp

Add this to your `.vimrc` after loading `vim-lsp`:

```vim
if executable('perllsp')
  augroup perllsp_vim
    autocmd!
    autocmd User lsp_setup call lsp#register_server({
          \ 'name': 'perllsp',
          \ 'cmd': {server_info->['perllsp', '--stdio']},
          \ 'allowlist': ['perl'],
          \ })
  augroup END
endif
```

Useful default mappings (optional):

```vim
nmap <buffer> gd <plug>(lsp-definition)
nmap <buffer> gr <plug>(lsp-references)
nmap <buffer> K  <plug>(lsp-hover)
nmap <buffer> <leader>rn <plug>(lsp-rename)
```

## Option B: coc.nvim

If you already use coc.nvim, configure `perllsp` in `coc-settings.json`:

```json
{
  "languageserver": {
    "perl-lsp": {
      "command": "perllsp",
      "args": ["--stdio"],
      "filetypes": ["perl"]
    }
  }
}
```

For a full coc-focused walkthrough, see
[COC_NEOVIM_SETUP.md](./COC_NEOVIM_SETUP.md).

## Troubleshooting

- If Vim reports the server is not executable, run `:echo executable('perllsp')`
  and fix your `PATH`.
- If the server starts but no Perl features appear, confirm filetype detection:
  `:set filetype?` should return `filetype=perl`.
- If features are intermittent, check client logs:
  - vim-lsp: `:LspStatus`
  - coc.nvim: `:CocInfo`

# Vim Setup Guide for perl-lsp

This guide covers classic Vim setup (not Neovim's built-in LSP client). Use
this when you want `perllsp` features such as diagnostics, go-to-definition,
references, hover, formatting, and rename directly in Vim.

## Prerequisites

- Vim with Perl filetype detection enabled
- `perllsp` installed and available on your `PATH`
- one LSP client plugin:
  - [`vim-lsp`](https://github.com/prabirshrestha/vim-lsp), or
  - [`coc.nvim`](https://github.com/neoclide/coc.nvim)

Client-specific notes:

- `vim-lsp`: Vim 8+ is supported; newer Vim 9 builds enable better virtual text
  and inlay-hint support.
- `coc.nvim`: Vim 9.0.0438+ and Node.js 16.18.0+ are required.

Verify `perllsp` before changing Vim configuration:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Perl Filetype Detection

The LSP client starts only when Vim sets the buffer filetype to `perl`.

Check a Perl buffer with:

```vim
:set filetype?
```

It should print:

```text
filetype=perl
```

If `.t` test files or CGI-like files are not detected as Perl, add:

```vim
augroup perl_filetypes
  autocmd!
  autocmd BufRead,BufNewFile *.t setfiletype perl
  autocmd BufRead,BufNewFile *.cgi,*.fcgi setfiletype perl
augroup END
```

## Option A: vim-lsp

Install `vim-lsp` with your preferred plugin manager, then add this to `.vimrc`
after the plugin is loaded:

```vim
function! s:perl_lsp_root_uri(server_info) abort
  let l:root = lsp#utils#find_nearest_parent_file_directory(
        \ expand('%:p'),
        \ ['.perl-lsp.toml', 'Makefile.PL', 'Build.PL', 'cpanfile', 'dist.ini', '.git']
        \ )

  if empty(l:root)
    let l:root = getcwd()
  endif

  return lsp#utils#path_to_uri(l:root)
endfunction

if executable('perllsp')
  augroup perllsp_vim_lsp
    autocmd!
    autocmd User lsp_setup call lsp#register_server({
          \ 'name': 'perl-lsp',
          \ 'cmd': {server_info -> ['perllsp', '--stdio']},
          \ 'allowlist': ['perl'],
          \ 'root_uri': function('s:perl_lsp_root_uri'),
          \ })
  augroup END
endif

function! s:on_perl_lsp_buffer_enabled() abort
  setlocal omnifunc=lsp#complete
  setlocal signcolumn=yes

  nmap <buffer> gd <plug>(lsp-definition)
  nmap <buffer> gr <plug>(lsp-references)
  nmap <buffer> K  <plug>(lsp-hover)
  nmap <buffer> <leader>rn <plug>(lsp-rename)
  nmap <buffer> <leader>ca <plug>(lsp-code-action)
endfunction

augroup perllsp_vim_lsp_mappings
  autocmd!
  autocmd User lsp_buffer_enabled
        \ if &filetype ==# 'perl' |
        \   call s:on_perl_lsp_buffer_enabled() |
        \ endif
augroup END
```

### Optional vim-lsp server settings

Prefer `.perl-lsp.toml` for project-wide settings shared across editors. For
Vim-specific settings, pass workspace configuration through `workspace_config`:

```vim
if executable('perllsp')
  augroup perllsp_vim_lsp
    autocmd!
    autocmd User lsp_setup call lsp#register_server({
          \ 'name': 'perl-lsp',
          \ 'cmd': {server_info -> ['perllsp', '--stdio']},
          \ 'allowlist': ['perl'],
          \ 'root_uri': function('s:perl_lsp_root_uri'),
          \ 'workspace_config': {
          \   'perl': {
          \     'workspace': {
          \       'includePaths': ['lib', '.', 'local/lib/perl5']
          \     },
          \     'inlayHints': {
          \       'enabled': v:true
          \     }
          \   }
          \ },
          \ })
  augroup END
endif
```

For inlay hints in Vim, you may also need a recent Vim 9 build and:

```vim
let g:lsp_inlay_hints_enabled = 1
```

## Option B: coc.nvim

Install `coc.nvim`, then open Coc configuration:

```vim
:CocConfig
```

Add:

```json
{
  "languageserver": {
    "perl-lsp": {
      "command": "perllsp",
      "args": ["--stdio"],
      "filetypes": ["perl"],
      "rootPatterns": [
        ".perl-lsp.toml",
        "Makefile.PL",
        "Build.PL",
        "cpanfile",
        "dist.ini",
        ".git"
      ],
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"]
          }
        }
      }
    }
  }
}
```

Coc uses Vim filetypes, not filename extensions. If the server does not start,
check:

```vim
:CocCommand document.echoFiletype
```

It should report `perl`.

### Optional Coc mappings

`coc.nvim` does not install opinionated mappings for every user. Add mappings
only if they do not conflict with your existing Vim setup.

```vim
nmap <silent> gd <Plug>(coc-definition)
nmap <silent> gr <Plug>(coc-references)
nmap <silent> K  :call CocActionAsync('doHover')<CR>
nmap <leader>rn <Plug>(coc-rename)
nmap <leader>ca <Plug>(coc-codeaction)
```

For a full Coc-focused walkthrough, see
[COC_NEOVIM_SETUP.md](./COC_NEOVIM_SETUP.md). The LSP configuration model also
applies to Vim, but Neovim-specific keybindings or paths may differ.

## Verify It Is Running

1. Open a Perl file such as `lib/My/Module.pm`, `script/app.pl`, or `t/basic.t`.
2. Run `:set filetype?` and confirm `filetype=perl`.
3. Introduce a temporary syntax error.
4. Confirm diagnostics appear.
5. Try hover, definition lookup, or references lookup.
6. Remove the syntax error after testing.

Useful commands:

```vim
" vim-lsp
:LspStatus
:LspDocumentDiagnostics

" coc.nvim
:CocInfo
:CocOpenLog
:CocCommand workspace.showOutput
```

## Troubleshooting

### Vim cannot find `perllsp`

Check from Vim:

```vim
:echo executable('perllsp')
```

It should print `1`.

Check from a shell:

```bash
command -v perllsp
perllsp --version
perllsp --health
perllsp --info
```

On Windows PowerShell:

```powershell
where perllsp
perllsp --version
perllsp --health
perllsp --info
```

If Vim was launched from a GUI, it may not inherit the same `PATH` as your
terminal. Use an absolute path in the LSP config if needed.

### Server starts but no Perl features appear

Check the filetype:

```vim
:set filetype?
```

If it is empty or wrong, set it manually for the current buffer:

```vim
:setfiletype perl
```

Then add a persistent ftdetect rule for that file extension.

### Diagnostics do not appear

For `vim-lsp`:

```vim
:LspStatus
:LspDocumentDiagnostics
```

For `coc.nvim`:

```vim
:CocInfo
:CocOpenLog
:CocCommand document.echoFiletype
:CocCommand workspace.showOutput
```

Also verify outside Vim:

```bash
perllsp --check path/to/file.pl
```

### Module resolution is wrong

Prefer project-level `.perl-lsp.toml` for shared include paths:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]
```

Or pass client-specific settings through `vim-lsp` `workspace_config` or Coc
`settings`.

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input
from the editor. Use these commands for manual checks instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

For server-side behavior and configuration details, see
[docs/reference/CONFIG.md](../reference/CONFIG.md) and
[docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).

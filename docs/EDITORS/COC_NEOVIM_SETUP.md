# coc.nvim Setup Guide for perl-lsp

This guide shows how to run `perllsp` from coc.nvim in Neovim.

For Neovim's built-in LSP client, see `NEOVIM_SETUP.md`. Use this guide only if
you prefer the coc.nvim client.

## Prerequisites

- Neovim 0.8.0 or later
- Node.js 16.18.0 or later
- coc.nvim installed
- `perllsp` installed and available on your `PATH`
- A Perl project opened from the project root

If you use classic Vim with coc.nvim, use Vim 9.0.0438 or later.

Verify `perllsp` before changing coc.nvim settings:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Install `perllsp`

### Cargo

```bash
cargo install perllsp
```

### Homebrew

```bash
brew install perl-lsp
```

### From Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perllsp --locked
```

### Prebuilt Binary

Download the archive for your platform from GitHub Releases, extract it, and put
the `perllsp` binary on your `PATH`.

Release assets use the `perllsp-<version>-<target>` naming pattern. Check the
release page before copying a version number.

## Install coc.nvim

Install coc.nvim with your preferred plugin manager. With vim-plug:

```vim
Plug 'neoclide/coc.nvim', {'branch': 'release'}
```

Then restart Neovim and run:

```vim
:PlugInstall
```

coc.nvim does not install Perl LSP support automatically. You must either
install a Coc extension that provides Perl support or configure a language
server in `coc-settings.json`. This guide uses manual language-server
configuration.

## Configure coc.nvim

Open your coc.nvim config:

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

coc.nvim uses Vim/Neovim filetypes, not filename extensions. The server starts
only when the active buffer has `filetype=perl`.

## Optional: Project-Local coc.nvim Config

For one project, run:

```vim
:CocLocalConfig
```

This creates or edits `.vim/coc-settings.json` in the current workspace.

Prefer `.perl-lsp.toml` for settings shared across editors. Use local coc config
only for coc.nvim-specific behavior.

Example `.perl-lsp.toml`:

```toml
[perl]
include_paths = ["lib", ".", "local/lib/perl5", "vendor/lib"]

[features]
inlay_hints = true

[diagnostics]
perlcritic = false
perlcritic_severity = 3
```

## Optional: Filetype Detection

Check a Perl buffer:

```vim
:set filetype?
```

or:

```vim
:CocCommand document.echoFiletype
```

Expected result:

```text
perl
```

If `.t`, `.psgi`, `.cgi`, or `.fcgi` files are not detected as Perl, add
filetype rules before coc.nvim starts the language server:

```vim
augroup perl_filetypes
  autocmd!
  autocmd BufRead,BufNewFile *.t setfiletype perl
  autocmd BufRead,BufNewFile *.psgi setfiletype perl
  autocmd BufRead,BufNewFile *.cgi,*.fcgi setfiletype perl
augroup END
```

For Lua-based Neovim config:

```lua
vim.filetype.add({
  extension = {
    t = 'perl',
    psgi = 'perl',
    cgi = 'perl',
    fcgi = 'perl',
    PL = 'perl',
  },
})
```

## Recommended coc.nvim UI Settings

These are optional. Keep the first-run config small, then add UI settings after
the server attaches successfully.

```json
{
  "diagnostic.enable": true,
  "diagnostic.virtualText": true,
  "diagnostic.virtualTextPrefix": "■",
  "suggest.enablePreselect": true,
  "suggest.minTriggerInputLength": 1,
  "suggest.noselect": false,
  "suggest.enablePreview": true,
  "suggest.enableFloat": true,
  "signature.enable": true,
  "signature.target": "float",
  "codeLens.enable": true,
  "inlayHint.enable": true,
  "inlayHint.display": true,
  "inlayHint.enableParameter": true
}
```

## Recommended Keybindings

Add only the mappings you want. These follow coc.nvim's standard examples.

```vim
" Trigger completion.
if has('nvim')
  inoremap <silent><expr> <c-space> coc#refresh()
else
  inoremap <silent><expr> <c-@> coc#refresh()
endif

" Diagnostics.
nmap <silent> [g <Plug>(coc-diagnostic-prev)
nmap <silent> ]g <Plug>(coc-diagnostic-next)

" Navigation.
nmap <silent> gd <Plug>(coc-definition)
nmap <silent> gy <Plug>(coc-type-definition)
nmap <silent> gi <Plug>(coc-implementation)
nmap <silent> gr <Plug>(coc-references)

" Hover.
nnoremap <silent> K :call CocActionAsync('doHover')<CR>

" Rename and code actions.
nmap <leader>rn <Plug>(coc-rename)
nmap <leader>ca <Plug>(coc-codeaction-cursor)
nmap <leader>qf <Plug>(coc-fix-current)

" Formatting.
nmap <leader>f <Plug>(coc-format-selected)
xmap <leader>f <Plug>(coc-format-selected)

" Lists.
nnoremap <silent><nowait> <space>a :<C-u>CocList diagnostics<CR>
nnoremap <silent><nowait> <space>o :<C-u>CocList outline<CR>
nnoremap <silent><nowait> <space>s :<C-u>CocList -I symbols<CR>
```

Optional format command:

```vim
command! -nargs=0 Format :call CocActionAsync('format')
```

Optional format on save:

```vim
augroup perl_lsp_format
  autocmd!
  autocmd BufWritePre *.pl,*.pm,*.t,*.psgi call CocAction('format')
augroup END
```

## Verify It Is Running

1. Restart Neovim.

2. Open a Perl file such as `lib/My/Module.pm`, `script/app.pl`, or `t/basic.t`.

3. Confirm the filetype:

   ```vim
   :set filetype?
   ```

4. Check coc.nvim status:

   ```vim
   :CocInfo
   ```

5. Check the active filetype as coc.nvim sees it:

   ```vim
   :CocCommand document.echoFiletype
   ```

6. Open the output channel:

   ```vim
   :CocCommand workspace.showOutput
   ```

7. Introduce a temporary syntax error and confirm diagnostics appear.

8. Remove the syntax error after testing.

You can also test a file outside Neovim:

```bash
perllsp --check path/to/file.pl
```

## Troubleshooting

### coc.nvim does not start `perllsp`

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

Check from Neovim:

```vim
:!command -v perllsp
:CocInfo
:CocOpenLog
:CocCommand workspace.showOutput
```

If Neovim was launched from a GUI, it may not inherit the same `PATH` as your
terminal. Use an absolute path if needed:

```json
{
  "languageserver": {
    "perl-lsp": {
      "command": "/absolute/path/to/perllsp",
      "args": ["--stdio"],
      "filetypes": ["perl"]
    }
  }
}
```

### No diagnostics or completion

Check the active filetype:

```vim
:set filetype?
:CocCommand document.echoFiletype
```

It must be `perl`.

Then check diagnostics:

```vim
:CocDiagnostics
```

Also check a file outside Neovim:

```bash
perllsp --check path/to/file.pl
```

### Module resolution issues

Prefer `.perl-lsp.toml` for project-wide include paths:

```toml
[perl]
include_paths = ["lib", ".", "local/lib/perl5", "vendor/lib"]
```

### Slow performance

Reduce result caps:

```json
{
  "languageserver": {
    "perl-lsp": {
      "settings": {
        "perl": {
          "limits": {
            "workspaceSymbolCap": 100,
            "referencesCap": 200,
            "completionCap": 50
          },
          "workspace": {
            "resolutionTimeout": 25
          }
        }
      }
    }
  }
}
```

### Formatting does not work

Confirm `perltidy` is installed:

```bash
perltidy --version
```

Then run:

```vim
:call CocActionAsync('format')
```

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP input from
coc.nvim. Use these commands for manual checks instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

## See Also

- `docs/EDITORS/NEOVIM_SETUP.md` — Neovim built-in LSP client
- `docs/EDITORS/VIM_SETUP.md` — classic Vim with vim-lsp or coc.nvim
- `docs/reference/CONFIG.md`
- `docs/how-to/TROUBLESHOOTING.md`
- `docs/how-to/EDITOR_SETUP.md`

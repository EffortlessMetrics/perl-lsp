# coc.nvim Setup Guide for perl-lsp

This comprehensive guide helps you set up and configure the Perl Language Server in Vim/Neovim using coc.nvim.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Basic Setup](#basic-setup)
- [Configuration](#configuration)
- [Features](#features)
- [Keybindings](#keybindings)
- [Plugins](#plugins)
- [Troubleshooting](#troubleshooting)
- [Advanced Configuration](#advanced-configuration)

---

## Prerequisites

### Required

- **Vim** 8.0+ or **Neovim** 0.3+
- **Node.js** 14+ (for coc.nvim)
- **perl-lsp** server installed (see [Installation](#installation))

### Optional but Recommended

- **vim-plug** or **packer.nvim** (plugin manager)
- **coc.nvim** (LSP client)
- **coc-snippets** (snippet support)
- **Perl** 5.10 or later (for syntax validation)
- **perltidy** (for code formatting)
- **perlcritic** (for linting)

---

## Installation

### Install the Server

Choose one of the following methods:

#### Option 1: Install from crates.io (Recommended)

```bash
cargo install perl-lsp
```

#### Option 2: Download Pre-built Binary

Download from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases):

```bash
# Linux (x86_64)
curl -LO https://github.com/EffortlessMetrics/perl-lsp/releases/latest/download/perl-lsp-linux-x86_64.tar.gz
tar xzf perl-lsp-linux-x86_64.tar.gz
sudo mv perl-lsp /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/EffortlessMetrics/perl-lsp/releases/latest/download/perl-lsp-darwin-aarch64.tar.gz
tar xzf perl-lsp-darwin-aarch64.tar.gz
sudo mv perl-lsp /usr/local/bin/
```

#### Option 3: Build from Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp
```

### Verify Installation

```bash
# Check version
perl-lsp --version

# Quick health check
perl-lsp --health
# Should output: ok 0.9.0
```

### Install coc.nvim

#### Using vim-plug

Add to your `.vimrc` or `init.vim`:

```vim
" Use release branch (recommended)
Plug 'neoclide/coc.nvim', {'branch': 'release'}
```

Then run:

```vim
:PlugInstall
```

#### Using packer.nvim (Neovim)

Add to your `init.lua`:

```lua
use {
  'neoclide/coc.nvim',
  branch = 'release',
  run = ':CocInstall'
}
```

#### Manual Installation

```bash
# Release branch
cd ~/.vim/bundle
git clone --branch release https://github.com/neoclide/coc.nvim.git

# Or for Neovim
cd ~/.local/share/nvim/site/pack/coc/start
git clone --branch release https://github.com/neoclide/coc.nvim.git
```

---

## Basic Setup

### Create coc.nvim Configuration

Create `~/.vim/coc-settings.json` (or `~/.config/nvim/coc-settings.json` for Neovim):

```json
{
  "languageserver": {
    "perl": {
      "command": "perl-lsp",
      "args": ["--stdio"],
      "filetypes": ["perl"],
      "rootPatterns": ["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"],
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"],
            "useSystemInc": false,
            "resolutionTimeout": 50
          },
          "inlayHints": {
            "enabled": true,
            "parameterHints": true,
            "typeHints": true
          },
          "limits": {
            "workspaceSymbolCap": 200,
            "referencesCap": 500,
            "completionCap": 100
          }
        }
      }
    }
  }
}
```

### Verify Setup

1. Restart Vim/Neovim
2. Open a `.pl` or `.pm` file
3. Check if coc.nvim is loaded:
   ```vim
   :CocInfo
   ```
4. Check if perl-lsp is running:
   ```vim
   :CocCommand workspace.showOutput
   ```

---

## Configuration

### Full Configuration

Add to `~/.vim/coc-settings.json`:

```json
{
  "languageserver": {
    "perl": {
      "command": "perl-lsp",
      "args": ["--stdio"],
      "filetypes": ["perl"],
      "rootPatterns": ["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"],
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"],
            "useSystemInc": false,
            "resolutionTimeout": 50
          },
          "inlayHints": {
            "enabled": true,
            "parameterHints": true,
            "typeHints": true,
            "chainedHints": false,
            "maxLength": 30
          },
          "testRunner": {
            "enabled": true,
            "command": "perl",
            "args": [],
            "timeout": 60000
          },
          "limits": {
            "workspaceSymbolCap": 200,
            "referencesCap": 500,
            "completionCap": 100,
            "astCacheMaxEntries": 100,
            "maxIndexedFiles": 10000,
            "maxTotalSymbols": 500000,
            "workspaceScanDeadlineMs": 30000,
            "referenceSearchDeadlineMs": 2000
          }
        }
      },
      "env": {
        "PERL5LIB": "${workspaceFolder}/lib",
        "PERL_MB_OPT": "--install_base ${workspaceFolder}/local"
      }
    }
  },
  "diagnostic.enable": true,
  "diagnostic.displayByAle": false,
  "diagnostic.errorSign": "✗",
  "diagnostic.warningSign": "⚠",
  "diagnostic.infoSign": "ℹ",
  "diagnostic.hintSign": "➤",
  "diagnostic.virtualText": true,
  "diagnostic.virtualTextPrefix": "■",
  "suggest.enablePreselect": true,
  "suggest.minTriggerInputLength": 1,
  "suggest.noselect": false,
  "suggest.enablePreview": true,
  "suggest.floatEnable": true,
  "signature.enable": true,
  "signature.target": "float",
  "signature.triggerSignatureWait": 50,
  "codeLens.enable": true,
  "codeLens.position": "eol",
  "inlayHint.enable": true,
  "inlayHint.display": true,
  "inlayHint.enableParameter": true,
  "inlayHint.enableType": true
}
```

### Project-Specific Configuration

Create `.vim/coc-settings.json` in your project root:

```json
{
  "languageserver": {
    "perl": {
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", "local/lib/perl5", "vendor/lib"]
          }
        }
      }
    }
  }
}
```

---

## Features

### Syntax Diagnostics

Real-time syntax error detection and reporting:

```perl
# Errors are highlighted as you type
my $x = 1
# Missing semicolon - error shown immediately
```

View diagnostics:
- Errors are shown in the gutter
- Hover over error markers for details
- View all diagnostics: `:CocDiagnostics`

### Go to Definition

Navigate to symbol definitions:

```vim
:CocAction('jumpDefinition')
```

Or use keybinding: `gd`

```perl
use MyModule;

MyModule::some_function();
# ^ gd here jumps to the definition
```

### Find References

Find all usages of a symbol:

```vim
:CocAction('jumpReferences')
```

Or use keybinding: `gr`

```perl
sub my_function {
    return 42;
}

# ^ Find references here shows all calls to my_function
```

### Hover Information

View documentation and type information:

```vim
:CocAction('doHover')
```

Or use keybinding: `K`

### Code Completion

Intelligent code completion:

```vim
:CocAction('suggest')
```

Or use keybinding: `Ctrl+Space`

```perl
use MyModule;

MyModule::  # Press Ctrl+Space for completion
```

### Document Symbols

Navigate symbols in the current file:

```vim
:CocList symbols
```

### Workspace Symbols

Search symbols across the entire workspace:

```vim
:CocList workspaceSymbols
```

### Rename Symbol

Rename symbols across the workspace:

```vim
:CocRename
```

Or use keybinding: `<leader>rn`

### Formatting

Format Perl code using perltidy:

```vim
:CocAction('format')
```

Or use keybinding: `<leader>f`

### Code Actions

Quick fixes and refactorings:

```vim
:CocAction('codeAction')
```

Or use keybinding: `<leader>ca`

Available actions:
- Extract variable
- Extract subroutine
- Optimize imports
- Add missing pragmas

### Inlay Hints

Inline type and parameter hints:

```perl
sub my_function($name, $count) {
    return "Hello, $name x$count";
}

my_function("World", 5);
# ^ Shows: my_function(/* name: */ "World", /* count: */ 5)
```

Enable inlay hints:
```vim
:CocCommand inlayHints.toggle
```

---

## Keybindings

### Recommended Keybindings

Add to your `.vimrc` or `init.vim`:

```vim
" Use Tab for trigger completion with characters ahead and navigate
inoremap <silent><expr> <TAB>
      \ pumvisible() ? "\<C-n>" :
      \ <SID>check_back_space() ? "\<TAB>" :
      \ coc#refresh()
inoremap <expr><S-TAB> pumvisible() ? "\<C-p>" : "\<C-h>"

function! s:check_back_space() abort
  let col = col('.') - 1
  return !col || getline('.')[col - 1]  =~# '\s'
endfunction

" Use <c-space> to trigger completion
if has('nvim')
  inoremap <silent><expr> <c-space> coc#refresh()
else
  inoremap <silent><expr> <c-@> coc#refresh()
endif

" Make <CR> auto-select the first completion item and notify coc.nvim to
" format on enter
inoremap <silent><expr> <cr> pumvisible() ? coc#_select_confirm()
                              \: "\<C-g>u\<CR>\<c-r>=coc#on_enter()\<CR>"

" Use `[g` and `]g` to navigate diagnostics
nmap <silent> [g <Plug>(coc-diagnostic-prev)
nmap <silent> ]g <Plug>(coc-diagnostic-next)

" GoTo code navigation
nmap <silent> gd <Plug>(coc-definition)
nmap <silent> gy <Plug>(coc-type-definition)
nmap <silent> gi <Plug>(coc-implementation)
nmap <silent> gr <Plug>(coc-references)

" Use K to show documentation in preview window
nnoremap <silent> K :call <SID>show_documentation()<CR>

function! s:show_documentation()
  if (index(['vim','help'], &filetype) >= 0)
    execute 'h '.expand('<cword>')
  elseif (coc#rpc#ready())
    call CocActionAsync('doHover')
  else
    execute '!' . &keywordprg . " " . expand('<cword>')
  endif
endfunction

" Highlight the symbol and its references when holding the cursor
autocmd CursorHold * silent call CocActionAsync('highlight')

" Symbol renaming
nmap <leader>rn <Plug>(coc-rename)

" Formatting
nmap <leader>f <Plug>(coc-format-selected)
xmap <leader>f <Plug>(coc-format-selected)

" Apply AutoFix to problem on the current line
nmap <leader>qf  <Plug>(coc-fix-current)

" Map function and class text objects
xmap if <Plug>(coc-funcobj-i)
omap if <Plug>(coc-funcobj-i)
xmap af <Plug>(coc-funcobj-a)
omap af <Plug>(coc-funcobj-a)
xmap ic <Plug>(coc-classobj-i)
omap ic <Plug>(coc-classobj-i)
xmap ac <Plug>(coc-classobj-a)
omap ac <Plug>(coc-classobj-a)

" Remap <C-f> to scroll float windows
nnoremap <silent><nowait><expr> <C-f> coc#float#has_scroll() ? coc#float#scroll(1) : "\<C-f>"
nnoremap <silent><nowait><expr> <C-b> coc#float#has_scroll() ? coc#float#scroll(0) : "\<C-b>"

" Add status line support
set statusline^=%{coc#status()}%{get(b:,'coc_current_function','')}

" Show all diagnostics
nnoremap <silent><nowait> <space>a  :<C-u>CocList diagnostics<cr>

" Manage extensions
nnoremap <silent><nowait> <space>e  :<C-u>CocList extensions<cr>

" Show commands
nnoremap <silent><nowait> <space>c  :<C-u>CocList commands<cr>

" Find symbol of current document
nnoremap <silent><nowait> <space>o  :<C-u>CocList outline<cr>

" Search workspace symbols
nnoremap <silent><nowait> <space>s  :<C-u>CocList -I symbols<cr>

" Resume latest coc list
nnoremap <silent><nowait> <space>p  :<C-u>CocListResume<CR>
```

---

## Plugins

### coc-snippets

For snippet support:

```vim
" Install coc-snippets
:CocInstall coc-snippets

" Add to .vimrc or init.vim
let g:coc_snippet_next = '<tab>'
let g:coc_snippet_prev = '<s-tab>'
```

### coc-pairs

For auto-closing brackets:

```vim
:CocInstall coc-pairs
```

### coc-lists

For fuzzy finding:

```vim
:CocInstall coc-lists
```

### coc-explorer

For file explorer:

```vim
:CocInstall coc-explorer

" Add keybinding
nnoremap <space>e :CocCommand explorer<CR>
```

### coc-git

For Git integration:

```vim
:CocInstall coc-git
```

### coc-yank

For yank history:

```vim
:CocInstall coc-yank

" Add keybindings
nnoremap <silent> <space>y  :<C-u>CocList -A --normal yank<cr>
```

---

## Troubleshooting

### Server Not Starting

**Symptoms**: No diagnostics, no completion, error in output

**Solutions**:

1. **Verify binary is in PATH**:
   ```vim
   :!which perl-lsp
   ```

2. **Check coc.nvim info**:
   ```vim
   :CocInfo
   ```

3. **Check coc.nvim logs**:
   ```vim
   :CocCommand workspace.showOutput
   ```

4. **Test server manually**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

### No Diagnostics

**Symptoms**: No errors shown for invalid code

**Solutions**:

1. **Check file type**:
   ```vim
   :set filetype?
   ```
   Should output: `filetype=perl`

2. **Set file type manually**:
   ```vim
   :set filetype=perl
   ```

3. **Verify diagnostics enabled**:
   ```vim
   :CocCommand workspace.toggleDiagnostics
   ```

### Slow Performance

**Symptoms**: Lag when typing, slow completions

**Solutions**:

1. **Reduce result caps**:
   ```json
   {
     "languageserver": {
       "perl": {
         "settings": {
           "perl": {
             "limits": {
               "workspaceSymbolCap": 100,
               "referencesCap": 200,
               "completionCap": 50
             }
           }
         }
       }
     }
   }
   ```

2. **Disable system @INC**:
   ```json
   {
     "languageserver": {
       "perl": {
         "settings": {
           "perl": {
             "workspace": {
               "useSystemInc": false
             }
           }
         }
       }
     }
   }
   ```

3. **Reduce resolution timeout**:
   ```json
   {
     "languageserver": {
       "perl": {
         "settings": {
           "perl": {
             "workspace": {
               "resolutionTimeout": 25
             }
           }
         }
       }
     }
   }
   ```

### Module Resolution Issues

**Symptoms**: Can't find modules, go-to-definition fails

**Solutions**:

1. **Check include paths**:
   ```json
   {
     "languageserver": {
       "perl": {
         "settings": {
           "perl": {
             "workspace": {
               "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
             }
           }
         }
       }
     }
   }
   ```

2. **Verify module exists**:
   ```bash
   perl -e 'use Module::Name;'
   ```

3. **Check workspace root**:
   ```vim
   :CocCommand workspace.workspaceFolders
   ```

### Formatting Not Working

**Symptoms**: Format command does nothing or errors

**Solutions**:

1. **Install perltidy**:
   ```bash
   # macOS
   brew install perltidy

   # Ubuntu/Debian
   sudo apt-get install perltidy

   # CentOS/RHEL
   sudo yum install perl-Perl-Tidy
   ```

2. **Check perltidy works**:
   ```bash
   perltidy --version
   ```

3. **Verify formatting enabled**:
   ```vim
   :CocAction('format')
   ```

---

## Advanced Configuration

### Multi-Root Workspace

For workspaces with multiple folders:

```vim
" Add to .vimrc or init.vim
function! s:workspace_folders() abort
  let folders = []
  let root = finddir('.git/..', expand('%:p:h').';')
  if !empty(root)
    call add(folders, { 'uri': 'file://' . fnamemodify(root, ':p'), 'name': fnamemodify(root, ':t') })
  endif
  return folders
endfunction

augroup coc_workspace
  autocmd!
  autocmd BufEnter * call coc#rpc#notify('workspace/didChangeWorkspaceFolders', {'added': s:workspace_folders()})
augroup END
```

### Custom Handlers

Override default LSP handlers:

```vim
" Custom hover handler
function! s:custom_hover() abort
  call CocActionAsync('doHover')
  " Add custom logic here
endfunction

nnoremap <silent> K :call <SID>custom_hover()<CR>
```

### Diagnostic Configuration

Customize diagnostic display:

```json
{
  "diagnostic.errorSign": "✗",
  "diagnostic.warningSign": "⚠",
  "diagnostic.infoSign": "ℹ",
  "diagnostic.hintSign": "➤",
  "diagnostic.virtualText": true,
  "diagnostic.virtualTextPrefix": "■",
  "diagnostic.virtualTextLineSeparator": " └ ",
  "diagnostic.virtualTextCurrentLineOnly": false
}
```

### Auto-Commands

Set up auto-commands for automatic actions:

```vim
" Format on save
autocmd BufWritePre *.pl :call CocAction('format')

" Auto-start coc.nvim for Perl files
autocmd FileType perl :CocCommand workspace.toggleDiagnostics
```

### Performance Tuning

For large workspaces, adjust performance settings:

```json
{
  "suggest.maxCompleteItemCount": 50,
  "suggest.minTriggerInputLength": 1,
  "suggest.triggerCompletionWait": 50,
  "suggest.noselect": false,
  "suggest.enablePreselect": true,
  "suggest.completionItemKindLabels": {
    "function": "ƒ",
    "method": "m",
    "variable": "v",
    "constant": "c",
    "class": "C",
    "interface": "I",
    "module": "M"
  }
}
```

### Debug Logging

Enable detailed logging for troubleshooting:

```vim
" Enable debug logging
:CocCommand workspace.toggleDebug

" View logs
:CocCommand workspace.showOutput
```

---

## Complete Example Configuration

### .vimrc or init.vim

```vim
" Plugin setup (using vim-plug)
call plug#begin('~/.vim/plugged')
Plug 'neoclide/coc.nvim', {'branch': 'release'}
call plug#end()

" coc.nvim configuration
let g:coc_global_extensions = [
      \ 'coc-snippets',
      \ 'coc-pairs',
      \ 'coc-lists',
      \ 'coc-explorer',
      \ 'coc-git',
      \ 'coc-yank'
      \]

" Use Tab for trigger completion
inoremap <silent><expr> <TAB>
      \ pumvisible() ? "\<C-n>" :
      \ <SID>check_back_space() ? "\<TAB>" :
      \ coc#refresh()

function! s:check_back_space() abort
  let col = col('.') - 1
  return !col || getline('.')[col - 1]  =~# '\s'
endfunction

" Use <c-space> to trigger completion
if has('nvim')
  inoremap <silent><expr> <c-space> coc#refresh()
else
  inoremap <silent><expr> <c-@> coc#refresh()
endif

" Make <CR> auto-select the first completion item
inoremap <silent><expr> <cr> pumvisible() ? coc#_select_confirm()
                              \: "\<C-g>u\<CR>\<c-r>=coc#on_enter()\<CR>"

" Navigate diagnostics
nmap <silent> [g <Plug>(coc-diagnostic-prev)
nmap <silent> ]g <Plug>(coc-diagnostic-next)

" GoTo code navigation
nmap <silent> gd <Plug>(coc-definition)
nmap <silent> gy <Plug>(coc-type-definition)
nmap <silent> gi <Plug>(coc-implementation)
nmap <silent> gr <Plug>(coc-references)

" Show documentation
nnoremap <silent> K :call <SID>show_documentation()<CR>

function! s:show_documentation()
  if (index(['vim','help'], &filetype) >= 0)
    execute 'h '.expand('<cword>')
  elseif (coc#rpc#ready())
    call CocActionAsync('doHover')
  else
    execute '!' . &keywordprg . " " . expand('<cword>')
  endif
endfunction

" Highlight symbol under cursor
autocmd CursorHold * silent call CocActionAsync('highlight')

" Symbol renaming
nmap <leader>rn <Plug>(coc-rename)

" Formatting
nmap <leader>f <Plug>(coc-format-selected)
xmap <leader>f <Plug>(coc-format-selected)

" Apply AutoFix
nmap <leader>qf  <Plug>(coc-fix-current)

" Symbol text objects
xmap if <Plug>(coc-funcobj-i)
omap if <Plug>(coc-funcobj-i)
xmap af <Plug>(coc-funcobj-a)
omap af <Plug>(coc-funcobj-a)
xmap ic <Plug>(coc-classobj-i)
omap ic <Plug>(coc-classobj-i)
xmap ac <Plug>(coc-classobj-a)
omap ac <Plug>(coc-classobj-a)

" Scroll float windows
nnoremap <silent><nowait><expr> <C-f> coc#float#has_scroll() ? coc#float#scroll(1) : "\<C-f>"
nnoremap <silent><nowait><expr> <C-b> coc#float#has_scroll() ? coc#float#scroll(0) : "\<C-b>"

" Status line
set statusline^=%{coc#status()}%{get(b:,'coc_current_function','')}

" CocList commands
nnoremap <silent><nowait> <space>a  :<C-u>CocList diagnostics<cr>
nnoremap <silent><nowait> <space>e  :<C-u>CocList extensions<cr>
nnoremap <silent><nowait> <space>c  :<C-u>CocList commands<cr>
nnoremap <silent><nowait> <space>o  :<C-u>CocList outline<cr>
nnoremap <silent><nowait> <space>s  :<C-u>CocList -I symbols<cr>
nnoremap <silent><nowait> <space>p  :<C-u>CocListResume<CR>

" Format on save
autocmd BufWritePre *.pl :call CocAction('format')

" Auto-start for Perl files
autocmd FileType perl :CocCommand workspace.toggleDiagnostics
```

### coc-settings.json

```json
{
  "languageserver": {
    "perl": {
      "command": "perl-lsp",
      "args": ["--stdio"],
      "filetypes": ["perl"],
      "rootPatterns": ["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"],
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"],
            "useSystemInc": false,
            "resolutionTimeout": 50
          },
          "inlayHints": {
            "enabled": true,
            "parameterHints": true,
            "typeHints": true,
            "maxLength": 30
          },
          "limits": {
            "workspaceSymbolCap": 200,
            "referencesCap": 500,
            "completionCap": 100
          }
        }
      },
      "env": {
        "PERL5LIB": "${workspaceFolder}/lib",
        "PERL_MB_OPT": "--install_base ${workspaceFolder}/local"
      }
    }
  },
  "diagnostic.enable": true,
  "diagnostic.errorSign": "✗",
  "diagnostic.warningSign": "⚠",
  "diagnostic.infoSign": "ℹ",
  "diagnostic.hintSign": "➤",
  "diagnostic.virtualText": true,
  "diagnostic.virtualTextPrefix": "■",
  "suggest.enablePreselect": true,
  "suggest.minTriggerInputLength": 1,
  "suggest.noselect": false,
  "suggest.enablePreview": true,
  "suggest.floatEnable": true,
  "signature.enable": true,
  "signature.target": "float",
  "signature.triggerSignatureWait": 50,
  "codeLens.enable": true,
  "codeLens.position": "eol",
  "inlayHint.enable": true,
  "inlayHint.display": true,
  "inlayHint.enableParameter": true,
  "inlayHint.enableType": true
}
```

---

## See Also

- [Getting Started](../GETTING_STARTED.md) - Quick start guide
- [Configuration Reference](../CONFIG.md) - Complete configuration options
- [Troubleshooting Guide](../TROUBLESHOOTING.md) - Common issues and solutions
- [Performance Tuning](../PERFORMANCE_TUNING.md) - Performance optimization guide
- [Editor Setup](../EDITOR_SETUP.md) - Other editor configurations
- [coc.nvim Documentation](https://github.com/neoclide/coc.nvim)

# LSP Feature Status

Auto-generated from `features.toml` - DO NOT EDIT

Version: 0.8.5 | LSP: 3.18

## debug

| Feature | Spec | Status | Description |
|---------|------|--------|-------------|
| dap.inline_values | DAP 1.0 | 📋 Planned | Inline variable values while debugging |
| dap.breakpoints | DAP 1.0 | 📋 Planned | Breakpoint support |

## workspace

| Feature | Spec | Status | Description |
|---------|------|--------|-------------|
| workspace_symbol | LSP 3.0 | ✅ Complete | Search symbols across workspace |
| workspace_diagnostics | LSP 3.17 | ✅ Complete | Workspace-wide pull diagnostics |
| execute_command | LSP 3.0 | ✅ Complete | Execute commands (Perl::Critic, etc) |
| workspace_folders | LSP 3.6 | ✅ Complete | Multi-root workspace support |
| file_operations | LSP 3.16 | ✅ Complete | File rename/create/delete tracking |
| workspace_edit | LSP 3.0 | ⚠️ Experimental | Apply workspace-wide edits |
| moniker | LSP 3.16 | 📋 Planned | Cross-project symbol identity |

## notebook

| Feature | Spec | Status | Description |
|---------|------|--------|-------------|
| notebook_document_sync | LSP 3.17 | 📋 Planned | Notebook document synchronization |
| notebook_cell_execution | LSP 3.17 | 📋 Planned | Execute notebook cells |

## window

| Feature | Spec | Status | Description |
|---------|------|--------|-------------|
| progress | LSP 3.0 | ✅ Complete | Progress reporting |
| show_message | LSP 3.0 | ✅ Complete | Show messages to user |
| log_message | LSP 3.0 | ✅ Complete | Log messages |
| work_done_progress | LSP 3.15 | ⚠️ Experimental | Detailed work progress |

## text document

| Feature | Spec | Status | Description |
|---------|------|--------|-------------|
| completion | LSP 3.0 | ✅ Complete | Code completion with 150+ built-in functions |
| hover | LSP 3.0 | ✅ Complete | Hover information with documentation |
| signature_help | LSP 3.0 | ✅ Complete | Signature help with parameter hints |
| definition | LSP 3.0 | ✅ Complete | Go to definition |
| declaration | LSP 3.14 | ✅ Complete | Go to declaration |
| references | LSP 3.0 | ✅ Complete | Find references across workspace |
| document_symbol | LSP 3.0 | ✅ Complete | Document symbols with hierarchy |
| code_action | LSP 3.0 | ✅ Complete | Code actions and quick fixes |
| code_lens | LSP 3.0 | 🔧 Preview | Code lens with reference counts |
| formatting | LSP 3.0 | ✅ Complete | Document formatting with Perl::Tidy |
| range_formatting | LSP 3.0 | ✅ Complete | Range formatting |
| on_type_formatting | LSP 3.0 | ✅ Complete | On-type formatting with auto-indent |
| rename | LSP 3.0 | ✅ Complete | Rename symbol across files |
| document_link | LSP 3.0 | ✅ Complete | Document links to modules and docs |
| folding_range | LSP 3.0 | ✅ Complete | Folding ranges with fallback |
| selection_range | LSP 3.15 | ✅ Complete | Smart selection expansion |
| semantic_tokens | LSP 3.16 | ✅ Complete | Enhanced syntax highlighting |
| inlay_hint | LSP 3.17 | ✅ Complete | Parameter names and type hints |
| type_hierarchy | LSP 3.17 | ✅ Complete | Type hierarchy for classes/roles |
| call_hierarchy | LSP 3.16 | 🔧 Preview | Call hierarchy navigation |
| pull_diagnostics | LSP 3.17 | ✅ Complete | Pull model diagnostics |
| inline_completion | LSP 3.18 | 📋 Planned | Inline AI-powered completions |
| type_definition | LSP 3.6 | ⚠️ Experimental | Go to type definition |
| implementation | LSP 3.6 | ⚠️ Experimental | Go to implementation |
| document_color | LSP 3.6 | 📋 Planned | Color decorators |
| linked_editing_range | LSP 3.16 | 📋 Planned | Linked editing of related tokens |


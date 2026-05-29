# LSP Features Overview

**Version**: 0.12.4 | **LSP spec**: 3.18 | **Source of truth**: [`features.toml`](../../features.toml)

119 features — 94 LSP, 24 DAP, 1 experimental. All are GA and
advertised unless noted otherwise. This document groups them by user-facing
area to help you find what you need quickly.

## Text Document — Intelligence

### Completion (`lsp.completion`)

Code completion with 150+ built-in functions. Perl-specific highlights:
- **DBI type inference**: `$dbh->` suggests database handle methods, `$sth->` suggests statement handle methods
- **Moo/Moose**: option-key completion for `has` declarations
- **Workspace symbols**: method completion from indexed workspace symbols
- **Auto-import**: suggests `use Module` when calling an unimported function
- **XS API**: completion for XS extension development
- **File paths**: path completion inside `open`, `require`, `use lib`
- **Test helpers**: `Test::More` function completion

Additional resolution step: `lsp.completion_item_resolve` fetches full
documentation and import edits lazily.

### Hover (`lsp.hover`)

Rich hover with:
- **POD rendering**: inline documentation rendered to Markdown
- **Moo/Moose attributes**: shows `isa`, `is`, `required`, `predicate`, `builder`, `clearer`
- **Inherited methods**: resolves up the class hierarchy
- **Special variables**: explains `$!`, `$/`, `%ENV`, etc.
- **Pragmas**: documentation for `use strict`, `use warnings`, feature pragmas
- **Regex explainer**: breaks down regex patterns inline
- **File-test operators**: `-e`, `-f`, `-d` reference
- **XS::Typemap integration**

### Signature Help (`lsp.signature_help`)

Parameter hints as you type function calls, including built-in functions and
workspace-indexed subroutines.

### Go-To Navigation

| Feature | ID |
|---|---|
| Go to definition | `lsp.definition` |
| Go to declaration | `lsp.declaration` |
| Go to type definition | `lsp.type_definition` |
| Go to implementation | `lsp.implementation` |
| Prepare rename (validate + placeholder) | `lsp.prepare_rename` |

All navigation features integrate with workspace indexing for cross-file resolution.

### References & Hierarchy

| Feature | ID | Notes |
|---|---|---|
| Find references | `lsp.references` | Workspace-wide |
| Document symbols | `lsp.document_symbol` | Hierarchical symbol tree |
| Document highlights | `lsp.document_highlight` | Local variable references |
| Call hierarchy | `lsp.call_hierarchy` | Caller/callee navigation |
| Type hierarchy | `lsp.type_hierarchy` | Moo/Moose class inheritance |
| Moniker | `lsp.moniker` | Cross-project symbol identity |

## Text Document — Actions & Refactoring

### Code Actions (`lsp.code_action`)

9 action kinds:
- **QuickFix**: diagnostic fixes for undefined/unused variables, missing `strict`/`warnings`, deprecated patterns
- **Refactor**: general refactoring entry point
- **RefactorExtract**: extract variable, extract subroutine
- **RefactorInline**: inline variable or subroutine
- **RefactorRewrite**: C-style `for` → `foreach`, postfix-`if` conversion
- **Source**: general source transformations
- **SourceOrganizeImports**: sort and deduplicate `use` statements
- **SourceFixAll**: apply all safe quick fixes in one step
- **SourceModernize**: replace legacy patterns (`local $_`, bareword filehandles, two-arg `open`)

Lazy loading via `lsp.code_action_resolve`.

### Refactoring Engine (`lsp.refactoring`)

Dedicated Perl refactoring engine (264 tests):
- Workspace-wide atomic symbol rename with rollback
- Import optimization (analyze and rewrite `use`/`require` statements)
- Code modernization (bareword filehandles, two-arg `open`, missing pragmas)
- Extract module, move subroutine, inline variable, inline subroutine
- Backup/rollback support via `RefactoringEngine`

### Rename (`lsp.rename`)

Rename a symbol across all files in the workspace. Use `lsp.prepare_rename`
first to validate the rename and get the default placeholder text.

## Text Document — Formatting

| Feature | ID | Notes |
|---|---|---|
| Document formatting | `lsp.formatting` | Native formatter by default; Perl::Tidy compatibility is explicit opt-in |
| Range formatting | `lsp.range_formatting` | Native single-range edits |
| Multi-range formatting | `lsp.ranges_formatting` | `textDocument/rangesFormatting` (@proposed) |
| On-type formatting | `lsp.on_type_formatting` | Auto-indent on `{`, `}`, `;` |
| Format on save | `lsp.will_save_wait_until` | Via willSaveWaitUntil |

## Text Document — Diagnostics

All diagnostics use the pull model (`lsp.pull_diagnostics`) and are also
pushed via `lsp.publish_diagnostics`.

| Code | ID | Description |
|---|---|---|
| PL100 | `lsp.diagnostic.missing_strict` | Missing `use strict` — respects Moo/Moose/Modern::Perl equivalents |
| PL101 | `lsp.diagnostic.missing_warnings` | Missing `use warnings` — respects Moo/Moose/Modern::Perl equivalents |
| PL102 | `lsp.diagnostic.unused_variable` | Declared but never-used variables (suppressed by `_` prefix) |
| PL406 | `lsp.diagnostic.unreachable_code` | Code after `return`/`die`/`exit`/`croak` — tagged `Unnecessary` |
| PL502/PL503 | `lsp.diagnostic.phase_scoped_pragmas` | `use strict`/`warnings` inside `BEGIN`/`END` only — quick fix moves to file scope |

Markdown-formatted diagnostic messages: `lsp.diagnostic.markup_message_support`
(opt-in by client capability).

Workspace-wide diagnostics: `lsp.workspace_diagnostics`.

## Text Document — Visual Aids

### Semantic Tokens (`lsp.semantic_tokens`)

24 token types including `namespace`, `class`, `function`, `method`,
`variable`, `parameter`, `keyword`, `regexp`, `label`, and three
embedded-language types (`sql_string`, `sql_heredoc_keyword`,
`json_heredoc_key`).

13 modifiers including `declaration`, `readonly`, `deprecated`, `async`,
`scalarVariable`, `arrayVariable`, `hashVariable`.

Supports both full (`semanticTokens/full`) and range (`semanticTokens/range`)
requests per LSP 3.16. Delta-encoded `[u32; 5]` wire format.

### Inlay Hints (`lsp.inlay_hint`)

Parameter-name labels for 14 Perl built-ins: `open`, `split`, `substr`,
`push`, `map`, `grep`, `sort`, `join`, `sprintf`, `printf`, `index`,
`rindex`, `splice`, `pack`/`unpack`.

Type labels for literal expressions: `Num`, `Str`, `Hash`, `Array`, `Regex`, `CodeRef`.

Hints are scoped to the visible editor region. `label.location` jump-to-definition
via resolve; tooltip text deferred to `inlayHint/resolve` per LSP 3.17.

### Code Lens (`lsp.code_lens`)

Reference counts displayed above subroutines and packages. Resolved lazily
via `lsp.code_lens_resolve`; refreshed server-side via `lsp.code_lens_refresh`.

### Document Links (`lsp.document_link`)

Clickable links from `use Module` and `require 'file.pl'` to their
definitions. Resolved lazily via `lsp.document_link_resolve`.

### Other Visual

| Feature | ID |
|---|---|
| Folding ranges | `lsp.folding_range` |
| Smart selection expansion | `lsp.selection_range` |
| Linked editing | `lsp.linked_editing_range` |
| Color decorators | `lsp.document_color` / `lsp.color_presentation` |
| Inline values (debug) | `lsp.inline_value` |

## Workspace

### Symbol Search (`lsp.workspace_symbol`)

Substring and prefix matching across all indexed files. Features:
- Proximity ranking: symbols in the active document's package hierarchy score higher
- Multi-root workspace support with per-folder URI attribution
- Incremental re-index with content-hash early-exit (unchanged files skip re-parse)
- Stale-index cleanup on file removal
- Parse-storm throttling for large batch indexing
- Validated at CPAN scale (10k+ files)

### File Operations

The server tracks file lifecycle events to keep the workspace index accurate:

| Notification/Request | ID |
|---|---|
| `willCreateFiles` | `lsp.will_create_files` |
| `didCreateFiles` | `lsp.did_create_files` |
| `willRenameFiles` | `lsp.will_rename_files` |
| `didRenameFiles` | `lsp.did_rename_files` |
| `willDeleteFiles` | `lsp.will_delete_files` |
| `didDeleteFiles` | `lsp.did_delete_files` |
| Watched file changes | `lsp.did_change_watched_files` |

### Other Workspace

| Feature | ID |
|---|---|
| Multi-root workspace | `lsp.workspace_folders` |
| Execute command (Perl::Critic, etc.) | `lsp.execute_command` |
| Workspace-wide edits | `lsp.workspace_edit` / `lsp.apply_edit` |
| Configuration pull | `lsp.configuration` |
| Configuration change | `lsp.did_change_configuration` |
| Virtual document content | `lsp.text_document_content` (@proposed) |

## Window / Protocol

| Feature | ID |
|---|---|
| Progress reporting | `lsp.progress` / `lsp.work_done_progress` |
| Show message | `lsp.show_message` / `lsp.show_message_request` |
| Show document (open URI) | `lsp.show_document` |
| Log message | `lsp.log_message` |
| Telemetry events | `lsp.telemetry_event` |
| Server trace logging | `lsp.log_trace` |

## Notebook Documents

| Feature | ID | Notes |
|---|---|---|
| Notebook sync | `lsp.notebook_document_sync` | didOpen/didChange/didSave/didClose |
| Cell execution summary | `lsp.notebook_cell_execution` | Tracks `executionSummary` metadata; LSP does not execute cells — not advertised |

## Debug Adapter Protocol (DAP)

24 DAP features. See [DAP_IMPLEMENTATION_SPECIFICATION.md](DAP_IMPLEMENTATION_SPECIFICATION.md)
for the full protocol reference.

### Core Debug Loop (`dap.core`)

Full VS Code debug loop: initialize/launch/configurationDone, break/step
(next/stepIn/stepOut/continue), stack frames/scopes/variables, evaluate,
setVariable, disconnect/terminate.

### Breakpoints

| Feature | ID |
|---|---|
| Source breakpoints | `dap.breakpoints.basic` |
| Hit-count breakpoints | `dap.breakpoints.hit_condition` |
| Logpoints | `dap.breakpoints.logpoints` |
| Function breakpoints | `dap.breakpoints.function` |
| Watchpoints (data breakpoints) | `dap.watchpoints` |

### Other DAP

| Feature | ID |
|---|---|
| Exception breakpoints (`die`, `warn`) | `dap.exceptions.die` / `dap.exceptions.warn` |
| Inline variable values | `dap.inline_values` |
| Debug console autocomplete | `dap.completions` |
| Loaded modules view | `dap.modules` |
| Thread listing | `dap.threads` |
| Pause execution | `dap.pause` |

## Perl-Specific Extensions

| Feature | ID | Notes |
|---|---|---|
| Inline AI completions | `lsp.inline_completion` | @proposed |
| Streaming inline AI completions | `experimental.perlInlineCompletionStream` | `textDocument/perlInlineCompletionStream` via `$/progress` |

## Refresh Cycle

The server can ask the client to discard its cache for:
`code_lens_refresh`, `semantic_tokens_refresh`, `inlay_hint_refresh`,
`inline_value_refresh`, `diagnostic_refresh`, `folding_range_refresh` (@proposed).

## Text Synchronization

| Feature | ID |
|---|---|
| Open/change/close | `lsp.text_document_sync` |
| Save | `lsp.did_save` |
| willSave | `lsp.will_save` |
| willSaveWaitUntil (format on save) | `lsp.will_save_wait_until` |

---

For editor setup and configuration, see [CONFIGURATION.md](CONFIGURATION.md).
For the complete Perl parser feature list, see [FEATURES.md](FEATURES.md).

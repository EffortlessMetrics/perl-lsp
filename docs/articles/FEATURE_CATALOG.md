# perl-lsp Feature Catalog

perl-lsp implements 116 LSP and DAP features, all at GA maturity, tracked in [`features.toml`](../../features.toml). This catalog describes what each capability area does and why it matters for Perl development.

This is not a marketing summary — it is a structured tour of the actual implementation, grouped by what you use it for.

---

## Navigation

The navigation features answer the question: "where is this thing, and what else is related to it?"

**Go to Definition** (`lsp.definition`) — jump from a subroutine call, variable reference, or module use to its definition. Works across files in the workspace. When you see `$obj->render()`, you can navigate directly to the `render` method.

**Go to Declaration** (`lsp.declaration`) — for Perl this is particularly useful with forward declarations (prototypes) and `our` variable declarations, where the declaration site differs from the definition site.

**Go to Type Definition** (`lsp.type_definition`) — navigates from an object or variable to the package that defines its type. Useful in codebases using Moose, Moo, or any OO framework.

**Go to Implementation** (`lsp.implementation`) — in an OO codebase, navigates from a method in a base class or role to its concrete implementations in derived classes.

**Find References** (`lsp.references`) — locate every usage of a symbol across the workspace. Works for subroutines, methods, variables, and modules. Results are grouped by file.

**Document Symbols** (`lsp.document_symbol`) — shows the symbol tree for the current file: subroutines, packages, variables, and their nesting. Editors use this to populate the breadcrumb bar and file outline.

**Workspace Symbols** (`lsp.workspace_symbol`) / **Workspace Symbol Resolve** (`lsp.workspace_symbol_resolve`) — search for any symbol across the entire workspace. Useful in large multi-package codebases. `resolve` fetches additional details (documentation, location) for a symbol result.

**Call Hierarchy** (`lsp.call_hierarchy`) — shows which functions call a given function (incoming calls) and which functions a given function calls (outgoing calls). Helps understand code flow before refactoring.

**Type Hierarchy** (`lsp.type_hierarchy`) — shows the inheritance chain for a package: parent classes, child classes, and roles/mixins. Useful in Moose/Moo codebases to understand the full class model.

**Document Links** (`lsp.document_link`) / **Document Link Resolve** (`lsp.document_link_resolve`) — turns module references (`use My::Module`) and documentation links into clickable hyperlinks in the editor. `resolve` fetches the actual target URI when clicked.

---

## Completion

Completion features help write code faster and more accurately.

**Code Completion** (`lsp.completion`) / **Completion Item Resolve** (`lsp.completion_item_resolve`) — suggests completions as you type: 150+ built-in Perl functions, workspace symbols (subroutines, methods, variables from all files in the project), module names from the workspace and common CPAN distributions, and keywords. `resolve` fetches the full documentation and detail for a selected completion item.

**Signature Help** (`lsp.signature_help`) — shows the parameter signature for a subroutine as you type a call. When you type `open(`, the signature help shows `open(FILEHANDLE, EXPR)` or the custom signature for workspace-defined subs.

**Inline Completion** (`lsp.inline_completion`) — AI-powered ghost-text completions. Appears as editor ghost text as you type, showing predicted completions based on context. This is separate from the traditional tab-completion list.

---

## Diagnostics

Diagnostics report problems in your code as you edit.

**Published Diagnostics** (`lsp.publish_diagnostics`) — the traditional push model: the server sends diagnostics when a file is opened or changed. Covers parse errors, undefined subroutine calls, variable scope problems, and other static analysis findings.

**Pull Diagnostics** (`lsp.pull_diagnostics`) / **Workspace Diagnostics** (`lsp.workspace_diagnostics`) — the LSP 3.17 pull model: the editor requests diagnostics on demand, enabling richer diagnostics (including workspace-wide analysis) without re-publishing on every keystroke. `workspace_diagnostics` extends this to the entire workspace simultaneously.

**Diagnostic Refresh** (`lsp.diagnostic_refresh`) — when the server needs to push all clients to refresh their diagnostics (after a configuration change or workspace rescan), this server-to-client request triggers the refresh.

---

## Hover and Information

**Hover** (`lsp.hover`) — shows documentation and type information when you hover over a symbol. For built-in functions, shows the Perl documentation. For workspace symbols, shows the subroutine signature and any POD documentation found in the file. For modules, shows the module's documentation summary.

**Inlay Hints** (`lsp.inlay_hint`) / **Inlay Hint Resolve** (`lsp.inlay_hint_resolve`) — shows parameter names and type annotations inline in the editor, without cluttering the source code. When you call `process($user, $role, $timeout)`, inlay hints can display the parameter names `user:`, `role:`, `timeout:` next to the arguments. `resolve` fetches the label location for a hint when clicked.

**Inline Values** (`lsp.inline_value`) — during debugging, shows the current value of variables inline in the editor, next to the relevant code. Distinct from the debug panel — the values appear directly in the source view.

**Document Highlights** (`lsp.document_highlight`) — highlights all occurrences of the symbol under the cursor within the current file. Faster than find-references for the "show me all the places this variable appears locally" use case.

---

## Editing and Refactoring

**Rename** (`lsp.rename`) / **Prepare Rename** (`lsp.prepare_rename`) — renames a symbol across all files in the workspace. `prepare_rename` validates that the rename is safe and computes the placeholder text shown in the rename dialog before the rename is applied.

**Code Actions** (`lsp.code_action`) / **Code Action Resolve** (`lsp.code_action_resolve`) — context-sensitive suggestions for fixing or improving code. Examples: organize imports, add missing `use` statements, modernize old Perl idioms, apply quick fixes for diagnostics. `resolve` fetches the full edit for a code action when selected.

**Formatting** (`lsp.formatting`) — formats the entire document using Perl::Tidy. Produces consistently formatted Perl code.

**Range Formatting** (`lsp.range_formatting`) — formats a selected range rather than the entire file.

**Multi-Range Formatting** (`lsp.ranges_formatting`) — formats multiple non-contiguous ranges in a single operation (LSP 3.18 `textDocument/rangesFormatting`).

**On-Type Formatting** (`lsp.on_type_formatting`) — applies auto-formatting as you type. Handles auto-indentation after entering `{`, `;`, or `\n`.

**Linked Editing** (`lsp.linked_editing_range`) — simultaneously edits all occurrences of a token when one is changed. Useful for editing matching delimiters or synchronized variable names.

**Folding Ranges** (`lsp.folding_range`) — defines the foldable regions in a file: subroutines, blocks, heredocs, comment regions. Editors use this for code folding (collapse/expand sections).

**Selection Range** (`lsp.selection_range`) — smart selection expansion: start with the cursor, then expand the selection to progressively larger syntactic units (identifier, expression, statement, block, subroutine, file). Triggered by the "Expand Selection" editor command.

---

## Semantic Analysis

**Semantic Tokens** (`lsp.semantic_tokens`) / **Semantic Tokens Refresh** (`lsp.semantic_tokens_refresh`) — provides context-aware syntax highlighting beyond what TextMate grammars can express. Semantic tokens know whether `$foo` is a local variable, a package global, or a parameter — and color them differently. `refresh` requests the client to re-fetch tokens after a server-side change.

**Code Lens** (`lsp.code_lens`) / **Code Lens Resolve** (`lsp.code_lens_resolve`) / **Code Lens Refresh** (`lsp.code_lens_refresh`) — shows actionable annotations above code elements. Displays reference counts above subroutine definitions ("12 references"), test counts, and other contextual information. `resolve` fetches the command for a lens. `refresh` requests the client to re-fetch all lenses.

**Document Color** (`lsp.document_color`) / **Color Presentation** (`lsp.color_presentation`) — detects color literals in Perl code (hex strings used as CSS values, `Imager` color constants, etc.) and shows a color picker. `color_presentation` provides the format string when a color is changed via the picker.

**Moniker** (`lsp.moniker`) — assigns stable, cross-project identities to symbols. Used by code intelligence platforms (like Sourcegraph or GitHub's code navigation) to link symbols across repositories.

---

## Text Synchronization

These features maintain the server's view of open files in sync with the editor.

**Text Document Sync** (`lsp.text_document_sync`) — `didOpen`, `didChange`, `didClose` notifications. The server maintains a live copy of every open file and updates it as you edit.

**Did Save** (`lsp.did_save`) — notifies the server when a file is saved. Can trigger a workspace rescan or diagnostics refresh.

**Will Save** (`lsp.will_save`) — notifies the server just before a file is saved. Allows the server to intercept and add pre-save behavior.

**Will Save Wait Until** (`lsp.will_save_wait_until`) — a synchronous variant: the editor waits for the server to respond before completing the save. Used for format-on-save (the server returns the formatted edits, which the editor applies before writing).

---

## Workspace Management

**Workspace Folders** (`lsp.workspace_folders`) — supports multi-root workspaces: a single editor window with multiple Perl project roots, each with its own configuration and indexing scope.

**File Operations** (`lsp.file_operations`, `lsp.will_create_files`, `lsp.did_create_files`, `lsp.will_rename_files`, `lsp.did_rename_files`, `lsp.will_delete_files`, `lsp.did_delete_files`) — tracks file system operations. When you rename or delete a file, the server updates cross-file references, symbol tables, and cached indexes.

**Workspace Edit** (`lsp.workspace_edit`) — applies multi-file edits. Used by rename and other refactoring operations that touch multiple files.

**Execute Command** (`lsp.execute_command`) — runs server-side commands from the editor. Used for invoking Perl::Critic on the current file, running custom Perl tools, and triggering workspace operations that do not fit the standard request/response model.

**Configuration** (`lsp.configuration`) / **Did Change Configuration** (`lsp.did_change_configuration`) / **Did Change Watched Files** (`lsp.did_change_watched_files`) — manages server configuration. The server can request specific configuration from the client. The client notifies the server when configuration or watched files change.

**Apply Edit** (`lsp.apply_edit`) — the server requests the client to apply a set of edits. Used for refactoring operations initiated by the server.

**Text Document Content** (`lsp.text_document_content`) / **Text Document Content Refresh** (`lsp.text_document_content_refresh`) — LSP 3.18 virtual file support. Provides content for URIs that are not backed by filesystem files (generated code, decompiled modules, documentation). `refresh` requests the client to re-fetch virtual content.

---

## Protocol and Lifecycle

**Initialize** (`lsp.initialize`) / **Initialized** (`lsp.initialized`) — the startup handshake. The client sends capabilities, the server responds with its own capabilities and initial configuration. `initialized` completes the handshake.

**Shutdown** (`lsp.shutdown`) / **Exit** (`lsp.exit`) — graceful server termination. `shutdown` requests a clean shutdown; `exit` is the final notification. The server exits with code 0 after a clean shutdown, 1 otherwise.

**Cancel Request** (`lsp.cancel_request`) — cancels an in-flight request. Used for expensive operations (workspace symbol search, full diagnostics) that the user aborts by navigating away.

**Client Register Capability** (`lsp.client_register_capability`) / **Client Unregister Capability** (`lsp.client_unregister_capability`) — dynamic capability registration. The server registers new capabilities (or withdraws them) after initialization, without requiring a restart.

**Set Trace** (`lsp.set_trace`) / **Log Trace** (`lsp.log_trace`) — runtime trace level control. The client sets the server's trace verbosity; the server emits trace messages at the configured level.

---

## Window and Notifications

**Show Message** (`lsp.show_message`) — sends a notification message to the editor (info, warning, error level). Used for surfacing important server-side events.

**Show Message Request** (`lsp.show_message_request`) — sends a message with interactive choices and waits for the user to respond. Used for prompts like "File was modified outside the editor — reload?"

**Show Document** (`lsp.show_document`) — asks the client to open and display a URI. Used to navigate to documentation or related files.

**Log Message** (`lsp.log_message`) — sends a log entry to the editor's output channel. Not shown to the user directly; used for diagnostic logging.

**Work Done Progress** (`lsp.work_done_progress`) / **Work Done Progress Create** (`lsp.work_done_progress_create`) / **Work Done Progress Cancel** (`lsp.work_done_progress_cancel`) / **Progress** (`lsp.progress`) — progress reporting for long-running operations (workspace indexing, corpus analysis). The server reports progress; the editor displays a progress indicator. `create` and `cancel` handle server-initiated progress tokens.

**Telemetry Event** (`lsp.telemetry_event`) — structured telemetry from the server to the client. Used for aggregating usage data (no personally identifying information).

---

## Notebook Support

**Notebook Document Sync** (`lsp.notebook_document_sync`) — synchronizes notebook documents (Jupyter-style) with the server. Handles `didOpen`, `didChange`, `didSave`, and `didClose` for notebook cells containing Perl code.

**Notebook Cell Execution** (`lsp.notebook_cell_execution`) — tracks execution summary metadata for notebook cells. The LSP server does not execute cells; this feature tracks the execution state as reported by the notebook kernel.

---

## Debugging (DAP)

perl-lsp includes a built-in Debug Adapter Protocol server in `crates/perl-dap/`. This is separate from the LSP server but ships in the same binary.

**Core Debug Loop** (`dap.core`) — the complete VS Code debug lifecycle: initialize, launch/attach, configuration done, break, step (in/over/out), stack frames, scopes, variables, evaluate expressions, set variable, disconnect, and terminate. This is the foundation that everything else builds on.

**Source Breakpoints** (`dap.breakpoints.basic`) — set and clear breakpoints by file and line number. Breakpoints are verified (confirmed to be on executable lines) or unverified (pending, e.g., in code not yet loaded). Breakpoints have replace semantics: setting breakpoints for a file replaces all previous breakpoints in that file.

**Hit-Count Breakpoints** (`dap.breakpoints.hit_condition`) — breakpoints that only trigger after being hit a specified number of times, or on every Nth hit. Useful for debugging loops where the problem occurs on the 100th iteration.

**Logpoints** (`dap.breakpoints.logpoints`) — breakpoints that emit a log message and continue execution without stopping. Used to add temporary logging to production code without modifying source files.

**Exception Breakpoints: die** (`dap.exceptions.die`) — break when Perl throws an exception via `die`. Configures which exception categories to break on (all uncaught, specific patterns).

**Exception Breakpoints: warn** (`dap.exceptions.warn`) — break when Perl emits a warning via `warn`, `carp`, or `cluck`.

**Inline Values** (`dap.inline_values`) — shows variable values inline in the source editor during a debugging session (native adapter only).

**Debug Console Completions** (`dap.completions`) — autocomplete in the debug console. Suggests Perl keywords, variables in the current scope, and loaded module names.

**Modules View** (`dap.modules`) — shows all currently loaded modules via `%INC` inspection. Useful for understanding which version of a module was loaded.

**Watchpoints** (`dap.watchpoints`) — data breakpoints that trigger when a variable's value changes. Implemented via the Perl debugger's `w` and `W` commands.

---

## Feature Count

116 features: 87 LSP features across text document, workspace, window, notebook, and protocol areas, plus 24 DAP debug features and 5 perl-lsp extension features. All are at GA maturity. The canonical list is in [`features.toml`](../../features.toml). The DAP subsystem's 14 previously uncatalogued handlers were added in PR #4107 after a direct audit of `dispatch.rs`.

---

*All features verified against `features.toml` version 0.12.3. Feature descriptions derived from `features.toml` descriptions and test files.*

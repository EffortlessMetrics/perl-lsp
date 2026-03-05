# perl-lsp-import-management

Standalone SRP microcrate for Perl import statement utilities used by LSP code actions.

Responsibilities:
- map common function names to likely module imports
- collect import lines from source text
- categorize and sort imports deterministically
- find source-range boundaries for import blocks

# perl-lsp-completion-items

Data model and sorting utilities used by Perl LSP completion providers.

This crate isolates completion item representation and deterministic
`deduplicate_and_sort` behavior so completion generation can remain focused on
context extraction.

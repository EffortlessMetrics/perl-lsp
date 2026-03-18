# perl-lsp-perltidy

Standalone SRP microcrate for Perl formatting via `perltidy`.

This crate owns one responsibility: translating formatter configuration into
`perltidy` command-line invocations, caching formatted output, and providing a
small built-in formatter fallback for environments where `perltidy` is not
available.

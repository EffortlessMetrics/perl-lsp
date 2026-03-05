# perl-lsp-parallel

Parallel worker-pool processing microcrate for Perl LSP workloads.

This crate owns one responsibility: distribute a list of file-like work items
across a bounded number of worker threads and collect results.

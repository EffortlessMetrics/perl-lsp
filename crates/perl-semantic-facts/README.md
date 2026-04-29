# perl-semantic-facts

`perl-semantic-facts` defines a neutral semantic fact vocabulary shared across analysis layers.

This crate intentionally does **not** parse Perl, store workspace indexes, or implement LSP
providers. It exists to provide typed IDs and interoperable fact structures that can be
produced and consumed by other crates.

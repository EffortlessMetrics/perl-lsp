# perl-semantic-facts

`perl-semantic-facts` defines a neutral, serializable vocabulary for semantic facts produced by
Perl analysis layers.

This crate intentionally does **not**:
- parse Perl,
- provide LSP handlers,
- or own workspace storage/indexing.

It provides stable typed IDs, fact records, provenance metadata, and confidence levels that can be
shared between parser/semantic/indexing/provider components.

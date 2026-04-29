# perl-semantic-facts

`perl-semantic-facts` defines a neutral, serializable vocabulary for semantic analysis facts in the `perl-lsp` workspace.

It provides:

- Typed, stable IDs (`FileId`, `EntityId`, `OccurrenceId`, etc.)
- Core fact records (`AnchorFact`, `EntityFact`, `OccurrenceFact`, `EdgeFact`, `DiagnosticFact`)
- Relationship and classification enums (`EntityKind`, `OccurrenceKind`, `EdgeKind`)
- Provenance and confidence metadata (`Provenance`, `Confidence`)

This crate intentionally does **not**:

- parse Perl,
- provide LSP protocol/provider behavior, or
- own workspace storage/indexing.

It is intended as a shared interchange layer between parser/semantic/indexer/exporter components.

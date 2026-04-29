# perl-semantic-facts

`perl-semantic-facts` defines a neutral, serializable semantic fact vocabulary shared
across parsing, semantic analysis, and workspace indexing layers.

This crate intentionally does **not**:
- parse Perl source,
- provide LSP behavior,
- own workspace storage/index persistence.

It provides stable typed identifiers, facts, confidence/provenance metadata, and enums
for entity/occurrence/edge classification.

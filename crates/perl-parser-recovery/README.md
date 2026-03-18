# perl-parser-recovery

Recovery-oriented parser infrastructure for the Perl parser workspace.

This crate extracts the recovery-specific parser context and `RecoveryParser`
implementation out of `perl-parser-core` so they can evolve independently while
remaining available through `perl-parser-core` and `perl-parser` compatibility
re-exports.

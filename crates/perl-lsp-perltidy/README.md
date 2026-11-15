# perl-lsp-perltidy

Compatibility facade retained for downstream migration.

- Use [`perl-tidy`](../perl-tidy/) for the Rust-native formatter.
- Use [`perl-tidy-perltidy`](../perl-tidy-perltidy/) only for explicit
  external `Perl::Tidy` compatibility.

The historical root exports and `perl_lsp_perltidy::native` path remain
available in this release. Removal is gated on downstream migration rather
than an unconditional date.

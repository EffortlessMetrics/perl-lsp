# perl-test-generators

Reusable `proptest` strategies for Perl-focused property tests.

This crate provides generators for:

- Perl variables
- Perl module paths
- Unicode strings for UTF-8 / UTF-16 roundtrip tests
- Non-empty Unicode strings for call sites that require content

See [docs/reference/PROPERTY_TESTING.md](../../docs/reference/PROPERTY_TESTING.md)
for usage patterns and guidance.

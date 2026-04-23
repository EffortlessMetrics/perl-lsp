# Fuzz Testing Reproduction Cases

This directory contains minimal safe reproduction cases discovered during bounded fuzz testing of the tree-sitter-perl parsing ecosystem.

## Contents

- `crash-*` files: Minimal inputs that trigger parser crashes or undefined behavior
- Each file represents a specific edge case that requires investigation and fixes

## Test Integration

These cases are automatically included in the regression test suite to prevent regressions after fixes are applied.

## Security Context

These reproduction cases may represent security vulnerabilities in parsing logic. Handle with appropriate security practices and do not execute untrusted fuzz inputs directly in production environments.
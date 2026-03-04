# perl-workspace-skip

Canonical directory skip rules shared by workspace discovery and LSP workspace tooling.

This crate centralizes checks like `.git`, `target`, `node_modules`, and `.cache` so
workspace walkers and git-output filters remain consistent.

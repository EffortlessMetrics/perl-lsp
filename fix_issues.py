#!/usr/bin/env python3
import re

# Fix references.rs - need to fix the double Some issue
ref_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/references.rs'
with open(ref_path, 'r') as f:
    lines = f.readlines()

new_lines = []
for i, line in enumerate(lines):
    # Line 43 has "Some(Some(reference.kind))," but should be "Some(reference.kind),"
    if 'Some(Some(reference.kind)),' in line:
        new_lines.append(line.replace('Some(Some(reference.kind)),', 'Some(reference.kind),\n'))
    # Line 74 has "Some(Some(reference.kind))," but should be "Some(reference.kind),"
    elif 'Some(Some(reference.kind)),' in line:
        new_lines.append(line.replace('Some(Some(reference.kind)),', 'Some(reference.kind),\n'))
    else:
        new_lines.append(line)

with open(ref_path, 'w') as f:
    f.writelines(new_lines)
print("references.rs fixed")

# Fix mod.rs - need to revert the incorrect matches! replacement
mod_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/mod.rs'
with open(mod_path, 'r') as f:
    content = f.read()

# The issue is that matches!(s.kind, Some(crate::symbol::SymbolKind::Subroutine)) is WRONG
# s.kind is a SymbolKind, not an Option<SymbolKind>
content = content.replace(
    'matches!(s.kind, Some(crate::symbol::SymbolKind::Subroutine))',
    'matches!(s.kind, crate::symbol::SymbolKind::Subroutine)'
)

with open(mod_path, 'w') as f:
    f.write(content)
print("mod.rs fixed")

print("\nFixes applied!")

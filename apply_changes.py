#!/usr/bin/env python3
import sys

# Read and update symbol.rs
symbol_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/symbol.rs'
with open(symbol_path, 'r') as f:
    content = f.read()

old_sig = 'pub fn find_symbol(&self, name: &str, from_scope: ScopeId, kind: SymbolKind) -> Vec<&Symbol> {'
new_sig = 'pub fn find_symbol(&self, name: &str, from_scope: ScopeId, kind_filter: Option<SymbolKind>) -> Vec<&Symbol> {'

if old_sig in content:
    content = content.replace(old_sig, new_sig)
    content = content.replace(
        'if symbol.scope_id == scope_id && symbol.kind == kind {',
        'if symbol.scope_id == scope_id && kind_filter.map_or(true, |k| symbol.kind == k) {'
    )
    content = content.replace(
        'if symbol.declaration.as_deref() == Some("our") && symbol.kind == kind {',
        'if symbol.declaration.as_deref() == Some("our") && kind_filter.map_or(true, |k| symbol.kind == k) {'
    )
    with open(symbol_path, 'w') as f:
        f.write(content)
    print("symbol.rs updated")
else:
    print(f"ERROR: Could not find old signature in symbol.rs")
    print("Looking for:", old_sig)
    sys.exit(1)

# Read and update references.rs
ref_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/references.rs'
with open(ref_path, 'r') as f:
    content = f.read()

# Fix duplicate Some issue first
content = content.replace('Some(Some(reference.kind))', 'Some(reference.kind)')

# Now fix remaining reference.kind to Some(reference.kind)
content = content.replace('reference.kind,', 'Some(reference.kind),')
content = content.replace('reference.kind)', 'Some(reference.kind))')

with open(ref_path, 'w') as f:
    f.write(content)
print("references.rs updated")

# Read and update mod.rs
mod_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/mod.rs'
with open(mod_path, 'r') as f:
    content = f.read()

content = content.replace(
    'crate::symbol::SymbolKind::Subroutine)',
    'Some(crate::symbol::SymbolKind::Subroutine))'
)

with open(mod_path, 'w') as f:
    f.write(content)
print("mod.rs updated")

# Read and update comprehensive_unit_tests.rs
test_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/tests/comprehensive_unit_tests.rs'
with open(test_path, 'r') as f:
    content = f.read()

content = content.replace(
    'SymbolKind::scalar())',
    'Some(SymbolKind::scalar()))'
)
content = content.replace(
    'SymbolKind::Subroutine)',
    'Some(SymbolKind::Subroutine))'
)

with open(test_path, 'w') as f:
    f.write(content)
print("comprehensive_unit_tests.rs updated")

# Read and update node_analysis.rs
node_path = '/home/hermes/repos/perl-lsp/crates/perl-semantic-analyzer/src/analysis/semantic/node_analysis.rs'
with open(node_path, 'r') as f:
    content = f.read()

# Fix find_symbol calls
content = content.replace(
    'self.symbol_table.find_symbol(name, scope_id, kind)',
    'self.symbol_table.find_symbol(name, scope_id, Some(kind))'
)
content = content.replace(
    'self.symbol_table.find_symbol(name, 0, kind)',
    'self.symbol_table.find_symbol(name, 0, Some(kind))'
)
content = content.replace(
    'SymbolKind::Subroutine)',
    'Some(SymbolKind::Subroutine))'
)

with open(node_path, 'w') as f:
    f.write(content)
print("node_analysis.rs updated")

print("\nAll changes applied!")

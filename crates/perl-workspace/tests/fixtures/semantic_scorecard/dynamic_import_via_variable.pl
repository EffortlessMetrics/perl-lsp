package DynamicImportViaVariable;

# Pattern: $module->import(qw(foo))
# The module variable holds the module name, then import is called dynamically.
# 'foo' is "plausibly imported" — not exactly known. Suppress diagnostic.
my $module = "Some::Module";
require $module;
$module->import(qw(foo));

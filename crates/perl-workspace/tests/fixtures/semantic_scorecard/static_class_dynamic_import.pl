package StaticClassDynamicImport;

# Pattern: Foo->import(@names)
# Static class name but dynamic symbol list — symbols not exactly known.
# 'bar' is plausibly imported but cannot be exactly verified.
use POSIX ();
POSIX->import(@POSIX::EXPORT_OK);

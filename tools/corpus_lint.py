#!/usr/bin/env python3
"""
Lint the tree-sitter-perl test corpus for consistency and correctness.
"""
import re
import sys
import pathlib
import json
import collections

ROOT = pathlib.Path(__file__).resolve().parents[1] / "test" / "corpus"
SEC_RE = re.compile(r"^=+\s*$")
ID_RE = re.compile(r"^[a-z0-9._-]+$")
META_RE = re.compile(r"^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$")

# Known valid tags (can be extended)
KNOWN_TAGS = {
    # Core language features
    "scalar", "array", "hash", "reference", "typeglob", "filehandle",
    "string", "number", "undef", "boolean", "list", "slice",
    
    # Operators
    "operator", "assignment", "arithmetic", "comparison", "logical",
    "bitwise", "string-ops", "range", "flipflop", "smartmatch",
    "ternary", "comma", "fat-comma", "augmented-assignment",
    
    # Regex
    "regex", "regex-code", "anchor", "pos", "\\G", "substitution",
    "transliteration", "tr", "split", "join", "qr", "match",
    "sets", "branch-reset", "named-capture", "capture", "lookahead",
    "lookbehind", "conditional", "recursive", "verbs", "modifiers",
    
    # Quotes and strings
    "qw", "qq", "qx", "heredoc", "interpolation", "escape",
    "quoting", "backticks", "command",
    
    # Control flow
    "flow", "if", "unless", "while", "until", "for", "foreach",
    "given", "when", "default", "labels", "next", "last", "redo",
    "continue", "goto", "die", "eval", "return", "yield",
    
    # Subroutines
    "sub", "subroutine", "prototype", "signature", "attributes",
    "anonymous", "closure", "lexical", "state", "our", "my", "local",
    
    # OO and packages
    "package", "class", "method", "field", "bless", "isa", "can",
    "DOES", "VERSION", "import", "export", "use", "require", "no",
    "version", "vstring", "module", "pragma", "mro", "SUPER",
    
    # Built-ins
    "builtin", "function", "print", "say", "open", "close", "read",
    "write", "seek", "tell", "stat", "file-test", "directory",
    "system", "exec", "fork", "wait", "pipe", "socketpair", "kill",
    "alarm", "sleep", "time", "localtime", "gmtime", "caller",
    "wantarray", "defined", "exists", "delete", "keys", "values",
    "each", "sort", "reverse", "map", "grep", "pack", "unpack",
    "vec", "tie", "untie", "tied", "dbm", "sysread", "syswrite",
    
    # Special vars
    "special-var", "magic", "punctuation-var", "English", "argv",
    "env", "inc", "sig", "error", "errno", "child-error", "eval-error",
    
    # Features and pragmas
    "feature", "experimental", "pragma", "strict", "warnings",
    "utf8", "bytes", "integer", "constant", "overload", "sigtrap",
    "locale", "encoding", "charnames", "unicode", "bignum", "bigint",
    
    # I/O and IPC
    "io", "perlio", "layers", "encoding", "binmode", "select",
    "dup", "fd", "stdio", "ipc", "shared", "threads", "lock",
    
    # Data structures
    "autovivification", "circular-ref", "weak-ref", "magic-increment",
    "magic-decrement", "dereference", "postfix-deref", "circumfix-deref",
    
    # Declarations
    "declaration", "format", "data", "end", "begin", "check", "init",
    "unitcheck", "pod", "documentation", "comment",
    
    # Contexts
    "scalar-context", "list-context", "void-context", "boolean-context",
    "string-context", "numeric-context", "integer-context",
    
    # Other
    "do", "block", "expression", "statement", "modifier", "indirect",
    "argv0", "constants", "__FILE__", "__LINE__", "__PACKAGE__",
    "__SUB__", "__DIR__", "debugger", "B", "Devel", "XS",
    
    # Experimental/modern
    "signatures", "postderef", "lexical_subs", "refaliasing",
    "declared_refs", "regex_sets", "const_attr", "try", "catch",
    "finally", "defer", "async", "await", "match", "case",
    
    # Flags (not tags but included for completeness)
    "lexer-sensitive", "ambiguous", "error-node-expected",
    "version-gated", "experimental", "slow", "tripwire",
    
    # Additional specific items
    "CORE", "CORE::GLOBAL", "bareword", "symbolic-ref",
    "re-eval", "code-block", "embedded-code", "phases"
}

def lint_file(filepath):
    """Lint a single corpus file."""
    issues = []
    warnings = []
    
    with filepath.open("r", encoding="utf-8") as f:
        lines = f.readlines()
    
    # Check trailing newline
    if lines and not lines[-1].endswith("\n"):
        issues.append(f"Missing trailing newline")
    
    # Count sections
    section_count = 0
    section_ids = []
    i = 0
    
    while i < len(lines):
        if SEC_RE.match(lines[i]):
            section_count += 1
            
            # Get title
            title = lines[i+1].strip() if i+1 < len(lines) else "(untitled)"
            
            # Skip second delimiter
            if i+2 < len(lines) and SEC_RE.match(lines[i+2]):
                i += 3
            else:
                i += 2
            
            # Parse metadata
            meta = {}
            while i < len(lines) and lines[i].startswith("#"):
                m = META_RE.match(lines[i].rstrip())
                if m:
                    meta[m.group("k")] = m.group("v").strip()
                i += 1
            
            # Check @id
            if "id" not in meta:
                issues.append(f"Section '{title}': missing @id")
            else:
                id_val = meta["id"]
                if not ID_RE.match(id_val):
                    issues.append(f"Section '{title}': invalid @id format '{id_val}'")
                section_ids.append(id_val)
            
            # Check tags
            if "tags" in meta:
                tags = [t.strip() for t in meta["tags"].replace(",", " ").split() if t.strip()]
                unknown = [t for t in tags if t not in KNOWN_TAGS]
                if unknown:
                    warnings.append(f"Section '{title}': unknown tags {unknown}")
            
            # Check perl version format
            if "perl" in meta:
                perl = meta["perl"]
                if not re.match(r"^\d+\.\d+\+?$", perl):
                    warnings.append(f"Section '{title}': unusual perl version format '{perl}'")
        else:
            i += 1
    
    # Check section count
    if section_count > 15:
        warnings.append(f"Has {section_count} sections (recommended ≤ 15)")
    elif section_count == 0:
        issues.append("No test sections found")
    
    return issues, warnings, section_ids

def main():
    """Main linting function."""
    print("Linting tree-sitter-perl test corpus...")
    print()
    
    # Find all corpus files
    files = sorted(f for f in ROOT.glob("*.txt") 
                   if not f.name.startswith("_"))
    
    if not files:
        print("ERROR: No corpus files found!", file=sys.stderr)
        return 1
    
    all_issues = []
    all_warnings = []
    all_ids = []
    
    # Lint each file
    for filepath in files:
        issues, warnings, ids = lint_file(filepath)
        
        if issues or warnings:
            print(f"📄 {filepath.name}")
            
        for issue in issues:
            print(f"  ❌ {issue}")
            all_issues.append((filepath.name, issue))
        
        for warning in warnings:
            print(f"  ⚠️  {warning}")
            all_warnings.append((filepath.name, warning))
        
        if issues or warnings:
            print()
        
        all_ids.extend(ids)
    
    # Check for duplicate IDs across all files
    id_counts = collections.Counter(all_ids)
    dupes = [id for id, count in id_counts.items() if count > 1]
    if dupes:
        print("❌ Duplicate IDs found across files:")
        for dupe in dupes:
            print(f"  - {dupe} (appears {id_counts[dupe]} times)")
        all_issues.append(("global", f"Duplicate IDs: {dupes}"))
        print()
    
    # Summary
    print("=" * 50)
    print("Summary:")
    print(f"  Files checked: {len(files)}")
    print(f"  Total sections: {len(all_ids)}")
    print(f"  Unique IDs: {len(set(all_ids))}")
    print(f"  Issues: {len(all_issues)}")
    print(f"  Warnings: {len(all_warnings)}")
    print()
    
    if all_issues:
        print("❌ Linting failed with errors")
        return 1
    elif all_warnings:
        print("⚠️  Linting passed with warnings")
        return 0
    else:
        print("✅ Linting passed!")
        return 0

if __name__ == "__main__":
    sys.exit(main())
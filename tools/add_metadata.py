#!/usr/bin/env python3
"""
Add basic metadata to existing corpus files that don't have it.
"""
import re
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1] / "test" / "corpus"
SEC_RE = re.compile(r"^=+\s*$")
META_RE = re.compile(r"^#\s*@(id|tags|perl|flags):")

# Tag mappings based on file names and content
FILE_TAG_MAP = {
    "builtins-core.txt": ["builtin", "function"],
    "special-vars-magic.txt": ["special-var", "magic"],
    "operators-augassign.txt": ["operator", "augmented-assignment"],
    "cli-env-pod.txt": ["argv", "env", "pod", "documentation"],
    "time-and-caller.txt": ["time", "caller", "wantarray", "eval"],
    "sysproc-and-net.txt": ["system", "exec", "fork", "pipe", "socket"],
    "vec-bitwise-dbm.txt": ["vec", "bitwise", "dbm", "tie"],
    "debugger-b-backend.txt": ["debugger", "B", "Devel"],
    "fuzz-tripwires.txt": ["tripwire", "regex-code", "qw", "indirect"]
}

def add_metadata_to_file(filepath):
    """Add metadata to sections that don't have it."""
    with filepath.open("r", encoding="utf-8") as f:
        lines = f.readlines()
    
    base_tags = FILE_TAG_MAP.get(filepath.name, [])
    modified = False
    new_lines = []
    i = 0
    section_count = 0
    
    while i < len(lines):
        if SEC_RE.match(lines[i]):
            section_count += 1
            # Add section delimiter
            new_lines.append(lines[i])
            
            # Add title
            if i+1 < len(lines):
                title = lines[i+1].strip()
                new_lines.append(lines[i+1])
                i += 2
            else:
                title = "(untitled)"
                i += 1
            
            # Skip second delimiter if exists
            if i < len(lines) and SEC_RE.match(lines[i]):
                new_lines.append(lines[i])
                i += 1
            
            # Check if metadata already exists
            has_metadata = False
            if i < len(lines) and lines[i].startswith("#") and "@" in lines[i]:
                has_metadata = True
            
            if not has_metadata:
                # Generate ID
                file_base = filepath.stem.replace("-", "_").replace(".", "_")
                id_str = f"{file_base}.{section_count:03d}"
                
                # Generate tags based on title
                title_lower = title.lower()
                tags = list(base_tags)
                
                # Add specific tags based on content
                if "math" in title_lower:
                    tags.append("arithmetic")
                if "string" in title_lower:
                    tags.append("string")
                if "list" in title_lower or "array" in title_lower:
                    tags.append("list")
                if "hash" in title_lower:
                    tags.append("hash")
                if "file" in title_lower:
                    tags.append("file-test")
                if "magic" in title_lower or "special" in title_lower:
                    tags.append("punctuation-var")
                if "error" in title_lower:
                    tags.append("error")
                if "time" in title_lower:
                    tags.extend(["localtime", "gmtime"])
                if "regex" in title_lower or "match" in title_lower:
                    tags.append("regex")
                if "pod" in title_lower:
                    tags.append("comment")
                
                # Add metadata
                new_lines.append(f"# @id: {id_str}\n")
                new_lines.append(f"# @tags: {' '.join(sorted(set(tags)))}\n")
                new_lines.append("# @perl: 5.8+\n")
                
                # Add flags if needed
                if "error" in title_lower:
                    new_lines.append("# @flags: error-node-expected\n")
                elif filepath.name == "fuzz-tripwires.txt":
                    new_lines.append("# @flags: lexer-sensitive tripwire\n")
                
                new_lines.append("\n")
                modified = True
            
            # Copy rest of section
            while i < len(lines) and not SEC_RE.match(lines[i]):
                new_lines.append(lines[i])
                i += 1
        else:
            new_lines.append(lines[i])
            i += 1
    
    if modified:
        # Write back
        with filepath.open("w", encoding="utf-8") as f:
            f.writelines(new_lines)
        return True
    return False

def main():
    """Add metadata to all corpus files."""
    files = sorted(f for f in ROOT.glob("*.txt") 
                   if not f.name.startswith("_"))
    
    updated = 0
    for filepath in files:
        if add_metadata_to_file(filepath):
            print(f"✓ Updated {filepath.name}")
            updated += 1
        else:
            print(f"  Skipped {filepath.name} (already has metadata)")
    
    print(f"\nUpdated {updated} files")

if __name__ == "__main__":
    main()
#!/usr/bin/env python3
"""
Generate index and coverage reports for the tree-sitter-perl test corpus.
"""
import re
import json
import sys
import pathlib
import collections

ROOT = pathlib.Path(__file__).resolve().parents[1] / "test" / "corpus"
SEC_RE = re.compile(r"^=+\s*$")
META_RE = re.compile(r"^#\s*@(?P<k>id|tags|perl|flags):\s*(?P<v>.*)$")

def parse_file(filepath):
    """Parse a corpus file and extract sections with metadata."""
    with filepath.open("r", encoding="utf-8") as fh:
        lines = fh.readlines()
    
    sections = []
    i = 0
    while i < len(lines):
        if SEC_RE.match(lines[i]):
            # Found section delimiter
            title = lines[i+1].strip() if i+1 < len(lines) else ""
            
            # Skip second delimiter
            if i+2 < len(lines) and SEC_RE.match(lines[i+2]):
                i += 3
            else:
                i += 2
            
            # Extract metadata
            meta = {}
            while i < len(lines) and lines[i].startswith("#"):
                m = META_RE.match(lines[i].rstrip())
                if m:
                    meta[m.group("k")] = m.group("v").strip()
                i += 1
            
            # Find body start
            body_start = i
            
            # Find next section or EOF
            while i < len(lines) and not SEC_RE.match(lines[i]):
                i += 1
            
            sections.append({
                "file": filepath.name,
                "title": title,
                "meta": meta,
                "body_start": body_start + 1,  # 1-indexed for humans
                "body_end": i
            })
        else:
            i += 1
    
    return sections

def main():
    # Find all .txt files in corpus
    files = sorted(f for f in ROOT.glob("*.txt") 
                   if not f.name.startswith("_"))
    
    all_sections = []
    for f in files:
        all_sections.extend(parse_file(f))
    
    # Validate and normalize
    ids = []
    for s in all_sections:
        if "id" not in s["meta"]:
            print(f"ERROR: Missing @id in {s['file']}: {s['title']}", file=sys.stderr)
            sys.exit(1)
        ids.append(s["meta"]["id"])
    
    # Check for duplicates
    id_counts = collections.Counter(ids)
    dupes = [id for id, count in id_counts.items() if count > 1]
    if dupes:
        print(f"ERROR: Duplicate @id values: {dupes}", file=sys.stderr)
        sys.exit(1)
    
    # Process sections
    for s in all_sections:
        # Parse tags (comma or space separated)
        tags_str = s["meta"].get("tags", "")
        s["tags"] = [t.strip() for t in tags_str.replace(",", " ").split() if t.strip()]
        
        # Extract perl version
        s["perl"] = s["meta"].get("perl", "")
        
        # Parse flags
        flags_str = s["meta"].get("flags", "")
        s["flags"] = [f.strip() for f in flags_str.replace(",", " ").split() if f.strip()]
    
    # Generate index
    index = []
    for s in all_sections:
        index.append({
            "id": s["meta"]["id"],
            "file": s["file"],
            "title": s["title"],
            "tags": s["tags"],
            "perl": s["perl"],
            "flags": s["flags"]
        })
    
    # Write index.json
    index_path = ROOT / "_index.json"
    with index_path.open("w", encoding="utf-8") as f:
        json.dump(index, f, indent=2)
    print(f"✓ Generated {index_path}")
    
    # Generate tags map
    tag_map = collections.defaultdict(list)
    for s in index:
        for tag in s["tags"]:
            tag_map[tag].append(s["id"])
    
    # Write tags.json
    tags_path = ROOT / "_tags.json"
    with tags_path.open("w", encoding="utf-8") as f:
        json.dump(dict(tag_map), f, indent=2, sort_keys=True)
    print(f"✓ Generated {tags_path}")
    
    # Generate coverage summary
    summary = []
    summary.append("# Tree-sitter Perl Corpus Coverage Summary")
    summary.append("")
    summary.append(f"**Total files:** {len(files)}")
    summary.append(f"**Total test cases:** {len(all_sections)}")
    summary.append("")
    
    # By file
    summary.append("## Test cases by file")
    summary.append("")
    file_counts = collections.Counter(s["file"] for s in index)
    for fname, count in sorted(file_counts.items()):
        summary.append(f"- `{fname}`: {count} tests")
    summary.append("")
    
    # By tag
    summary.append("## Test cases by tag")
    summary.append("")
    tag_counts = collections.Counter(tag for s in index for tag in s["tags"])
    for tag, count in sorted(tag_counts.items(), key=lambda x: (-x[1], x[0])):
        summary.append(f"- **{tag}**: {count}")
    summary.append("")
    
    # By flag
    flag_counts = collections.Counter(flag for s in index for flag in s["flags"])
    if flag_counts:
        summary.append("## Test cases by flag")
        summary.append("")
        for flag, count in sorted(flag_counts.items()):
            summary.append(f"- **{flag}**: {count}")
        summary.append("")
    
    # By Perl version
    perl_versions = collections.Counter(s["perl"] for s in index if s["perl"])
    if perl_versions:
        summary.append("## Test cases by Perl version requirement")
        summary.append("")
        for version, count in sorted(perl_versions.items()):
            summary.append(f"- **{version}**: {count}")
        summary.append("")
    
    # Write summary
    summary_path = ROOT / "COVERAGE_SUMMARY.md"
    with summary_path.open("w", encoding="utf-8") as f:
        f.write("\n".join(summary))
    print(f"✓ Generated {summary_path}")
    
    print(f"\nCorpus statistics:")
    print(f"  Files: {len(files)}")
    print(f"  Tests: {len(all_sections)}")
    print(f"  Tags: {len(tag_counts)}")
    print(f"  Flags: {len(flag_counts)}")

if __name__ == "__main__":
    main()
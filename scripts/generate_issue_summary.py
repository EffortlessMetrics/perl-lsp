#!/usr/bin/env python3
import json
import subprocess
import datetime
from datetime import timedelta

def get_issues():
    cmd = [
        "gh", "issue", "list",
        "--limit", "500",
        "--state", "open",
        "--json", "number,title,labels,updatedAt,comments,url,body"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error fetching issues: {result.stderr}")
        return []
    return json.loads(result.stdout)

def parse_date(date_str):
    # Format: 2025-10-02T12:34:56Z
    return datetime.datetime.fromisoformat(date_str.replace('Z', '+00:00'))

def classify_priority(labels):
    label_names = [l['name'] for l in labels]
    if 'priority:critical' in label_names or 'P0' in label_names or 'P0-critical' in label_names:
        return 0, "Critical (P0)"
    if 'priority:high' in label_names or 'P1-high' in label_names:
        return 1, "High (P1)"
    if 'priority:medium' in label_names or 'P2-medium' in label_names:
        return 2, "Medium (P2)"
    if 'priority:low' in label_names or 'P3-low' in label_names:
        return 3, "Low (P3)"
    return 4, "Untagged"

def classify_category(labels, title):
    label_names = [l['name'].lower() for l in labels]
    title_lower = title.lower()
    
    if 'dap' in label_names or 'dap' in title_lower:
        return "DAP"
    if 'parser' in label_names or 'parser' in title_lower:
        return "Parser"
    if 'lsp' in label_names or 'lsp' in title_lower:
        return "LSP"
    if 'testing' in label_names or 'test' in title_lower:
        return "Testing"
    if 'documentation' in label_names or 'docs' in title_lower:
        return "Documentation"
    if 'tooling' in label_names or 'infrastructure' in label_names:
        return "Tooling"
    return "Other"

def generate_report(issues):
    now = datetime.datetime.now(datetime.timezone.utc)
    stale_threshold = now - timedelta(days=90)
    
    # Sort and Group
    issues_by_priority = {0: [], 1: [], 2: [], 3: [], 4: []}
    stale_issues = []
    
    for issue in issues:
        prio_code, _ = classify_priority(issue['labels'])
        issues_by_priority[prio_code].append(issue)
        
        updated_at = parse_date(issue['updatedAt'])
        if updated_at < stale_threshold:
            stale_issues.append(issue)

    # Markdown Generation
    lines = []
    lines.append(f"# GitHub Issues Summary - Perl LSP Repository")
    lines.append(f"**Generated**: {now.strftime('%Y-%m-%d')}")
    lines.append(f"**Total Open Issues**: {len(issues)}")
    lines.append(f"")
    lines.append(f"---")
    lines.append(f"")
    lines.append(f"## 📊 Executive Summary")
    lines.append(f"")
    lines.append(f"The repository currently has **{len(issues)} open issues**.")
    lines.append(f"")
    
    # Priority Sections
    priority_names = {
        0: "🚨 Priority 0 - CRITICAL",
        1: "🔥 Priority High",
        2: "⚙️ Priority Medium",
        3: "🔧 Priority Low",
        4: "📋 Untagged / Triage"
    }
    
    for code in range(5):
        group_issues = issues_by_priority[code]
        if not group_issues:
            continue
            
        lines.append(f"## {priority_names[code]} ({len(group_issues)} issues)")
        lines.append(f"")
        
        # Further group by category
        by_cat = {}
        for issue in group_issues:
            cat = classify_category(issue['labels'], issue['title'])
            if cat not in by_cat:
                by_cat[cat] = []
            by_cat[cat].append(issue)
            
        for cat in sorted(by_cat.keys()):
            lines.append(f"### {cat}")
            for issue in by_cat[cat]:
                updated = parse_date(issue['updatedAt']).strftime('%Y-%m-%d')
                labels = ", ".join([f"`{l['name']}`" for l in issue['labels']])
                lines.append(f"#### #{issue['number']}: {issue['title']}")
                lines.append(f"- **Updated**: {updated}")
                if labels:
                    lines.append(f"- **Labels**: {labels}")
                lines.append(f"- [View Issue]({issue['url']})")
                lines.append(f"")
    
    # Stale Section
    if stale_issues:
        lines.append(f"---")
        lines.append(f"## ⚠️ Stale Issues (>90 days)")
        for issue in stale_issues:
            updated = parse_date(issue['updatedAt']).strftime('%Y-%m-%d')
            lines.append(f"- **#{issue['number']}** {issue['title']} (Last updated: {updated})")
            
    return "\n".join(lines)

if __name__ == "__main__":
    issues = get_issues()
    report = generate_report(issues)
    print(report)

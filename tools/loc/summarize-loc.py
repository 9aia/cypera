#!/usr/bin/env python3
"""Merge per-repo tokei JSON files into one org report.

Usage: summarize-loc.py <results-dir> <org-name>

Reads <results-dir>/<repo>.json produced by count-loc.sh, writes
<results-dir>/org.json and prints a markdown summary table to stdout.
"""
import json
import sys
from pathlib import Path


def stats(v):
    return {
        "files": len(v.get("reports", [])),
        "lines": v["code"] + v["comments"] + v["blanks"],
        "code": v["code"],
        "comments": v["comments"],
        "blanks": v["blanks"],
    }


def main():
    results = Path(sys.argv[1])
    org = sys.argv[2] if len(sys.argv) > 2 else "ktbhealth"

    repos = {}
    languages = {}
    errors = []

    errors_file = results / "errors.jsonl"
    if errors_file.exists():
        for line in errors_file.read_text().splitlines():
            if line.strip():
                errors.append(json.loads(line)["repo"])

    # iterdir + endswith so dotfile repos like ".github" are included.
    for path in sorted(p for p in results.iterdir() if p.name.endswith(".json")):
        if path.name == "org.json":
            continue
        repo = path.stem
        data = json.loads(path.read_text())
        # tokei's "Total" node has empty reports/children — aggregate the
        # per-language entries instead.
        files = sum(len(v.get("reports", [])) for k, v in data.items() if k != "Total")
        t = data["Total"]
        total = {
            "files": files,
            "lines": t["code"] + t["comments"] + t["blanks"],
            "code": t["code"],
            "comments": t["comments"],
            "blanks": t["blanks"],
        }
        repos[repo] = total
        for lang, v in data.items():
            if lang == "Total":
                continue
            s = stats(v)
            agg = languages.setdefault(
                lang, {"files": 0, "lines": 0, "code": 0, "comments": 0, "blanks": 0}
            )
            for k in agg:
                agg[k] += s[k]

    org_total = {"files": 0, "lines": 0, "code": 0, "comments": 0, "blanks": 0}
    for r in repos.values():
        for k in org_total:
            org_total[k] += r[k]

    org_json = {
        "org": org,
        "repos": repos,
        "languages": dict(
            sorted(languages.items(), key=lambda kv: -kv[1]["code"])
        ),
        "total": org_total,
        "errors": errors,
    }
    (results / "org.json").write_text(json.dumps(org_json, indent=2) + "\n")

    out = [
        f"## Lines of code — {org} org (tokei)",
        "",
        "| Repo | Files | Lines | Code |",
        "|---|---:|---:|---:|",
    ]
    for repo in sorted(repos):
        r = repos[repo]
        out.append(f"| {repo} | {r['files']:,} | {r['lines']:,} | {r['code']:,} |")
    t = org_total
    out.append(
        f"| **Total** | **{t['files']:,}** | **{t['lines']:,}** | **{t['code']:,}** |"
    )
    if errors:
        out += ["", f"_Skipped (clone/count failed): {', '.join(errors)}_"]
    print("\n".join(out))


if __name__ == "__main__":
    main()

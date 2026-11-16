#!/usr/bin/env python3
"""Render codebase-metrics docs pages from org.json.

Usage: render-loc-mdx.py <org.json> <locale>=<output.mdx> [<locale>=<output.mdx> ...]

Fully regenerates each output file — they are generated output, never
hand-edit. Strings come from strings.json next to this script; unknown
locales fall back to "en".
"""
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

def table(headers, rows):
    # First column left-aligned, the rest numeric/right-aligned.
    sep = "|" + "|".join(["---"] + ["---:"] * (len(headers) - 1)) + "|"
    lines = ["| " + " | ".join(headers) + " |", sep]
    lines += ["| " + " | ".join(r) + " |" for r in rows]
    return "\n".join(lines)


def js_rows(rows):
    # Compact JSON is a valid JS object/array literal for MDX props.
    return json.dumps(rows, separators=(",", ":"))


def render(data, s, lang):
    t = data["total"]
    updated = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    biggest = max(data["repos"], key=lambda r: data["repos"][r]["code"], default=None)
    repo_rows = [
        [
            f"**{repo}**" if repo == biggest else f"`{repo}`",
            f"{r['files']:,}",
            f"{r['lines']:,}",
            f"{r['code']:,}",
        ]
        for repo, r in sorted(data["repos"].items())
    ]
    repo_rows.append(
        [f"**{s['total']}**", f"**{t['files']:,}**", f"**{t['lines']:,}**", f"**{t['code']:,}**"]
    )

    lang_rows = [
        [
            lang_name,
            f"{v['files']:,}",
            f"{v['lines']:,}",
            f"{v['code']:,}",
            f"{v['comments']:,}",
            f"{v['blanks']:,}",
        ]
        for lang_name, v in data["languages"].items()
    ]

    repo_chart = [
        {
            "name": repo,
            "files": r["files"],
            "lines": r["lines"],
            "code": r["code"],
            "comments": r["comments"],
            "blanks": r["blanks"],
        }
        for repo, r in sorted(data["repos"].items(), key=lambda kv: -kv[1]["code"])
    ]
    lang_chart = [
        {
            "name": lang_name,
            "files": v["files"],
            "lines": v["lines"],
            "code": v["code"],
            "comments": v["comments"],
            "blanks": v["blanks"],
        }
        for lang_name, v in data["languages"].items()
    ]

    parts = [
        "---",
        f'title: "{s["title"]}"',
        f'description: "{s["description"]}"',
        "icon: ChartColumn",
        "---",
        "",
        f"## {s['per_repo']}",
        "",
        f'<LocRepoChart locale="{lang}" data={{{js_rows(repo_chart)}}} />',
        "",
        table([s["col_repo"], s["col_files"], s["col_lines"], s["col_code"]], repo_rows),
        "",
        f"## {s['per_lang']}",
        "",
        f'<LocLangChart locale="{lang}" data={{{js_rows(lang_chart)}}} />',
        "",
        table(
            [s["col_lang"], s["col_files"], s["col_lines"], s["col_code"], s["col_comments"], s["col_blanks"]],
            lang_rows,
        ),
        "",
    ]
    if data.get("errors"):
        parts += [f"**{s['skipped']}:** {', '.join(data['errors'])}", ""]
    parts += [
        f"_{s['updated']} {updated}._",
    ]
    return "\n".join(parts) + "\n"


def main():
    org_json = Path(sys.argv[1])
    strings = json.loads((Path(__file__).parent / "strings.json").read_text())
    data = json.loads(org_json.read_text())

    for spec in sys.argv[2:]:
        locale, _, out = spec.partition("=")
        if not out:
            sys.exit(f"bad output spec {spec!r} — expected <locale>=<path>")
        s = strings.get(locale)
        if s is None:
            print(f"warning: no strings for locale {locale!r}, using en", file=sys.stderr)
            s = strings["en"]
        out_path = Path(out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(render(data, s, locale))
        print(f"wrote {out_path}")


if __name__ == "__main__":
    main()

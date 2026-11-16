#!/usr/bin/env bash
# Count lines of code with tokei.
#
# Two modes:
#   org mode (default): clones each repo in $REPOS from github.com/$ORG
#     and counts it. Used by the loc-org reusable workflow.
#   single mode: when SINGLE_REPO_NAME is set, counts the current
#     directory ($PWD) as that repo name — used by loc-repo where the
#     caller's checkout is already present.
#
# Writes one JSON file per repo plus errors.jsonl into $OUT.
#
# Excludes come from [loc].excludes in cypera.toml at the repo root
# (resolved relative to this script). LOC_CONFIG overrides the path;
# if the file is absent or unreadable, the built-in fallback list is
# used — keep it in sync with cypera.toml.
#
# Env:
#   ORG               GitHub org/user for clones (default: ktbhealth)
#   REPOS             space-separated repo names (org mode only)
#   SINGLE_REPO_NAME  count $PWD instead of cloning (single mode)
#   LOC_READ_TOKEN    optional PAT — required for private repos.
#                     Public repos clone anonymously when unset.
#   LOC_CONFIG        path to cypera.toml (default: <repo>/cypera.toml)
#   OUT               results dir (default: loc-results)
set -euo pipefail

OUT="${OUT:-loc-results}"
ORG="${ORG:-ktbhealth}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOC_CONFIG="${LOC_CONFIG:-$REPO_ROOT/cypera.toml}"

mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
: > "$OUT/errors.jsonl"

# Load excludes from cypera.toml; fall back to the built-in list.
EXCLUDES=()
if [ -f "$LOC_CONFIG" ]; then
  while IFS= read -r pattern; do
    [ -n "$pattern" ] && EXCLUDES+=("$pattern")
  done < <(python3 - "$LOC_CONFIG" <<'PY' 2>/dev/null || true
import sys
try:
    import tomllib
except ModuleNotFoundError:
    try:
        import toml as tomllib  # type: ignore
    except ModuleNotFoundError:
        sys.exit(0)
try:
    with open(sys.argv[1], "rb") as f:
        cfg = tomllib.load(f)
    for e in cfg.get("loc", {}).get("excludes", []):
        print(e)
except Exception:
    pass
PY
)
fi
if [ "${#EXCLUDES[@]}" -eq 0 ]; then
  # Fallback — mirror of [loc].excludes in cypera.toml.
  EXCLUDES=(
    node_modules .git coverage
    dist .output .wrangler
    .alchemy .turbo .vite
    .vercel .next storybook-static
    paraglide drizzle
    route-types.d.ts routeTree.gen.ts
    bun.lock package-lock.json
    yarn.lock pnpm-lock.yaml
  )
fi

TOKEI_ARGS=()
for pattern in "${EXCLUDES[@]}"; do
  TOKEI_ARGS+=(--exclude "$pattern")
done

count_dir() {
  # $1 = source dir, $2 = output json path
  (cd "$1" && tokei "${TOKEI_ARGS[@]}" --output json . > "$2")
}

if [ -n "${SINGLE_REPO_NAME:-}" ]; then
  if count_dir "$PWD" "$OUT/$SINGLE_REPO_NAME.json" 2>"$OUT/$SINGLE_REPO_NAME.tokei.err"; then
    echo "counted $SINGLE_REPO_NAME"
  else
    echo "{\"repo\": \"$SINGLE_REPO_NAME\", \"error\": \"tokei failed\"}" >> "$OUT/errors.jsonl"
    rm -f "$OUT/$SINGLE_REPO_NAME.json"
    exit 1
  fi
  exit 0
fi

REPOS="${REPOS:?REPOS is required in org mode}"

for repo in $REPOS; do
  dir="$OUT/src-$repo"
  if [ -n "${LOC_READ_TOKEN:-}" ]; then
    clone_url="https://x-access-token:${LOC_READ_TOKEN}@github.com/${ORG}/${repo}.git"
  else
    clone_url="https://github.com/${ORG}/${repo}.git"
  fi
  if ! git clone --quiet --depth 1 "$clone_url" "$dir" 2>"$OUT/$repo.clone.err"; then
    echo "{\"repo\": \"$repo\", \"error\": \"clone failed\"}" >> "$OUT/errors.jsonl"
    rm -rf "$dir"
    continue
  fi
  if count_dir "$dir" "$OUT/$repo.json" 2>"$OUT/$repo.tokei.err"; then
    rm -rf "$dir"
  else
    echo "{\"repo\": \"$repo\", \"error\": \"tokei failed\"}" >> "$OUT/errors.jsonl"
    rm -rf "$dir" "$OUT/$repo.json"
  fi
done

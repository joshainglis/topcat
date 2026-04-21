#!/usr/bin/env bash
# Print the pg-dev-buddy milestone manifest as a table.
#
# Reads docs/pg-dev-buddy-plans/milestones.toml. Output columns:
#   id, model, prereqs, title.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$REPO_ROOT/docs/pg-dev-buddy-plans/milestones.toml"

if [[ ! -f "$MANIFEST" ]]; then
  echo "Manifest not found: $MANIFEST" >&2
  exit 66
fi

if ! python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)' 2>/dev/null; then
  echo "python3 >= 3.11 required (for stdlib tomllib). Run inside 'nix develop'." >&2
  exit 69
fi

python3 - "$MANIFEST" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    data = tomllib.load(f)
ms = data["milestone"]
ids = sorted(ms.keys())
id_w   = max(len(i) for i in ids)
mod_w  = max(len(ms[i]["model"]) for i in ids)
prq_w  = max(len(",".join(ms[i].get("prereqs", [])) or "-") for i in ids)
header = f"{'id':<{id_w}}  {'model':<{mod_w}}  {'prereqs':<{prq_w}}  title"
sep    = f"{'-'*id_w}  {'-'*mod_w}  {'-'*prq_w}  -----"
print(header)
print(sep)
for i in ids:
    m = ms[i]
    pq = ",".join(m.get("prereqs", [])) or "-"
    print(f"{i:<{id_w}}  {m['model']:<{mod_w}}  {pq:<{prq_w}}  {m.get('title', '')}")
PY

#!/usr/bin/env bash
# Launch an interactive Claude session preconfigured for one pg-dev-buddy milestone.
#
# Each milestone runs in its own git worktree on its own branch (default
# `pgdb/<id>` off `main`) so multiple milestone sessions can run in parallel
# without stepping on each other. The worktree is created on first run and
# reused on subsequent runs (so --resume works seamlessly).
#
# Reads docs/pg-dev-buddy-plans/milestones.toml for the model assignment and
# prerequisite list, prints a short briefing, sets up the worktree, then
# `exec`s `claude --model <m> --name "pgdb/<id>" <bootstrap-prompt>` with
# the worktree as cwd. Any extra args are forwarded to claude.
#
# Usage: run-milestone.sh <milestone-id> [extra claude args...]
# Examples:
#   run-milestone.sh m07
#   run-milestone.sh m12 --resume
#   run-milestone.sh m11 --permission-mode acceptEdits
#
# Environment overrides:
#   TOPCAT_PGDB_WORKTREE_DIR  — path of the worktree (default: ../topcat-pgdb-<id>)
#   TOPCAT_PGDB_BASE_BRANCH   — base branch when creating a new worktree (default: main)

set -euo pipefail

ID="${1:-}"
shift || true

if [[ -z "$ID" ]]; then
  cat >&2 <<EOF
Usage: $(basename "$0") <milestone-id> [extra claude args...]

Examples:
  $(basename "$0") m07
  $(basename "$0") m12 --resume

Run scripts/pg-dev-buddy/list-milestones.sh to see all ids.
EOF
  exit 64
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$REPO_ROOT/docs/pg-dev-buddy-plans/milestones.toml"

if [[ ! -f "$MANIFEST" ]]; then
  echo "Manifest not found: $MANIFEST" >&2
  exit 66
fi

if ! command -v claude >/dev/null 2>&1; then
  echo "claude CLI not found on PATH" >&2
  exit 69
fi

# Python 3.11+ for tomllib. macOS system python is 3.9 and would fail with
# a confusing ModuleNotFoundError; check up-front and point at the nix shell.
if ! python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 11) else 1)' 2>/dev/null; then
  echo "python3 >= 3.11 required (for stdlib tomllib). Run inside 'nix develop'." >&2
  exit 69
fi

# Read fields one per line so titles with spaces survive. tomllib is in stdlib
# from Python 3.11+ (the topcat dev shell satisfies this). Command substitution
# (not process substitution) so set -e catches a missing milestone id.
RAW="$(python3 - "$MANIFEST" "$ID" <<'PY'
import sys, tomllib
manifest, mid = sys.argv[1], sys.argv[2]
with open(manifest, "rb") as f:
    data = tomllib.load(f)
m = data.get("milestone", {}).get(mid)
if not m:
    sys.exit(f"unknown milestone id: {mid}")
print(m["model"])
print(m["file"])
print(m.get("title", ""))
print(",".join(m.get("prereqs", [])) or "-")
PY
)"
mapfile -t FIELDS <<< "$RAW"

MODEL="${FIELDS[0]}"
FILE="${FIELDS[1]}"
TITLE="${FIELDS[2]}"
PREREQS="${FIELDS[3]}"

PLAN_REL="docs/pg-dev-buddy-plans/$FILE"
PLAN_ABS="$REPO_ROOT/$PLAN_REL"
if [[ ! -f "$PLAN_ABS" ]]; then
  echo "Plan file not found: $PLAN_REL" >&2
  exit 66
fi

# Resolve worktree location and branch.
DEFAULT_WORKTREE="$(cd "$(dirname "$REPO_ROOT")" && pwd)/topcat-pgdb-$ID"
WORKTREE_DIR="${TOPCAT_PGDB_WORKTREE_DIR:-$DEFAULT_WORKTREE}"
# Canonicalize so the macOS /var <-> /private/var symlink (and friends)
# doesn't break the registered-worktree comparison below.
WORKTREE_DIR="$(python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$WORKTREE_DIR")"
BASE_BRANCH="${TOPCAT_PGDB_BASE_BRANCH:-main}"
BRANCH="pgdb/$ID"

# Set up the worktree if it doesn't exist; reuse if it does.
if git -C "$REPO_ROOT" worktree list --porcelain \
   | awk '$1=="worktree"{print $2}' \
   | grep -Fxq "$WORKTREE_DIR"; then
  WORKTREE_STATUS="reused"
elif [[ -e "$WORKTREE_DIR" ]]; then
  echo "Path exists but is not a registered worktree: $WORKTREE_DIR" >&2
  echo "Move or remove it, or set TOPCAT_PGDB_WORKTREE_DIR." >&2
  exit 73
else
  if git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH"; then
    git -C "$REPO_ROOT" worktree add "$WORKTREE_DIR" "$BRANCH" >&2
    WORKTREE_STATUS="created (existing branch $BRANCH)"
  else
    git -C "$REPO_ROOT" worktree add -b "$BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH" >&2
    WORKTREE_STATUS="created (new branch $BRANCH off $BASE_BRANCH)"
  fi
fi

# Advisory check: warn if a prereq's branch hasn't merged into BASE_BRANCH.
# Doesn't block — many workflows merge upstream as PRs land, so the prereq
# may be on main without a `pgdb/<id>` branch existing at all.
PREREQ_WARNINGS=""
if [[ "$PREREQS" != "-" ]]; then
  IFS=',' read -ra PREREQ_LIST <<< "$PREREQS"
  for pq in "${PREREQ_LIST[@]}"; do
    pq_branch="pgdb/$pq"
    if git -C "$REPO_ROOT" show-ref --verify --quiet "refs/heads/$pq_branch"; then
      if ! git -C "$REPO_ROOT" merge-base --is-ancestor "$pq_branch" "$BASE_BRANCH" 2>/dev/null; then
        PREREQ_WARNINGS+="    ⚠ $pq_branch exists but is not merged into $BASE_BRANCH"$'\n'
      fi
    fi
  done
fi

cat <<INFO
─────────────────────────────────────────────────────────────────
  Milestone     : $ID — $TITLE
  Plan          : $PLAN_REL
  Model         : $MODEL
  Prerequisites : $PREREQS
  Worktree      : $WORKTREE_DIR ($WORKTREE_STATUS)
  Branch        : $BRANCH
─────────────────────────────────────────────────────────────────
INFO

if [[ -n "$PREREQ_WARNINGS" ]]; then
  printf "  Prerequisite advisory:\n%s\n" "$PREREQ_WARNINGS"
fi

# Keep the bootstrap prompt small so it stays cacheable. Claude reads the
# plan and architecture files via its own tools.
PROMPT="Work on $PLAN_REL.

The plan file is self-contained and references docs/pg-dev-buddy-architecture.md
for cross-cutting concerns. Read both before proposing changes.

Before writing code:
  1. Confirm every prerequisite milestone listed in the plan has actually landed.
  2. Review the plan's Open Questions and resolve any that block your starting point.
  3. Surface any contract drift between this plan and the upstream artifacts it consumes.

Then proceed phase by phase. Use TaskCreate to track phase progress.

This session is running in a dedicated git worktree on branch '$BRANCH' (off
'$BASE_BRANCH'). Commit milestone work to this branch — do not switch
branches. Do not push and do not open a PR; wait for an explicit instruction
from the user before any push or PR action.

After what would have been the final commit of this milestone — i.e. all phases
green, verification snapshot passing, milestone-completing commit made — run
the /simplify skill against the milestone's changed code. If /simplify produces
edits, commit them as a separate follow-up commit (suggested message:
'refactor($ID): simplify per /simplify pass'). If /simplify produces no edits,
say so and stop — do not create an empty commit."

cd "$WORKTREE_DIR"
exec claude --model "$MODEL" --name "pgdb/$ID" "$@" "$PROMPT"

# M6 — shadow_db lifecycle

> Build the `shadow_main`, `shadow_head`, and `shadow_verify` lifecycle on the
> shadow cluster: template-based creation, SHA-256 input cache, event-trigger
> installation, and the `topcat shadow` CLI. Shadows are the substrate that
> M8+ (differ, migration_generator, migration_verifier) depend on; getting
> lifecycle, caching, and connection discipline right here is a prerequisite
> for correctness and the performance targets further downstream.

## Context

A "shadow" is a throwaway PostgreSQL database on a **shadow cluster** (a
separate cluster URL from the dev db — see architecture §Config surface).
Three roles, per architecture §Shadow database lifecycle:

- `shadow_main` — all committed source files applied to a fresh db. Cached.
- `shadow_head` — working-tree files applied to a fresh db. Not cached
  (working tree changes too frequently).
- `shadow_verify` — a clone of `shadow_main` used by the migration verifier
  (M10) to apply a candidate forward migration and assert it lands at
  `shadow_head`'s structure.

All three are built with `CREATE DATABASE … TEMPLATE` off a cached template
db when possible, making reuse cheap. Every shadow has the `_topcat` event
triggers installed observationally (via M5's install API).

M6 is the first milestone that owns its own throwaway clusterful of
databases. From here onward, every correctness check in M8/M10 runs against
a shadow. Any latency in shadow creation multiplies across the migration
pipeline.

## Prerequisites

This milestone depends concretely on:

- **M1 — `pg/client`**: URL parsing splitting a cluster URL from a db name,
  connection pool with per-db routing, typed errors for "db in use"
  (required for `CREATE DATABASE … TEMPLATE` — see Risks), "database does
  not exist", and connection refused. `shadow_db` calls `pg/client` to
  connect, run `CREATE DATABASE`, and kill residual connections to
  templates before reuse.
- **M2 — `schema_model` + `pg/catalog_reader`**: not directly consumed by
  `shadow_db` itself, but `topcat shadow build head` drives the File DAG
  apply that in turn populates `shadow_head`. M6 exposes the lifecycle; M2
  provides the types the applier passes through.
- **M5 — `pg/event_trigger` install API**: `shadow_db` calls
  `event_trigger::install(&client, Role::Observational)` on every freshly
  created shadow before the first user DDL runs (with the
  `topcat.bootstrapping = 'true'` flag discipline from M1/M5). This is
  the sole public API `shadow_db` needs from M5 — not the trigger body
  itself.

Soft prerequisite: M0 has stubbed `topcat shadow build` and
`topcat shadow clean` returning `not implemented`; M6 fills them in.

## Scope

### In scope

- `src/pg/shadow_db/**`: lifecycle module (template creation, reuse,
  cache, teardown).
- `topcat shadow build [main|head|verify] [--rebuild]` CLI handler.
- `topcat shadow clean [--all]` CLI handler.
- SHA-256 input cache keyed over the sorted file-set (path + content),
  persisted under `shadow_main_cache_dir`.
- Event-trigger install on every shadow immediately after creation.
- `shadow_db_url` wiring: parse cluster URL, create/drop named dbs on it.
- Connection hygiene for `CREATE DATABASE … TEMPLATE` (terminate
  backends on the template before cloning).

### Out of scope

- File DAG apply itself — that already exists for file-granularity concat;
  M6 just invokes it pointed at a freshly created shadow db.
- Parallel subgraph apply — deferred to M20.
- Shadow-cluster provisioning (user's responsibility — topcat fails-closed
  if the cluster URL is unreachable).
- Cache garbage collection heuristics beyond `topcat shadow clean`.
- Rename detection or node_id resolution in shadow builds (M11).

## Deliverables (files and modules)

New module `src/pg/shadow_db/`:

| File | Purpose |
|---|---|
| `mod.rs` | Public API: `build_main`, `build_head`, `build_verify`, `clean`, `clean_all`. Re-exports error types. |
| `lifecycle.rs` | `ShadowRole` enum, orchestration of create → event_trigger install → apply → finalize. Owns the `CREATE DATABASE … TEMPLATE` call and connection-drop discipline. |
| `cache.rs` | `CacheKey` (SHA-256 over sorted `(path, content_hash)` pairs), `CacheIndex` (on-disk manifest in `shadow_main_cache_dir`), lookup/store/evict. |
| `template.rs` | Named-template management: promote a built `shadow_main` to a reusable template db (`_topcat_shadow_tpl_<key>`), clone from template, mark/unmark `datistemplate`. |
| `naming.rs` | Deterministic db-name generation: `_topcat_shadow_main`, `_topcat_shadow_head`, `_topcat_shadow_verify`, `_topcat_shadow_tpl_<short_key>`. Single source of truth so `clean` can sweep reliably. |
| `errors.rs` | Typed errors: `TemplateInUse`, `ClusterUnreachable`, `CacheCorrupt`, `TriggerInstallFailed`, etc. |

CLI wiring:

| File | Change |
|---|---|
| `src/cli/` (new `shadow.rs` group) | Arg structs for `build` and `clean`. Reuses existing global flags. |
| `src/commands/shadow/` (new) | `mod.rs` routing `build` / `clean`; `build.rs` (dispatch on role); `clean.rs`. Follows the `commands/analyze/` and `commands/clean/` modularization pattern. |
| `src/main.rs` | Register `shadow` subcommand. |

Config:

| Change | Details |
|---|---|
| `settings/configs/postgres.rs` | Add `shadow_main_cache_dir: PathBuf` (default `~/.cache/topcat/shadows`). `shadow_db_url` already present from M0. |
| `settings/validation` | Validate `shadow_main_cache_dir` is writable at startup; fall back to a safe default with warning if unset and home dir unresolvable. |

Cache on-disk layout (under `shadow_main_cache_dir`):

```
shadows/
  index.json            # CacheIndex: map<cache_key, { template_db, built_at, pg_version }>
  inputs/
    <cache_key>.txt     # audit trail: sorted list of (path, sha256) inputs that produced this key
```

The actual template db lives **in the shadow cluster**, not on disk;
`index.json` is just the lookup from cache_key to the cluster-side template
name. This keeps cache eviction cheap (`DROP DATABASE`) and survives host
migrations (index rebuilds by scanning `_topcat_shadow_tpl_*` databases).

## Implementation phases

### Phase 1 — Naming, cluster plumbing, clean

Land `naming.rs`, `lifecycle.rs` stubs that only create/drop empty dbs,
and the full `topcat shadow clean` path. This unblocks testing later phases
because clean works before build works.

- `ShadowRole::Main | Head | Verify` enum.
- `create_shadow(role)` / `drop_shadow(role)` — empty dbs for now.
- Connection-termination helper (`pg_terminate_backend` over
  `pg_stat_activity` filtered by datname) used before any `DROP DATABASE`.
- `topcat shadow clean` sweeps by name prefix.
- `topcat shadow clean --all` additionally drops every
  `_topcat_shadow_tpl_*`.

### Phase 2 — Event trigger install + head build

Wire the M5 `event_trigger::install` call into `create_shadow`. Implement
`topcat shadow build head` which runs the File DAG apply against a freshly
created db (no caching).

- After `CREATE DATABASE`, connect to it, install triggers with
  `topcat.bootstrapping = 'true'` inside the install transaction.
- Apply the working-tree file set using the existing File DAG apply path.
- No template reuse for head — always fresh.

### Phase 3 — SHA-256 cache + template reuse for main

Implement `cache.rs` and `template.rs`. `topcat shadow build main` becomes
cache-aware:

1. Compute `cache_key` from sorted `(path, content_sha256)` of committed
   files.
2. Lookup in `index.json`. Hit → `CREATE DATABASE _topcat_shadow_main
   TEMPLATE _topcat_shadow_tpl_<key>`. Miss → fresh build, then promote the
   result to a template db and record in the index.
3. On template promotion: disconnect the builder session, set
   `datistemplate = true` and `datallowconn = false` on the template db.

### Phase 4 — Verify clone + `--rebuild` + polish

- `topcat shadow build verify` clones `shadow_main` (which must exist —
  fail-closed with actionable error pointing at `shadow build main`).
- `--rebuild` flag bypasses cache and rebuilds from scratch, overwriting
  the template entry.
- Concurrency: advisory lock `pg_advisory_lock(hashtext('topcat_shadow_'
  || role))` on the **shadow cluster** (any one db in it) around each
  build so that two `shadow build head` runs serialize rather than
  deadlocking on `DROP DATABASE`.
- Wire up the `topcat shadow` CLI to the routing module.
- Performance measurement: log `shadow.build.duration_ms` with
  `template_reuse=bool` for observability.

## Risks (copied and amplified)

### `CREATE DATABASE … TEMPLATE` requires zero active connections to the template (high impact)

From roadmap §M6. Postgres rejects the clone if any session is connected
to the template. Day-one mitigations:

- `template.rs::clone_from_template` runs
  `SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname =
  $1 AND pid <> pg_backend_pid()` before the `CREATE DATABASE` call.
- Our own `pg/client` pool must drop any pooled connection to a db that
  is about to act as a template. Add a `release_all(datname)` hook on the
  pool invoked by `template.rs` before the clone.
- Set `datallowconn = false` on promoted template dbs — forces accidental
  connects to fail loud rather than silently blocking clones.

### Cache-key churn from `-- node_id:` header rewrites (medium impact)

From roadmap §M6 and architecture §Three-way node_id resolution. M11's
conflict auto-resolve rewrites the loser file's `-- node_id:` line, which
would change the content hash and invalidate the cache post-merge — exactly
when reuse matters most. Day-one mitigation:

- `cache.rs::content_hash_for(path)` reads the file and computes SHA-256
  over content with `-- node_id:` lines **stripped** (line-anchored regex,
  same byte range removed before hashing). Record both the full file hash
  and the stripped hash in `inputs/<cache_key>.txt` for auditability.
- Unit test: two files differing only in `-- node_id:` produce identical
  cache keys; two files differing in any other comment do not.
- This is scoped narrowly to `-- node_id:` (the only header topcat rewrites
  silently). `-- requires:`, `-- layer:`, `-- exists:`, `-- name:` remain
  part of the cache key.

### Performance targets (medium impact)

Architecture §Performance and caching binds:

| Workload | Target |
|---|---|
| Shadow rebuild (template reuse) | ~2s |
| Shadow rebuild (from scratch, 1000 objects) | < 90s |

Day-one mitigations:

- Structured log `shadow.build.duration_ms` emitted on every build so
  regressions are visible from M6 onward (don't wait for M20).
- One `shadow build main` integration test on the 1000-object fixture
  with a `<90s` soft assertion (warn, don't fail) so regressions surface
  in CI without flaking.
- Template reuse path is measured separately; a reuse exceeding 5s is a
  sign that connection-termination is racing or `datistemplate` wasn't
  set — fail the test in that case (hard assertion, <5s).

### Shadow cluster permission surprise (medium impact)

`shadow_db_url` points at a **cluster**, so `CREATE DATABASE` requires
`CREATEDB` or superuser on that cluster role. If absent, every build
fails with an opaque `permission denied`. Day-one: probe at the top of
`build_*` with `SELECT has_database_privilege(current_user, 'template1',
'CONNECT'), rolcreatedb FROM pg_roles WHERE rolname = current_user` and
fail-closed with an actionable message ("role needs CREATEDB on the
shadow cluster").

### Cache corruption / stale template references (low impact)

`index.json` could reference a template db that was manually dropped.
Day-one: on lookup hit, verify the template db exists (`SELECT 1 FROM
pg_database WHERE datname = $1`); missing → treat as miss, rebuild,
overwrite the index entry.

## Tests

Unit:

- `cache.rs`: SHA-256 key is deterministic across input reorders; identical
  input → identical key; `-- node_id:` rewrite doesn't change the key.
- `naming.rs`: round-trip `ShadowRole ↔ db_name`, template-key shortening.

Integration (against ephemeral pg via nix / docker as used in M1):

- **Cache hit.** Build main once, rebuild — second build uses
  `CREATE DATABASE … TEMPLATE`, completes in <5s, logs
  `template_reuse=true`.
- **Cache miss.** Mutate a committed file's content, rebuild — cache key
  changes, fresh build runs, logs `template_reuse=false`, index updated.
- **Template reuse with blocking connection.** Open a session against
  the template db, invoke clone — the lifecycle terminates the session
  and the clone succeeds.
- **Concurrent `shadow build head`.** Fire two
  `topcat shadow build head` processes in parallel; the advisory lock
  serializes them, both succeed, neither errors on `DROP DATABASE`.
- **`shadow build verify` with no main.** Fails-closed with message
  pointing at `topcat shadow build main`.
- **`shadow clean --all`.** Creates main/head/verify + two templates;
  `--all` drops all five.
- **Event trigger observational install.** After
  `shadow build main`, `SELECT COUNT(*) FROM pg_event_trigger WHERE
  evtname LIKE '_topcat_%'` returns the expected count; `_topcat.node_
  registry` on the shadow has rows for every applied object.
- **`-- node_id:` churn doesn't invalidate cache.** Build main, then
  rewrite `-- node_id:` in one file to a different UUID, rebuild — cache
  hits.

## Verification snapshot

Run from a project root with `[postgres]` configured and a reachable
shadow cluster:

```bash
# Happy path: build every shadow role. (new)
topcat shadow build main            # builds or reuses template
topcat shadow build head            # always fresh
topcat shadow build verify          # clones shadow_main

# Force rebuild (skip cache). (new)
topcat shadow build main --rebuild

# Inspect what's on the shadow cluster.
psql "$TOPCAT_SHADOW_DB_URL/postgres" -c \
    "SELECT datname, datistemplate, datallowconn FROM pg_database \
     WHERE datname LIKE '_topcat_shadow_%' ORDER BY datname"

# Verify event triggers installed on the shadow.
psql "$TOPCAT_SHADOW_DB_URL/_topcat_shadow_main" -c \
    "SELECT evtname, evtevent, evtenabled FROM pg_event_trigger \
     WHERE evtname LIKE '_topcat_%'"

# Verify registry populated via the observational trigger.
psql "$TOPCAT_SHADOW_DB_URL/_topcat_shadow_main" -c \
    "SELECT COUNT(*) FROM _topcat.node_registry"

# Cleanup. (new)
topcat shadow clean                 # drops main/head/verify
topcat shadow clean --all           # additionally drops _topcat_shadow_tpl_*

# Cache inspection (manual).
ls ~/.cache/topcat/shadows/
cat ~/.cache/topcat/shadows/index.json
```

Smoke assertion for CI:

```bash
topcat shadow build main
time topcat shadow build main       # second run must be < 5s (template reuse)
```

## Open questions to resolve before starting

1. **`shadow_db_url` cluster vs db.** Roadmap §Open questions item 4.
   Architecture §Config surface comments the URL as "cluster; topcat
   creates shadow_main/head/verify". Confirm with the user that
   `shadow_db_url = "postgres://localhost"` (no db path) is the canonical
   form and that topcat routes to `postgres` (or `template1`) as the
   bootstrap db before issuing `CREATE DATABASE`. Blocker: without a
   decision, `pg/client` URL parsing (from M1) doesn't know whether to
   treat an empty db segment as error or as "cluster-level".
2. **Cache dir on non-Unix.** `~/.cache/topcat/shadows` resolves via
   `dirs::cache_dir()` on macOS/Linux but is different on Windows. Decide
   whether topcat-pg is Unix-only for now (simpler) or needs a
   cross-platform story. Roadmap is silent; the rest of topcat runs on
   Windows so default to cross-platform via `dirs::cache_dir()`.
3. **Template db ownership.** Should promoted templates be owned by the
   role in `shadow_db_url`, or by a dedicated `_topcat` role? Impacts who
   can `DROP DATABASE` on `shadow clean --all`. Default: same role as the
   `shadow_db_url` connection. Revisit if permission model from M5
   reshapes this.
4. **Concurrent build policy.** Should `shadow build main --rebuild`
   racing a `shadow build verify` (which reads main as template) be
   allowed with the advisory lock serializing, or fail-fast with "main
   is being rebuilt"? Default: serialize. Concurrent-rebuild integration
   test is the canary.

## Session sizing notes

Roadmap estimates ~1-2 sessions.

- Session 1: Phases 1–3 (naming, clean, head, cache + template reuse).
  Land the integration tests for cache hit/miss and concurrent
  `shadow build head` here because they validate the lock discipline
  before `verify` piles on.
- Session 2 (if needed): Phase 4 (verify, `--rebuild`, polish,
  observability logging, performance smoke test). Fold into session 1 if
  the cache layer converges cleanly.

Trigger for a second session: cache-key decisions on `-- node_id:`
stripping turn out to need a discussion (e.g., the stripped form is
ambiguous with user comments), or the advisory-lock scheme needs to grow
to per-role locks.

## Referenced architecture sections

- `## Shadow database lifecycle` (line 264) — template reuse, cache-per-input,
  observational trigger on every shadow.
- `## Config surface` (line 1070) — `shadow_db_url` (cluster),
  `shadow_main_cache_dir`, env var precedence.
- `## Performance and caching` (line 848) — template-reuse ~2s and
  fresh-build < 90s targets; SHA-256-over-sorted-file-set cache key
  definition.
- `## Event trigger` (line 273) and `### Bootstrap and pg_restore path`
  (line 327) — what M6 must call into from M5 when installing triggers
  on a freshly created shadow.
- `## Identity and node_id` § `### Three-way node_id resolution` (line 242)
  — why the cache key must strip `-- node_id:` headers.
- `## Component map` (line 1134) — `shadow_db` module responsibility
  statement: "Template management, fresh builds, cleanup, caching."
- `## CLI surface` (line 1097) — `topcat shadow build` / `topcat shadow
  clean` signatures.

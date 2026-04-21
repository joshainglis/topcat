# M5 — event_trigger + node_registry

> Install `_topcat_ddl_end` and `_topcat_sql_drop` event triggers on a managed
> Postgres database; capture every DDL command into `_topcat.node_registry`
> atomically with the user's transaction; handle the pg_restore bulk-load
> path via new `topcat pause` / `topcat resume` subcommands; prove UUIDv7
> node identity works end-to-end.

## Context

M5 is the moment topcat stops being a static file tool and starts owning a
live Postgres catalog. Every managed db grows two event triggers
(`_topcat_ddl_end`, `_topcat_sql_drop`) that atomically keep
`_topcat.node_registry` in lock-step with the pg catalog. This is the single
correctness bet that every later milestone (M6 shadow, M7 object_dag, M8
differ, M11 node_id conflict scan) depends on — if the trigger misses a
`CREATE`, drops desync, or double-counts `_topcat`'s own install statements,
the whole object DAG downstream is lying.

The event trigger also supplies stable, time-ordered node identity. M1 ships
the `_topcat.gen_uuidv7()` function; M5 proves registry rows populate with
monotonically-increasing UUIDv7s in production DDL transactions, which M11
relies on for the `older-wins` conflict rule.

M5 is green-field under `src/pg/event_trigger/**`. No legacy topcat code
touches this module — it sits alongside the existing `pg/client` module
added in M1.

## Prerequisites

Depends on M1 and M2. Concrete artifacts expected to exist before M5 starts:

- `src/pg/client.rs` (or `src/pg/client/mod.rs`) — async Postgres client
  wrapper, shipped by M1, with connection pooling and superuser probe.
- `_topcat` meta-schema v1 fully installed by M1's bootstrapper. Concretely:
  `_topcat.schema_version`, `_topcat.node_registry` (with
  `UNIQUE (classid, objid, objsubid)`), `_topcat.node_aliases`,
  `_topcat.sync_log`, `_topcat.migration_registry`, `_topcat.gen_uuidv7()`,
  and the `_topcat.on_ddl_command_end()` / `_topcat.on_sql_drop()` function
  bodies — all created inside a bootstrap transaction that sets
  `topcat.bootstrapping = 'true'`.
- Meta-migration runner from M1 that compares `_topcat.schema_version` to
  the built-in migration set embedded in the binary, applying forward
  migrations idempotently on every connect.
- `src/schema_model/**` from M2 — `SchemaModel` typed rows for every object
  class topcat manages, plus the `CatalogReader` that produces them from a
  live connection. M5 uses the reader for its full-catalog rescan after
  `resume`.
- Integration-test harness that can spin up an ephemeral Postgres (superuser
  role available) and wire it to the M1 client. Reused here; not rebuilt.

## Scope

### In scope

- Install / uninstall logic for `_topcat_ddl_end` (on `ddl_command_end`) and
  `_topcat_sql_drop` (on `sql_drop`). Idempotent; safe to re-run.
- Bootstrap guard via `topcat.bootstrapping = 'true'` so the trigger ignores
  its own installation and every subsequent `_topcat` meta-migration.
- Tag-filter wiring: every item in architecture §Tag inventory → captured,
  out-of-list, or gap-case handling documented and exercised.
- `pg_event_trigger_ddl_commands()` rows + `pg_identify_object_as_address()`
  lateral join inserted into `node_registry` with `ON CONFLICT
  (classid, objid, objsubid) DO NOTHING` for idempotency.
- `pg_event_trigger_dropped_objects()` delete path in the sql_drop trigger.
- Extension-owned object filter (`pg_depend.deptype = 'e'`) — exclude from
  registry at insert time, not just filter at read time.
- Dynamic-DDL flag plumbing: when DDL inside a function body produces
  catalog rows whose `pg_depend` doesn't link to an originator, M5 records
  the orphan-managed marker; M9 later gates migration generation on it.
- `topcat pause` (`ALTER EVENT TRIGGER … DISABLE` for both triggers) and
  `topcat resume` (re-enable + full-catalog rescan via M2 CatalogReader to
  repopulate registry for objects created while disabled).
- UUIDv7 end-to-end verification: registry rows inserted in the same
  transaction as the DDL carry time-ordered ids whose sort order matches
  insertion order.

### Out of scope

- Drift detection on the dev db against the declared SchemaModel — lands in
  M8 differ / M16 reconcile.
- Shadow-db event-trigger installation wiring — M6 consumes M5's
  install/uninstall API.
- `topcat sync`, `topcat diff`, `topcat migrate *`, `topcat reconcile`.
- Body-dependency inference of dynamic SQL inside function bodies — that is
  M4's parser; M5 only records the catalog-side orphan marker.
- ACL / ownership / comment round-trip — already covered by M2 SchemaModel
  + M3 ddl_emitter.

## Deliverables (files and modules)

- `src/pg/event_trigger/mod.rs` — public API: `install(&Client) -> Result`,
  `uninstall(&Client) -> Result`, `pause(&Client) -> Result`,
  `resume(&Client, &CatalogReader) -> Result<RescanReport>`,
  `status(&Client) -> TriggerStatus`.
- `src/pg/event_trigger/install.rs` — idempotent install transaction that
  sets `topcat.bootstrapping = 'true'`, creates the two triggers (if
  absent), and verifies both `_topcat_ddl_end` and `_topcat_sql_drop` are
  enabled after commit. Trigger bodies are loaded from
  `src/pg/event_trigger/sql/v1_ddl_end.sql` and
  `src/pg/event_trigger/sql/v1_sql_drop.sql` via `include_str!`.
- `src/pg/event_trigger/pause.rs` — `pause` issues `ALTER EVENT TRIGGER
  _topcat_ddl_end DISABLE;` and the same for `_topcat_sql_drop`, atomically.
- `src/pg/event_trigger/resume.rs` — re-enables both, then invokes the M2
  `CatalogReader` to walk every managed object class, upserting into
  `node_registry` with the same conflict target; returns a `RescanReport`
  counting inserted / already-present / dropped-while-paused rows.
- `src/pg/event_trigger/filters.rs` — schema filter (excluding `_topcat`,
  `pg_catalog`, `information_schema`) and extension-owned (`deptype = 'e'`)
  filter. Shared by trigger body SQL and the resume-path rescan.
- `src/pg/event_trigger/superuser.rs` — pre-flight probe (`SELECT rolsuper
  FROM pg_roles WHERE rolname = current_user`). On `false`, returns
  `Error::SuperuserRequired` with remediation text.
- `src/pg/event_trigger/sql/v1_ddl_end.sql` — exact body from architecture
  §`_topcat` schema DDL (version 1) `on_ddl_command_end`.
- `src/pg/event_trigger/sql/v1_sql_drop.sql` — exact body from architecture
  §`_topcat` schema DDL (version 1) `on_sql_drop`.
- `src/commands/pause.rs` — `topcat pause` CLI handler. Connects via M1
  client, calls `event_trigger::pause`, prints status.
- `src/commands/resume.rs` — `topcat resume` CLI handler. Connects, calls
  `event_trigger::resume`, prints the `RescanReport`.
- `src/main.rs` / `src/cli/mod.rs` — register `Pause` and `Resume`
  subcommands alongside existing ones. Global flags (`--config`,
  `--log-level`, `--log-format`, `--log-file`, `--mode`) inherited.
- `tests/event_trigger/install_uninstall.rs` — ephemeral-pg integration
  tests covering install → catalog DDL → registry populated → uninstall →
  registry untouched but trigger gone.
- `tests/event_trigger/pause_resume.rs` — pause → `pg_restore`-style bulk
  DDL → resume → full-catalog rescan repopulates registry; drops while
  paused are reflected correctly after rescan.
- `tests/event_trigger/tag_inventory.rs` — one test per item in architecture
  §Tag inventory (captured + out-of-list + gap cases). Drives each DDL,
  asserts presence/absence in registry.
- `tests/event_trigger/uuidv7.rs` — insert N DDLs in sequence; assert
  `node_registry.node_id` values decoded as UUIDv7 sort monotonically and
  their embedded ms timestamp matches `created_at` within ±1s.

## Implementation phases

1. **Phase 1 — superuser probe + install/uninstall (read-only risk).**
   Ship `superuser.rs` and `install.rs`. Install transaction creates the
   two triggers if missing, asserts they're enabled, commits. Uninstall
   drops both triggers (not the `_topcat` schema). Integration tests: fresh
   db install, double install (idempotent), install as non-superuser (hard
   fail with `Error::SuperuserRequired`), uninstall, reinstall.
2. **Phase 2 — tag inventory coverage.** Author
   `tests/event_trigger/tag_inventory.rs` driving every captured tag, every
   out-of-list tag (should never appear in registry), and every gap case
   (dynamic-DDL orphan marker, extension-owned filter, `_topcat` self-DDL
   ignored, user-defined event triggers captured as objects). Trigger-body
   SQL gets any tweaks needed to pass — this is where filters land.
3. **Phase 3 — pause / resume + rescan.** Implement `pause.rs` and
   `resume.rs`. Wire `topcat pause` / `topcat resume` CLI. Integration
   test: pause → apply a pg_dump-shaped batch containing CREATEs that
   would otherwise fire the trigger → resume → rescan repopulates registry
   to match live catalog.
4. **Phase 4 — UUIDv7 end-to-end verification.** Author
   `tests/event_trigger/uuidv7.rs`. Write N DDLs in a loop (with small
   sleep to force timestamp advancement), read registry, decode each
   `node_id` as UUIDv7, assert sort order = insert order and embedded
   timestamp matches `created_at` within tolerance. This is the first time
   topcat proves UUIDv7 works under real trigger load; failure here
   indicates M1's `_topcat.gen_uuidv7()` body needs fixing, not M5.

## Risks (copied and amplified)

- **Superuser required to install event triggers.** Event triggers are
  cluster-wide and only `rolsuper` may create them. *Day-one mitigation*:
  `superuser.rs` probes `pg_roles.rolsuper` before the install transaction
  and fails fast with `Error::SuperuserRequired { role: <role>, hint: "run
  as a superuser or ask a DBA to grant and re-run" }`. Never swallow the
  error into a generic "permission denied" — the remediation is specific.
  Open question (see below): non-superuser fallback vs hard-fail for M5's
  happy path.
- **Bootstrap chicken-and-egg.** The `_topcat` schema must exist before the
  trigger installs, and the install statement itself is DDL — if the
  trigger fires on its own `CREATE EVENT TRIGGER`, it tries to insert into
  a registry whose PK columns reference the trigger itself. *Day-one
  mitigation*: the install transaction runs `SET LOCAL
  topcat.bootstrapping = 'true'` before any `CREATE` statement. Both
  trigger bodies open with `IF current_setting('topcat.bootstrapping',
  true) = 'true' THEN RETURN; END IF;`. Dedicated test: run install, then
  query `node_registry` — it must contain zero rows referencing the
  `_topcat` schema or the trigger functions.
- **Dynamic DDL inside function bodies.** A plpgsql function executing
  `EXECUTE 'CREATE TABLE …'` produces catalog rows with no `pg_depend`
  edge back to the caller. The event trigger *does* fire, so those rows
  get inserted into `node_registry` — but they're orphan-managed.
  *Day-one mitigation*: capture them now with an explicit
  `orphan_managed = true` flag (or sentinel file reference) that M9
  migration_generator consults to enforce fail-closed behavior behind
  `--allow-dynamic-ddl`. Do not try to resolve the originator in M5 —
  that's M4 body_parser's job.
- **Extension-owned objects (`pg_depend.deptype = 'e'`).** Installing an
  extension (`CREATE EXTENSION postgis`) creates hundreds of catalog rows;
  they must not flood `node_registry`. *Day-one mitigation*: both trigger
  bodies must explicitly filter `WHERE NOT EXISTS (SELECT 1 FROM pg_depend
  d WHERE d.classid = r.classid AND d.objid = r.objid AND d.deptype =
  'e')`. Verify with an integration test that installs an extension and
  asserts only the `extension` row itself (not its owned objects) enters
  the registry.
- **User-created event triggers.** A user-defined event trigger is a
  managed object like any other — it must enter `node_registry`. Topcat's
  own `_topcat_ddl_end` / `_topcat_sql_drop` must *not*. *Day-one
  mitigation*: exclude topcat's triggers by exact name match, not by any
  broader pattern — a user who happens to name a trigger `_topcat_custom`
  should still be captured. Test: create a user event trigger named
  `audit_ddl`; assert it appears in registry. Create one named
  `_topcat_ddl_end` (pathological) — document behavior (probably reject at
  install on collision with topcat's own).

## Tests

All tests live under `tests/event_trigger/` and run against an ephemeral
Postgres spawned by the M1 harness (superuser role available). Each test
starts from a clean database, runs M1 bootstrap, then M5 install, then the
scenario under test.

- **`install_uninstall.rs`** — install idempotency (double-install
  succeeds), uninstall leaves `_topcat` schema and `node_registry` intact
  but both triggers absent from `pg_event_trigger`, non-superuser install
  fails with `Error::SuperuserRequired`.
- **`tag_inventory.rs`** — one test per architecture §Tag inventory row.
  Captured tags: each DDL runs, registry row appears with correct
  `address_type` and `object_names`. Out-of-list tags (`CREATE ROLE`,
  `ALTER SYSTEM`, `VACUUM`): registry row count unchanged. Gap cases:
  dynamic-DDL orphan flagged, extension-owned excluded, `_topcat` self-DDL
  ignored, user event trigger captured.
- **`pause_resume.rs`** — happy-path restore: install → create object A →
  `pause` → apply dump that creates B, C, drops A → `resume` → assert
  registry contains B, C and not A; assert trigger state `ENABLED` at
  end. Edge: `resume` without prior `pause` is a no-op rescan; `pause`
  twice is idempotent.
- **`uuidv7.rs`** — insert 50 DDLs with a 5 ms gap; read registry ordered
  by `created_at`; assert `node_id` column sorts identically when
  interpreted as UUIDv7 (the first 48 bits are unix-ms). Decode each
  `node_id`'s timestamp; assert it matches `created_at` within 1 s.
- **`bootstrap_guard.rs`** — install trigger, then run a meta-migration
  (v2 if M5 ships with one; else re-run v1 as a no-op) with
  `topcat.bootstrapping = 'true'`; assert zero new rows in
  `node_registry` reference `_topcat` objects.

## Verification snapshot

```bash
# Prereq: M1 bootstrap (creates _topcat schema v1, gen_uuidv7, registry tables)
topcat --config topcat.toml meta-schema upgrade

# Install event triggers on the managed db
cargo run --release -- install-triggers --db-url "$DEV_DB_URL"

# Confirm both triggers present and enabled
psql "$DEV_DB_URL" -c "SELECT evtname, evtenabled FROM pg_event_trigger
                       WHERE evtname IN ('_topcat_ddl_end','_topcat_sql_drop');"

# Apply some DDL, watch node_registry populate
psql "$DEV_DB_URL" -c "CREATE TABLE demo(id bigserial PRIMARY KEY, name text);"
psql "$DEV_DB_URL" -c "SELECT node_id, address_type, object_names, created_at
                       FROM _topcat.node_registry ORDER BY created_at DESC LIMIT 5;"

# Bulk-restore dance (NEW in M5)
topcat pause                         # (new) disables both triggers
pg_restore -d "$DEV_DB_URL" big_dump.sql
topcat resume                        # (new) re-enables + full-catalog rescan

# UUIDv7 monotonicity check (registry order == insertion order)
psql "$DEV_DB_URL" -c "SELECT node_id FROM _topcat.node_registry
                       ORDER BY created_at
                       FETCH FIRST 10 ROWS ONLY;" \
  | cargo run --release -- debug uuidv7-verify --stdin

# Uninstall (reversibility check)
cargo run --release -- uninstall-triggers --db-url "$DEV_DB_URL"
psql "$DEV_DB_URL" -c "SELECT evtname FROM pg_event_trigger
                       WHERE evtname LIKE '_topcat_%';"   # expect 0 rows
```

## Open questions to resolve before starting

- **Non-superuser fallback vs hard-fail.** Event triggers strictly require
  `rolsuper`. Option A: hard-fail at install with
  `Error::SuperuserRequired` and a remediation hint — simple, honest,
  matches architecture §Event trigger. Option B: degraded mode where
  topcat runs without triggers and relies solely on `sync`-time
  `CatalogReader` diffs — loses atomic registry updates and drift
  detection. Recommendation: **hard-fail in M5**; revisit Option B in a
  later milestone only if real users demand it. Lock this before Phase 1.
- **Collision behavior when a user already has a trigger named
  `_topcat_ddl_end` or `_topcat_sql_drop`.** Refuse at install
  (recommended; rename the user's trigger) or take over (dangerous; their
  trigger silently disappears). Decide before Phase 1.
- **`RescanReport` shape.** Minimum: `inserted`, `already_present`,
  `deleted_since_pause`. Nice-to-have: per-schema breakdown, timing.
  Decide before Phase 3 to avoid a churn when M6 shadow_db consumes the
  report.
- **Where `orphan_managed` lives.** Architecture mentions `parse_status =
  dynamic_ddl` as a function-level attribute (M4's territory). M5 only
  needs to tag the catalog row. Options: extra column on `node_registry`,
  or side table `_topcat.orphan_nodes`. Recommendation: side table to
  keep `node_registry` lean. Decide before Phase 2.

## Session sizing notes

~2 sessions. Session 1: Phase 1 (superuser + install/uninstall) and
Phase 2 (tag inventory + gap cases). Session 2: Phase 3 (pause/resume +
rescan) and Phase 4 (UUIDv7 verification). Hard budget: do not expand
Session 2 into M6 shadow-db work, even if trigger install on a shadow db
feels natural — shadow-db lifecycle is a full milestone of its own and
sliding into it here will sink M5's correctness focus.

## Referenced architecture sections

- `docs/pg-dev-buddy-architecture.md` §Event trigger — purpose, tag
  inventory (Captured, Out-of-list, Gap cases), bootstrap and pg_restore
  path, superuser requirement.
- `docs/pg-dev-buddy-architecture.md` §`_topcat` meta-schema versioning
  and subsection `_topcat` schema DDL (version 1) — canonical trigger
  body SQL, `node_registry` schema, `gen_uuidv7`, `schema_version`.
- `docs/pg-dev-buddy-architecture.md` §Identity and node_id — why UUIDv7
  is a v1 correctness requirement, not a nice-to-have.
- `docs/pg-dev-buddy-architecture.md` §Body-dependency inference §Policy —
  dynamic-DDL fail-closed policy that M5's orphan-managed marker supplies
  inputs to.
- `docs/pg-dev-buddy-architecture.md` §CLI surface — `topcat pause`,
  `topcat resume` slot into the documented command set.
- `docs/pg-dev-buddy-roadmap.md` §M5 — scope, deliverables, risks, size
  (source of truth this plan expands).

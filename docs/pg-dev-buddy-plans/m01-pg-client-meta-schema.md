# M1 — pg/client + meta-schema versioning

> Source of truth: docs/pg-dev-buddy-roadmap.md §M1, docs/pg-dev-buddy-architecture.md §`_topcat` meta-schema versioning, §`_topcat` schema DDL (version 1), §Bootstrap and pg_restore path, §Config surface, §CLI surface, §Component map.

## Context

M1 lays down the first two pillars of pg-dev-buddy: a typed sync PostgreSQL client (`pg/client`) used by every downstream milestone, and a versioned meta-schema (`_topcat`) that stores node identity, sync checkpoints, and migration history. Everything after M1 assumes a working connection, a known `_topcat.schema_version`, and that `gen_uuidv7()` is callable. The event-trigger install itself is deferred to M5 per the roadmap; M1 ships only the data-plane DDL so that M2–M4 can develop against a real managed db without the chicken-and-egg of the observational triggers.

## Prerequisites

M0 must be complete and merged. Concrete M0 artifacts this milestone depends on:

- `src/settings/configs/postgres.rs` defining `PostgresConfig` (fields: `dev_db_url`, `shadow_db_url`, `shadow_main_cache_dir`, `require_superuser`, `strict_mode`, `worker_count`), registered in `src/settings/mod.rs`.
- `[postgres]` block parsing in `topcat.toml` and `TOPCAT_POSTGRES__*` env-var plumbing (verified by `topcat config show` printing the block).
- CLI stub subcommand `topcat meta-schema upgrade` returning "not implemented" with non-zero exit. M1 replaces this stub with a real implementation.
- CLI stubs for `sync`, `shadow`, `diff`, `migrate`, `reconcile`, `write-deps`, `pause`, `resume`, `uninstall`, `meta-schema` are present (only `meta-schema upgrade` is wired in M1).

## Scope

Roadmap verbatim: *"connection management, transaction helpers, DDL application, and `_topcat` schema install/upgrade per architecture §`_topcat` meta-schema versioning and the v1 DDL listing."*

Amplification: the primary correctness claim is the meta-migration runner — given any `_topcat.schema_version ∈ {absent, 1, …, N}` where N is the version compiled into the binary, running `topcat meta-schema upgrade` (or any command that auto-upgrades on connect) must arrive at N and be a no-op on a second run. Version-in-db > N must fail-closed.

### In scope for this milestone

- `pg/client` module: URL parsing, a small sync connection pool, a transaction helper that honours `SET LOCAL topcat.bootstrapping = 'true'`, a DDL-apply helper, and a typed error taxonomy (`ConnectionRefused`, `AuthFailed`, `SuperuserMissing`, `VersionUnsupported`, `MetaSchemaAhead`, `Sql(pg_error)`).
- `meta_migrations` module: embedded-as-`include_str!` SQL files, an ordered registry `&[(u32, &str)]`, a runner that reads `_topcat.schema_version`, applies each missing version inside a single transaction with `SET LOCAL topcat.bootstrapping = 'true'`, and inserts the version row.
- v1 embedded SQL file: `CREATE SCHEMA _topcat`, `schema_version`, `node_registry`, `node_aliases`, `sync_log`, `migration_registry`, `_topcat.gen_uuidv7()`. **The two `CREATE EVENT TRIGGER` statements and the two trigger functions (`on_ddl_command_end`, `on_sql_drop`) are deferred to M5** so every event-trigger concern lives in one milestone and M1 does not require superuser for install (only `CREATE SCHEMA` / `CREATE TABLE` privilege in the target db).
- `topcat meta-schema upgrade [--force]` wired to the runner. `--force` re-runs the latest version's DDL if idempotent or explicitly bumps a stuck version row; default is "apply missing versions".
- Auto-upgrade-on-connect: every future `pg/client::connect` call verifies `schema_version = N`; if lower, runs the upgrade; if higher, returns `MetaSchemaAhead`.
- Integration tests against ephemeral pg (nix-launched `postgresql`), exercising fresh install, re-run idempotency, and upgrade-from-lower-version simulated via a hand-written "v0 only" fixture.

### Explicitly NOT in scope (deferred to M5)

- `CREATE EVENT TRIGGER _topcat_ddl_end` and `_topcat_sql_drop`.
- `_topcat.on_ddl_command_end()` and `_topcat.on_sql_drop()` trigger function bodies. (Shipping the functions without the triggers buys nothing, so defer both.)
- `topcat pause` / `topcat resume` (event-trigger enable/disable) — lives with M5.
- Superuser-required privilege checks beyond "schema create" — the superuser fail-closed at connect shifts to M5 where it actually matters. M1 surfaces a non-fatal warning if `rolsuper = false` telling the user they will need superuser by M5.
- `node_registry` population logic. M1 only creates the table; M5 populates it via triggers.

## Deliverables (files and modules)

| Kind | Path | Status | Description |
|---|---|---|---|
| crate-dep | `Cargo.toml` | extend | Add `postgres = "0.19"` (sync client, no tokio) and `thiserror = "2"` if not present. Add `testcontainers = "0.20"` or equivalent under `[dev-dependencies]` only if nix-launched pg proves flaky. |
| module-root | `src/lib.rs` | extend | `pub mod pg; pub mod meta_migrations;` |
| command wiring | `src/main.rs` | extend | Route `meta-schema upgrade` to `commands::meta_schema::run`. |
| cli wiring | `src/cli/mod.rs` | extend | Replace the M0 stub for `MetaSchemaCommand` with arg parsing for `upgrade --force`. |
| module | `src/pg/mod.rs` | new | `pub mod client;` re-exports. |
| module | `src/pg/client.rs` | new | `Client` struct wrapping `postgres::Client`, `connect(&PostgresConfig)`, `transaction<F>(f)`, `apply_ddl(&str)`, `PgError` taxonomy. |
| module | `src/meta_migrations/mod.rs` | new | `pub fn current_version() -> u32`, `pub fn upgrade(client: &mut Client) -> Result<UpgradeOutcome>`, `pub fn check(client: &mut Client) -> Result<VersionCheck>`. |
| module | `src/meta_migrations/registry.rs` | new | `pub const MIGRATIONS: &[(u32, &str)] = &[(1, include_str!("sql/v001_tables.sql"))];` |
| sql asset | `src/meta_migrations/sql/v001_tables.sql` | new | v1 DDL: `_topcat` schema, `schema_version`, `node_registry`, `node_aliases`, `sync_log`, `migration_registry`, `gen_uuidv7()`. Starts with `SET LOCAL topcat.bootstrapping = 'true';`, ends with `INSERT INTO _topcat.schema_version (version) VALUES (1);`. Event triggers omitted. |
| command | `src/commands/meta_schema.rs` | new | `run(settings, args) -> Result<()>`: connects, dispatches to `meta_migrations::upgrade`, prints summary. |
| unit tests | `src/meta_migrations/tests.rs` | new | Pure-Rust tests on the registry (versions sorted, monotonic, parseable). |
| integration tests | `tests/pg_client_tests.rs` | new | Spins an ephemeral pg, runs upgrade fresh / re-run / downgrade-refused / version-ahead-refused. Skipped when `TOPCAT_TEST_PG_URL` is unset in CI so devs without a pg binary do not see spurious failures. |
| fixture | `tests/input/meta_migrations/v0_only.sql` | new | Simulates a pre-v1 install (empty `_topcat.schema_version` with version 0) for upgrade-from-lower tests. |

## Implementation phases

### Phase 1 — pg/client skeleton

- **Goal**: a sync `Client` that connects to a URL from `PostgresConfig`, runs DDL in a transaction, and returns typed errors.
- **Tasks**:
  1. Add `postgres = "0.19"` and `thiserror` to `Cargo.toml`. Reject async alternatives in a one-line code comment: `tokio-postgres` drags an async runtime into a sync CLI; `sqlx` drags a runtime *and* compile-time query checking neither needed here.
  2. `src/pg/client.rs`: `Client::connect(&PostgresConfig) -> Result<Self, PgError>`. Parse `dev_db_url` via `postgres::Config::from_str`. Map `io::ErrorKind::ConnectionRefused` → `PgError::ConnectionRefused`, SQLSTATE `28P01` / `28000` → `AuthFailed`.
  3. `Client::transaction<F, R>(&mut self, f: F) -> Result<R, PgError>` that sets `SET LOCAL topcat.bootstrapping = 'true'` before invoking the closure. Used by both the meta-migration runner and any caller that must bypass the (future, M5) event trigger.
  4. `Client::apply_ddl(&mut self, sql: &str) -> Result<(), PgError>` — runs `simple_query`, maps `postgres::Error` → `PgError::Sql`.
- **Verification**: `cargo build`; `cargo clippy -- -D warnings`. No DB tests yet.

### Phase 2 — meta_migrations runner

- **Goal**: embedded v1 SQL applied idempotently; version checked on connect.
- **Tasks**:
  1. Write `src/meta_migrations/sql/v001_tables.sql` (the v1 DDL, event triggers omitted — see Scope).
  2. `src/meta_migrations/registry.rs`: sorted `MIGRATIONS` slice. `current_version()` returns the max key.
  3. `src/meta_migrations/mod.rs::check(client)` reads `SELECT max(version) FROM _topcat.schema_version`. Missing `_topcat` schema → `Some(0)` equivalent. Returns `VersionCheck { in_db, in_tool }`.
  4. `upgrade(client)` wraps the whole upgrade in a single transaction per version (architecture allows a transaction per version; this keeps a partial upgrade recoverable). Fails-closed if `in_db > in_tool` with `PgError::MetaSchemaAhead`.
  5. Wire `Client::connect` to call `check` and auto-upgrade if `in_db < in_tool` and the caller passed `auto_upgrade = true` (default for the CLI, false for tests).
- **Verification**: unit tests in `src/meta_migrations/tests.rs` (version ordering). Integration tests deferred to Phase 4.

### Phase 3 — CLI wiring

- **Goal**: `topcat meta-schema upgrade` is real, not a stub.
- **Tasks**:
  1. `src/cli/mod.rs`: extend `MetaSchemaCommand` to accept `--force` and the `upgrade` subcommand.
  2. `src/commands/meta_schema.rs`: build a `Client`, call `upgrade`, print `UpgradeOutcome` ("already at version N", "upgraded M → N", or error with remediation hint).
  3. `src/main.rs`: route the subcommand.
- **Verification**: `cargo run -- meta-schema upgrade --help` prints the new flag. Manual run against a local pg succeeds.

### Phase 4 — ephemeral-pg integration tests

- **Goal**: exercise the runner against a real PostgreSQL binary.
- **Tasks**:
  1. Test harness: prefer a nix-launched `postgresql` via `nix develop` (already in the devShell) with a per-test temp `PGDATA`. If flaky, fall back to `testcontainers` under `dev-dependencies`.
  2. Tests: `fresh_install`, `rerun_idempotent`, `downgrade_refused` (insert `INSERT INTO _topcat.schema_version VALUES (999)` then run upgrade; assert `MetaSchemaAhead`), `partial_v0_upgrade` (use the `v0_only` fixture).
  3. Gate on env var `TOPCAT_TEST_PG_URL` or a `topcat_test_pg` feature flag so CI without pg does not fail.
- **Verification**: `cargo test --test pg_client_tests` passes locally with pg available; is skipped cleanly without it.

## Risks (copied and amplified from roadmap)

| Risk | Severity | Day-one mitigation |
|---|---|---|
| Superuser requirement (roadmap) | medium | M1 does **not** require superuser — it creates a schema and tables. The superuser check is pushed to M5 where event triggers actually need it. M1 emits a one-line warning if `rolsuper = false` so the user knows future milestones will fail without it. Fail-closed error surface is defined (`PgError::SuperuserMissing`) but unused until M5. |
| UUIDv7 impl choice (roadmap) | medium | **Resolved: pure-SQL polyfill** shipped in `v001_tables.sql` as the `_topcat.gen_uuidv7()` body. Rationale: zero install burden (no `pg_uuidv7` extension dep), works on stock pg15+, matches architecture phrasing "shipped by topcat at install time." Body draft: `48-bit unix-ms timestamp || version 7 nibble || 12-bit random || variant bits || 62-bit random`, composed with `gen_random_uuid()` (always available via `pgcrypto` which is the only extension we install if missing — or via `pg_catalog` on pg18+). See Open Questions for the fallback path if `gen_random_uuid()` proves unavailable. |
| v1 DDL drift between milestones | medium | The file `v001_tables.sql` is frozen once M1 merges. Future changes ship as `v002_*.sql`, never by editing v1. Enforce with a test: `sha256(include_str!("sql/v001_tables.sql")) == "<pinned>"`. When M5 adds event triggers, it is a *new version file*, not a rewrite. |
| Embedded SQL string-append injection | low | Meta-migration SQL is compile-time constant (`include_str!`). No format-string concatenation. Document this rule in a comment in `registry.rs`. |
| `postgres` crate version lock-in | low | `postgres = "0.19"` is maintained by sfackler alongside `tokio-postgres`; abandoning it later means rewriting the client. Accept the risk; the API surface is small. |

## Tests

- **Unit** (`src/meta_migrations/tests.rs`): version ordering, monotonicity, `current_version()` equals the last entry, SQL is non-empty and starts with `SET LOCAL topcat.bootstrapping`.
- **Integration** (`tests/pg_client_tests.rs`): fresh install, re-run idempotent (second upgrade is a no-op; `schema_version` table has exactly one row per version), downgrade refused, `v0_only` upgrade-to-latest succeeds, `Client::transaction` commits on Ok and rolls back on Err.
- **Property** (`tests/property_tests.rs`, extend): property test that for any ordered subset of `MIGRATIONS` representing the "in-db" state, running `upgrade` arrives at the latest version and the set of applied versions equals the full set. (Uses a mock client — no real pg needed.)
- **Fixtures**: `tests/input/meta_migrations/v0_only.sql` (creates an empty `_topcat.schema_version` with version 0 row only). Used by the upgrade-from-lower integration test.

## Verification snapshot (definition of done)

```bash
# Lint and build
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release

# Unit + property tests (no pg required)
cargo test meta_migrations

# Integration tests with an ephemeral pg (requires pg binary on PATH)
export TOPCAT_TEST_PG_URL="$(pg_tmp)"   # or equivalent
cargo test --test pg_client_tests       # (new)

# End-to-end against a real dev db
createdb topcat_m1_smoke
export TOPCAT_POSTGRES__DEV_DB_URL="postgres:///topcat_m1_smoke"
cargo run -- config show                # confirms [postgres] block from M0
cargo run -- meta-schema upgrade        # (new) — prints "upgraded 0 -> 1"
cargo run -- meta-schema upgrade        # (new) — prints "already at version 1"
psql topcat_m1_smoke -c "SELECT version FROM _topcat.schema_version"  # returns 1
psql topcat_m1_smoke -c "SELECT _topcat.gen_uuidv7()"                 # returns a v7 uuid
dropdb topcat_m1_smoke
```

## Open questions to resolve before starting

1. **UUIDv7 source**. **Resolved: pure-SQL polyfill** in `v001_tables.sql`. Rejected alternatives: (a) the `pg_uuidv7` extension adds a deploy-time install step the user must manage; (b) a plpgsql polyfill is slower than pure-SQL and buys no portability. If pg18+ ships `uuidv7()` natively in `pg_catalog`, v2 can redirect `_topcat.gen_uuidv7()` to the native function — v1 stays pure-SQL for portability across pg15/16/17.
2. **`gen_random_uuid()` availability**. Stock pg13+ ships it in `pg_catalog`; older pg needs `pgcrypto`. M1 targets pg15+, so `gen_random_uuid()` is always available — confirm by adding a `SELECT gen_random_uuid()` probe to Phase 4 integration tests. If it fails on a supported pg version, v1 must `CREATE EXTENSION IF NOT EXISTS pgcrypto` and document that.
3. **pg client crate**. **Resolved: `postgres = "0.19"`** (sync). Rejected: `tokio-postgres` pulls in a runtime for a sync CLI; `sqlx` adds compile-time query checking that topcat does not need and a runtime dependency.
4. **Transaction granularity for upgrades**. **Resolved: one transaction per version** (not one giant transaction across all versions). A partial upgrade then leaves `schema_version` at the last successfully-applied version and the next run picks up from there.
5. **`auto_upgrade` on connect**. **Resolved: true for CLI, false for tests.** Every CLI command that connects calls `meta_migrations::upgrade` implicitly; tests call it explicitly so they can assert against pre-upgrade states.

## Session sizing notes

Roadmap estimates ~1-2 sessions. Split suggestion if session-1 overruns:

- **Session 1**: Phases 1 + 2 + 3. Ship `pg/client`, the runner, and the CLI wiring, with unit tests only. Demonstrable via manual `createdb` / `cargo run -- meta-schema upgrade`.
- **Session 2**: Phase 4. Integration tests against pg15, pg16, pg17 via nix-launched postgres or testcontainers. Pin the `v001_tables.sql` SHA to prevent drift.

## Referenced architecture sections

- `## `_topcat` meta-schema versioning` — defines the version-read / apply-missing / refuse-downgrade protocol this milestone implements.
- `### `_topcat` schema DDL (version 1)` — source for every table shipped in `v001_tables.sql`. Event-trigger lines explicitly excluded per M1 scope.
- `### Bootstrap and pg_restore path` — motivates the `SET LOCAL topcat.bootstrapping = 'true'` discipline baked into `Client::transaction`. M1 establishes the convention even though the trigger consumer lands in M5.
- `## Config surface` — defines `[postgres]` fields consumed by `Client::connect`.
- `## CLI surface` — `topcat meta-schema upgrade [--force]` definition.
- `## Component map` — placement of `pg/client` and `meta_migrations` modules; confirms M1 touches exactly these two rows and nothing else.

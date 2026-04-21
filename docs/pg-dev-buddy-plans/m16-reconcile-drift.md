# M16 — reconcile + drift detection

> Source of truth: docs/pg-dev-buddy-roadmap.md §M16, docs/pg-dev-buddy-architecture.md §Core concepts (Managed database), §Event trigger, §Identity and node_id (Conflict taxonomy, Orphan registry row), §CLI surface (Reconcile and maintenance), §Observability (`reconcile.*` events), §Component map (`reconcile`).

## Context

M16 closes the loop on the "managed database" promise made in architecture
§Core concepts: *"Topcat owns managed dbs unconditionally: drops unmanaged
objects on reconcile."* Two classes of state divergence are handled:

1. **Drift** — DDL executed on the dev db outside topcat. M5's event trigger
   is already firing on every `ddl_command_end`; the hook point exists, but
   no `unmanaged` bit is recorded and no command exposes it. M16 adds the
   flag, flips it on out-of-band DDL, and ships `topcat reconcile` so the
   operator can either absorb (adopt the object into the registry so it
   round-trips back into source) or revert (drop it, matching the "topcat
   owns managed dbs" rule).
2. **Orphan registry rows** — catalog rows present in the registry with no
   source file claiming them. The §Conflict taxonomy row "Orphan registry
   row" already commits sync to drop such objects, but with
   `--root-pattern` protection that currently has no owner. M16 is that
   owner: it surfaces orphans, respects `--root-pattern`, and emits drop
   actions that `sync` (M12) consumes.

`topcat sync` stubs land in M0 and are implemented in M12; M5 ships the
event trigger. This milestone assumes both and adds the drift/reconcile
layer on top. Parallelizable with M15 (split/merge), M17 (bootstrap),
M18 (uninstall) per the roadmap dependency graph — none of them touch the
`_topcat.node_registry` schema or the `reconcile` command surface.

## Prerequisites

Depends on:

- **M5** (`pg/event_trigger` + `_topcat.node_registry`): the trigger exists
  and is capturing every `ddl_command_end`. M16 extends the trigger body
  to set `unmanaged = true` on rows whose DDL did not originate from
  topcat (detected via the `topcat.bootstrapping` GUC and a new
  `topcat.syncing` GUC set by sync).
- **M12** (`sync` loop): the canonical writer that clears `unmanaged` when
  it applies DDL through `ddl_emitter`. Reconcile's `--absorb` and
  `--revert` both delegate the actual DDL execution path to sync's apply
  machinery so there is exactly one code path for schema changes.

Green-field modules introduced by this milestone:

- `src/reconcile/` — new crate-internal module. No prior file to extend.
- `src/commands/reconcile.rs` — M0 landed this as a stub that returns
  `NotImplemented("reconcile lands in M16")`. M16 replaces the body.

Leverages but does not modify:

- `src/sql_config.rs` / `src/settings/configs.rs` — the existing
  `AnalysisConfig.root_patterns` / `root_nodes` / `root_regex` /
  `root_dirs` are reused verbatim for orphan protection. The CLI arg
  group `src/cli/analysis.rs` already exposes `--root-pattern`,
  `--root-nodes`, `--root-regex`, `--root-dir`; M16 flattens the whole
  group into `ReconcileArgs`.
- `src/exceptions.rs` — `TopCatError` variants for conflict/drift cases.
  New variants as needed (see Deliverables).

Open roadmap questions that block this milestone: none new. Q5
(event-trigger absence permission model) was resolved at M5 (hard-fail
on non-superuser). Q1 (UUIDv7 source) resolved at M1. Nothing else in
§Open questions touches reconcile semantics.

## Scope

Roadmap verbatim: *"Drift detection via event trigger: DDL not originated
from topcat marks objects `unmanaged`. `topcat reconcile
[--absorb|--revert] [--pattern <glob>]` CLI. Orphan row handling with
`--root-pattern` protection."*

### In scope for this milestone

- `_topcat` schema version 2 migration: add `unmanaged boolean NOT NULL
  DEFAULT false` and `unmanaged_since timestamptz` columns to
  `_topcat.node_registry`; backfill existing rows to `false`.
- Event-trigger body update (M5 owns the install path; M16 ships the new
  body behind a version bump): when `current_setting('topcat.syncing',
  true) <> 'true'` and `current_setting('topcat.bootstrapping', true) <>
  'true'` at trigger time, newly-inserted rows get `unmanaged = true,
  unmanaged_since = now()`; `ALTER` events on existing managed rows set
  the flag without touching other columns.
- `src/reconcile/` module with:
  - `detector.rs` — `DriftReport { drift: Vec<DriftRow>, orphans:
    Vec<OrphanRow> }`. Reads the registry, cross-references catalog and
    file DAG, returns a structured report. No side effects.
  - `actions.rs` — `AbsorbAction` (registry row gets a source file
    written + `unmanaged = false`) and `RevertAction` (queues a
    `Change::Drop` for sync to execute). Pure data, no I/O.
  - `root_filter.rs` — wraps the existing `analysis::root_matcher`
    helpers to decide whether an orphan row is protected. Protected
    rows become `OrphanRow { protected: true, .. }` and are surfaced
    in the report but never absorbed/reverted without `--force-orphan`.
  - `mod.rs` — public API `fn detect(settings) -> Result<DriftReport>`,
    `fn absorb(report, pattern, settings) -> Result<Plan>`, `fn
    revert(report, pattern, settings) -> Result<Plan>`.
- `topcat reconcile [--absorb|--revert] [--pattern <glob>]
  [--force-orphan] [--mode dry-run|execute]` CLI, replacing the M0 stub.
  Default invocation (`topcat reconcile`) is "report only" — prints
  drift + orphan tables, exits 0 if clean, exits 1 if any finding.
- `reconcile.drift.absorbed`, `reconcile.drift.reverted`,
  `reconcile.orphan` JSON events (architecture §Observability §Event
  catalog already names these; M16 is the first emitter).
- `topcat sync` integration: sync reads `unmanaged` rows during its
  pre-apply conflict scan. The default policy (see Risks) is **error** —
  sync refuses to run while any `unmanaged = true` row exists, pointing
  the operator at `topcat reconcile`. `sync --force` overrides and
  applies the "drop unmanaged" rule from §Core concepts.

### Explicitly NOT in scope

- **Historical drift forensics.** We record `unmanaged_since` but not
  the full DDL text. The event trigger in PL/pgSQL cannot portably
  capture the source SQL without deparse round-trips; architecture does
  not require it.
- **Shadow-db drift reconciliation.** Shadows are throwaway
  (§Shadow database lifecycle); drift on a shadow is meaningless. The
  event trigger on shadows stays observational and the `unmanaged`
  column stays at its default.
- **Cross-cluster reconcile.** Deploy-target drift is out of scope per
  §Cross-environment risks. M16 is strictly dev-db local.
- **`absorb` reverse-generation of source file text.** Absorb calls
  back into the `ddl_emitter` + header-generator path owned by M3/M11;
  M16 invokes the existing API, does not re-derive emission logic.
- **Automatic reconcile on every command.** No ambient behaviour.
  Reconcile runs only when the operator invokes it, or when `sync`
  encounters unmanaged rows and refuses.

## Deliverables (files and modules)

| Kind        | Path                                            | Status | Description |
|-------------|-------------------------------------------------|--------|-------------|
| new module  | src/reconcile/mod.rs                            | create | Public API: `detect`, `absorb`, `revert`, `DriftReport`, `Plan`. Re-exports submodule types. |
| new module  | src/reconcile/detector.rs                       | create | Reads `_topcat.node_registry`, joins with catalog + file DAG. Produces `DriftReport`. |
| new module  | src/reconcile/actions.rs                        | create | `AbsorbAction`, `RevertAction`, `Plan`. Delegates to `ddl_emitter` for source emission and to `sync` apply for DROP execution. |
| new module  | src/reconcile/root_filter.rs                    | create | Thin adapter: `is_protected_orphan(row, &AnalysisConfig)`. Reuses `analysis::root_matcher`. |
| new module  | src/reconcile/report.rs                         | create | Text + JSON renderers for `DriftReport`. Uses `display_utils::table`. |
| new test    | src/reconcile/tests.rs                          | create | Unit tests for detector (table-driven), actions, root filter. |
| extend      | src/commands/reconcile.rs                       | modify | Replace M0 stub. `ReconcileArgs { global, analysis, action: AbsorbRevertNone, pattern, force_orphan, mode }`. Routes to `reconcile::detect/absorb/revert`. |
| extend      | src/commands/mod.rs                             | modify | Register `reconcile` submodule (M0 already did this, but confirm it exports the non-stub `execute`). |
| extend      | src/lib.rs                                      | modify | `pub mod reconcile;`. |
| new SQL     | src/meta_migrations/v2_unmanaged.sql            | create | Embedded `_topcat` migration. `ALTER TABLE _topcat.node_registry ADD COLUMN unmanaged bool NOT NULL DEFAULT false, ADD COLUMN unmanaged_since timestamptz;` Update `_topcat.on_ddl_command_end` body to flip the flag when `topcat.syncing` is not set. Ends with `INSERT INTO _topcat.schema_version (version) VALUES (2);`. |
| extend      | src/meta_migrations/mod.rs                      | modify | Register `v2_unmanaged.sql` in the embedded-migration registry set up at M5. |
| extend      | src/sync/apply.rs                               | modify | At the start of every sync transaction: `SET LOCAL topcat.syncing = 'true'`. At the pre-apply conflict scan: query `_topcat.node_registry WHERE unmanaged = true`; if any rows exist and `--force` is not set, fail-closed with an error pointing at `topcat reconcile`. After successful apply: `UPDATE _topcat.node_registry SET unmanaged = false, unmanaged_since = NULL WHERE node_id = ANY(...)` for every node the apply touched. |
| extend      | src/exceptions.rs                               | modify | Add `TopCatError::DriftDetected { count: usize }`, `TopCatError::OrphanProtected { patterns: Vec<String> }`, `TopCatError::ReconcileRequiresAction` (when `reconcile` is invoked against drift without `--absorb` or `--revert`, in execute mode). |
| extend      | src/cli/mod.rs                                  | modify | No new arg group required; reconcile flattens existing `GlobalArgs` + `AnalysisArgs` + `ExecutionArgs` + `InputArgs`. |
| new fixture | tests/input/reconcile/basic_drift/              | create | Fixture project: one SQL file (`users.sql`), a migration plan that applies it, a shell helper that runs an out-of-band `ALTER TABLE users ADD COLUMN ad_hoc int`. |
| new fixture | tests/input/reconcile/orphan_with_root/         | create | Fixture: registry row backed by `roots/api.sql` which is deleted from disk. Used to exercise `--root-pattern "roots/**"` protection. |
| new test    | tests/cli_reconcile_tests.rs                    | create | Integration suite. See §Tests for the six required cases. |
| extend      | tests/common/mod.rs (if present) or new helper  | modify | Helper to spin an ephemeral pg instance with `_topcat` installed at version 2. Follows M5's existing pattern; extends rather than duplicates. |
| extend      | topcat.toml.example                             | modify | Document that `[analysis]` `root_patterns`/`root_nodes` also govern reconcile orphan protection. Single-line comment near the existing `[analysis]` block. |
| extend      | docs/pg-dev-buddy-architecture.md               | modify | Tiny clarifying edit: §Conflict taxonomy "Orphan registry row" row links to §CLI surface `topcat reconcile` — only if the edit is <3 lines. Skip otherwise to avoid churn. |

No new Cargo dependencies. `ddl_emitter` (M3), `sync` apply (M12),
`display_utils`, `analysis::root_matcher` are all already in tree.

## Implementation phases

### Phase 1 — Registry schema + event trigger body

**Goal**: `_topcat` schema version 2 lands; unmanaged bit is recorded on
out-of-band DDL; sync-originated DDL leaves the bit false.

**Tasks**:
- [ ] Write `src/meta_migrations/v2_unmanaged.sql` with the `ALTER
      TABLE` and the updated trigger body. Use `CREATE OR REPLACE
      FUNCTION _topcat.on_ddl_command_end` so the install path doesn't
      change order.
- [ ] Register the file in `src/meta_migrations/mod.rs` (registry pattern
      established at M5).
- [ ] Integration test `test_meta_upgrade_v1_to_v2`: boot a pg with v1,
      run any topcat command, assert schema_version = 2 and the columns
      exist.
- [ ] Integration test `test_trigger_marks_out_of_band_ddl_unmanaged`:
      connect via a second connection without setting `topcat.syncing`,
      run `CREATE TABLE t (...)`, assert the new `node_registry` row has
      `unmanaged = true`.
- [ ] Integration test `test_sync_sets_topcat_syncing_guc`: apply DDL
      through the sync path (even a stub), assert the resulting
      registry row has `unmanaged = false`.

**Verification**:

```bash
cargo test --test meta_schema_upgrade_tests
cargo test --test event_trigger_tests test_trigger_marks_out_of_band_ddl_unmanaged
```

### Phase 2 — Detector + root filter + report

**Goal**: `reconcile::detect()` returns a `DriftReport` that the
integration tests can assert against. No side effects yet.

**Tasks**:
- [ ] Define `DriftRow`, `OrphanRow`, `DriftReport` in
      `src/reconcile/detector.rs`. Fields: `node_id`, `pg_address`,
      `unmanaged_since`, `kind` (enum `Drift` / `Orphan`), `protected:
      bool`, `protecting_pattern: Option<String>`.
- [ ] Implement `detect(&Settings)`:
      1. Query `_topcat.node_registry` for rows where `unmanaged = true`
         (drift) or where `file IS NULL` (orphan candidates).
      2. For orphan candidates, run the root-filter pass.
      3. Cross-check each drift row against the catalog (via
         `catalog_reader` M2) so deleted-but-still-unmanaged rows are
         reclassified as `orphan` not `drift`.
- [ ] Implement `root_filter::is_protected_orphan(row,
      &AnalysisConfig)`. For each pattern kind (`root_nodes`,
      `root_patterns`, `root_regex`, `root_dirs`), return the first
      match so the report can cite which rule protected the row.
- [ ] Implement `report::render_text` and `report::render_json` using
      `display_utils`. JSON shape matches the §Observability event
      schema so the same serializer can emit events.
- [ ] Unit tests (table-driven) in `src/reconcile/tests.rs`.

**Verification**:

```bash
cargo test --lib reconcile
```

### Phase 3 — Absorb + revert actions, CLI wiring

**Goal**: `topcat reconcile --absorb` and `topcat reconcile --revert`
produce correct plans; `--mode execute` applies them via sync's apply
path.

**Tasks**:
- [ ] `actions::AbsorbAction`:
      1. For each drift `DriftRow`: call `ddl_emitter::emit_for(node_id)`
         → SQL text. Call `header_generator::generate(&node_id, &body)`
         → header prelude. Write to `<src>/<schema>/<name>.sql` in the
         input dir. If the file exists, fail-closed (operator must
         resolve manually).
      2. `UPDATE _topcat.node_registry SET unmanaged = false,
         unmanaged_since = NULL WHERE node_id = ...`. Run under a
         transaction with `SET LOCAL topcat.syncing = 'true'` so the
         event trigger doesn't re-flip the bit.
- [ ] `actions::RevertAction`:
      1. Build a `ChangeSet` with one `Change::Drop` per drift/orphan
         row. Hand to `sync::apply`.
      2. Sync runs with `SET LOCAL topcat.syncing = 'true'` so the
         `sql_drop` trigger cleans up the registry row as usual.
- [ ] Replace the stub body in `src/commands/reconcile.rs`:
      - Parse args: exactly one of `--absorb`/`--revert` may be set; if
        neither set and `--mode execute`, return
        `TopCatError::ReconcileRequiresAction`.
      - `--pattern` filters the set of rows the action operates on
        (glob-match against `pg_address`). Orthogonal to
        `--root-pattern`, which protects orphans from
        revert/absorb entirely.
      - `--force-orphan` allows action on protected orphan rows.
      - In `--mode dry-run`, render the plan via `report`. In
        `--mode execute`, run the action and emit the observability
        events.
- [ ] Register the CLI's analysis arg group on `ReconcileArgs` so
      `--root-pattern`/`--root-nodes`/etc. apply.
- [ ] Update `src/commands/mod.rs` dispatch.

**Verification**:

```bash
cargo build
cargo run -- reconcile --help
cargo run -- reconcile                       # (exits 1 if drift, 0 if clean)
cargo run -- reconcile --absorb --mode dry-run
cargo run -- reconcile --revert --mode execute --pattern "legacy.*"
```

### Phase 4 — Sync integration

**Goal**: sync refuses to proceed while drift exists (unless `--force`);
sync clears the unmanaged bit on every object it applies.

**Tasks**:
- [ ] In `src/sync/apply.rs` (M12), wrap the transaction opener with
      `SET LOCAL topcat.syncing = 'true'`.
- [ ] In the pre-apply conflict scan step, add a `SELECT count(*) FROM
      _topcat.node_registry WHERE unmanaged = true` check. On > 0 and
      not `--force`: return `TopCatError::DriftDetected { count }`. The
      error `Display` directs the user at `topcat reconcile`.
- [ ] With `--force`: proceed, and the catalog reads + drop-cascade-
      recreate logic naturally handles the unmanaged objects per the
      §Core concepts rule ("drops unmanaged objects on reconcile"). The
      `--force` path is a pre-existing flag from M12; M16 documents its
      reconcile semantics.
- [ ] Integration test pair: `test_sync_refuses_with_drift` +
      `test_sync_force_drops_drift`.

**Verification**:

```bash
cargo test --test sync_drift_tests
```

### Phase 5 — Integration tests, docs, observability

**Goal**: lock the three-mode contract into CI.

**Tasks**:
- [ ] `tests/cli_reconcile_tests.rs` covering §Tests §Integration cases.
- [ ] Emit `reconcile.orphan`, `reconcile.drift.absorbed`,
      `reconcile.drift.reverted` JSON events. Assert event payloads in
      one integration test using `--log-format json --log-file
      <tmp>`.
- [ ] Update `topcat.toml.example` with the cross-reference comment.
- [ ] Roadmap §M16 verification-snapshot row (`topcat reconcile` marked
      `(new)`) lands in this plan's §Verification snapshot below.

**Verification**:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Risks (copied and amplified)

Roadmap §M16 lists one risk verbatim: *"'Unmanaged' flag discipline.
Need a clear rule for how sync interacts with unmanaged objects (drop,
preserve, error?)."* §Core concepts already commits to "drops unmanaged
objects on reconcile." M16 picks the specific interaction rules below
and ships them on day one so a later session does not silently re-open
the policy decision.

- **Sync-vs-unmanaged default policy.** Severity: **high**. The
  architecture's "drop on reconcile" rule is correct for `sync --force`
  but lethal for a casual `topcat sync`. Day-one mitigation: sync
  without `--force` **errors** if any unmanaged row exists, with an
  error message that names `topcat reconcile --absorb` and `topcat
  reconcile --revert` as the two resolutions. `sync --force` preserves
  the §Core concepts "unconditional ownership" rule and drops unmanaged
  objects. This gives the operator one explicit opt-in and zero silent
  drops.
- **GUC-based origin detection is coarse.** Severity: medium. We detect
  topcat-originated DDL by checking
  `current_setting('topcat.syncing', true)`. A connection that forgets
  to set the GUC will be misclassified as drift. Day-one mitigation:
  every public entry point in `src/pg/client.rs` that opens a
  transaction intended to execute DDL calls a helper
  `begin_managed_txn()` that sets both `topcat.bootstrapping` (for meta
  migrations) and `topcat.syncing` (for sync / reconcile apply)
  consistently. A unit test on the helper asserts both GUCs are set.
- **Orphan root protection false positives.** Severity: medium. A
  `--root-pattern "**/api/*.sql"` that matches 500 orphan files will
  silently stop the operator from ever cleaning them up. Day-one
  mitigation: `topcat reconcile` always prints the count of protected
  orphan rows alongside the actionable rows so the operator notices
  the "50 orphans protected by root-pattern" summary line. Second-line
  mitigation: `--force-orphan` exists specifically to bypass the
  protection for one invocation, never in config.
- **Registry migration v1 → v2 on dbs already holding drift.**
  Severity: low. A dev db that was running topcat at schema v1 may
  already have drifted without topcat knowing. Day-one mitigation:
  the v2 upgrade sets `unmanaged = false` for every existing row
  (backfill to false, not true) — we assume no drift at upgrade time,
  which matches the "first run of v2 is the new baseline" semantics.
  Document this clearly in the v2 SQL file header.
- **`--pattern` glob semantics on `pg_address`.** Severity: low.
  Operators will expect `--pattern "public.*"` to match the `public`
  schema. Day-one mitigation: match the pattern against the full
  `schema.name` string using `globset` (already a dep). Unit-test the
  common shapes: `"*.t_foo"`, `"auth.*"`, `"foo_??"`.
- **Absorb collisions with existing source files.** Severity: low.
  Absorb writes to `<src>/<schema>/<name>.sql`; if the file exists
  (perhaps the operator hand-wrote it but forgot the `-- node_id:`
  header), silent overwrite destroys work. Day-one mitigation:
  fail-closed on file existence; force-overwrite requires
  `--force-absorb-overwrite` (added only if a real user asks — do not
  ship speculatively).

## Tests

### Unit tests (`src/reconcile/tests.rs`)

- `test_detector_classifies_drift_vs_orphan`.
- `test_detector_reclassifies_dropped_unmanaged_as_orphan`.
- `test_root_filter_protects_by_pattern`.
- `test_root_filter_protects_by_node_name`.
- `test_root_filter_protects_by_regex`.
- `test_root_filter_protects_by_dir`.
- `test_root_filter_cites_first_matching_rule`.
- `test_pattern_filter_against_pg_address`.
- `test_absorb_action_plan_emits_ddl_and_header`.
- `test_revert_action_plan_produces_drop_changeset`.

### Integration tests (`tests/cli_reconcile_tests.rs`)

Roadmap §M16 Tests verbatim: *"apply DDL outside topcat, run reconcile
with each mode, assert expected state."* Six required cases:

1. `test_reconcile_reports_drift_and_exits_nonzero` — apply
   `ALTER TABLE t ADD COLUMN x int` via a direct pg connection
   (no `topcat.syncing` GUC), invoke `topcat reconcile` in the default
   "report only" mode, assert stdout contains the drift row, exit 1.
2. `test_reconcile_absorb_writes_source_file_and_clears_flag` — same
   setup, run `topcat reconcile --absorb --mode execute`, assert the
   new source file exists with `-- node_id:` header and the registry
   row has `unmanaged = false`.
3. `test_reconcile_revert_drops_object` — same setup, run
   `--revert --mode execute`, assert the catalog no longer contains the
   column and the registry row is gone.
4. `test_reconcile_orphan_protected_by_root_pattern` — delete a source
   file that a root-pattern protects; `topcat reconcile` must list the
   orphan as protected, and `--revert --mode execute` must refuse to
   drop it; `--force-orphan --revert --mode execute` succeeds.
5. `test_sync_refuses_when_drift_present` — apply out-of-band DDL, run
   `topcat sync`, assert `DriftDetected` error and exit 1.
6. `test_sync_force_drops_drift` — apply out-of-band DDL, run
   `topcat sync --force`, assert the drifted object is dropped (per
   §Core concepts "unconditional ownership") and registry is clean.

Plus one meta-schema test:

7. `test_reconcile_invokes_meta_schema_upgrade_on_v1_db` — boot at
   schema v1, run `topcat reconcile`, assert schema is now v2 before
   the detection query runs.

### Fixtures

- `tests/input/reconcile/basic_drift/` — single `users.sql` file, a
  helper module in `tests/common/reconcile.rs` that applies it via
  `sync` and then issues out-of-band DDL over a raw `tokio-postgres`
  connection.
- `tests/input/reconcile/orphan_with_root/` — two files, one under
  `roots/` designated by `--root-pattern "roots/**"`.

### Property tests

None for M16. Reconcile is deterministic on registry snapshot +
catalog + file DAG; the property-test budget is already spent on
normalization (M2) and ddl_emitter round-trip (M3).

## Verification snapshot (definition of done)

```bash
# Build and format gates
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Full test suite
cargo test

# Stub-era reconcile behavior is replaced
cargo run -- reconcile --help                                        # (new)
cargo run -- reconcile                                               # (new) — report only
cargo run -- reconcile --absorb --mode dry-run                       # (new)
cargo run -- reconcile --absorb --mode execute --pattern "auth.*"    # (new)
cargo run -- reconcile --revert --mode execute --force-orphan        # (new)

# Sync drift integration
cargo run -- sync                                                    # errors if drift present
cargo run -- sync --force                                            # drops unmanaged objects

# Meta-schema upgrade path
cargo run -- meta-schema upgrade --force
psql -c "SELECT version FROM _topcat.schema_version ORDER BY version DESC LIMIT 1"  # → 2
psql -c "SELECT column_name FROM information_schema.columns
         WHERE table_schema='_topcat' AND table_name='node_registry'
           AND column_name IN ('unmanaged','unmanaged_since')"       # → 2 rows
```

## Open questions to resolve before starting

- **Should `reconcile` run `meta-schema upgrade` implicitly if the db
  is at v1?** Default assumption: yes, consistent with every other
  managed-db command per §`_topcat` meta-schema versioning. Confirm at
  session start; M16's tests depend on it.
- **`--pattern` matching domain.** Default assumption: matches against
  `pg_address` (`schema.name` string). Alternative: match against
  `file` path. Confirm — docs imply the former, but operators may
  prefer the latter for muscle-memory parity with `--root-pattern`.
- **Event-trigger GUC name.** Default assumption: `topcat.syncing`.
  Alternatives: `topcat.managed`, `topcat.internal`. Pick one and
  commit before writing the v2 SQL; renaming later is a second
  `_topcat` migration.

## Session sizing notes

Roadmap estimate: **~1–2 sessions**. Accurate. Natural split:

- Session 1: Phases 1–2 (schema v2, event-trigger body, detector +
  root filter + report). Testable independently — detector returns
  reports, nothing mutates.
- Session 2: Phases 3–5 (absorb, revert, sync integration, integration
  tests, observability events).

If Session 1 runs long, Phase 2 can slip into Session 2 without
breaking the sequencing — Phase 1 is independently shippable and gives
downstream work the `unmanaged` column it needs.

## Referenced architecture sections

- `## Core concepts` — "Managed database" definition commits to
  "drops unmanaged objects on reconcile", which M16 implements.
- `## Event trigger` §Tag inventory, §Purpose item 3 — "Drift detection
  on the dev db (out-of-band DDL → unmanaged flag)" is the trigger
  behaviour extended here.
- `## Identity and node_id` §Conflict taxonomy — "Orphan registry row"
  row names the `--root-pattern` protection that M16 owns.
- `## Sync` §Final drift check — the post-apply drift check M12 runs
  now also clears the `unmanaged` bit it finds.
- `## Observability` §Event catalog — `reconcile.orphan`,
  `reconcile.drift.absorbed`, `reconcile.drift.reverted` are named
  here; M16 ships the first emitters.
- `## CLI surface` §Reconcile and maintenance — the
  `topcat reconcile [--absorb|--revert] [--pattern <glob>]` signature
  lives here.
- `## Component map` — names `reconcile` as a top-level module; M16
  fills it in.
- `## _topcat meta-schema versioning` — dictates how the v1 → v2
  upgrade is shipped (embedded SQL with `SET LOCAL
  topcat.bootstrapping = 'true'` wrapper).

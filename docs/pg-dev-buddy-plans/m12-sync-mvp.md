# M12 — sync (dev loop) — MVP milestone

> After this milestone a usable pg-dev-buddy MVP exists. `topcat sync` is the
> end-to-end dev loop: mutate files, run one command, dev db matches the
> working tree. Everything from M13 onward is polish (reverses, bounces,
> split/merge, reconcile, bootstrap, uninstall, codegen, perf hardening).

## Context

M12 is the capstone of the MVP critical path. Every earlier milestone
(M2, M3, M5, M7, M8, M11) produces an artifact that sync orchestrates but
does not itself own. The sync module is purely a coordinator: read the
working tree, diff against the live dev db, walk the object DAG, apply
per-subgraph transactions under an advisory lock, checkpoint, recover on
crash, and assert post-condition drift-freedom.

Key invariant from architecture §Apply granularity: the DDL emitter is the
single `SchemaModel → DDL` path. Sync re-applies only the dropped nodes
(object-granular), guaranteeing byte-for-byte parity with the DDL a
migration would emit for the same change. This is also what makes the sync
flow safe to re-run after a crash — the recreate step is a pure function of
the candidate SchemaModel.

All sync code lives under `src/sync/`, a green-field directory for this
milestone. The only pre-existing topcat code touched is the CLI dispatch
surface (`src/cli/`, `src/commands/`, `src/main.rs`, `src/lib.rs`) where the
new `sync` subcommand is wired in.

## Prerequisites

Depends on six upstream milestones, each contributing a concrete artifact:

| Milestone | Artifact consumed by sync |
|---|---|
| **M2** | `schema_model` crate (canonical types); `pg/catalog_reader` (reads dev db → live SchemaModel; reads working tree files → candidate SchemaModel). |
| **M3** | `pg/ddl_emitter` — the single `SchemaModel → DDL` path; drives per-node recreate after `DROP ... CASCADE`. |
| **M5** | `pg/event_trigger` (`_topcat_ddl_end`, `_topcat_sql_drop`) installed on dev db; `_topcat.node_registry` populated atomically with DDL transactions. Sync relies on triggers firing inside its per-subgraph transactions so implicit objects (indexes, constraints) get registered automatically. |
| **M7** | `object_dag` module — the input to subgraph partitioning. Sync walks changed objects via the DAG to compute minimal subgraphs for per-subgraph transactions. |
| **M8** | `differ` + `ChangeSet` types — sync calls `differ(live, candidate) → dev_changeset`. |
| **M11** | `-- node_id:` header parse/write (on `file_node`); conflict scanner; `_topcat.node_aliases`; `--adopt` bootstrap semantics. Sync's conflict-scan stage is literally M11 wired into the pipeline. |

Sync consumes all six. The only net-new persistent-state addition M12 makes
is the `_topcat.sync_log` table (schema below).

## Scope

### In scope

- Pipeline orchestration per architecture §Sync (dev loop).
- Per-subgraph transaction loop with session-level advisory lock.
- `_topcat.sync_log` checkpointing and crash recovery.
- Object-granular drop-cascade-recreate via `pg/ddl_emitter`.
- Final drift check (read live SchemaModel, assert equal to candidate).
- `topcat sync [--adopt] [--force]` CLI command wired into existing
  dispatch.

### Out of scope (deferred)

- **Parallel subgraph apply.** Architecture §Performance and caching states
  independent subgraphs may apply in parallel up to `worker_count` (default
  4). M12 ships with single-worker semantics; M20 turns it on.
- **Reverse emission, bounces, split/merge** — M13–M15.
- **Reconcile / drift absorption** — M16. Final drift check in M12 is a
  fail-closed assertion, not a reconcile.
- **Multi-developer shared dev dbs** (architecture §Concurrency explicitly a
  non-goal).

## Deliverables (files and modules)

### New module — `src/sync/`

| File | Responsibility |
|---|---|
| `src/sync/mod.rs` | Public entry point: `run_sync(cfg, flags) -> Result<SyncReport>`. Wires the stages in order. |
| `src/sync/pipeline.rs` | Stage ordering per architecture §Sync: parse → conflict scan → read live → diff → subgraph loop → final drift check. |
| `src/sync/subgraph.rs` | Partition `dev_changeset` into independent subgraphs using `object_dag`. One subgraph = one transaction. |
| `src/sync/apply.rs` | Per-subgraph: acquire advisory lock, open txn, `DROP ... CASCADE`, re-emit via `pg/ddl_emitter`, commit, release lock. |
| `src/sync/sync_log.rs` | `_topcat.sync_log` read/write. Insert `state = in_progress` at subgraph start; update to `done` at commit; `failed` with error on abort. |
| `src/sync/recovery.rs` | On startup: scan `sync_log` for `state = in_progress` rows; retry from scratch (drop-cascade + re-emit). Idempotence is the correctness claim. |
| `src/sync/drift_check.rs` | Final step: read live SchemaModel, compare with candidate, fail-closed on mismatch. |
| `src/sync/errors.rs` | `SyncError` enum. |

### Modified files (existing, touched only for wiring)

- `src/cli/` — add `SyncArgs` struct and `--adopt`, `--force` flags.
- `src/commands/` — add `sync.rs` dispatcher that calls `sync::run_sync`.
- `src/main.rs` / `src/lib.rs` — register the subcommand.

### Persistent state

- `_topcat.sync_log (sync_id uuid, subgraph_root uuid, state text, started_at timestamptz, finished_at timestamptz, error text)` — table DDL ships as a `_topcat` meta-migration (M1/M5 versioning path). Architecture §Transactions and failure recovery.
- Session-level advisory lock keyed `pg_advisory_lock(hashtext('topcat_sync'))` (architecture §Transactions and failure recovery).

### CLI surface (new)

- `topcat sync` — sync dev db to working tree.
- `topcat sync --adopt` — bootstrap node_ids on an existing dev db with no
  topcat history; register ID generation; emit no migration (architecture
  §Bootstrapping, "Adoption migration").
- `topcat sync --force` — skip the interactive confirmation shown when
  drops will cascade through many dependents.

## Implementation phases

### Phase 1 — skeleton + CLI wiring (≈ half session)

- Create `src/sync/` with empty `mod.rs` exposing `run_sync` stub.
- Wire `topcat sync [--adopt] [--force]` into CLI and commands dispatch.
- `_topcat.sync_log` table DDL as a `_topcat` meta-migration.
- End state: `topcat sync` prints "not implemented"; meta-migration creates
  the table on next connection.

### Phase 2 — happy-path pipeline (≈ 1 session)

- Implement `pipeline.rs` stages in order (parse, conflict scan, live read,
  diff, subgraph loop, drift check).
- `subgraph.rs` partitioning: start with "one subgraph per changed node"
  for M12 to keep single-worker simple; finer partitioning tracked for M20
  alongside parallelism.
- `apply.rs` per-subgraph transaction with advisory lock, `DROP ... CASCADE`,
  re-emit via `ddl_emitter`.
- `sync_log.rs` writes `in_progress` → `done` transitions.
- `drift_check.rs` final equality check.
- End state: clean-tree `topcat sync` is a no-op; mutate-a-column `topcat
  sync` converges.

### Phase 3 — crash recovery + idempotence (≈ half session)

- `recovery.rs` scans `sync_log` on startup for stranded `in_progress` rows
  and retries from scratch.
- Fault-injection harness: abort a transaction mid-apply (SIGKILL the
  process between `DROP` and `COMMIT`), start a new `topcat sync`, assert
  convergence.
- `sync_log` entries for the failed attempt become `failed` with captured
  error; recovery replaces them with a fresh `in_progress` row.

### Phase 4 — `--adopt` and `--force` flags (≈ half session)

- `--adopt`: skip the diff + apply stages entirely; walk the live catalog,
  assign node_ids via the existing M11 machinery, record in
  `node_registry`, exit. No subgraph transactions run.
- `--force`: suppress the interactive prompt shown before cascading drops.

### Phase 5 — integration tests + polish (≈ half session)

- End-to-end fixture tests (see `## Tests` below).
- Observability events: `sync.start`, `sync.subgraph.start`,
  `sync.subgraph.complete`, `sync.subgraph.failed`, `sync.complete`
  (architecture §Observability event catalog).
- Performance measurement against architecture §Performance and caching
  targets (100 objects < 3s; 1000 objects < 15s incremental).

## Risks (copied and amplified)

### Lock duration discipline

Architecture §Transactions and failure recovery is emphatic: many small
transactions, not one big transaction. A single transaction spanning the
whole sync would hold `AccessExclusiveLock` across dozens of objects for
the full duration and freeze any concurrent session on the dev db.

**Day-one mitigations:**
- Advisory lock is session-level (`pg_advisory_lock`), not transaction-scoped, so it serializes topcat processes without piggybacking on any single DDL transaction.
- Each subgraph opens a fresh transaction in `apply.rs`; explicit commit
  releases object locks before the next subgraph starts.
- Integration test asserts advisory-lock-held duration and that object
  locks from subgraph N are released before subgraph N+1 acquires any.
- Treat "transaction spanning multiple subgraphs" as a test-time invariant
  violation, not merely a perf concern.

### Parallel subgraph apply deferred to M20

Architecture §Performance and caching allows independent subgraphs to run
in parallel up to a configurable `worker_count` (default 4). Shipping
parallelism inside M12 explodes the testing surface (interleaved
advisory-lock semantics, per-worker connection pooling, deadlock risk on
cross-subgraph FK edges that M7 missed).

**Day-one mitigations:**
- M12 pins `worker_count = 1`; the subgraph loop in `apply.rs` is a plain
  sequential iterator.
- Config key `worker_count` exists but is clamped to 1 on read with a
  debug-log line noting "pinned in M12; see M20".
- Subgraph partitioning in `subgraph.rs` already produces an ordered list
  of independent subgraphs, so M20's parallel executor is a drop-in
  replacement, not a refactor.

### Idempotence under crash

`state = in_progress` rows must be retryable from scratch. Any per-subgraph
step that is not idempotent (e.g. appending to a non-transactional log
file, issuing a one-shot side-effect) leaks state on retry.

**Day-one mitigations:**
- Every per-subgraph side effect is either (a) inside the transaction or
  (b) derived purely from the candidate SchemaModel + `node_registry`.
- `recovery.rs` treats recovery as "run the same subgraph apply again,
  which will `DROP ... CASCADE` the partially-created state and re-emit".
- Fault-injection test in Phase 3 kills the process in three distinct
  places: between `DROP` and `CREATE`, between `CREATE` and `COMMIT`, and
  between `COMMIT` and `sync_log` update. All three must converge on the
  next `topcat sync`.

## Tests

### End-to-end (primary)

1. **Happy path — column change.** Start dev db at state A. Mutate a
   working-tree source file (add a column). Run `topcat sync`. Read dev db
   SchemaModel. Assert equal to candidate. Assert `node_registry` updated
   atomically; `sync_log` has one `done` row.
2. **Happy path — no-op.** Clean tree vs clean dev db. `topcat sync`
   produces zero subgraph transactions; final drift check passes.
3. **Crash recovery.** Start a sync with a fault-injection hook that
   `SIGKILL`s the process mid-apply of subgraph N. Start a fresh `topcat
   sync`. Assert:
   - `sync_log` has the crashed row retried to `done` (or a new row
     supersedes it, depending on implementation choice).
   - Dev db SchemaModel matches candidate.
   - No orphaned objects from the aborted DROP+CREATE.
4. **Repeat across the three kill points** listed under "Idempotence under
   crash" mitigations.
5. **Advisory lock serializes.** Spawn two `topcat sync` processes against
   the same dev db with a slow fixture. Assert the second process does not
   return until the first has exited cleanly.
6. **`--adopt` bootstrap.** Existing dev db with no node_ids. Run `topcat
   sync --adopt`. Assert: no subgraph transactions ran; every catalog
   object has a `node_registry` entry; no migration emitted.

### Unit tests (thin layer, integration is primary)

- `subgraph.rs` partition correctness on synthetic ChangeSets.
- `sync_log.rs` state transitions (`in_progress` → `done` / `failed`).
- `drift_check.rs` mismatch detection on forged SchemaModel pairs.

## Verification snapshot

After M12 ships, the CLI surface gains:

- `topcat sync` (new)
- `topcat sync --adopt` (new)
- `topcat sync --force` (new)

Running `topcat sync --help` prints the flags; running `topcat sync` on a
dev db with the event triggers installed drives the end-to-end loop;
`_topcat.sync_log` has rows after the first sync.

## Open questions to resolve before starting

From roadmap §Open questions item 2: **`worker_count` semantics** is
ambiguous — parallel across subgraphs, across object-types within a
subgraph, or across files? Architecture §Performance and caching implies
"across independent subgraphs" (object-DAG level) but does not state it in
so many words.

**Resolution for M12:** pick "across independent subgraphs" as the
intended-future stance, but pin `worker_count = 1` in code. M12 ships
single-worker regardless; the choice matters only for M20's implementation
plan. Confirm with the user at milestone kickoff that "parallel across
subgraphs" is the right stance so the partitioning code in `subgraph.rs`
produces the right shape of unit.

## Session sizing notes

Roadmap estimates ~2–3 sessions. The five phases above sum to
approximately 3 sessions:

- Session 1: Phase 1 + Phase 2 (skeleton + happy-path pipeline).
- Session 2: Phase 3 + Phase 4 (recovery + adopt/force flags).
- Session 3: Phase 5 (integration tests, observability events, perf
  measurement).

Compression to 2 sessions is plausible if the partitioning logic in
`subgraph.rs` stays trivial (one node per subgraph) and fault-injection
harness is reused from the M10 verifier test setup.

## Referenced architecture sections

All paths relative to repo root.

- `docs/pg-dev-buddy-architecture.md` §Sync (dev loop) — the pipeline.
- §Apply granularity — object-granular drop-cascade-recreate contract.
- §Transactions and failure recovery — `sync_log` schema and advisory lock.
- §Concurrency — single-developer local-dev-db scope.
- §Performance and caching — parallelism deferral and perf targets.
- §Observability — `sync.*` event catalog.
- §CLI surface — `topcat sync [--adopt] [--force]`.
- §Component map — `sync` module row.
- §Bootstrapping — "Adoption migration" paragraph (`--adopt` semantics).

Roadmap: `docs/pg-dev-buddy-roadmap.md` §M12; §Open questions item 2.

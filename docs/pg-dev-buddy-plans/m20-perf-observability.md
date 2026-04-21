# M20 — perf + observability hardening

> Ongoing hardening that rides on top of the pg-dev-buddy MVP (M0-M12). Lands
> the full observability event catalog from architecture §Observability,
> captures criterion benchmarks against the targets in architecture
> §Performance and caching, and parallelizes catalog reads (per-object-type)
> and DDL apply (across independent subgraphs, up to `worker_count`). Ordering
> is load-bearing: instrumentation first, benchmarks second, parallelism
> third — parallelism can mask correctness regressions, and without
> instrumentation we cannot see them.

## Context

After M12 ships, pg-dev-buddy has a functional MVP: dev sync works, forward
migrations generate and verify. What it does not yet have is the operational
surface the architecture promises.

Two gaps:

1. **Observability coverage is incomplete.** Architecture §Observability
   enumerates a fixed event catalog (sync, diff, migration, reconcile, parse,
   conflict, shadow, meta events). M0-M12 emit the subset required for their
   happy paths; M20 guarantees every catalogued event has at least one
   production emission site, with the full JSON schema shape (`ts`, `level`,
   `event`, `sync_id`, `migration_id`, `node_id`, `file`, `pg_address`,
   `message`, `details`).
2. **Performance is unmeasured and serial.** Architecture §Performance and
   caching lists six binding targets (e.g. incremental sync on 1,000 objects
   in under 15s). M0-M12 made no parallelism commitments. `CatalogReader`
   runs its per-object-type queries sequentially; DDL apply walks subgraphs
   one at a time.

M20 closes both gaps. It exists as a dedicated milestone because the risk
profile is unique — parallelism is where correctness bugs hide, and
benchmarks without instrumentation cannot explain regressions. The phases
below enforce this ordering.

This milestone is explicitly **open-ended** per the roadmap. It is a
standing bucket of hardening work rather than a single-session ship.

## Prerequisites

Before starting M20, confirm these concrete artifacts from the MVP path are
in place:

- **From M12** (`sync` MVP):
  - `src/sync/` orchestrating the §Sync (dev loop) pipeline.
  - `_topcat.sync_log` checkpointing with `(sync_id, subgraph_root, state)`.
  - Session-level advisory lock `pg_advisory_lock(hashtext('topcat_sync'))`.
  - Per-subgraph transactions (architecture §Transactions and failure
    recovery).
- **From M7** (`object_dag`):
  - `src/object_dag/` with subgraph extraction (the connected components of
    the reconciled DAG, which are the units of parallel apply).
  - Stable traversal order (`stable_topo`) so parallel workers produce
    deterministic inter-subgraph orderings.
- **From M5** (`event_trigger`):
  - `_topcat_ddl_end` and `_topcat_sql_drop` triggers live on the dev db.
  - Registry population atomic with DDL transactions.
- **From M2** (`schema_model` + `catalog_reader`):
  - `src/pg/catalog_reader/` with one typed reader per object type (the
    unit of parallel catalog-read).
- **From M3** (`ddl_emitter`):
  - `src/pg/ddl_emitter/` as the single SchemaModel → DDL path (parallel
    apply calls into this; it must be `Send + Sync` or wrappable in an
    `Arc`).
- **From M6** (`shadow_db`):
  - `src/shadow_db/` with cache-keyed template reuse (measurement target
    in §Performance and caching).
- **From M1** (`pg/client`):
  - `src/pg/client/` connection pool. Parallel readers and appliers both
    check out connections from this pool; pool sizing must cover
    `worker_count` × (reader + applier) with headroom.
- **Existing tooling**:
  - `rayon = "1.10"` already in `Cargo.toml` — no dep debate needed.
  - `tracing` / structured-log plumbing already exists in `logging.rs`;
    M20 extends rather than replaces.
  - `criterion` must be added as a dev-dependency with `[[bench]]` entries
    in `Cargo.toml` — M20 is the first milestone to introduce it.

If any of the above are missing or the shapes differ from what this plan
assumes, surface it at M20 kickoff and reconcile before writing code. In
particular, `object_dag` must expose a subgraphs iterator that yields
independent connected components; without it, cross-subgraph parallelism
has no unit.

## Scope

### In scope

- Full event-catalog coverage from architecture §Observability — every
  listed event name has at least one production emission site with the
  full JSON shape.
- Criterion benchmarks mirroring each row of the §Performance and caching
  target table (incremental sync at 100 and 1,000 objects, fresh sync,
  migrate generate, shadow rebuild template, shadow rebuild scratch).
- Parallel catalog reads: per-object-type queries run in parallel across a
  rayon pool, bounded by `worker_count`.
- Parallel DDL apply: independent subgraphs from `object_dag` apply in
  parallel, bounded by `worker_count`.
- Regression gates: benchmark CI and an instrumentation-coverage test that
  fails the build if a catalogued event is never emitted.
- `--log-format json|text`, `--log-level`, `--log-file` global flags
  finalized per architecture §Observability (M0 stubbed these; M20
  guarantees their behavior).

### Out of scope

- **Extending the event catalog.** The list in §Observability is fixed;
  M20 ensures coverage, not expansion. Adding a new event requires an
  architecture doc change first.
- **New parallelism units.** Cross-file parallelism is explicitly not
  introduced — files are not the unit of apply in object-granular sync.
  Within-object-type parallelism for catalog reads is also out of scope;
  parallelism stops at "one reader per object type".
- **Tracing spans vs logs.** Architecture specifies structured JSON logs,
  one object per line. M20 does not introduce OpenTelemetry / distributed
  tracing.
- **Shared-dev-db concurrency.** Architecture §Concurrency explicitly
  rules this out; `worker_count > 1` is for intra-process parallelism
  only, not multi-process coordination.
- **Benchmark targets for M13-M19 features.** Reverse migrations, bounces,
  split/merge, reconcile, bootstrap, uninstall, codegen — benchmark
  scaffolding is reusable, but their target rows don't exist in
  §Performance and caching, so no commitments.

## Deliverables (files and modules)

### Observability

- `src/logging.rs` (extend): builder for the full event envelope per
  §Observability. One `emit!` macro (or equivalent) that takes `event`,
  optional `sync_id`, `migration_id`, `node_id`, `file`, `pg_address`,
  `details`, and enforces the schema shape at compile time where
  possible (e.g. event name as an enum, not a string literal).
- `src/logging/events.rs` (new): `Event` enum with one variant per
  catalogued event name. Discriminant renders to the exact string in
  §Observability (`sync.start`, `diff.edge.conflict`, etc.). Adding an
  event requires editing this file plus the architecture doc — the two
  stay in lockstep.
- Emission sites wired across every module that owns the relevant state
  transition:
  - `src/sync/` → `sync.*`, `shadow.*` (where sync drives shadow rebuild)
  - `src/pg/differ/` → `diff.edge.conflict`, `diff.stale_declared`,
    `diff.unknown_target`
  - `src/pg/migration_generator/` → `migration.generate.*`
  - `src/pg/migration_verifier/` → `migration.verify.*`
  - `src/reconcile/` → `reconcile.*` (M16; may be stubbed if M16 not yet
    landed, with the emission site guard-gated on the module existing)
  - `src/pg/body_parser/` → `parse.*`
  - `src/pg/conflict_scanner/` → `conflict.*`
  - `src/shadow_db/` → `shadow.rebuild.*`, `shadow.cache.*`
  - `src/meta_migrations/` → `meta.schema.upgrade.*`
- `src/logging/coverage_test.rs` (new): build-time (or `#[test]`)
  assertion that every `Event` enum variant has a `grep`-findable
  emission site in `src/` outside the events module itself. Regression
  gate against coverage decay.

### Performance

- `benches/` (new directory). Criterion bench files, one per target row
  in §Performance and caching:
  - `benches/sync_incremental.rs` — 100 and 1,000 object fixtures, 5%
    changed. Targets: < 3s, < 15s.
  - `benches/sync_fresh.rs` — 1,000 objects from empty. Target: < 60s.
  - `benches/migrate_generate.rs` — diff two shadows at 1,000 objects.
    Target: < 10s.
  - `benches/shadow_rebuild_template.rs` — template reuse. Target: ~2s.
  - `benches/shadow_rebuild_scratch.rs` — 1,000 objects from scratch.
    Target: < 90s.
- `Cargo.toml` (extend): `[[bench]]` entries, `criterion =
  { version = "0.5", features = ["html_reports"] }` under
  `[dev-dependencies]`.
- `benches/common/mod.rs` (new): fixture-building helpers shared across
  benches (deterministic schema generators by object count, shadow-cluster
  provisioning harness).

### Parallel catalog reads

- `src/pg/catalog_reader/` (extend): driver function that invokes the
  per-object-type readers via `rayon::join` / `par_iter` over the static
  list of object types. Each reader gets its own pooled connection.
  Output merged into one `SchemaModel` in deterministic order (by object
  type discriminant, then by `pg_address`).
- Connection-pool sizing audit: `pg/client` pool size must be at least
  `worker_count + 2` (one slot for event-trigger-install, one for
  meta-schema maintenance, plus one per worker).

### Parallel DDL apply

- `src/sync/apply.rs` (extend; M12 lands it first): given the subgraph
  iterator from `object_dag`, dispatch up to `worker_count` subgraphs at
  a time via `rayon::scope`. Each worker:
  - Acquires its own connection from `pg/client`.
  - Opens its own per-subgraph transaction.
  - Takes the per-subgraph advisory lock (distinct from the
    session-level `topcat_sync` lock, which is held by the main sync
    process for the lifetime of the whole sync).
  - Calls `ddl_emitter` for each dropped → recreated node.
  - Writes `_topcat.sync_log` entries for its subgraph.
- `src/sync/worker_pool.rs` (new, optional): thin wrapper around
  `rayon::ThreadPoolBuilder` that binds the pool to `worker_count`.
  Deterministic ordering of subgraph dispatch (by `subgraph_root` UUIDv7,
  which gives time-ordered determinism across runs when the input is
  the same).

### Regression gates

- `tests/perf_regression.rs` (new): smoke-run-style integration test
  that runs each bench in `--test` mode and asserts runtime within a
  relaxed multiple (e.g. 3×) of the §Performance and caching target.
  CI-visible; not an allocator-benchmark replacement.
- `tests/observability_coverage.rs` (new): enumerates `Event` variants,
  `grep`s `src/` for each variant's discriminant string, fails if any
  variant has zero emission sites outside `src/logging/events.rs`.

## Implementation phases

Phase ordering enforces the top-level risk mitigation: parallelism can
hide correctness regressions, so observability lands first and benchmarks
capture the serial baseline before any parallelization.

### Phase 1 — observability scaffolding (no parallelism)

- Introduce `src/logging/events.rs` with the full `Event` enum.
- Extend `src/logging.rs` with the envelope builder and `emit!`
  equivalent that produces the §Observability JSON shape.
- Wire emission sites across every module that owns a state transition
  in the event catalog (see Deliverables list).
- Add `tests/observability_coverage.rs`. Passes once every variant has
  at least one emission site.
- Finalize `--log-format`, `--log-level`, `--log-file` behavior against
  the schema.

Gate: every event in §Observability fires in integration tests;
coverage test green.

### Phase 2 — benchmarks capturing the serial baseline

- Add `criterion` dev-dep, `[[bench]]` entries, `benches/` scaffolding.
- Write benches for each §Performance and caching row.
- Run benches on the current (serial) implementation and record results
  in the repo (e.g. `benches/baseline.json`) as the pre-optimization
  baseline. These numbers document what we had before parallelism —
  they are the reference for detecting correctness-masking regressions.
- Add `tests/perf_regression.rs` with targets relaxed to baseline × 1.5
  (not the §Performance and caching targets yet — those are the goal of
  Phase 4).

Gate: benches run, baselines captured, regression test in CI.

### Phase 3 — parallel catalog reads (lower-risk parallelism)

- Parallelize `CatalogReader::read` across object-type readers. Reads
  are side-effect-free; the correctness surface is merging output
  deterministically.
- Connection-pool audit: confirm `pg/client` pool sizing covers the new
  concurrency.
- Determinism test: read the same db twice in parallel mode, assert
  byte-identical SchemaModel (rules out non-deterministic merge).
- Bench delta: compare Phase 2 baselines to Phase 3 numbers; regression
  in any non-read-dominated bench is a red flag.

Gate: determinism test green; bench improvements on shadow-rebuild-scratch
and migrate-generate; no regression elsewhere.

### Phase 4 — parallel DDL apply across subgraphs (higher-risk)

- Wire `rayon::scope`-based dispatch in `src/sync/apply.rs`.
- Lock discipline: session-level `topcat_sync` advisory lock held by
  the main process (serializes concurrent topcat CLIs); per-subgraph
  locks held only within each worker's transaction (serializes workers
  against external writers to that subgraph).
- Fault-injection tests: kill a worker mid-apply, assert `_topcat.sync_log`
  `state = in_progress` rows are retried idempotently on next sync.
- Ratchet `tests/perf_regression.rs` from baseline × 1.5 toward the
  §Performance and caching targets once bench numbers land under them.

Gate: all M12 sync tests still green under `worker_count > 1`;
fault-injection tests green; bench numbers meet §Performance and
caching targets.

### Phase 5 — regression ratchet and CI wiring

- CI job runs benches on a representative machine profile, compares to
  baseline, fails on regression beyond tolerance.
- `tests/perf_regression.rs` now asserts the §Performance and caching
  targets directly (no relaxation multiplier).
- Document the observability contract in `CLAUDE.md` / user-facing docs
  so downstream milestones (M13+) know to emit new events via the enum.

Gate: CI green; benches within targets; observability coverage test
green; no `println!` / `eprintln!` for diagnostics in `src/`.

## Risks (copied and amplified)

### Risk: parallelism can hide correctness regressions

From roadmap §M20: "parallelism can hide correctness regressions. Land
instrumentation first, optimize second."

Day-one mitigations:

- **Phase ordering is the mitigation.** Observability (Phase 1) and
  baselines (Phase 2) are non-negotiable prerequisites to any
  parallelism work. A plan that ships parallelism without
  instrumentation is a plan that ships undiagnosable regressions.
- **Determinism test before and after parallelization.** Read the same
  db twice, assert byte-identical SchemaModel under `worker_count = 1`
  and `worker_count > 1`. Non-determinism appearing at high
  `worker_count` is a correctness bug masquerading as a perf win.
- **Fault-injection under parallelism.** M12 fault-injection tests
  already exercise crash recovery; M20 re-runs them with
  `worker_count > 1`. A parallel apply that doesn't leave
  `_topcat.sync_log` in a retryable state is a correctness regression.
- **Advisory-lock discipline explicit in code review.** The
  session-level `topcat_sync` lock vs per-subgraph locks interaction is
  the highest-risk correctness surface. Code review gate: every lock
  acquisition has a comment stating scope and exclusion invariant.

### Risk: `worker_count` misapplied

The roadmap explicitly flags `worker_count` semantics as an open
question. Misapplication — e.g. spawning N workers per subgraph rather
than N across subgraphs — produces connection-pool exhaustion and
non-deterministic ordering.

Day-one mitigations:

- **Proposal baked into deliverables.** `worker_count` means: (a)
  parallel object-type readers at catalog-read time, and (b) parallel
  independent subgraphs at apply time. Nowhere else. Not per-file, not
  within-subgraph.
- **Single `ThreadPoolBuilder` sized exactly to `worker_count`.**
  Other rayon calls inside the pool inherit it; no second unbounded
  pool.
- **Connection-pool sizing test.** Assert `pg/client` pool size ≥
  `worker_count + 2`; fail-closed at startup otherwise.

### Risk: benchmark numbers drift silently

Criterion benches without CI enforcement rot into documentation. A
green test suite can coexist with a 2× perf regression.

Day-one mitigations:

- **Regression gate lives in `tests/`, not `benches/`.** Integration
  test runs a reduced version of each bench under `cargo test`, so the
  main CI job catches regressions without running full criterion
  overhead.
- **Baseline committed to repo.** `benches/baseline.json` is reviewed
  in PRs; a knowingly-accepted regression requires updating the
  baseline with a justification in the PR description.
- **Benchmark targets are the §Performance and caching table.** M20
  does not invent targets; it measures against the ones the
  architecture committed to.

### Risk: observability coverage test false-negatives

The coverage test `grep`s for event discriminant strings. If an
emission site is gated behind a feature flag or a non-MVP module (e.g.
`reconcile` before M16 lands), the test falsely passes or falsely
fails.

Day-one mitigations:

- **Variants have an explicit `phase` annotation** marking which
  milestone introduces the emission site. Coverage test only requires
  variants whose `phase` has shipped.
- **Feature-gated emission sites included in the grep.** The grep runs
  against `src/` including feature-gated modules — cfg gating affects
  compilation, not source-text search.

## Tests

### Unit tests

- `Event` → discriminant string round-trip: every variant renders to its
  exact §Observability name and parses back.
- Log envelope builder: required fields (`ts`, `level`, `event`) always
  present; optional fields (`sync_id`, `node_id`, `file`, `pg_address`,
  `details`) omitted when None rather than serialized as `null` where the
  schema allows either (match architecture §Observability example
  literally).

### Instrumentation regression tests

- `tests/observability_coverage.rs` — enumerates `Event` variants, asserts
  each has ≥ 1 emission site in `src/` outside `src/logging/events.rs`.
  Runs on every build; failure means an event was orphaned.
- `tests/observability_shape.rs` — integration test that runs
  representative operations (a sync, a migrate generate, a shadow rebuild)
  with `--log-format json`, parses every line, asserts schema conformance
  against the §Observability shape.

### Criterion benches

One bench per §Performance and caching row. Not unit-tested for value
assertions (`criterion` owns that); the `tests/perf_regression.rs`
companion asserts runtime bounds under `cargo test`.

### Parallelism regression tests

- Determinism under `worker_count = 1` vs `worker_count = 8`: identical
  `SchemaModel` output from catalog read; identical `_topcat.sync_log`
  row set from sync apply.
- Fault-injection: kill worker mid-apply, next sync converges.
- Pool-exhaustion: `worker_count = pool_size - 1`, `worker_count =
  pool_size`, `worker_count = pool_size + 1` — last case fails-closed at
  startup, not at runtime.

### Property tests (optional)

- Commutativity of catalog-read merge: running parallel readers in any
  order yields the same SchemaModel bytes.

Follow `creating-tests-topcat` for helpers and assertion styles; follow
`testing-topcat` for integration-test harness setup.

## Verification snapshot

Copy-pasteable commands run after each phase lands:

```bash
cd /Users/josha/oss/topcat
nix develop --command cargo build
nix develop --command cargo test --package topcat
nix develop --command cargo test --package topcat observability_coverage
nix develop --command cargo test --package topcat observability_shape
nix develop --command cargo test --package topcat perf_regression
nix develop --command cargo clippy --package topcat -- -D warnings
nix develop --command cargo bench --package topcat
```

Expected:

- Build green.
- `observability_coverage` and `observability_shape` tests pass —
  every §Observability event has a production emission site and the
  JSON shape matches.
- `perf_regression` tests pass — each §Performance and caching target
  met or under the relaxed multiplier (depending on phase).
- Clippy clean on new modules.
- Criterion benches report numbers within the §Performance and caching
  targets (Phase 4 onward).

Phase-gated acceptance:

- Phase 1 complete: `observability_coverage` and `observability_shape`
  pass; no other tests change.
- Phase 2 complete: benches run; `baseline.json` committed.
- Phase 3 complete: parallel-catalog-read determinism test passes; no
  regression in non-parallelized benches.
- Phase 4 complete: parallel-apply determinism and fault-injection
  tests pass; §Performance and caching targets met.
- Phase 5 complete: CI benchmark job wired; targets asserted without
  relaxation multiplier.

## Open questions to resolve before starting

1. **`worker_count` semantics.** Architecture §Config surface ships
   `worker_count = 4` but §Performance and caching and §Sync (dev loop)
   are ambiguous about its scope. Roadmap §"Open questions" #2 lists
   the three candidates: across subgraphs, within object-types within
   a subgraph, or across files. **Proposal**: `worker_count` applies
   in exactly two places — (a) per-object-type parallel readers at
   catalog-read time (rayon `par_iter` over the static object-type
   list, capped at `worker_count`), and (b) independent-subgraph
   parallel appliers at DDL apply time (capped at `worker_count`).
   Files are not the unit of apply in object-granular sync, so
   cross-file parallelism does not exist. Within-subgraph parallelism
   is explicitly out (pg safety — catalog-level serialization inside
   an object family). Confirm with the architecture doc author before
   Phase 3.
2. **Criterion result persistence format.** Baselines in
   `benches/baseline.json` (criterion's native format) or a
   hand-rolled JSON summary? **Proposal**: criterion's native format,
   committed to the repo. Ubiquitous tooling, easier PR review.
3. **Benchmark hardware profile.** `cargo bench` numbers are
   machine-dependent. **Proposal**: CI runs benches on a fixed
   instance type; developer-machine numbers are informational only.
   Locking this down before Phase 5 wiring.
4. **Feature-gated emission sites and coverage.** Some events
   (`reconcile.*`) belong to a milestone (M16) that may not have
   shipped when M20 begins. **Proposal**: `Event` variants carry a
   `phase` annotation; coverage test checks only variants whose phase
   has landed. Document the gating rule in `CLAUDE.md`.
5. **`--log-format text` fallback shape.** Architecture §Observability
   specifies JSON; text format is mentioned only as a CLI flag.
   **Proposal**: text format is human-readable rendering of the JSON
   envelope (one line per event, key fields surfaced); not a separate
   event schema. Text format is a debugging convenience; the JSON
   shape is the contract.
6. **Per-subgraph advisory lock key.** The session-level lock is
   `hashtext('topcat_sync')`. **Proposal**: per-subgraph locks use
   `hashtext('topcat_subgraph:' || subgraph_root::text)`. Distinct
   32-bit keyspace from the session lock; deterministic per subgraph;
   won't collide with user locks under normal use.

Resolve each at the start of the M20 session (or the start of the
phase that first blocks on it) or escalate.

## Session sizing notes

Roadmap estimate: **open-ended**. M20 is a standing hardening bucket,
not a single-session ship.

Recommended session breakdown — one phase per session, independently
landable:

- Session 1: Phase 1 — observability scaffolding. Full event-catalog
  coverage, coverage test, shape test. Touches every module with a
  state transition but adds no behavior.
- Session 2: Phase 2 — criterion benches and baselines. Independent
  of Phase 1 in principle, but sequenced after it so regressions
  introduced by later phases can be explained via the log stream.
- Session 3: Phase 3 — parallel catalog reads. Read-only parallelism;
  determinism test is the acceptance gate.
- Session 4: Phase 4 — parallel DDL apply. Highest-risk session; may
  itself split into 4a (wiring) and 4b (fault-injection hardening).
- Session 5: Phase 5 — regression ratchet and CI wiring.

Phases 3-5 can reorder only if determinism tests do not regress.
Phases 1 and 2 must precede 3-5. If the MVP plus M13-M19 work
introduces new hot-path operations, additional bench rows land in
follow-up sessions against this same milestone number.

Single-phase completion is viable if:

- The module already emits to `tracing` and only needs discriminant-enum
  discipline (Phase 1 shrinks).
- Subgraph iteration from M7 yields clean independent components with
  no fix-up work (Phase 4 shrinks).
- Connection-pool sizing is already parameterized on `worker_count`
  (Phases 3 and 4 shrink).

## Referenced architecture sections

- §Observability — event catalog, JSON envelope shape, log levels, and
  CLI flag behavior.
- §Performance and caching — six binding target rows, caching policy,
  `worker_count` default.
- §Sync (dev loop) — pipeline shape; parallel apply rides on the
  per-subgraph structure.
- §Transactions and failure recovery — `_topcat.sync_log` checkpointing,
  session-level advisory lock, per-subgraph transaction model.
- §Concurrency — single-dev-db assumption; `worker_count` is
  intra-process, not cross-process.
- §Config surface — `worker_count = 4` default and its config location.
- §Edge authority — subgraph components produced by the reconciled
  object DAG are the units of parallel apply.

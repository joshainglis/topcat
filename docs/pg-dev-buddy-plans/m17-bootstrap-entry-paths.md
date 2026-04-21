# M17 — bootstrap entry paths

> Source of truth: docs/pg-dev-buddy-roadmap.md §M17, docs/pg-dev-buddy-architecture.md §Bootstrapping, §Bootstrap and pg_restore path, §CatalogReader, SchemaModel, and DDL emitter, §Event trigger, §Three-way node_id resolution, §Sync (dev loop).

## Context

M17 is the adoption surface: how an existing project starts using pg-dev-buddy. The architecture lists three on-ramps and one refinement:

1. **Fresh project** — empty dev db, first `CREATE` writes a node_id.
2. **Schema-as-code files, no topcat history** — topcat generates `-- requires:` from SQL discovery, applies files in File-DAG order, and the event trigger writes node_ids back to source.
3. **Migrations-only** — run `pg_dump --schema-only`, apply to an ephemeral db, then reverse-generate one source file per object via the M3 `ddl_emitter` (not from pg_dump text).
4. **Adoption migration** — `sync --adopt` records the initial state without emitting a migration. The flag itself ships in M11; M17 refines the recorded-state semantics and user-facing messaging.

M17 does not ship new correctness machinery. Every piece it needs — ddl_emitter, event triggers, node_id conflict resolution, the sync pipeline, File DAG, SQL discovery — already exists by the time this milestone starts. The work is orchestration and UX: stitch the pieces into three coherent flows, make header write-back safe, and make the first diff after adoption quiet.

The load-bearing risk is that the migrations-only path must produce source files bit-identical to what a steady-state M12 sync would produce. The architecture's mitigation is explicit and non-negotiable: **apply the dump, read the catalog, emit via `ddl_emitter`**. Never rewrite pg_dump text into topcat files.

## Prerequisites

M17 sits at the end of the MVP-plus branch. It depends on the M12 sync pipeline and on several earlier artifacts that are load-bearing for individual entry paths:

- **M1** — `_topcat` meta-schema install and `topcat meta-schema upgrade`. Fresh-project path installs this before anything else.
- **M3** — `src/pg/ddl_emitter` module (`emit(model: &SchemaModel) -> String`). The migrations-only path emits source files through this, never through pg_dump text. Per-object-type emitters must cover every type the catalog read returns.
- **M5** — `src/pg/event_trigger` install/uninstall, `_topcat_ddl_end`, `_topcat_sql_drop`, UUIDv7 node_id assignment proved end-to-end. Fresh-project and schema-as-code paths rely on the trigger to write node_ids as objects are created.
- **M11** — `-- node_id:` parse/write extension to `src/file_node/parsing.rs`, `node_alias_resolver` module, `topcat sync --adopt` initial scaffolding. M17 refines the `--adopt` flag's user-facing semantics (no migration emitted, adoption message, post-adopt first-diff is empty).
- **M12** — `src/sync/` orchestrator, `_topcat.sync_log` checkpointing, advisory lock discipline. Every M17 entry path terminates in a steady-state sync.
- **Existing topcat** — `src/commands/update.rs` (header write-back, rename-on-discovery), `src/file_node/parsing.rs` (header parser extended in M11), `src/sql_parser/` (SQL discovery used by path 2), `src/commands/import/pg_dump.rs` (reference only — M17 path 3 does *not* reuse this split-pg-dump-text path; it does apply-then-read).
- Nix dev shell with `cargo`, `postgresql_15`, `postgresql_16`, `postgresql_17` binaries (for the integration tests that exercise each path end-to-end).

## Scope

Roadmap verbatim: *"three adoption flows from architecture §Bootstrapping."*

### In scope for this milestone

- `topcat bootstrap` command with three subcommands (`fresh`, `from-files`, `from-dump`) that each orchestrate an entry path end-to-end and converge on a steady-state dev db.
- Non-destructive header write-back for path 2: preserve user-authored content, only mutate the header block, support dry-run.
- Catalog-read → `ddl_emitter` → source-file emission for path 3, including filename derivation, directory layout, and one-file-per-object default with a per-family override.
- `sync --adopt` refinement: after M11 wires the flag, M17 adds the "first diff is empty" assertion, the "no migration file generated" messaging, and an adoption-completion summary.
- Integration tests with one fixture project per path.
- Docs: a new `adopting-pg-dev-buddy` skill keyed off existing `discovering-sql-dependencies` patterns.

### Explicitly NOT in scope

- New normalization rules (M2 territory). If path 3's round-trip produces churn, the fix is an M2 patch, not an M17 workaround.
- New `ddl_emitter` coverage (M3 territory). If an object type emits incorrectly, fix it in the M3 module.
- Conflict resolution (M11 owns the three-way algorithm; M17 calls into it).
- Reverse migrations (M13).
- Uninstall (M18 — the mirror of M17).
- `topcat pause` / `topcat resume` bulk-restore flow (shipped with M5).

## Deliverables (files and modules)

| Kind | Path | Status | Purpose |
|---|---|---|---|
| module-root | `src/bootstrap/mod.rs` | new | Dispatch for the three entry paths. Public entry `run(settings, args) -> Result<BootstrapOutcome, TopCatError>`. |
| module | `src/bootstrap/fresh.rs` | new | Path 1: ensure `_topcat` installed (via `meta_migrations::upgrade`), install event triggers (via `pg::event_trigger::install`), print "dev db is ready — first `sync` will assign node_ids". No files touched. |
| module | `src/bootstrap/from_files.rs` | new | Path 2: run `update` logic (SQL discovery → `-- requires:` headers), build File DAG, topo-sort, apply files through `pg::client::apply_ddl` on a freshly-prepared db, then re-read `_topcat.node_registry` and write `-- node_id:` headers back via `header_generator`. Never destructive. |
| module | `src/bootstrap/from_dump.rs` | new | Path 3: apply `pg_dump --schema-only` output to an ephemeral db (reusing `shadow_db` from M6 for the throwaway), `catalog_reader::read(&client) -> SchemaModel`, iterate objects, call `ddl_emitter::emit_one(&object) -> String`, build a topcat-shaped source file per object, write to the target directory. |
| module | `src/bootstrap/emit_source_file.rs` | new | Shared: given a `schema_model::Object` and its emitted DDL, synthesize the topcat source file text (headers + emitted DDL), pick the filename (via M11's node-name → filename rules), choose the output path (per-family directory default). |
| module | `src/bootstrap/adopt.rs` | new | Refines M11's `sync --adopt`: runs a standard sync, asserts the post-apply diff is empty, writes an `_topcat.sync_log` row with `kind = 'adopt'`, and prints an adoption summary (node_id count, schema count, "first real migration will be clean"). |
| cli | `src/cli/bootstrap.rs` | new | `clap` `Args` group for `topcat bootstrap <fresh\|from-files\|from-dump>`. Flags: `--input <DIR>`, `--output <DIR>` (path 3), `--dump <FILE>` (path 3), `--mode <dry-run\|execute>` (default dry-run for paths 2/3), `--force`, `--adopt/--no-adopt` (default `--adopt` for paths 2/3). |
| command | `src/commands/bootstrap.rs` | new | Wrap `bootstrap::run`, route to the dispatch, return `BootstrapOutcome` to `main`. |
| extend | `src/main.rs` | extend | Register the new `Bootstrap` subcommand. |
| extend | `src/commands/update.rs` | extend | Expose the existing "discover → write headers" pipeline as a library function `update::discover_and_write(&config) -> Result<Vec<FileNode>, TopCatError>` so `bootstrap::from_files` reuses it without duplicating 445 lines. |
| extend | `src/file_node/parsing.rs` | extend | Path 2's write-back must be idempotent and preserve user content; add a `write_node_id_back(path: &Path, node_id: Uuid) -> Result<(), FileNodeError>` that rewrites only the header block (leaves body byte-identical). |
| extend | `src/header_generator/mod.rs` | extend | Add a `merge_node_id_headers` entry that accepts a `HashMap<NodeName, Uuid>` and applies to a `FileNode` set in memory, then delegates to existing in-place write logic. Non-destructive by construction: treats `-- node_id:` as additive, never rewrites other headers. |
| extend | `src/sync/mod.rs` | extend | Add `SyncOptions::adopt_mode: bool`. When true: skip migration emission, assert post-sync diff is empty, stamp `_topcat.sync_log.kind`. |
| integration tests | `tests/bootstrap_fresh.rs` | new | Empty dev db → `topcat bootstrap fresh` → assert `_topcat` installed, event triggers present, `node_registry` empty. |
| integration tests | `tests/bootstrap_from_files.rs` | new | Fixture `tests/input/bootstrap_from_files/` with 12 .sql files, some requiring discovery, no topcat headers → run path → assert files got `-- requires:` + `-- node_id:` headers and the dev db matches the file set. |
| integration tests | `tests/bootstrap_from_dump.rs` | new | Fixture `tests/input/bootstrap_from_dump/source.dump` (a real `pg_dump --schema-only` captured at test-prep time) → run path → assert emitted source files round-trip (apply back to a fresh db, re-read, SchemaModel equal to the original dump's SchemaModel). |
| fixture | `tests/input/bootstrap_from_files/` | new | Twelve-file schema-as-code project without topcat headers. Enough object types to exercise SQL discovery and layer routing. |
| fixture | `tests/input/bootstrap_from_dump/` | new | Dump file + expected-output directory tree (checked in so CI regressions on ddl_emitter show up as diff noise here). |
| skill | `.claude/skills/adopting-pg-dev-buddy/SKILL.md` | new | User-facing skill keyed off `adopting-pg-dev-buddy`, `bootstrap`, `adoption`. References existing `discovering-sql-dependencies`. |

## Implementation phases

### Phase 1 — scaffolding and path 1

Ship `src/bootstrap/mod.rs`, `src/cli/bootstrap.rs`, and `src/bootstrap/fresh.rs`. Wire the subcommand through `src/main.rs`. Path 1 is the smallest: it installs `_topcat` and the event triggers (both already implemented by M1/M5), so the work is glue and a clear "you are ready to sync" message. Integration test for path 1 locks the flow. ~0.3 session.

### Phase 2 — path 2 (schema-as-code files)

Reuse `update::discover_and_write` (Phase-2 prerequisite refactor of `src/commands/update.rs`). Add `src/file_node/parsing.rs::write_node_id_back` and the `header_generator::merge_node_id_headers` helper. Orchestration: run update, build File DAG, topo-sort, apply files via `pg::client::apply_ddl` in order, read `_topcat.node_registry`, write node_ids back. Dry-run preview prints the planned header mutations without touching files; execute mode mutates. Integration test covers both preview and execute, plus a "user-edited file mid-header" case that must not clobber user content. ~0.7 session.

### Phase 3 — path 3 (migrations-only, ddl_emitter round-trip)

This is the risky path. Pipeline:

1. Accept a `pg_dump --schema-only` file path.
2. Spin an ephemeral pg via the M6 shadow infrastructure (throwaway db; the cache isn't relevant here).
3. `psql -f <dump>` via `pg::client::apply_ddl` with `topcat.bootstrapping = true` so the event trigger doesn't record these objects (we don't want the ephemeral db's trigger noise).
4. `catalog_reader::read(&client)` → `SchemaModel`.
5. For each object: `ddl_emitter::emit_one(&object)` → DDL string; `bootstrap::emit_source_file` wraps it with headers (including a generated name, layer routing via auto-mapping, and a placeholder `-- node_id:` that the first real sync will fill in).
6. Write files to the target directory.
7. Final assertion: apply the emitted set to a *second* ephemeral db, re-read, assert `SchemaModel` equality with the first read. If this fails, abort — the ddl_emitter has a round-trip hole and M3 must fix it before M17 ships.

No new emitter coverage in this phase. If any object type has no emitter, fail-closed with a clear error pointing at the responsible M3 module. ~0.7 session.

### Phase 4 — adoption refinement and docs

Extend `sync --adopt` to record the adoption sync in `_topcat.sync_log` with `kind = 'adopt'` and print the summary. Write the `adopting-pg-dev-buddy` skill. Integration test asserts the first post-adoption `topcat migrate generate` produces an empty ChangeSet. ~0.3 session.

## Risks (copied and amplified)

- **Reverse-generation from pg_dump must match ddl_emitter output.** Roadmap risk verbatim; amplified: if the emitted source files differ from what a round-tripped sync would produce, every project onboarded via path 3 starts with a migration full of churn, undermining the adoption promise. Day-one mitigation: never read pg_dump text; apply the dump, read the catalog, emit via `ddl_emitter::emit_one`. Phase 3 step 7 (round-trip assertion on the emitted set) is the gate — if it fails, M17 is blocked on an M3 fix, not on M17 code.
- **Header write-back in path 2 must be non-destructive.** Roadmap risk verbatim; amplified: users who already edited a file between `topcat update` and the node_id write-back step will lose content if we rewrite the whole file. Day-one mitigation: `write_node_id_back` rewrites only the comment-prefixed header block (matches `^<comment_str>\s+[a-z_]+:`); the body (everything after the first non-header line) is byte-identical to the input. Property test: for every file in the path-2 fixture, emit, then diff body-only — must be zero bytes.
- **Event-trigger noise during path 3's dump-apply.** The trigger sees every catalog row from the dump and tries to assign node_ids, polluting `node_registry` with objects we'll then emit and re-apply. Day-one mitigation: run the dump-apply with `SET LOCAL topcat.bootstrapping = 'true'` (already supported by `pg::client::transaction`); verify `node_registry` is empty after the read step, before any emission.
- **Orphaned `node_registry` rows from path 2 re-runs.** Running path 2 twice on the same project (e.g. to add more files) must not produce duplicate registry rows or node_id collisions. Day-one mitigation: path 2 treats an existing `_topcat` schema as "resume, don't reinstall" — rerun `update`, apply only new files, only write node_ids for files that don't already have one.
- **Fixture dump drift.** `tests/input/bootstrap_from_dump/source.dump` is generated by a specific pg version, and pg_dump output format drifts. Day-one mitigation: check in the dump *and* the script that regenerates it; CI runs the regeneration on nightly to catch drift before a user does.
- **`--adopt` semantics overlap with M11.** M11 ships the flag; M17 refines. Risk: parallel work produces two implementations. Day-one mitigation: M17 only *extends* `SyncOptions`; the flag's parse path is owned by M11. If M11 hasn't landed by M17 start, this milestone blocks — no forking of the adopt path in a new module.

## Tests

- **Path 1 (fresh).** Empty dev db → `topcat bootstrap fresh` → assert `_topcat.schema_version` = current, event triggers installed, `node_registry` empty, exit 0. Re-run: no-op, exit 0, no noise.
- **Path 2 (from-files).** Fixture `tests/input/bootstrap_from_files/` without topcat headers → `topcat bootstrap from-files -i <fixture> --mode execute` → assert every .sql file got `-- requires:` + `-- node_id:` headers; file bodies (below the header block) byte-identical to input; dev db schema equals the fixture's intended state; post-run `topcat migrate generate` produces an empty ChangeSet.
- **Path 2 dry-run.** Same fixture, dry-run → zero files modified; stdout lists planned header mutations.
- **Path 2 partial re-run.** Add one new file to the fixture, re-run → only the new file gets headers written, existing node_ids preserved, no duplicate registry rows.
- **Path 3 (from-dump).** Fixture `tests/input/bootstrap_from_dump/source.dump` → `topcat bootstrap from-dump --dump <fixture> --output <tmpdir>` → emitted files round-trip (apply to fresh db, re-read, SchemaModel equals original read).
- **Path 3 ddl_emitter gap.** Inject a deliberately uncovered object type into a fixture dump → run path 3 → expect fail-closed with a diagnostic naming the missing emitter.
- **Adoption.** Run any path followed by `topcat sync --adopt` → assert `_topcat.sync_log` has a `kind = 'adopt'` row; immediately run `topcat migrate generate` → empty ChangeSet, exit 0.
- **Post-adoption first real change.** After adoption, modify one object in source, run `topcat migrate generate` → produces a migration file containing exactly the one change (no adoption-noise carry-over).

## Verification snapshot

When M17 is done, a developer can point `topcat bootstrap` at an existing project and end up with a fully-adopted dev db, source files carrying the full topcat header set, and an empty first-migration diff. All three paths produce the same steady state. The path-3 round-trip assertion passes on pg15, pg16, and pg17. `cargo test` is green; `cargo clippy -- -D warnings` is clean. The `adopting-pg-dev-buddy` skill loads and walks a user through picking a path.

## Open questions to resolve before starting

1. **File layout for path 3.** One file per object flat in `output/`, or `output/<schema>/<family>/<name>.sql`? The `import pg-dump` command already has a layout convention (see `src/commands/import/pg_dump.rs`). Reuse that layout or introduce a new one? Pick before Phase 3.
2. **Path-2 node_id write-back ordering.** Write back after each file applies, or collect and write at the end? Per-file is safer if the run aborts mid-way but adds N round-trips to `node_registry`. Decide and document.
3. **Path 3 dump source.** Does the user pass a file, or does topcat shell out to `pg_dump` against a provided URL? File-only keeps the dependency surface small; URL mode is more ergonomic. Start file-only, add URL mode only if requested.
4. **`--adopt` on path 1.** Path 1 starts from empty — `--adopt` is a no-op by definition. Flag error, warning, or silent pass-through? Pick silent pass-through and document.
5. **Interaction with `topcat pause` / `topcat resume` (M5).** Path 3's dump-apply step is conceptually similar to a bulk restore. Should it reuse `pause`/`resume`, or just set the bootstrapping flag directly? Reusing buys consistency; bypassing is simpler. Lean toward bypass since the ephemeral db is thrown away after the catalog read.
6. **Fixture regeneration cadence.** Path-3 fixtures drift with pg_dump. Weekly CI re-run, or monthly, or only when a pg minor version releases? Pick and encode in the skill.

## Session sizing notes

~2 sessions total, split roughly 50/50:

- **Session 1** — Phases 1 and 2: path 1 scaffolding plus the full schema-as-code flow (path 2) with header write-back safety. Writing `write_node_id_back` and proving its body-preservation property soaks up most of the session's risk budget.
- **Session 2** — Phases 3 and 4: the migrations-only path, the round-trip assertion on emitted source, adoption refinement, skill docs. Phase 3 is the riskier half; if the round-trip assertion fails on a real dump, session 2 ends with an M3 bug report rather than an M17 landing.

Parallelizable with M15 (split/merge), M16 (reconcile), M18 (uninstall) per the roadmap. If M11 slips, M17 path-2 and path-3 still ship (they generate node_ids via the event trigger); the `--adopt` refinement blocks until M11 lands.

## Referenced architecture sections

- §Bootstrapping — the three-path taxonomy that defines M17's surface.
- §Bootstrap and pg_restore path — `_topcat.bootstrapping` flag discipline, bulk-restore analogy for path 3's dump-apply.
- §CatalogReader, SchemaModel, and DDL emitter — contract for path 3's read-then-emit pipeline.
- §Event trigger — node_id assignment semantics driving paths 1 and 2.
- §Three-way node_id resolution — conflict rules that `sync --adopt` leans on.
- §Sync (dev loop) — the pipeline each entry path converges on.
- §`_topcat` meta-schema versioning — install sequencing for path 1.
- §Identity and node_id — header contract that path 2's write-back must honour.

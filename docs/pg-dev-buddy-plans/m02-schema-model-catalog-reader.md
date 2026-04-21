# M2 — schema_model + catalog_reader

> Source of truth: docs/pg-dev-buddy-roadmap.md §M2, docs/pg-dev-buddy-architecture.md §CatalogReader, SchemaModel, and DDL emitter, §Normalization rules, §Object types covered, §Partitioned tables and inheritance, §Component map.

## Context

M2 delivers the canonical typed representation of a PostgreSQL schema (`SchemaModel`) together with the readers that populate it (`pg/catalog_reader`). This is the single biggest milestone on the critical path — it underpins every downstream module (ddl_emitter, body_parser, differ, migration_generator, object_dag, codegen).

Per the roadmap (lines 16–19): *"Correctness first, orchestration last. SchemaModel + ddl_emitter with round-trip property tests are the foundation. A wrong normalization rule in M2/M3 corrupts every downstream artifact."* M2 owns "semantic equivalence, not textual" normalization (architecture §Normalization rules). Miss one hot-spot and the differ emits spurious changes forever.

M2 also ships the first user-visible observability command: `topcat schema dump [--format json]`. It produces the JCS JSON (RFC 8785) serialization that every later module consumes and compares.

## Prerequisites

Depends on M1 having shipped the following concrete artifacts:

- `pg/client` module (src/pg/client/): URL parsing, connection pool, typed error taxonomy (superuser missing, connection refused, version unsupported, etc.).
- `meta_migrations` module with v1 DDL installed: tables `_topcat.schema_version`, `_topcat.node_registry`, `_topcat.node_aliases`, `_topcat.sync_log`, `_topcat.migration_registry`, plus `_topcat.gen_uuidv7()` function. (Event triggers deferred to M5.)
- `topcat meta-schema upgrade` command wired up.
- Bootstrap-flag discipline (`SET LOCAL topcat.bootstrapping = 'true'`) usable by later milestones — M2 itself never writes DDL.
- `[postgres]` config block with `dev_db_url` and superuser handling from M0.
- Integration-test harness against ephemeral pg15/16/17 established in M1.

M2 consumes M1's `pg/client` for all catalog queries; it does not open raw connections itself.

## Scope

### In scope for this milestone

- Canonical typed `SchemaModel` enum variants for every object type listed in architecture §Object types covered.
- Per-object-type readers in `pg/catalog_reader` that return typed row streams filtered against `pg_catalog`, `information_schema`, `_topcat`, and extension-owned objects (`pg_depend.deptype = 'e'`).
- All normalization rules from architecture §Normalization rules, applied inside the reader before the model leaves the module.
- Partition-tree and inheritance resolution (architecture §Partitioned tables and inheritance): `partition_of`, `partition_strategy`, `partition_bounds`, recursive sub-partition traversal without cycles or duplicate emission.
- JCS JSON serialization (RFC 8785): UTF-8, sorted keys, no insignificant whitespace, stable across runs.
- `topcat schema dump [--format json]` CLI command for observability and as the surface the tests diff against.
- Read-twice determinism property test.
- Fixture-based unit tests covering every object type × every normalization hot spot.

### Explicitly NOT in scope (deferred)

- `ddl_emitter` / SchemaModel → DDL — M3.
- Round-trip property test (emit → apply → read → equal) — M3 owns this; M2 only owns read → read determinism.
- `body_parser` / plpgsql inferred-edge extraction — M4. `Function.inferred_edges` field exists in the model with empty default; populated by M4.
- Event-trigger installation — M5. Reader runs against any live db regardless.
- `shadow_db` lifecycle — M6.
- `object_dag` / edge authority — M7.
- `differ`, `ChangeSet`, content-hash — M8.
- `migration_generator` — M9.
- `node_id` identity / registry wiring in the model — M11 extends `file_node`; M2 leaves `node_id` as an `Option<Uuid>` populated by catalog lookup against `_topcat.node_registry` when present, `None` otherwise.
- `codegen` plugins — M19.

## Deliverables (files and modules)

| Kind | Path | Status | Description |
|---|---|---|---|
| New module | `src/schema_model/mod.rs` | new | Top-level types, `SchemaModel` aggregate, JCS serialization entry point. |
| New module | `src/schema_model/table.rs` | new | `Table`, `Column`, `Constraint`, `Index`, `RlsPolicy`, identity/serial/default union, partition fields. |
| New module | `src/schema_model/function.rs` | new | `Function`, `Procedure`, args with modes/defaults, `parse_status`, empty `inferred_edges`. |
| New module | `src/schema_model/view.rs` | new | `View`, `MaterializedView`; canonical `pg_get_viewdef(…, true)`. |
| New module | `src/schema_model/sequence.rs` | new | `Sequence` with identity linkage. |
| New module | `src/schema_model/type_.rs` | new | `Type` enum: `Enum`, `Composite`, `Domain`, `Range`, `Multirange`. |
| New module | `src/schema_model/trigger.rs` | new | User-managed `Trigger` (topcat's own `_topcat_ddl_end`/`_topcat_sql_drop` excluded by name). |
| New module | `src/schema_model/policy.rs` | new | RLS `Policy` record, referenced from `Table.rls_policies`. |
| New module | `src/schema_model/rule.rs` | new | `Rule` records. |
| New module | `src/schema_model/schema.rs` | new | `Schema`, `Extension`. |
| New module | `src/schema_model/text_search.rs` | new | Text search configs/dictionaries/parsers/templates. |
| New module | `src/schema_model/fdw.rs` | new | Foreign data wrapper, server, user mapping. |
| New module | `src/schema_model/operator.rs` | new | Operators, operator classes, operator families. |
| New module | `src/schema_model/aggregate.rs` | new | Aggregate functions. |
| New module | `src/schema_model/cast.rs` | new | Casts and conversions. |
| New module | `src/schema_model/statistics.rs` | new | Extended statistics objects. |
| New module | `src/schema_model/publication.rs` | new | Publications. |
| New module | `src/schema_model/access_method.rs` | new | Access methods. |
| New module | `src/schema_model/event_trigger.rs` | new | User-managed event triggers (topcat's own excluded by name). |
| New module | `src/schema_model/acl.rs` | new | ACL/grant representation, per-object. |
| New module | `src/schema_model/jcs.rs` | new | RFC 8785 JSON Canonicalization (sorted keys, UTF-8, no whitespace). |
| New module | `src/schema_model/tests.rs` | new | Unit tests for model construction and JCS serialization determinism. |
| New module | `src/pg/catalog_reader/mod.rs` | new | Reader façade: `fn read_schema(&Client) -> SchemaModel`, dispatches per object type, applies `pg_depend.deptype = 'e'` filter and `_topcat`/`pg_catalog`/`information_schema` exclusion. |
| New module | `src/pg/catalog_reader/tables.rs` | new | Tables, columns, constraints, indexes, RLS, partition tree. |
| New module | `src/pg/catalog_reader/functions.rs` | new | Functions, procedures; exact `pg_proc.prosrc` capture. |
| New module | `src/pg/catalog_reader/views.rs` | new | Views, matviews; `pg_get_viewdef(…, true)`. |
| New module | `src/pg/catalog_reader/sequences.rs` | new | Sequences. |
| New module | `src/pg/catalog_reader/types.rs` | new | Enum/composite/domain/range/multirange. |
| New module | `src/pg/catalog_reader/triggers.rs` | new | User triggers. |
| New module | `src/pg/catalog_reader/policies.rs` | new | RLS policies. |
| New module | `src/pg/catalog_reader/rules.rs` | new | Rules. |
| New module | `src/pg/catalog_reader/schemas.rs` | new | Schemas, extensions. |
| New module | `src/pg/catalog_reader/long_tail.rs` | new | Text search, FDW/server/user-mapping, operators, aggregates, casts, conversions, statistics, publications, access methods, event triggers. (Split into sub-files if this file exceeds ~400 lines.) |
| New module | `src/pg/catalog_reader/normalization.rs` | new | Shared normalization helpers: `varchar`/`text` collapse, identity/serial/default union, reloptions sort, collation version strip. |
| New module | `src/pg/catalog_reader/partition_walker.rs` | new | Partition-tree recursion (cycle-guarded, dedup-guarded). |
| New command | `src/commands/schema_dump/mod.rs` | new | `topcat schema dump [--format json]` — connects via `pg/client`, calls `catalog_reader::read_schema`, writes JCS JSON to stdout. (Register under existing `schema` command namespace.) |
| Extended | `src/cli/execution.rs` or new `src/cli/pg.rs` | extend | Expose `--format json` / db-url args for `schema dump`. |
| Extended | `src/main.rs` | extend | Wire `schema dump` subcommand to the new command module. |
| New tests | `tests/schema_model_jcs.rs` | new | JCS output byte-stable across runs. |
| New tests | `tests/catalog_reader_determinism.rs` | new | Read twice, assert structural and JCS-byte equality. |
| New tests | `tests/catalog_reader_fixtures.rs` | new | Per object-type × normalization-hot-spot coverage via fixture SQL files. |
| New fixtures | `tests/pg_fixtures/schema_model/*.sql` | new | Fixture schemas (one per hot spot); loaded via ephemeral pg from M1's harness. |
| Deps | `Cargo.toml` | extend | Add `postgres` (sync client; no tokio), `uuid` (for `node_id` and partition sanity), `sha2` (JCS hash helper — if not already present), `rust_decimal` or equivalent only if numeric normalization demands it. Prefer a JCS helper via existing `serde_json` + manual sort before adding a new crate. |

Object-type coverage target (exhaustive, from architecture §Object types covered): **tables (incl. partitioned), columns, constraints (PK, FK, UNIQUE, CHECK, EXCLUDE), indexes, functions, procedures, views, materialized views, sequences, types (enum/composite/domain/range/multirange), triggers, policies, rules, schemas, extensions, text search configs/dictionaries/parsers/templates, foreign data wrappers/servers/user mappings, operators, operator classes/families, aggregates, casts, conversions, statistics objects, publications, access methods, user-managed event triggers.** Topcat's own event triggers (`_topcat_ddl_end`, `_topcat_sql_drop`) excluded by name.

## Implementation phases

The roadmap recommends an M2a/M2b split (lines 179–182). Honor it.

### Phase M2a — Core object types + determinism scaffolding

- **Goal**: Land the model shape, JCS serializer, reader façade, normalization helpers, and all "core" object types so later milestones can start designing against a stable contract. M2a alone is enough for M3's ddl_emitter to begin for the common case.
- **Tasks**:
  1. Create `src/schema_model/` with `mod.rs` plus types for: `Schema`, `Extension`, `Table`/`Column`/`Constraint`/`Index`/`RlsPolicy`, `Function`/`Procedure`, `View`/`MaterializedView`, `Sequence`, `Type` (all 5 variants). Derive `Serialize`, `Deserialize`, `PartialEq`, `Eq`, `Hash` where stable.
  2. Implement `schema_model/jcs.rs` (RFC 8785): sorted keys, UTF-8, no insignificant whitespace, number canonicalization. Unit-test byte-stability.
  3. Create `src/pg/catalog_reader/mod.rs` with the façade `pub fn read_schema(client: &pg::Client) -> Result<SchemaModel, ReaderError>`, plus `normalization.rs` and `partition_walker.rs`.
  4. Implement readers for the M2a object set. Apply the full §Normalization rules table inside each reader.
  5. Partition-tree recursion: walk by `pg_inherits`, guard against cycles (shouldn't happen, but fail-loud), deduplicate multi-level emissions.
  6. Wire `topcat schema dump [--format json]` under the existing `schema` command namespace.
- **Verification**:
  - `cargo build` clean; `cargo clippy` clean.
  - `cargo test schema_model::` passes including JCS byte-stability.
  - `cargo test --test catalog_reader_determinism` passes for M2a object set.
  - Manual: `topcat schema dump --format json` against a fixture db produces byte-identical output on two runs (`diff` returns empty).

### Phase M2b — Long-tail object types

- **Goal**: Close the coverage gap so M7/M8/M9 never hit an "unsupported object type" surprise.
- **Tasks**:
  1. Add schema_model types and readers for: triggers, policies (as standalone module too, even though referenced from Table), rules, text search configs/dictionaries/parsers/templates, FDW/server/user-mapping, operators, operator classes/families, aggregates, casts, conversions, statistics objects, publications, access methods, user-managed event triggers.
  2. Exclude topcat's own event triggers (`_topcat_ddl_end`, `_topcat_sql_drop`) by name. Exclude `_topcat`-schema objects wholesale.
  3. Extend the extension-owned filter (`pg_depend.deptype = 'e'`) to the long tail.
  4. Expand fixtures to cover every long-tail type.
- **Verification**:
  - `cargo test --test catalog_reader_fixtures` covers every object type with at least one fixture.
  - Read-twice determinism holds on a schema that exercises every long-tail type.
  - `topcat schema dump --format json` on the all-types fixture emits every object type at least once.

### Phase M2c — Hardening pass

- **Goal**: Catch non-determinism, partition-recursion bugs, and normalization gaps before M3 builds on them.
- **Tasks**:
  1. Run read-twice property test across pg15/16/17.
  2. Fuzz partition tree depth (3+ levels) and hash/range/list mix.
  3. Explicitly verify collation-version strip against a schema with pg15+ collation-version metadata.
  4. Write the per-hot-spot normalization test matrix (one test per row in architecture §Normalization rules).
- **Verification**:
  - All property tests green across three pg major versions.
  - `cargo clippy --all-targets -- -D warnings` clean.
  - `topcat schema dump` output diffed with `git diff --no-index` against a committed golden file is empty.

## Risks (copied and amplified from roadmap)

| Risk | Severity | Day-one mitigation |
|---|---|---|
| **Normalization is where correctness lives.** Miss one hot spot → differ emits spurious changes forever. | high | Before writing any reader: lift the full §Normalization rules table into `normalization.rs` as a TODO-block header with one helper-fn per row. Read-twice property test fails the build if any field is non-deterministic. Add a named test per hot spot (`test_varchar_text_collapse`, `test_identity_serial_default_union`, etc.). |
| **Partition-tree recursion.** Sub-partitioning recurses; cycle or dedup bug corrupts the model. | high | Build `partition_walker.rs` with an explicit `visited: HashSet<Oid>` guard. Fixture covers 3-level range-of-list-of-hash. Assert each partition appears exactly once in the output. |
| **Collation-version hashes.** pg15+ attaches a collation version to catalog rows; the architecture strips it. Roadmap flags this as "verify it is always safe". | medium | Day one: write a fixture that exercises ICU collations, dump on pg15 and pg17, assert byte-equal. Document the invariant at the top of `normalization.rs`. If an ICU upgrade changes semantics, the test will fail and surface the boundary. |
| **JCS compliance drift.** Any sort order / whitespace / number regression silently corrupts every downstream hash. | medium | Dedicated `jcs.rs` module with a single public `fn to_jcs(value: &impl Serialize) -> String`. Byte-level tests against hand-authored expected output for every primitive JSON shape. Do not route through `serde_json::to_string` ad-hoc elsewhere. |
| **Extension-owned object filter (`deptype = 'e'`) gaps.** Miss a join and user objects get excluded, or extension objects leak in. | medium | Centralize the filter in `catalog_reader/mod.rs` as `fn exclude_extension_objects(oid_list: &[Oid]) -> Vec<Oid>`; every per-type reader calls it. Integration test installs `pgcrypto`, `citext`, `ltree` and asserts zero extension-owned rows leak. |
| **Cross-pg-version catalog schema drift.** pg15/16/17 diverge on `pg_statistic_ext`, `pg_range`, `pg_publication`. | medium | Run read-twice and fixture tests across all three in CI. If behavior differs, branch in the reader with an explicit `pg_version` check, not an implicit cast. |
| **`node_id` lookup coupling to `_topcat.node_registry`.** M2 reads when present; M11 writes. Don't block M2 on M11. | low | Treat `node_id: Option<Uuid>` as nullable. Reader joins `pg_class.oid` against `_topcat.node_registry.pg_address` if the schema exists; returns `None` otherwise. No writes. |

## Tests

Fixture layout (`tests/pg_fixtures/schema_model/`):

- `01_tables_basic.sql` — tables with every column type, defaults, NOT NULL, comments.
- `02_constraints.sql` — PK, FK (self-ref, cross-schema), UNIQUE (with WHERE), CHECK, EXCLUDE.
- `03_indexes.sql` — btree, hash, gin, gist, brin, partial, expression, INCLUDE, covering.
- `04_partitioned_range.sql` — range partitioning, 3-level depth, default partition.
- `05_partitioned_list.sql` — list partitioning.
- `06_partitioned_hash.sql` — hash partitioning, modulus/remainder.
- `07_inheritance.sql` — classic table inheritance (not partitioned).
- `08_generated_columns.sql` — `GENERATED ALWAYS AS (...) STORED`.
- `09_identity_columns.sql` — `GENERATED … AS IDENTITY` with sequence options; also classic `serial`/`bigserial`.
- `10_rls.sql` — RLS enabled, forced, enabled-without-policies, multiple policies per table.
- `11_views.sql` — simple view, view with column list, nested view.
- `12_matviews.sql` — materialized views with indexes.
- `13_functions.sql` — sql, plpgsql, C-lang stub, varargs, default args, OUT/INOUT, `SECURITY DEFINER`.
- `14_procedures.sql` — pg11+ procedures with transactions.
- `15_sequences.sql` — standalone sequences, owned-by.
- `16_types_enum.sql`, `17_types_composite.sql`, `18_types_domain.sql`, `19_types_range.sql`, `20_types_multirange.sql`.
- `21_triggers.sql` — BEFORE/AFTER/INSTEAD OF, ROW/STATEMENT, WHEN clause, constraint triggers.
- `22_rules.sql` — ON INSERT/UPDATE/DELETE, SELECT rules as views.
- `23_text_search.sql` — config, dictionary, parser, template.
- `24_fdw.sql` — foreign data wrapper, server, user mapping, foreign table.
- `25_operators.sql` — operator, operator class, operator family.
- `26_aggregates.sql` — custom aggregate with state fn + final fn.
- `27_casts_conversions.sql`.
- `28_statistics.sql` — extended statistics objects.
- `29_publications.sql`.
- `30_access_methods.sql`.
- `31_user_event_triggers.sql`.
- `32_collations.sql` — ICU + libc collations; exercises the version-strip.
- `33_reloptions.sql` — fillfactor, toast options; exercises sort-canonicalization.
- `34_comments.sql` — `COMMENT ON` for every object kind.
- `35_acls.sql` — grants on tables, functions, schemas, sequences.
- `99_all_objects.sql` — superset fixture for end-to-end smoke.

Test categories:

- **Unit** (`src/schema_model/tests.rs`): type construction, JCS byte-stability on hand-authored inputs.
- **Integration** (`tests/catalog_reader_fixtures.rs`): load each fixture into an ephemeral pg via M1's harness; assert the emitted `SchemaModel` matches a golden JSON snapshot checked into `tests/pg_fixtures/golden/`.
- **Determinism property** (`tests/catalog_reader_determinism.rs`): for each fixture, call `read_schema` twice on the same connection; assert (a) `SchemaModel` equality via `PartialEq`, (b) JCS byte-equality. Run via proptest over a small fixture matrix. (This is the roadmap's "read twice, assert equal" guard.)
- **Cross-version** (`tests/catalog_reader_cross_version.rs`): run the `99_all_objects.sql` fixture against pg15, pg16, pg17. Assert the three `SchemaModel` outputs are equal modulo documented version-specific exclusions.
- **Extension-exclusion** (`tests/catalog_reader_extension_exclude.rs`): install `pgcrypto`, `citext`, `ltree`; assert no extension-owned row appears in the model.

## Verification snapshot (definition of done)

```bash
# 1. Build and lint
cargo build
cargo clippy --all-targets -- -D warnings

# 2. Unit + integration tests
cargo test schema_model::
cargo test --test catalog_reader_determinism
cargo test --test catalog_reader_fixtures
cargo test --test catalog_reader_cross_version      # requires pg15/16/17
cargo test --test catalog_reader_extension_exclude
cargo test --test schema_model_jcs

# 3. Read-twice byte determinism on a live db (new)
topcat schema dump --format json > /tmp/dump-a.json   # (new)
topcat schema dump --format json > /tmp/dump-b.json   # (new)
diff /tmp/dump-a.json /tmp/dump-b.json                # must be empty

# 4. All-objects fixture round-trips to a committed golden
psql -d topcat_test -f tests/pg_fixtures/schema_model/99_all_objects.sql
topcat schema dump --format json \
  | git diff --no-index tests/pg_fixtures/golden/99_all_objects.json -   # (new)
# exit 0 = no byte-level churn
```

Exit criteria:

- Every object type from architecture §Object types covered is exercised by at least one fixture and appears in the golden dump.
- Read-twice determinism holds on pg15, pg16, pg17.
- `cargo clippy` is clean with `-D warnings`.
- `topcat schema dump` output is JCS-compliant (validated by the hand-authored JCS test suite in `jcs.rs`).
- `Function.inferred_edges` is always empty (populated by M4); `parse_status` defaults to `ok`.

## Open questions to resolve before starting

The roadmap's §Open questions list (lines 756–778) has no items tagged M2, but the following M2-implicit questions must be resolved at session kickoff:

1. **JCS implementation source**. Hand-roll inside `schema_model/jcs.rs` (RFC 8785 is small) or adopt a crate (`serde_jcs`, `json-canon`)? Recommend hand-roll for audit clarity and zero new dependencies; confirm before writing.
2. **Catalog read strategy**. One mega-query joining all `pg_*` tables once, or per-object-type queries issued sequentially? Mega-query is faster but fragile across pg versions. Recommend per-type for M2; revisit in M20 perf pass.
3. **Postgres driver choice**. `postgres` crate (sync, blocking) vs `tokio-postgres` (async). Topcat is sync (no tokio in Cargo.toml); `postgres` is the obvious pick. Confirm in M1 and inherit.
4. **Collation version strip safety** — architecture itself flags as "verify" (roadmap line 169): is stripping `pg_collation.collversion` from the model always safe for schema-intent diffing? Build fixture 32 first and answer empirically before writing readers that assume the invariant.
5. **`node_id` coupling**. Does M2 read `_topcat.node_registry` to populate `SchemaModel::Table::node_id` (cheap join, useful for observability), or leave `node_id` as `None` until M11? Recommend read-if-present; do not block.
6. **ACL representation**. Architecture mentions `acl` on Table and Function. Normalize to a sorted `Vec<Grant>` or keep raw `aclitem[]` text? Recommend typed `Grant { grantee, privileges: Vec<Priv>, grant_option, granted_by }`; sort by grantee.
7. **Comment scope**. `COMMENT ON` is captured per object and per column (architecture §Normalization rules). Confirm per-index, per-constraint, per-trigger comments are also captured — answer at fixture 34.

Answer (1)–(4) before any code is written; (5)–(7) before M2a ships.

## Session sizing notes

Roadmap target (line 178): ~3–5 sessions with M2a/M2b split recommended.

**Split boundary**:

- **M2a (2 sessions)** — core objects: tables, columns, constraints, indexes, functions, procedures, views, materialized views, sequences, types (all 5 variants), schemas, extensions. Plus: JCS serializer, reader façade, normalization helpers, partition walker, `topcat schema dump` CLI, read-twice determinism property test, extension-owned filter. At the end of M2a, M3 and M19 can begin in parallel against the frozen core model.
- **M2b (1–2 sessions)** — long tail: triggers, policies (standalone), rules, text search (configs/dicts/parsers/templates), FDW (wrapper/server/user-mapping), operators + classes + families, aggregates, casts, conversions, statistics, publications, access methods, user event triggers. Extend fixture suite and read-twice tests.
- **M2c (1 session)** — hardening: cross-version property tests (pg15/16/17), partition-tree fuzzing, full normalization hot-spot matrix, golden snapshot commit.

Total: 4 sessions typical, 5 if M2b fixtures reveal pg-version surprises.

**Parallelization**: M2 has no internal parallelizable seams (every downstream depends on the completed model). Do not split M2a across sessions running concurrently.

## Referenced architecture sections

- §CatalogReader, SchemaModel, and DDL emitter (line 398) — model shape, SchemaModel↔DDL shared authority.
- §Normalization rules (line 427) — full hot-spot table; the correctness contract for M2.
- §Partitioned tables and inheritance (line 446) — partition-tree recursion, bounds, sub-partition semantics.
- §Object types covered (line 466) — exhaustive coverage list.
- §Pipeline (line 115) — where catalog_reader sits in the overall data flow.
- §Component map (line 1134) — the `schema_model`, `pg/catalog_reader`, and `pg/ddl_emitter` rows.

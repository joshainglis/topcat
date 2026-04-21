# M3 — ddl_emitter + round-trip property tests

> Anchors: `docs/pg-dev-buddy-roadmap.md` §M3 (ddl_emitter + round-trip
> property tests) and `docs/pg-dev-buddy-architecture.md`
> §CatalogReader, SchemaModel, and DDL emitter, §Normalization rules,
> §Partitioned tables and inheritance, §Object types covered.

## Context

M3 builds the single authoritative `SchemaModel → DDL` path. Every
downstream module (sync re-apply in M12, migration generation in M9,
verification in M10, reverse migrations in M13) funnels through this
emitter. A wrong emitter here corrupts every artifact above it, so the
property test is as load-bearing as the code: round-trip `SchemaModel →
DDL → apply to fresh db → catalog read → SchemaModel` must be bit-equal.

The build sits on top of the typed objects frozen in M2 and writes to
ephemeral Postgres via the client from M1. No async, no tokio, no
orchestration — this is the trusted kernel of pg-dev-buddy.

## Prerequisites

- **M2 lands first.** Every type the emitter consumes lives in
  `crate::schema_model`. Concretely:
  - `crate::schema_model::Table`, `::Column`, `::Constraint`, `::Index`
  - `crate::schema_model::Function`, `::Procedure`
  - `crate::schema_model::View`, `::MaterializedView`
  - `crate::schema_model::Sequence`, `::Type` (enum/composite/domain/range/
    multirange)
  - `crate::schema_model::Trigger`, `::Policy`, `::Rule`
  - `crate::schema_model::Schema`, `::Extension`
  - Long-tail types (text search, FDW/server/user mapping,
    operator/class/family, aggregate, cast, conversion, statistics,
    publication, access method, user event trigger) per architecture
    §Object types covered.
- **M1 `pg/client`** for applying DDL into ephemeral pg15/16/17 during
  round-trip tests. Sync API is fine; no tokio.
- **Normalization rules from M2** are authoritative — M3 emits DDL whose
  re-read lands on the same normalized value. M3 never adds new
  normalization; if a round-trip fails, the fix goes in M2.
- Nix dev shell with `cargo`, `postgresql_15`, `postgresql_16`,
  `postgresql_17` binaries (via `nix develop`).
- `proptest 1.5` already wired through `tests/property_tests.rs`.

## Scope

### In scope for this milestone

- `src/pg/ddl_emitter/` module tree with one emitter file per object type.
- Emitter-owned intra-object ordering (table body → indexes → constraints
  → RLS → triggers → comments; function attributes in a fixed sequence).
- `src/schema_model` → `String` emission only; no side effects, no writes.
- Function / procedure body emission using verbatim `pg_proc.prosrc`
  (architecture §Normalization rules, "Function body" row).
- Round-trip property harness in `tests/round_trip.rs` running against
  ephemeral pg15, pg16, pg17.
- Fixture corpus in `tests/input/pg_fixtures/` covering every
  normalization hot spot, every partition strategy, inheritance, RLS
  toggles, identity/serial/default union, generated columns, cross-schema
  references, comments.
- Internal debug subcommand `topcat __debug emit-ddl` (hidden from
  user-facing `--help`) so developers can dump SchemaModel → DDL when
  chasing a round-trip failure.

### Explicitly NOT in scope (deferred to M4+)

- Diffing two SchemaModels or producing a `ChangeSet` — that is M8.
- Topological ordering across objects (leaves-first drops, roots-first
  creates) — that is M9 via `object_dag` (M7).
- Multi-section migration routing (`pre_begin` / `transaction` /
  `post_commit`) — that is M9.
- Inferred edges from plpgsql bodies — that is M4 `body_parser`.
- Event-trigger capture or `_topcat` meta-schema integration — that is M5.
- Shadow-db lifecycle management (template reuse, caches) — that is M6.
  M3 uses plain ephemeral databases for its tests.
- A user-facing `topcat emit` command. The architecture explicitly warns
  this is too easy to misuse; emission is reachable only via the internal
  debug command.

## Deliverables (files and modules)

| Path | Kind | Purpose |
|---|---|---|
| `src/pg/mod.rs` | extend | Add `pub mod ddl_emitter;`. |
| `src/pg/ddl_emitter/mod.rs` | new | Public `emit(model: &SchemaModel) -> String` entry point; object dispatch. |
| `src/pg/ddl_emitter/writer.rs` | new | Shared `DdlWriter` buffer + identifier quoting + schema-qualification helpers. |
| `src/pg/ddl_emitter/schema.rs` | new | `CREATE SCHEMA`, comment, owner/ACL. |
| `src/pg/ddl_emitter/extension.rs` | new | `CREATE EXTENSION` with `WITH SCHEMA` + `VERSION`. |
| `src/pg/ddl_emitter/table.rs` | new | Table body, columns (identity/serial/default union), table-level constraints, `INHERITS`, partition clauses, storage params, comments. |
| `src/pg/ddl_emitter/index.rs` | new | `CREATE INDEX` (not `CONCURRENTLY` — M9 routes that). |
| `src/pg/ddl_emitter/constraint.rs` | new | PK / FK / UNIQUE / CHECK / EXCLUDE emitted after parent table. |
| `src/pg/ddl_emitter/rls.rs` | new | `ALTER TABLE … ENABLE/FORCE ROW LEVEL SECURITY` + per-policy DDL (both flags load-bearing per architecture). |
| `src/pg/ddl_emitter/function.rs` | new | `CREATE FUNCTION` / `PROCEDURE`; verbatim `prosrc`; volatility/strict/security/parallel/cost/rows/config in fixed order. |
| `src/pg/ddl_emitter/view.rs` | new | `CREATE VIEW` / `MATERIALIZED VIEW` from canonicalized `pg_get_viewdef`. |
| `src/pg/ddl_emitter/sequence.rs` | new | `CREATE SEQUENCE` with schema-qualified refs. |
| `src/pg/ddl_emitter/types.rs` | new | Enum / composite / domain / range / multirange emission. |
| `src/pg/ddl_emitter/trigger.rs` | new | Row/statement triggers; user-managed event triggers. |
| `src/pg/ddl_emitter/policy.rs` | new | Standalone `CREATE POLICY` when not bundled with table. |
| `src/pg/ddl_emitter/rule.rs` | new | `CREATE RULE`. |
| `src/pg/ddl_emitter/long_tail.rs` | new | Text search configs/dicts/parsers/templates; FDW / server / user mapping; operator classes/families; aggregates; casts; conversions; statistics; publications; access methods. One submodule per family if the file grows beyond ~400 LOC. |
| `src/pg/ddl_emitter/ordering.rs` | new | Intra-object emission order rules (documented constants). |
| `src/commands/debug/mod.rs` | new | Hidden `__debug` subcommand dispatch. |
| `src/commands/debug/emit_ddl.rs` | new | `topcat __debug emit-ddl` — read model from JSON or catalog and print DDL. |
| `src/cli/debug.rs` | new | `clap` group for the hidden debug subcommand. |
| `src/main.rs` | extend | Wire the hidden subcommand with `.hide(true)`. |
| `tests/round_trip.rs` | new | Property test entry point (runs on pg15/16/17). |
| `tests/input/pg_fixtures/` | new dir | SQL fixtures keyed by hot spot. |
| `Cargo.toml` | extend | Dev-dep `testcontainers` or keep the M1 ephemeral-pg harness; no new runtime deps. |

## Implementation phases

### Phase 1 — emitter skeleton + writer

Ship `writer.rs` + `mod.rs` with the public entry point, identifier
quoting, schema qualification, and a no-op dispatch for every object
type. Wire the hidden `__debug emit-ddl` command so downstream phases
have a way to eyeball output. No tests beyond a trivial "emits empty
schema" smoke test. ~0.3 session.

### Phase 2 — core objects (matches M2a split)

Tables (incl. partition clauses, inheritance, storage params), columns
(identity/serial/default union), constraints, indexes, functions
(verbatim `prosrc`), views/matviews, sequences, types, schemas,
extensions, RLS. Unit tests per emitter that assert textual output
against a few representative golden strings — just enough to catch
regressions; round-trip is the real test. ~1.2 sessions.

### Phase 3 — long tail

Triggers, policies (standalone), rules, text search, FDW/server/user
mapping, operator class/family, aggregate, cast, conversion,
statistics, publication, access method, user-managed event trigger.
Add each to the dispatch table; leave a `todo!()` for any family whose
M2 type isn't landed yet and open a follow-up. ~0.5 session.

### Phase 4 — round-trip harness

`tests/round_trip.rs` spins an ephemeral pg cluster (reusing the M1
helper), loads each fixture into db A, reads → emits → applies to a
fresh db B, reads again, and asserts SchemaModel equality. Proptest
generators compose a `Table`, `Function`, etc. from bounded random
pieces and feed the same pipeline. The pg version is a proptest
parameter so failures reproduce on a single version. ~0.8 session.

### Phase 5 — fixture sweep + hardening

Build fixture schemas for every row in architecture §Normalization rules
and every bullet in §Partitioned tables and inheritance. Run
`tests/round_trip.rs` on pg15, pg16, pg17. Any failure either points at
an emitter bug (fix here) or a missing normalization rule (land a bug
report against M2). ~0.3 session.

## Risks (copied and amplified from roadmap)

Roadmap flags three risks under M3; each gets a day-one mitigation.

- **"Single authoritative SchemaModel→DDL path" — if it's wrong, sync
  re-apply and migration generation both produce garbage.**
  - *Day-one mitigation.* Write the round-trip property harness in Phase
    1's smoke test (empty schema, single table) before writing any other
    emitter. The test scaffold exists from line one so every subsequent
    emitter lands against a green bar.
  - *Amplification.* The trap is forgetting that "produces compilable
    DDL" is not the bar. The bar is "re-read lands on the same
    normalized SchemaModel". A `CREATE TABLE` whose column order is
    stable but inverted from pg's preferred form still round-trips
    because normalization collapses it.

- **Intra-object DDL ordering (table body → indexes → constraints →
  RLS) — the emitter owns it, not the caller.**
  - *Day-one mitigation.* `src/pg/ddl_emitter/ordering.rs` declares the
    order as named constants (`TABLE_EMIT_ORDER: &[TableSection]`) with
    a doc comment citing architecture §CatalogReader, SchemaModel, and
    DDL emitter. Every emitter iterates these constants, never ad-hoc
    ordering.
  - *Amplification.* RLS is a two-axis problem: `enabled` and `forced`
    are independent flags (architecture §Normalization rules, RLS row).
    Both emit as separate `ALTER TABLE` statements even when only one
    changes; omit neither.

- **Function body fidelity — `prosrc` is verbatim, no whitespace
  normalization, but emitted DDL must re-yield the same `prosrc`.**
  - *Day-one mitigation.* Pick the dollar-quote tag dynamically (search
    the body for `$$`, `$function$`, `$body$`, …) so the emitted
    `$tag$...$tag$` can never collide with body content. Fixture:
    function whose body literally contains `$$` and `$function$` —
    ensures the resolver escalates.
  - *Amplification.* pg rewrites `prosrc` in subtle ways when the source
    uses certain legacy syntaxes (e.g. SQL-body functions landing with a
    parsed `BEGIN ATOMIC` form). If a fixture demonstrates this, M3 does
    *not* work around it — M2 normalizes, M3 emits. File the bug against
    M2 with the minimal repro and move on.

## Tests

### Property test

- `tests/round_trip.rs` (new) — proptest harness, parameter
  `pg_major_version ∈ {15, 16, 17}`.
- Pipeline: `model_a = read(db_a)` → `ddl = emit(model_a)` →
  `apply(db_b, ddl)` → `model_b = read(db_b)` → `assert_eq!(model_a,
  model_b)`.
- Shrinker surfaces the minimum failing object on regression.
- Cross-version run: CI matrix executes `cargo test --test round_trip`
  three times, once per `TOPCAT_PG_VERSION` env var.

### Fixture corpus (`tests/input/pg_fixtures/`)

Every row in architecture §Normalization rules must map to at least one
fixture:

- `varchar_text_collapse.sql` — `varchar`, `varchar(n)`, `character
  varying`, `text` side-by-side in one table.
- `identity_union.sql` — `serial`, `GENERATED BY DEFAULT AS IDENTITY`,
  `GENERATED ALWAYS AS IDENTITY`, plain `DEFAULT nextval(...)`.
- `view_canonicalization.sql` — view with unqualified column references.
- `generated_column.sql` — stored and virtual generated columns.
- `auto_named_constraints.sql` — unnamed PK, FK, UNIQUE, CHECK.
- `reloptions.sort.sql` — table with `fillfactor`, `autovacuum_*`
  storage params in arbitrary order.
- `rls_flags.sql` — all four (enabled, forced) × (enabled, forced) cells
  including a table with `FORCE ROW LEVEL SECURITY` but no policies.
- `comments_every_object.sql` — `COMMENT ON` for table, column,
  function, index, constraint.
- `function_body_verbatim.sql` — function body containing `$$` and
  `$function$` literals.
- `sequence_cross_schema.sql` — sequence in schema A used as default in
  table in schema B.
- `collation_stripped.sql` — column with explicit collation and
  expected-stripped version hash.

Partition/inheritance coverage:

- `partition_range.sql`, `partition_list.sql`, `partition_hash.sql`.
- `partition_default.sql` (DEFAULT partition).
- `partition_subpartitioning.sql` (three levels deep).
- `inheritance_multi_parent.sql`.
- `cross_schema_refs.sql` (FK from A.t to B.u; function in A returning B.u).

### Unit tests per emitter

- Textual golden strings in `tests/input/pg_fixtures/golden/*.sql`.
- Lightweight — only the most regression-prone bits (dollar-quote tag
  escalation, RLS flag matrix, partition bounds `DEFAULT` form).

## Verification snapshot (definition of done)

```bash
nix develop --command cargo fmt --check
nix develop --command cargo clippy -- -D warnings
nix develop --command cargo test --lib                       # (new) emitter unit tests
nix develop --command cargo test --test round_trip           # (new) round-trip property suite
TOPCAT_PG_VERSION=15 nix develop --command cargo test --test round_trip  # (new)
TOPCAT_PG_VERSION=16 nix develop --command cargo test --test round_trip  # (new)
TOPCAT_PG_VERSION=17 nix develop --command cargo test --test round_trip  # (new)
nix develop --command cargo run -- __debug emit-ddl --schema-model tests/input/pg_fixtures/golden/rls_flags.json  # (new) sanity smoke
```

All seven must pass. Round-trip must hold for every fixture on every pg
major.

## Open questions to resolve before starting

1. **Ephemeral pg harness.** M1 shipped something — confirm whether it
   offers a per-test-case `Database` handle or only a shared cluster.
   Round-trip needs two fresh databases per case and expects to drop
   them cleanly; if the harness is cluster-scoped only, add a `CREATE
   DATABASE … TEMPLATE template0` helper before Phase 4.
2. **Dollar-quote tag escalation policy.** Candidate order: `$$` → `$_$`
   → `$function$` → `$body$` → `$topcat_<n>$`. Confirm — some linters
   expect `$function$` as the default.
3. **`__debug` subcommand shape.** Does it read a `SchemaModel` from a
   JCS JSON file, connect to a db and read live, or both? Pick one for
   M3; the other can land with M12.
4. **Fixture format.** Hand-written `.sql` applied via `psql`, or typed
   `SchemaModel` JSON fed directly into the emitter? Round-trip needs
   both directions; hand-written SQL is the stricter test (pg's reader
   is the oracle). Default: `.sql` files.
5. **Matview refresh state.** Architecture doesn't explicitly specify
   whether matview `REFRESH` state round-trips. Assume emitter issues
   `CREATE MATERIALIZED VIEW … WITH NO DATA` and leaves refresh to
   migration apply; confirm with a fixture before locking it in.
6. **ACL and OWNER scope.** The architecture lists `owner, acl` on
   every object. M3 emits both — confirm the catalog_reader populates
   them in every relevant type before round-trip relies on them.

## Session sizing notes

Roadmap allocates ~2-3 sessions. Suggested split:

- **Session 1** (Phase 1 + most of Phase 2): writer, dispatch, core
  tables/columns/constraints/indexes/functions/views, first green
  round-trip on a trivial fixture.
- **Session 2** (remainder of Phase 2 + Phase 3 + Phase 4 skeleton):
  sequences, types, RLS, triggers, long tail with `todo!()` stubs where
  M2 types aren't ready, round-trip harness wired for pg16.
- **Session 3** (Phase 5): fixture sweep, pg15 and pg17 matrix,
  dollar-quote tag escalation fixture, risk-register fixtures
  (`function_body_verbatim.sql`, `rls_flags.sql`), clippy pass, commit.

If Phase 3's long tail balloons, split it off into M3b and lean on
`todo!()` for unblocking M4/M5. The round-trip harness must be green on
the core set before M3 is considered done.

## Referenced architecture sections

- §CatalogReader, SchemaModel, and DDL emitter
- §Normalization rules
- §Partitioned tables and inheritance
- §Object types covered
- §Identity and node_id (context for `node_id` field presence on every
  type — emitter emits neither node_id nor `-- node_id:` headers; those
  are M11)
- §Testing strategy (the round-trip property test lives under the
  general testing discipline described here)

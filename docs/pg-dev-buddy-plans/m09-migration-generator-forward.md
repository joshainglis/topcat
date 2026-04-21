# M9 — migration_generator (forward only)

> Turn a `ChangeSet` into a deterministic, multi-section forward migration
> file. No reverses, no bounces, no split/merge. Generator is fail-closed on
> anything that needs those — later milestones will relax the fence.

## Context

M9 is the first milestone that produces an artifact an operator can apply to
a database. Upstream milestones (M2 catalog_reader, M3 ddl_emitter, M7
object_dag, M8 differ) deliver the raw material: a `SchemaModel` pair, a
`ChangeSet` describing the transition, an `object_dag` that knows how to
order DDL, and a DDL emitter that turns structural change into text. M9
composes those into a file on disk that matches the format fixed in
architecture §Migration file format.

Scope is intentionally narrow. Forward only. Reverse generation lands in
M13 along with the migration registry that wires up `parents` and
`forward_id`. Bounce markers (operator-input workflow) land in M14. Split
and merge annotations land in M15. M9 must emit files that a future M10
(migration_verifier) can round-trip against a shadow database without
relying on any of those later features.

The single biggest risk is section routing. PostgreSQL forbids certain
statements inside a transaction block (`CREATE INDEX CONCURRENTLY`, `ALTER
SYSTEM`, some extension operations). Mis-routing turns a migration into an
error on apply. The mitigation is an allowlist-based router keyed on
statement AST kind, with fail-closed behavior for anything the allowlist
doesn't know.

## Prerequisites

Depends on, and names concrete artifacts from, three upstream milestones:

- **M3 — `pg/ddl_emitter`**. Per-object-type emitters that turn a
  `SchemaModel` (or a pair, for `Alter`) into DDL text. M9 never writes
  DDL by hand — it asks the emitter. The emitter owns intra-object ordering
  (table body → indexes → constraints → RLS → grants); M9 owns inter-object
  ordering. Emitter API must expose enough context for M9 to tag each
  produced statement with the information needed to route it (see Phase 2
  below).
- **M7 — `object_dag`**. Nodes `(node_id, pg_address, file, parse_status)`,
  edges `(kind: catalog | inferred | declared, confidence, source_location)`.
  M9 walks this DAG for ordering: drops in reverse topological order
  (leaves first), creates in forward topological order (roots first),
  alters in dependency-respecting order between them. Edge authority rules
  (architecture §Edge authority) are M7's responsibility; M9 trusts the
  resolved edge set.
- **M8 — `ChangeSet`**. `Vec<(node_id, change_kind, before_model?,
  after_model?)>` with `change_kind ∈ {Add, Drop, Alter, Rename,
  PartitionAttach, PartitionDetach, SplitExtract, MergeAbsorb}`. Plus
  content-hash computation per architecture §Content-hash stability
  (canonical JSON, excluded fields enumerated). M9 consumes the ChangeSet
  and reuses its hash as `migration_id`.

Also leveraged (already in tree): `src/stable_topo.rs` for deterministic
tie-breaking; `src/main.rs` for CLI wiring.

## Scope

**In scope:**

- `ChangeSet` → one multi-section SQL file on disk.
- Topological walk over `object_dag`: drops (leaves first), creates (roots
  first), alters (middle).
- Section routing: every emitted statement goes to exactly one of
  `pre_begin`, `transaction`, `post_commit` via an allowlist keyed on
  statement AST kind.
- Migration file header with every field from architecture §Migration file
  format: `migration_id`, `parents`, `is_reverse`, `forward_id`,
  `generated_at`, `pg_version`, `extension_versions`, `requires_input`,
  `changes_summary`.
- Deterministic output: identical `ChangeSet` + environment yields
  byte-for-byte identical file.
- CLI: `topcat migrate generate` produces a forward-only file.

**Fail-closed (explicit refusal, actionable error):**

- `change_kind = SplitExtract` — needs M15 split/merge annotations.
- `change_kind = MergeAbsorb` — same.
- `Alter` that requires a cast not implicitly available
  (cast-incompatible column type change) — needs bounce workflow (M14).
- `Add` of `NOT NULL` column to a populated table with no `DEFAULT` —
  needs bounce workflow (M14).
- Any node with `parse_status = dynamic_ddl`. An `--allow-dynamic-ddl`
  flag surface exists in the CLI layer but resolves to a fail-closed error
  in M9 with a pointer to M14.

**Out of scope (deferred):**

- Reverse migrations (M13).
- `parents` resolution to real migration IDs (M13 wires the registry;
  M9 emits literal `parents: []` placeholder).
- Bounce markers and `requires_input: true` workflow (M14). M9 always
  emits `requires_input: false`.
- Split/merge annotations (M15).
- Migration verification (M10 — consumes M9's output, not M9's concern).

## Deliverables (files and modules)

New crate-internal module at `src/migration_generator/`. Green-field; no
existing paths to modify beyond CLI wiring.

### New files

```
src/migration_generator/mod.rs              // Public API: generate(changeset, dag, ctx) -> Migration
src/migration_generator/walk.rs             // Topological walk: drops, creates, alters
src/migration_generator/router.rs           // Statement → section allowlist
src/migration_generator/header.rs           // Migration file header construction
src/migration_generator/writer.rs           // Serialize Migration -> String (bytes on disk)
src/migration_generator/error.rs            // GeneratorError + fail-closed reasons
src/migration_generator/tests/golden/       // Golden-file fixtures (ChangeSet JSON + expected .sql)
src/migration_generator/tests/routing.rs    // Unit tests for router
src/migration_generator/tests/walk.rs       // Unit tests for walk ordering
```

### Existing files to modify

- `src/main.rs` — register the `migrate generate` subcommand.
- `src/cli/` — add a new `cli/migrate.rs` argument group (input dirs,
  output path, shadow database references for the future verifier hook).
- `Cargo.toml` — no new dependencies anticipated; reuses whatever M8 uses
  for canonical JSON.

### Routing rules (allowlist)

Routing is keyed on statement AST kind emitted by M3, not on raw SQL text.
M9 treats anything not in the allowlist as a hard error.

| Statement class | Section | Notes |
|---|---|---|
| `CREATE INDEX CONCURRENTLY`, `REINDEX CONCURRENTLY`, `DROP INDEX CONCURRENTLY` | `post_commit` | pg forbids in tx. |
| `ALTER SYSTEM ...` | `pre_begin` | pg forbids in tx. |
| `CREATE DATABASE`, `DROP DATABASE`, `CREATE TABLESPACE`, `DROP TABLESPACE` | `pre_begin` | pg forbids in tx. |
| `VACUUM`, `CLUSTER`, `REINDEX` (non-concurrent whole-db) | `pre_begin` | pg forbids in tx. |
| `CREATE EXTENSION` without options, `DROP EXTENSION` | `transaction` | Transactional by default. |
| `CREATE EXTENSION ... WITH UPDATE`/certain options | `pre_begin` | Out-of-tx on specific extensions (enumerate by extname in router config). |
| `CREATE SCHEMA`, `CREATE TABLE`, `CREATE FUNCTION`, `CREATE TYPE`, `CREATE DOMAIN`, `CREATE VIEW`, `CREATE TRIGGER`, `CREATE POLICY`, `GRANT`, `REVOKE`, `COMMENT ON`, `ALTER ... OWNER TO`, all `ALTER TABLE` subforms except column-type-cast-incompatible, all `DROP ...` (non-concurrent, non-database) | `transaction` | Default case. |
| Anything not above | **fail-closed** | Generator returns `GeneratorError::UnknownRoute { kind }`. |

### Migration file header contract

M9 emits every field defined in architecture §Migration file format, with
these fixed values for forward-only scope:

- `migration_id`: sha256 of canonical-JSON `ChangeSet` (M8 computes it).
- `parents`: literal `[]`. M13 rewrites.
- `is_reverse`: literal `false`.
- `forward_id`: literal `null`.
- `generated_at`: UTC ISO-8601, second precision.
- `pg_version`: read from `shadow_main` at generation time (passed in
  via `GenerateContext`; M9 does not connect to pg directly).
- `extension_versions`: sorted `name=version` pairs, comma-separated.
- `requires_input`: literal `false`.
- `changes_summary`: per-node one-liner, format `<kind>: <node_id> <pg_address_short>`.

## Implementation phases

**Phase 1 — Router and error taxonomy** (~0.5 session). Implement
`router.rs` with the allowlist table above as a static lookup. Implement
`error.rs` with every fail-closed branch from §Scope enumerated as a
distinct variant (`UnknownRoute`, `SplitExtractUnsupported`,
`MergeAbsorbUnsupported`, `CastIncompatible`, `PopulatedNotNull`,
`DynamicDdlUnsupported`). Unit-test the router in isolation; no DAG, no
emitter required.

**Phase 2 — Topological walk** (~0.5 session). Implement `walk.rs`. For a
given `ChangeSet`, partition nodes by `change_kind`: drops, creates,
alters. Walk each partition against the `object_dag` with the required
order (drops: leaves-first = reverse topo; creates: roots-first = forward
topo; alters: forward topo but only among alter nodes, with dependency
edges honored against creates that precede them). Tie-break using
`src/stable_topo.rs` for determinism. Output: `Vec<PlannedStatement>`
where each statement carries its emitted SQL and AST kind for routing.

**Phase 3 — Header and writer** (~0.5 session). Implement `header.rs`
and `writer.rs`. Header field assembly is mechanical. Writer produces the
final `String` in the exact format of architecture §Migration file format:
header block, then `-- section: pre_begin`, then `-- section: transaction`
wrapped in `BEGIN; ... COMMIT;`, then `-- section: post_commit`. Empty
sections are omitted entirely (not left as empty markers). Output is
deterministic byte-for-byte.

**Phase 4 — CLI wiring and golden tests** (~0.5 session). Add
`topcat migrate generate` to `src/main.rs` / `src/cli/`. Flags: `-i`
(schema source, same idiom as existing commands), `--shadow-main`,
`--shadow-head`, `-o` (output file). Seed `tests/golden/` with five
fixtures covering: creates only, drops only, alters only, mixed, and a
fixture containing one `SplitExtract` (asserts fail-closed). Add a
deterministic re-run check: generate twice, compare bytes.

## Risks (copied and amplified)

**Section-routing rules must be exhaustive.** Any statement pg forbids in
a transaction block that routes to `transaction` becomes a runtime failure
on apply. Day-one mitigation: router is allowlist-based, unknown AST kind
returns `GeneratorError::UnknownRoute { kind }` with the offending kind
name in the error — no silent default to `transaction`. Maintenance
discipline: when M3 adds a new statement AST kind, CI fails M9's
router-coverage test until the table is updated.

**Parent migration references require the registry (M13).** M9 ships with
`parents: []` literal in both the header and in `changes_summary`. The
placeholder is called out explicitly in the header comments so an operator
reading the file isn't confused. M13 rewrites this field during migration
persistence. Day-one mitigation: document the placeholder in the header
template itself via a leading comment; golden-file tests assert the exact
placeholder string, so any accidental change requires an explicit update.

**Fail-closed surface must match the roadmap exactly.** The four cases
the roadmap names (`SplitExtract`, `MergeAbsorb`, cast-incompatible,
populated-NOT-NULL) plus `parse_status = dynamic_ddl` all have distinct
`GeneratorError` variants with actionable messages pointing to the
milestone that will unblock them (M15, M15, M14, M14, M14 respectively).
Day-one mitigation: exhaustive `match` on `change_kind` — the Rust
compiler forces the author to handle every variant. A golden-file fixture
exists for each fail-closed branch.

**Non-determinism.** `stable_topo` plus sorted `extension_versions` plus
fixed UTC formatting remove the usual sources. `generated_at` is the one
field that differs between runs; the deterministic-re-run test fixes it
via a clock injection point on `GenerateContext`.

**Cross-section statement dependencies.** A `post_commit` statement may
semantically depend on a `transaction` statement (normal case: an index
on a freshly-created table). Within-section the walk respects topo order;
between sections the runner guarantees ordering
(`pre_begin → transaction → post_commit`). Day-one mitigation: the walk
preserves topo-ordered emission of each statement, and the writer preserves
emission order within each section. Cross-section dependency inversion
(e.g. a `transaction` statement depending on a `post_commit` one) is
blocked by construction: the router's section assignment is a property of
the AST kind, not of dependency chains, and `post_commit` targets are
all of the "built late, depended on by nothing in this migration" variety
(`CREATE INDEX CONCURRENTLY` etc.).

## Tests

**Golden-file tests** live in `src/migration_generator/tests/golden/`.
Each fixture is a pair: a JSON file containing a serialized `ChangeSet`
(+ a stub `object_dag`) and the expected `.sql` output. Initial set:

```
golden/
  creates_only.changeset.json      creates_only.sql
  drops_only.changeset.json        drops_only.sql
  alters_only.changeset.json       alters_only.sql
  mixed.changeset.json             mixed.sql
  fail_split_extract.changeset.json   (no .sql, expects GeneratorError::SplitExtractUnsupported)
  fail_populated_not_null.changeset.json  (expects GeneratorError::PopulatedNotNull)
```

Determinism test: for every golden fixture, run the generator twice with
a fixed injected clock; assert byte-for-byte equality.

**Section-routing unit tests** in `src/migration_generator/tests/routing.rs`.
One test per row of the routing table. Plus negative tests: a synthesized
unknown AST kind asserts `GeneratorError::UnknownRoute`. Plus a
router-coverage test that enumerates every AST kind M3 can currently emit
and asserts the router has a rule for each; this test fails CI when M3
adds a new kind without a corresponding router entry.

**Walk ordering unit tests** in `src/migration_generator/tests/walk.rs`.
Fixture DAGs of 3–5 nodes. Assert drops come out leaves-first, creates
come out roots-first, alters respect dependency order.

Run:

```bash
cargo test --package topcat migration_generator
cargo test --package topcat migration_generator::tests::routing
```

## Verification snapshot

- `cargo build --release` succeeds.
- `cargo clippy -- -D warnings` clean on `src/migration_generator/**`.
- `cargo test migration_generator` green, including all golden fixtures.
- `topcat migrate generate` invoked on the `mixed` fixture produces a file
  that opens cleanly against a local shadow (manual spot check; full
  round-trip is M10's job).
- Deterministic re-run: running generate twice with the same input yields
  identical bytes.
- `parents: []` placeholder present in every generated header; documented
  in the plan as the M13 hand-off point.

## Open questions to resolve before starting

1. **`GenerateContext` shape.** What does M9 need from the caller —
   shadow connection strings, or pre-fetched `pg_version` and
   `extension_versions`? The latter keeps M9 pure and testable; the former
   matches M10's likely shape. Default assumption: pure. Confirm during
   CLI wiring.
2. **Empty-ChangeSet behavior.** No-op diff → no file written? Or emit a
   zero-change migration with a valid header? Default assumption: refuse
   with a clear error; no migration file on no-op.
3. **`changes_summary` format stability.** Is the per-node line format
   considered part of the content-hash input? Architecture §Content-hash
   stability says no (it's a summary, not structural). Confirm: hash is
   over the `ChangeSet` as M8 computes it, not over the summary text.
4. **Extension-install section routing.** Several extensions require
   `pre_begin` installation; most don't. Default assumption: router
   carries a small extname allowlist; pushed to `pre_begin` only for
   extensions on that list. Enumerate during Phase 1.
5. **AST kind surface from M3.** M9's router needs a stable enum of
   statement kinds. Confirm with M3 that such an enum is exposed, rather
   than M9 having to sniff emitted SQL text.

## Session sizing notes

Roadmap estimate: ~2 sessions. Breakdown aligns:

- **Session 1** (~4 hours): Phases 1 and 2 — router, error taxonomy,
  topological walk. Ends with router + walk passing their unit tests
  against stub ChangeSets and stub DAGs.
- **Session 2** (~4 hours): Phases 3 and 4 — header, writer, CLI wiring,
  golden fixtures. Ends with `cargo test migration_generator` green and
  `topcat migrate generate` producing a valid file end-to-end.

One extra session of slack if M3's AST-kind surface is not stable by the
time M9 starts; resolving that is Phase 1's first task and is the most
likely source of schedule slip.

## Referenced architecture sections

- §Diff and migration generation — input shape, topological walk policy.
- §Migration file format — header fields, section semantics, exact text
  layout the writer must reproduce.
- §Bounce markers — explicitly out of scope; linked here so the reader
  knows where the deferred surface will land (M14).
- §Content-hash stability — `migration_id` source; M9 consumes, M8 emits.
- §Object DAG — node/edge shape M9 walks.
- §Edge authority — edge resolution is M7's; M9 trusts resolved edges.
- §Component map — `migration_generator` row; M9 implements the forward
  half of that row.

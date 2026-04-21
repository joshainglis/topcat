# M15 — split/merge annotations

> Source of truth: docs/pg-dev-buddy-roadmap.md §M15, docs/pg-dev-buddy-architecture.md §Data migration taxonomy, §Split and merge annotations, §Bounce markers, §CatalogReader, SchemaModel, and DDL emitter, §Reverse migrations.

## Context

M15 lets users declare table-level refactors (splits and merges) as first-class
source annotations so the differ can emit the data-move DDL automatically
instead of bouncing the migration back to the operator. Without this
milestone, any vertical split of a wide table or any merge of two tables
lands in M14's `kind=refactor` bounce bucket and the operator hand-writes
the data-move SQL in the ack file — tolerable once, painful at scale.

The architecture already spells the contract out (§Split and merge
annotations): five new header keys, a fixed three- or four-statement DDL
template per refactor, and a clear fallback (`kind=refactor_complex`) for
moves that cannot be expressed as a column projection. M15 is the milestone
that wires this contract through the parser, the differ, and the emitter.

M15 is structurally small — new header keys, one new bounce kind beyond
those M14 already ships, a fallback bounce, and a tight set of emitter
templates — but it sits on the critical correctness axis: a
mis-parsed `merge_column_map` or a missed FK rewire can silently drop
production data. The risk controls in this plan are tuned accordingly:
eager parse-time validation, per-FK bounce enumeration, fixture tests
covering both the clean case and every failure mode.

## Prerequisites

Depends on **M14 — general bounce markers + ack workflow** (roadmap §M14).
M15 reuses M14's `-- topcat:bounce` grammar verbatim and its ack-file
workflow. The specific M14-owned artifacts M15 extends:

- M14's bounce-emitter helper (introduced in `migration_generator`; the
  exact module name is fixed by M14 — M15 must call that helper rather
  than hand-rolling `-- topcat:bounce` lines).
- M14's bounce-kind enum: M15 adds `fk_rewire` and `refactor_complex`
  variants. These names are fixed by architecture §Data migration taxonomy
  and must match exactly.
- M14's ack-file template generator (`topcat migrate ack`): once M15's new
  kinds exist, the templater must recognise them without code changes
  (this is the "add enum variant, templater keeps working" discipline
  baked into M14).

Further topcat-crate artifacts M15 extends (all already in `src/`):

- `src/file_node/parsing.rs` — where header keys are parsed today
  (`requires:`, `dropped_by:`, `layer:`, `exists:`). The five new keys land
  here. This file is the **primary** entry point for M15.
- `src/file_node/mod.rs` — `FileNode` struct: add fields for the parsed
  split/merge annotations so later stages can read them without re-parsing.
- `src/file_node/tests.rs` — unit tests for the new header keys.
- `src/exceptions.rs` — new `FileNodeError` variants for malformed
  `split_columns`, `split_key`, `merge_column_map`, and for unknown
  `split_from`/`merge_from` `node_id` references.
- `ddl_emitter` crate/module (introduced by M3, roadmap §M3) — M15 adds
  data-move DDL templates to it. The module does not exist in the current
  tree; M15 edits it in its M3-landed form.
- `migration_generator` (introduced by M9, roadmap §M9) — the differ that
  owns the decision "emit data-move DDL vs fall back to
  `kind=refactor_complex` bounce." M15 adds the split/merge branch to its
  dispatch.
- M2 `schema_model` / `catalog_reader` — required for FK enumeration on
  dropped columns. M15 reads FKs from the catalog snapshot the differ
  already loads; no new catalog queries required beyond what M8 (differ)
  relies on.
- `serde_json` — already a candidate transitive dep but may need to be
  added to `Cargo.toml` as a direct dependency if it isn't already, since
  the `merge_column_map` value is parsed eagerly as JSON at header-parse
  time.

New Cargo deps: `serde_json` (direct) if not already present. No async
runtime changes. No catalog-reader changes.

## Scope

Roadmap verbatim (§M15): *"declare refactors in source files, get automated
DDL."*

Amplified: five new file-header keys, emitter support for the standard
data-move DDL template, a new bounce kind for FK rewire decisions, and a
fallback bounce for non-projection moves.

### In scope for this milestone

- **Header parsing** in `src/file_node/parsing.rs` for:
  - `-- split_from: <node_id>` — references the source table being split.
  - `-- split_columns: col1, col2, ...` — columns moved to this file's
    object. Comma-separated identifiers; whitespace trimmed.
  - `-- split_key: <col>` — the column that carries the join key from
    source to target (typically the FK / PK). Single identifier.
  - `-- merge_from: <node_id>` — references the source table being merged
    into this file's object.
  - `-- merge_column_map: {"<src_qualified_col>":"<dst_qualified_col>", ...}` —
    JSON object literal mapping source columns to destination columns.
- **Eager parse-time validation** of every new key:
  - `split_columns` must be a non-empty list of SQL-identifier-shaped
    tokens; duplicates rejected.
  - `split_key` must be a single SQL-identifier-shaped token.
  - `merge_column_map` must parse as a JSON object via `serde_json`; keys
    and values must be `"schema.table.column"` or `"table.column"` strings;
    empty map rejected; duplicate values (same destination targeted twice)
    rejected.
  - `split_from` / `merge_from` values must parse as ULID/UUID-shaped
    `node_id` strings (concrete shape owned by M11 — delegate to M11's
    `node_id` parser rather than defining one here).
- **Emitter support for data-move DDL** (in the M3-landed `ddl_emitter`):
  - Split template (architecture §Split and merge annotations lines
    822–824): `CREATE TABLE <target>`, `INSERT INTO <target> (<split_key>,
    <split_columns>) SELECT ... FROM <source>`, `ALTER TABLE <source> DROP
    COLUMN ...`.
  - Merge template (architecture §Split and merge annotations lines
    839–841): `ALTER TABLE <target> ADD COLUMN` for any missing targets,
    `INSERT INTO <target> (<mapped_dst>) SELECT <mapped_src> FROM
    <source>`, `DROP TABLE <source>`.
- **`kind=fk_rewire` bounce emission** in the differ: for every FK in the
  pre-split catalog snapshot that references a column now being dropped,
  emit a `-- topcat:bounce id=<uuid> kind=fk_rewire` record scoped to a
  `placeholder_statement=true` statement, so the operator chooses whether
  the FK should now target the new (split) table or be removed.
- **`kind=refactor_complex` fallback bounce** in the differ: emitted when
  a declared split/merge cannot be expressed as the standard column
  projection. Trigger conditions (architecture §Split and merge
  annotations line 843–846):
  - a `merge_column_map` value uses a qualified column the source table
    does not actually have (detected via `schema_model`);
  - the column-map implies a type coercion beyond an implicit cast
    (detected via `schema_model` column types);
  - any source column in the split/merge participates in a computed
    generated-column expression or filter predicate the annotation does
    not declare;
  - any source column has an aggregation dependency (e.g. a materialised
    view or trigger reads it in an aggregate position) that breaks with a
    row-by-row copy.
- **Reverse-migration hooks** (architecture §Reverse migrations lines
  613–615): `SplitExtract` reverses to a merge, `MergeAbsorb` reverses to
  a split. M15 emits the `ChangeKind` variant for each forward case; the
  M13 reverse-migration generator already owns the reverse-direction
  templating. M15 does **not** re-implement reverse templating.
- **Unit + integration tests**: see §Tests.
- **Documentation**: extend `CLAUDE.md`'s §Key Concepts with the five new
  keys once implemented; extend `topcat.toml.example` only if any new
  config knobs are introduced (none are anticipated — the behaviour is
  fully driven by headers).

### Explicitly NOT in scope (deferred)

- **Runner-side ack validation for the new kinds** — M14 already ships a
  reference validator; it must not need changes for new enum variants.
  If the validator is hard-coded to a closed enum list, that's a M14 bug
  and gets fixed in M14, not here.
- **Automatic FK rewire** — architecture line 825 is explicit: FK
  consequences always bounce. M15 only enumerates and emits bounces; it
  never decides.
- **Row-level data-move with predicates / aggregation** — handled via
  `kind=refactor_complex` bounce. M15 does not emit filtered or aggregated
  INSERT SELECT statements.
- **Table-level rename** — already handled by M13 (roadmap §M13 rename
  detection). Splits and merges are orthogonal to renames.
- **Partition split / detach** — architecture §Object types covered line
  488 lists `PartitionDetach` as a distinct change; M15 does not cover
  it. Partition workflows are M14 `partition_bounds_complex` territory.
- **`split_from` / `merge_from` referring to a node in a different Object
  DAG subtree** — cross-schema splits are in scope as a shape; cross-DAG
  splits (if any meaning) are not.

## Deliverables (files and modules)

| Kind        | Path                                                | Status  | Description |
|-------------|-----------------------------------------------------|---------|-------------|
| extend      | src/file_node/parsing.rs                            | modify  | Add header-line recognition for `split_from:`, `split_columns:`, `split_key:`, `merge_from:`, `merge_column_map:`. Use the same `{comment_str} <key>:` prefix construction the existing keys use (line 97 onwards). Eagerly parse `merge_column_map` via `serde_json::from_str::<BTreeMap<String,String>>`. Reject malformed input with a new `FileNodeError` variant per failure mode. |
| extend      | src/file_node/mod.rs                                | modify  | Add optional struct fields to `FileNode`: `split_from: Option<NodeId>`, `split_columns: Vec<String>`, `split_key: Option<String>`, `merge_from: Option<NodeId>`, `merge_column_map: BTreeMap<String,String>`. Preserve existing `Ord` / `Hash` contract (exclude these fields from the sort key — they're refactor metadata, not identity). |
| extend      | src/file_node/tests.rs                              | modify  | Unit tests for every new key: happy path, empty value rejected, duplicate columns rejected, malformed JSON rejected, unknown node_id rejected, presence of both `split_from` and `merge_from` on the same file rejected. |
| extend      | src/exceptions.rs                                   | modify  | New `FileNodeError` variants: `InvalidSplitColumns`, `InvalidSplitKey`, `InvalidMergeColumnMap(serde_json::Error)`, `ConflictingRefactorAnnotations`. `Display` impls include the source file path. |
| extend      | Cargo.toml                                          | modify  | Add `serde_json = "1"` as a direct dependency if not already present (it may already be transitive via the `config` crate; do not rely on re-export). |
| extend      | ddl_emitter (introduced by M3)                      | modify  | Add `emit_split_data_move(source, target, split_key, split_columns)` and `emit_merge_data_move(source, target, column_map)` functions returning the three- / four-statement DDL blocks per architecture §Split and merge annotations. Single-responsibility: they emit the statements, they do **not** call the differ or decide bounces. |
| extend      | migration_generator (introduced by M9)              | modify  | In the ChangeSet dispatch, when a FileNode declares `split_from` / `merge_from` and the destination column set is a pure projection of the source (validated against `schema_model`), emit the data-move DDL; otherwise emit a `kind=refactor_complex` bounce. After the split `ALTER TABLE … DROP COLUMN`, enumerate FKs referencing the dropped columns from the catalog snapshot and emit one `kind=fk_rewire` bounce per FK. |
| extend      | migration_generator (introduced by M9)              | modify  | Add `ChangeKind::SplitExtract { source_node_id, split_key, columns }` and `ChangeKind::MergeAbsorb { source_node_id, column_map }` to the enum architecture §Object types covered line 488 names. M13's reverse generator consumes these — do not reshape them after that lands. |
| new test    | tests/input/split_merge/*                           | create  | Fixture SQL files per scenario — see §Tests for the enumerated matrix. |
| new test    | tests/cli_split_merge_tests.rs                      | create  | Integration tests: per-fixture, run the differ and assert the emitted migration contents (DDL + bounce markers). |
| extend      | CLAUDE.md (§Key Concepts → File Metadata)           | modify  | Document the five new keys alongside the existing `requires:` / `exists:` / `layer:` examples. Keep it terse — just a single `-- split_from: ...` example per refactor. |

No file creations in `src/` — all five new header keys live in
`parsing.rs`. No new modules. No new crates (`serde_json` is a dep bump
at most, not a new sub-crate). `ddl_emitter` and `migration_generator`
are edits to modules that land with M3 and M9 respectively.

## Implementation phases

### Phase 1 — Header parsing (in-crate, no dependencies on M3/M9)

**Goal**: the five new header keys parse, validate, and populate
`FileNode`. The File DAG rebuild ignores them for now — they're purely
metadata until the emitter picks them up.

**Tasks**:
- [ ] Extend `src/file_node/parsing.rs` with the five new prefix strings
  (mirror the `let dep_str = format!("{comment_str} requires:")` pattern
  at line 97).
- [ ] Add fields to `FileNode` and thread them through constructor and
  `Default`.
- [ ] Add `serde_json` parse of `merge_column_map`; wrap parse errors in
  `FileNodeError::InvalidMergeColumnMap`.
- [ ] Reject a file that declares both `split_from` and `merge_from` with
  `FileNodeError::ConflictingRefactorAnnotations`.
- [ ] Delegate `node_id` shape validation to M11's `node_id` parser if
  available; otherwise parse as UUID with `uuid::Uuid::parse_str` and
  upgrade to the ULID/UUID parser M11 ships when that lands.
- [ ] Unit tests in `src/file_node/tests.rs` for every happy path and
  every rejection path (minimum eight tests).

**Verification**:

```bash
cargo build
cargo test --lib file_node
cargo clippy --all-targets -- -D warnings
```

### Phase 2 — Emitter templates (extends M3 ddl_emitter)

**Goal**: `ddl_emitter` can produce the split and merge data-move DDL
given a decoded annotation. No differ integration yet.

**Tasks**:
- [ ] Implement `emit_split_data_move` per architecture §Split and merge
  annotations lines 822–824. Use SchemaModel-qualified names throughout;
  never interpolate raw identifiers (risk: quoting).
- [ ] Implement `emit_merge_data_move` per architecture §Split and merge
  annotations lines 839–841. Order `ADD COLUMN` → `INSERT … SELECT` →
  `DROP TABLE` exactly as specified; downstream reverse-migration logic
  relies on this ordering.
- [ ] Property tests (if the existing `ddl_emitter` round-trip harness
  from M3 is available): for a fixture SchemaModel pair,
  `parse(emit_split_data_move(...))` produces an AST with the expected
  three statement kinds in order.
- [ ] Smoke integration with a hand-built `FileNode` fixture.

**Verification**:

```bash
cargo test --lib ddl_emitter
cargo test --test ddl_emitter_round_trip    # assumes M3 shipped this
```

### Phase 3 — Differ integration, FK enumeration, fallback bounce

**Goal**: end-to-end. A split/merge-annotated file produces the correct
migration: data-move DDL + one `kind=fk_rewire` per affected FK + a
`kind=refactor_complex` when the projection isn't pure.

**Tasks**:
- [ ] In `migration_generator`, add the split/merge dispatch branch. Place
  it in the same decision point that already chooses `kind=refactor` for
  undeclared split/merge (architecture §Data migration taxonomy line 798).
- [ ] Pure-projection check: walk `merge_column_map` values and every
  column in `split_columns`; compare with SchemaModel column shape.
  Reject (→ `refactor_complex`) on any of the four trigger conditions in
  §Scope.
- [ ] FK enumeration for split case: iterate
  `schema_model.foreign_keys()` (M2 already provides this), filter to
  those referencing columns in `split_columns` of the source table, emit
  one bounce per FK with `object=<fk_name>` and a
  `placeholder_statement=true` `ALTER TABLE … DROP CONSTRAINT …` skeleton.
- [ ] FK enumeration for merge case: architecture line 841 has the merge
  drop the source table entirely — every FK referencing the source emits
  `kind=fk_rewire`. Same skeleton.
- [ ] `ChangeKind::SplitExtract` and `ChangeKind::MergeAbsorb` produced
  by this branch; verify they serialise into the content-hash in the
  order M9's canonicaliser expects (no map insertion-order bugs).
- [ ] Integration test harness: run the differ on each fixture in
  `tests/input/split_merge/` and snapshot-compare the emitted migration.

**Verification**:

```bash
cargo test --test cli_split_merge_tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Risks (copied and amplified)

Roadmap §M15 flags two risks. Both are real and neither has a trivial
mitigation.

- **Column-map grammar is JSON inside a comment.** Severity: medium.
  Easy to write (`-- merge_column_map: {"a":"b"}`), easy to mis-parse
  (line continuation, embedded `--`, nested quotes, UTF-8 BOM on Windows,
  trailing commas that `serde_json` rejects but JavaScript tooling
  produces). **Day-one mitigation**:
  1. Require the entire map on a single header line. Reject a map that
     spans multiple `-- merge_column_map:` lines with a clear error (no
     clever continuation rule — we own the grammar, we forbid
     ambiguity). Architecture shows a single-line map; codify it.
  2. Parse with `serde_json::from_str::<BTreeMap<String,String>>` at
     header-parse time, not lazily when the emitter runs. A broken file
     must fail `topcat update` / DAG build — never silently pass through
     to the migration generator.
  3. Validate keys and values are `"<ident>.<ident>"` or
     `"<ident>.<ident>.<ident>"` after JSON parse. Reject anything else
     before the annotation reaches the emitter.
  4. Provide a concrete error message per failure: `"merge_column_map
     must be a JSON object on a single header line; got: {source}"` with
     the file path.
- **FK consequences are subtle.** Severity: high. An FK targeting a
  column that `-- split_columns:` moves out of the source table will
  point at a dropped column if the differ blindly emits
  `ALTER TABLE source DROP COLUMN`. Worst case: `ON DELETE CASCADE` FKs
  silently destroy rows before the operator notices. **Day-one
  mitigation**:
  1. Enumerate every FK on every dropped column before emitting the
     `DROP COLUMN`. One `kind=fk_rewire` bounce per FK; each bounce
     carries the FK definition (name, columns, `ON UPDATE` /
     `ON DELETE` clauses) in the `message` field for operator review.
  2. The bounce's `placeholder_statement=true` body is an `ALTER TABLE
     … DROP CONSTRAINT …` skeleton. Operator replaces it with whatever
     rewire is correct: re-target, drop, or emit a new FK on the split
     target table.
  3. Refuse to emit any `DROP COLUMN` until the FK enumeration
     completes. If FK enumeration fails (catalog-reader error), the
     whole migration fails — never partially succeed past a FK check.
  4. Mirror this for merge: the `DROP TABLE` in the merge template drops
     the source table whole, so every FK referencing it rewires.
- **Same-batch split-and-merge on overlapping columns.** Severity:
  medium. A file `profiles` declaring `-- split_from: users` and another
  file declaring `-- merge_from: users` in the same batch could both
  fire against `users`, with undefined outcome. **Day-one mitigation**:
  during `file_dag` build, detect the collision and reject with a
  dedicated error. One refactor per source-per-batch.
- **`serde_json` parse-time cost on large header blocks.** Severity:
  low. Parsing runs once per file. Even a 100k-file repo is sub-second.
  No mitigation required; flag only so the perf milestone (M20) doesn't
  profile this in isolation and mis-attribute cost.

## Tests

Fixture layout under `tests/input/split_merge/`:

- `split_basic/` — one source, one new file declaring `-- split_from`,
  no FKs. Expected output: three statements, no bounces.
- `split_with_fk/` — source has one FK referencing a column in
  `-- split_columns`. Expected: three statements + one `kind=fk_rewire`
  bounce.
- `split_with_multi_fk/` — two FKs on split columns. Expected: three
  statements + two `kind=fk_rewire` bounces.
- `split_missing_key/` — declares `-- split_columns` with no
  `-- split_key`. Expected: `FileNodeError::InvalidSplitKey` at parse
  time; DAG never builds.
- `split_invalid_column/` — `-- split_columns` names a column absent
  from the source. Expected: `kind=refactor_complex` bounce (runtime
  check after SchemaModel loads, not parse-time).
- `merge_basic/` — two sources, one new file declaring `-- merge_from`
  with a clean column map, no FKs. Expected: three statements, no
  bounces.
- `merge_with_fk/` — source table has inbound FKs. Expected: three
  statements + one `kind=fk_rewire` per FK.
- `merge_malformed_json/` — `-- merge_column_map:` has a trailing
  comma. Expected: `FileNodeError::InvalidMergeColumnMap` at parse time.
- `merge_type_coercion/` — column map implies a non-trivial cast
  (e.g. `text` → `jsonb`). Expected: `kind=refactor_complex` bounce.
- `conflicting_annotations/` — a file declares both `-- split_from`
  and `-- merge_from`. Expected:
  `FileNodeError::ConflictingRefactorAnnotations` at parse time.
- `split_and_merge_same_source/` — one file splits from `users`,
  another merges from `users`, same batch. Expected: DAG build error,
  no migration emitted.

Test harness:

```rust
// tests/cli_split_merge_tests.rs pattern
for fixture in walk("tests/input/split_merge") {
    let output = run_differ(fixture);
    let snapshot = read_to_string(fixture.join("expected.sql"));
    assert_eq!(output, snapshot);
}
```

Unit tests in `src/file_node/tests.rs` cover every parse-time failure
mode exhaustively (the integration harness is one-shot; unit tests
iterate cheaply).

No property tests in M15 — the emitter's round-trip property is owned
by M3. M15's property would be "for random SchemaModel pairs with
random FK shapes, FK enumeration produces one bounce per FK on a
dropped column" — worth having, but parks under M20 hardening unless
time permits in the second session.

## Verification snapshot

```bash
# Build and format gates
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Focused suites
cargo test --lib file_node
cargo test --lib ddl_emitter
cargo test --test cli_split_merge_tests

# Existing behaviour unchanged
cargo test
cargo run -- concat -i tests/input/sql /tmp/m15-smoke.sql

# Smoke: annotated fixtures produce the expected migration shape
cargo run -- diff --from tests/input/split_merge/split_basic/before \
                  --to   tests/input/split_merge/split_basic/after
cargo run -- diff --from tests/input/split_merge/merge_with_fk/before \
                  --to   tests/input/split_merge/merge_with_fk/after \
  | grep -F "kind=fk_rewire"
cargo run -- diff --from tests/input/split_merge/merge_malformed_json/before \
                  --to   tests/input/split_merge/merge_malformed_json/after \
  2>&1 | grep -F "InvalidMergeColumnMap"
```

## Open questions to resolve before starting

1. **`merge_from` vs `merge_into` wording.** Architecture is internally
   inconsistent: line 797 and 798 (`Data migration taxonomy` table) say
   merges are declared via `-- merge_into:`, while the worked example at
   line 833 uses `-- merge_from:`. The roadmap §M15 deliverables list and
   this plan both follow the worked example (`-- merge_from:`).
   **Decision needed**: either (a) fix the taxonomy table to match
   `merge_from:`, or (b) fix the example and roadmap to match
   `merge_into:`. I recommend (a) — `merge_from:` is symmetric with
   `split_from:` and that symmetry has teaching value. Flag as an
   architecture-doc fix, not a M15 scope question; the plan adopts
   `merge_from:` until explicitly told otherwise.
2. **Where does the `node_id` parser live?** M11 owns node_id identity.
   If M11 lands before M15, use its parser. If not, use `uuid::Uuid`
   with an explicit `// node_id parser supplied by M11` code comment.
   Either way, M15 must not ship its own permanent `node_id` parser.
3. **Do we need a `--no-emit-split-merge` flag?** Nothing in the roadmap
   or architecture asks for one, but operators may want to test
   annotations without the differ firing data-moves. Default answer:
   no — if you don't want the DDL, don't declare the annotation.
   Reconsider if integration testing surfaces a real need.
4. **Cross-schema splits.** Can `-- split_from:` reference a node_id in
   a different schema than the new file's object? Architecture is
   silent. Default answer: yes, and the emitter fully qualifies every
   identifier in its output. Tests should include at least one
   cross-schema fixture.

## Session sizing notes

Roadmap estimate: **~2 sessions**. Realistic:

- **Session 1** — Phases 1 and 2 (header parsing + emitter templates).
  Self-contained; lands with unit tests and in-crate smoke. No
  dependencies on `migration_generator` being wired.
- **Session 2** — Phase 3 (differ integration + FK enumeration +
  fallback bounce) with full integration fixtures. This is where the
  real architectural risk lives.

Natural PR split: Session 1 → one PR (parsing + emitter, pure
extension). Session 2 → one PR (differ behaviour, guarded by the
fixture snapshots).

**Parallelizable with M16, M17, M18** (roadmap §M15 "Parallelizable
with"). M15 shares no modified files with M16 (reconcile +
drift detection edits `event_trigger` consumers), M17 (bootstrap edits
`shadow_db` startup), or M18 (uninstall edits `meta_schema`). All four
can be independent agent worktrees at the same time. Merge order is
arbitrary; none of them take cross-dependencies on each other.

## Referenced architecture sections

- `## Data migration taxonomy` — names the bounce kinds (`refactor`,
  `refactor_complex`, `fk_rewire`) and the default policy for undeclared
  split/merge.
- `### Split and merge annotations` — full header grammar and the
  three-/four-statement DDL templates M15 implements.
- `### Bounce markers` — the `-- topcat:bounce` grammar M14 ships and
  M15 reuses verbatim for `fk_rewire` and `refactor_complex`.
- `## CatalogReader, SchemaModel, and DDL emitter` — names the emitter
  module M15 extends with data-move templates and the catalog source for
  FK enumeration.
- `## Reverse migrations` — documents that `SplitExtract` reverses to a
  merge and `MergeAbsorb` reverses to a split. M15 emits the forward
  `ChangeKind` variants; M13 owns reverse templating.
- `## Object DAG` — provides the `node_id` identity model that
  `-- split_from:` and `-- merge_from:` reference.

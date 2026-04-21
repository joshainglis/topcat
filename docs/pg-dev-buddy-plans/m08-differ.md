# M8 — differ

> Self-contained briefing for the M8 milestone of the pg-dev-buddy roadmap.
> Produces a deterministic `ChangeSet` from two `SchemaModel` snapshots and
> the stable `migration_id` content-hash that downstream migration generation
> (M9) and verification (M10) pin against. Read alongside
> `docs/pg-dev-buddy-roadmap.md` §M8 and
> `docs/pg-dev-buddy-architecture.md` §Diff and migration generation /
> §Content-hash stability.

## Context

The differ is the pivot from "two catalog snapshots" to "a plan of DDL". It
takes two frozen `SchemaModel` values — typically `shadow_main` (last
committed state) and `shadow_head` (working-tree state) — and produces an
ordered, normalized `ChangeSet`. Every downstream artifact (forward
migration in M9, verifier in M10, reverse in M13, bounce ack scaffolding in
M14) keys off the `ChangeSet` and its SHA-256 content hash. Getting M8
wrong poisons every migration that follows, so determinism is non-negotiable:
the same two models must produce the same byte-for-byte `ChangeSet` on any
host, on any run, across process restarts.

M8 is structural only. Semantic questions ("is this rename safe?", "does
this `ALTER` need backfill?") belong to M9's generator and M14's bounce
logic. The differ reports *what* changed. It does not reason about *how* to
apply the change.

M8 is green-field in the sync Rust tree (`src/differ/**`). It consumes M2's
`SchemaModel` crate directly and produces a type that M9 reads.

## Prerequisites

**Hard dependency: M2 — schema_model + catalog_reader.** M8 cannot start
until M2 is merged.

Concrete `schema_model` APIs M8 depends on:

- **Typed row collections per object type.** `Table`, `Column`,
  `Constraint`, `Index`, `Function`, `View`, `MaterializedView`, `Sequence`,
  `Type` (enum/composite/domain/range/multirange), `Trigger`, `Policy`,
  `Rule`, `Schema`, `Extension`, plus the long tail (text search, FDW,
  operator/class/family, aggregate, cast, conversion, statistics,
  publication, access method, user-event-trigger). Full inventory in
  architecture §Object types covered.
- **Stable iteration over a `SchemaModel`.** Each object collection must
  expose deterministic ordering (either sorted at read time in M2 or sortable
  via a `BTreeMap`-style keyed view in M8). The differ sorts before
  comparing — no reliance on insertion order.
- **Keying by `node_id`.** Every typed row carries `node_id: Uuid` (UUIDv7).
  M8 uses `node_id` as the primary identity key for pairing before/after
  rows. Schema-qualified names are a secondary key (for cases where node_id
  is absent on one side — the Rename change_kind).
- **JCS JSON serialization per RFC 8785.** M2 ships `serde`-backed
  serialization that emits UTF-8, sorted object keys, no insignificant
  whitespace. M8 feeds `ChangeSet` through the same JCS path to compute the
  content hash.
- **Canonical forms already applied.** `varchar`/`text` collapse, identity
  vs serial vs default union, viewdef canonicalization, `reloptions`
  sorting, RLS tri-state, schema-qualified sequence/collation refs — all
  landed in M2 per architecture §Normalization rules. The differ does
  *not* re-normalize; it compares post-normalization values.
- **Partition tree metadata.** `partition_strategy`, `partition_of`,
  `partition_bounds`, and `inherits` fields on `Table`. M8 uses these to
  emit `PartitionAttach` / `PartitionDetach` change_kinds and to detect
  bound changes (architecture §Partitioned tables and inheritance).
- **Auto-generated constraint name fields are flagged or excluded.** M2
  strips auto-names from the hash input path per §Content-hash stability;
  M8 inherits that behavior and compares on the synthesized stable name.

## Scope

### In scope

- Per-object-type structural equality, driven by the canonical forms from
  M2 — not textual DDL comparison.
- `ChangeSet`: an ordered `Vec<Change>` where
  `Change = { node_id, change_kind, before: Option<ObjectModel>,
  after: Option<ObjectModel> }`.
- The `ChangeKind` enum with all eight variants the architecture names:
  `Add`, `Drop`, `Alter`, `Rename`, `PartitionAttach`, `PartitionDetach`,
  `SplitExtract`, `MergeAbsorb`.
- Content-hash computation: `migration_id = sha256(jcs(ChangeSet))` with
  the included/excluded field rules from architecture §Content-hash
  stability.
- A deterministic ordering of entries within a `ChangeSet` (keyed by
  `(object_kind, schema, name, node_id)`) so two runs on the same inputs
  produce identical byte sequences.
- Public API: `fn diff(before: &SchemaModel, after: &SchemaModel)
  -> ChangeSet` and `fn ChangeSet::migration_id(&self) -> MigrationId`.

### Out of scope (for M8)

- **Ordering against the Object DAG.** Topological ordering of statements
  (drops-leaves-first, creates-roots-first) is M9's job. M8 emits entries
  keyed for *determinism*, not execution order.
- **Rename identity resolution.** M8 detects `Rename` via `node_id` match
  with differing schema-qualified names, but the full registry-backed
  three-way resolution lands in M11 (architecture §Three-way node_id
  resolution).
- **DDL emission.** M3 owns `SchemaModel → SQL`. M8 produces structured
  change records only.
- **Split / merge inference.** `SplitExtract` and `MergeAbsorb` variants
  exist in `ChangeKind`, but M8 never synthesizes them — they are produced
  only when operator-supplied annotations arrive in M15.
- **Migration file construction, headers, sections, bounce markers.** All
  M9/M14 territory.
- **Shadow-db interaction.** The differ is pure: two models in, one
  ChangeSet out. No I/O.

## Deliverables (files and modules)

Create `src/differ/` with:

| Path | Purpose |
|---|---|
| `src/differ/mod.rs` | Public surface: `diff()` entry point, re-exports of `ChangeSet`, `Change`, `ChangeKind`, `MigrationId`. |
| `src/differ/change_kind.rs` | `ChangeKind` enum with `Add`, `Drop`, `Alter`, `Rename`, `PartitionAttach`, `PartitionDetach`, `SplitExtract`, `MergeAbsorb`. `serde` derive; enum variant names fixed as hash-input constants. |
| `src/differ/change_set.rs` | `Change` struct, `ChangeSet` newtype over `Vec<Change>`, deterministic insertion/sort, `migration_id()` method that routes through JCS + SHA-256. |
| `src/differ/equality.rs` | Per-object-type structural equality. One function per object kind (`table_eq`, `function_eq`, `view_eq`, etc.), each returning `Option<Change>` (None = no change, Some(Alter) with before/after, or a specialized variant). |
| `src/differ/pairing.rs` | Pairs `before`/`after` object collections by `node_id` first, schema-qualified name second. Emits `Add` for after-only, `Drop` for before-only, `Rename` for node_id match with name mismatch, `Alter` for everything else where `equality::*_eq` disagrees. |
| `src/differ/partitions.rs` | Detects `PartitionAttach` / `PartitionDetach` by walking `partition_of` deltas in the before/after table set. Bound changes surface as `Alter` on the child table; complex bound moves bubble to M14 via change_kind enrichment (flagged now, routed later). |
| `src/differ/hash.rs` | `migration_id()` implementation: serialize `ChangeSet` via M2's JCS path, SHA-256 the bytes, hex-encode. Excludes catalog OIDs, auto-generated constraint names, runtime stats (`last_analyzed`, `last_vacuumed`, `relpages`), per-object comments — per architecture §Content-hash stability. |
| `src/differ/hash_fields.rs` | Field-inclusion policy in one place: a small visitor/filter layer that strips the excluded fields before JCS serialization. Keeps "what counts for hash" auditable in one module rather than scattered across object definitions. |
| `tests/fixtures/differ/<change_kind>/{before,after}.json` | Minimal `SchemaModel` JCS fixtures for every `ChangeKind` variant. |
| `src/differ/tests.rs` (or `tests/differ_*.rs`) | Unit coverage per change_kind, identity test, hash-stability test. |

`ChangeSet` shape (frozen for M9's consumption):

```rust
pub struct Change {
    pub node_id: Option<Uuid>,   // None only for objects that lack one (e.g. pre-M5 legacy reads)
    pub kind: ChangeKind,
    pub before: Option<ObjectModel>,
    pub after: Option<ObjectModel>,
}

pub struct ChangeSet(pub Vec<Change>);
```

`MigrationId` is a newtype around `[u8; 32]` (or hex `String` — decide in
Phase 1) with `Display` as lowercase hex.

Content-hash field rules (copied from architecture §Content-hash stability,
enforced in `hash_fields.rs`):

- **Excluded from hash:** catalog OIDs, auto-generated constraint names,
  runtime statistics, per-object comments.
- **Included:** schema-qualified names, canonicalized column types,
  constraint expressions, function bodies (exact text), sequence
  parameters, RLS policy bodies, `pg_version`, `extension_versions`.

## Implementation phases

**Phase 1 — Types and serialization spine (~0.5 session).** Land
`ChangeKind`, `Change`, `ChangeSet`, `MigrationId`. Wire `serde` + JCS such
that an empty `ChangeSet` hashes to a known stable value. Decide
`MigrationId` representation (hex `String` recommended to match the header
format in architecture §Migration file format).

**Phase 2 — Pairing and equality for core object types (~1 session).**
Implement `pairing.rs` and `equality.rs` for: `Schema`, `Extension`,
`Table` (incl. columns, constraints, indexes), `Function`, `View`,
`MaterializedView`, `Sequence`, `Type`, `Trigger`, `Policy`. Each object
kind gets one equality function and a fixture pair covering Add, Drop,
Alter. `Rename` falls out of the pairing logic when node_ids match.

**Phase 3 — Partitions and the long tail (~0.25 session).** `partitions.rs`
for `PartitionAttach` / `PartitionDetach`. Long-tail object equality
(`Rule`, text search, FDW, operator classes, aggregates, casts,
conversions, statistics, publications, access methods, user event
triggers). Placeholder entries for `SplitExtract` / `MergeAbsorb` — types
compile, nothing produces them yet.

**Phase 4 — Hash determinism hardening and fixture sweep (~0.25 session).**
`hash_fields.rs` exclusion filter, round-trip hash-stability test under
reordered input, property test that `diff(a, a) == empty ChangeSet` for a
set of randomized fixture models.

## Risks (copied and amplified)

- **Determinism.** Same two models → same `ChangeSet` byte-for-byte → same
  `migration_id`. Three things must line up: JCS ordering from M2 (sorted
  keys, no whitespace), stable iteration over object collections (sort
  before pairing), and deterministic `Change` ordering within the
  `ChangeSet`. **Day-one mitigation:** write the hash-stability test
  *first*, before any equality logic — fail it by construction, watch it
  go green, and keep it in CI. Reorder fixture input arrays randomly in
  each run; hash must not move.
- **Rename detection identity.** M8 can only detect a `Rename` when
  `node_id` is present on both sides and names differ. The full identity
  machinery (registry lookup, clean-rename vs copy-paste vs unknown node_id
  distinguisher, three-way resolution on branch-merge collisions) lands in
  M11 per architecture §Conflict taxonomy. **Day-one mitigation:** M8
  ships the `Rename` variant and emits it when the node_id pairing says so;
  it does *not* try to recover identity from content similarity. If
  `node_id` is missing on one side, M8 emits paired `Add` + `Drop` and
  lets M11 promote it later.
- **Hash input field drift.** If fields included in the hash grow or
  shrink after M8 ships, existing `migration_id` values become unverifiable.
  **Day-one mitigation:** single source of truth in `hash_fields.rs`; add
  a comment pointing at architecture §Content-hash stability; land a golden
  file of a non-trivial `ChangeSet` JCS bytes + expected hash, so any
  accidental drift breaks CI loudly.
- **Partition-bound edge cases.** Simple bound changes become `Alter`;
  complex bound moves (overlaps requiring row redistribution) must bounce
  in M14 (architecture §Partitioned tables and inheritance notes
  `kind=partition_bounds_complex`). **Day-one mitigation:** M8 does not
  try to classify complexity — it emits the raw bound delta as an `Alter`
  with full before/after partition_bounds text. M9/M14 decide whether it
  needs a bounce.
- **`SplitExtract` / `MergeAbsorb` never synthesized.** M8 carries the
  variants but never emits them. **Day-one mitigation:** guard with a
  `debug_assert!` or `unreachable!` path in any code that would try to
  synthesize these, so M15 sees a clear wiring point.

## Tests

Fixture-driven, all comparisons done via M2's `SchemaModel` JCS form:

- **One fixture pair per `ChangeKind`** under `tests/fixtures/differ/`:
  `add/`, `drop/`, `alter/`, `rename/`, `partition_attach/`,
  `partition_detach/`. `split_extract/` and `merge_absorb/` get fixture
  pairs only to exercise the enum; the differ must *not* produce them.
- **Identity test.** `diff(model, model) == ChangeSet(vec![])` across
  every fixture model, with the empty-set hash asserted to a golden
  constant.
- **Round-trip hash stability under reordered input.** Load a
  `SchemaModel`, shuffle object-collection insertion order N times,
  re-diff against a different-but-fixed `after` model, assert all N
  `migration_id` values are byte-identical.
- **Per-object-kind equality unit tests.** Minimal `ChangeSet` produced
  for every normalization hot spot in architecture §Normalization rules
  (varchar length changes, identity ↔ serial conversions, reloptions
  reordering-only = no change, auto-constraint-name-only = no change,
  comment-only = no change — latter because comments are hash-excluded).
- **Golden `ChangeSet` test.** One moderately complex fixture pair
  (~10 objects, 3 kinds of changes) with a JCS-serialized `ChangeSet`
  checked into the repo; CI fails on any byte drift.
- **Rename-via-node_id test.** Same `node_id`, different
  schema-qualified name → exactly one `Rename` entry, no `Add`/`Drop`
  pair.
- **No rename without node_id test.** Missing `node_id` on one side →
  `Add` + `Drop`, never `Rename`.

## Verification snapshot

- `cargo build -p topcat` compiles cleanly with `src/differ/` included.
- `cargo test differ::` passes every fixture-pair and identity test.
- `cargo clippy -- -D warnings` clean in `src/differ/`.
- Golden `ChangeSet` JCS bytes + golden `migration_id` hex are committed
  and unchanged across three consecutive CI runs.
- A manual smoke run: load two M2 fixture schemas, print
  `diff(a, b).migration_id()` twice in a row — identical output.
- No new public API on `schema_model`; M8 is a pure consumer of M2.

## Open questions to resolve before starting

1. **`MigrationId` representation.** Hex `String` (matches architecture
   §Migration file format) vs `[u8; 32]` newtype with a `Display` impl. Hex
   `String` is simpler and matches the file-header emission path M9 will
   use; recommend going with that unless M9's draft surface disagrees.
2. **`ChangeSet` ordering key.** `(object_kind_ordinal, schema, name,
   node_id)` is the leading candidate. Confirm that object_kind ordering
   matches the determinism need without bleeding into M9's topological
   ordering.
3. **Auto-generated constraint names.** Architecture §Normalization rules
   says M2 synthesizes `<table>_<cols>_<kind>` and excludes the original
   from the hash. Confirm M2's output carries *only* the synthesized name,
   so M8's hash exclusion layer does not need a second strip pass.
4. **`pg_version` / `extension_versions` scope.** Architecture §Content-hash
   stability includes these in the hash, but they live on the
   `SchemaModel` container, not on individual change entries. Decide
   whether to include them in the `ChangeSet`'s hash envelope (recommended)
   or on each `Change` (redundant, bloats serialized output).
5. **`Rule` support.** Architecture lists rules in §Object types covered
   but they are rare and often fragile. Confirm M2 ships `Rule` typed rows
   in its first merge; if not, M8 stubs equality behind a tracking issue
   and leaves a follow-up note in the module docstring.
6. **Handling of `requires_input` hints.** Does M8 surface any hint that a
   particular `Alter` (e.g. a `NOT NULL` add on a populated column) will
   later need a bounce? Recommend no — that classification is M9/M14's
   job, and M8 stays purely structural.

## Session sizing notes

~2 sessions total:

- **Session 1.** Phase 1 (types + JCS + empty-set hash test) and Phase 2
  (pairing + equality for core object types, with fixtures).
- **Session 2.** Phase 3 (partitions + long tail) and Phase 4 (hash
  determinism hardening, golden `ChangeSet`, fixture sweep). Close out
  with the verification snapshot.

Parallelizable sub-work within Session 2: equality functions for the long
tail (casts, conversions, statistics, etc.) are independent of each other
and can be split across a pair if desired.

## Referenced architecture sections

- `docs/pg-dev-buddy-architecture.md` §CatalogReader, SchemaModel, and DDL
  emitter — source of `SchemaModel` shape.
- `docs/pg-dev-buddy-architecture.md` §Normalization rules — pre-applied
  canonical forms M8 depends on.
- `docs/pg-dev-buddy-architecture.md` §Partitioned tables and inheritance —
  `PartitionAttach` / `PartitionDetach` semantics and bound-change policy.
- `docs/pg-dev-buddy-architecture.md` §Object types covered — full
  inventory M8 must handle.
- `docs/pg-dev-buddy-architecture.md` §Diff and migration generation —
  `ChangeSet` shape and `change_kind` enumeration.
- `docs/pg-dev-buddy-architecture.md` §Content-hash stability —
  included/excluded field policy for `migration_id`.
- `docs/pg-dev-buddy-architecture.md` §Identity and node_id — primary
  pairing key.
- `docs/pg-dev-buddy-architecture.md` §Conflict taxonomy — context for why
  full `Rename` identity resolution defers to M11.
- `docs/pg-dev-buddy-architecture.md` §Three-way node_id resolution — the
  M11 machinery M8 stops short of.
- `docs/pg-dev-buddy-architecture.md` §Rename detection — mechanism M8
  implements the structural half of.
- `docs/pg-dev-buddy-architecture.md` §Migration file format — downstream
  consumer of `migration_id`.
- `docs/pg-dev-buddy-roadmap.md` §M2 — schema_model + catalog_reader —
  upstream dependency.
- `docs/pg-dev-buddy-roadmap.md` §M8 — differ — source scope and risk
  statements copied/amplified here.
- `docs/pg-dev-buddy-roadmap.md` §M9 — migration_generator (forward only) —
  immediate downstream consumer.
- `docs/pg-dev-buddy-roadmap.md` §M11 — node_id identity + conflict scan +
  three-way resolution — where `Rename` identity completes.
- `docs/pg-dev-buddy-roadmap.md` §M15 — split/merge annotations — where
  `SplitExtract` / `MergeAbsorb` get synthesized.

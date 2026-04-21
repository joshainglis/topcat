# M7 — object_dag

> Object-granular dependency graph for pg-dev-buddy. Combines catalog edges
> (from `pg_depend`), inferred edges (from plpgsql body parsing), and
> declared edges (from `-- requires:` headers) under a single edge-authority
> policy. This is the first module whose nodes are Postgres objects, not
> source files — the data structure every downstream milestone (differ,
> migration generator, sync) walks.

## Context

Pg-dev-buddy builds two DAGs. The existing File DAG (`src/file_dag/`)
orders source files for apply. The new **Object DAG** orders Postgres
objects for schema diffing, DDL emission, and migration ordering. They are
not isomorphic: one source file can create several objects (table + its
indexes + its policies), and one object's dependencies can come from
three independent sources that must be reconciled.

M7 lands the Object DAG once its three edge sources have stable contracts:

- **Catalog edges** come from M2 (`catalog_reader` walks `pg_depend`).
- **Inferred edges** come from M4 (`body_parser` parses plpgsql bodies).
- **Declared edges** come from user `-- requires:` headers (already parsed
  by the existing `file_node` module).

This ordering is deliberate — the roadmap pushes `object_dag` behind
`body_parser` so inferred edges are real, not stubbed. M7 is the
earliest milestone where all three edge kinds can be compared on equal
footing.

The output of M7 is consumed by M8 (differ walks it for change ordering),
M9 (migration generator uses it for topological emit order), and M12
(sync orchestrates object-granular apply against it). Get edge authority
wrong here and every downstream artifact inherits the mistake.

## Prerequisites

Before starting M7, confirm these concrete artifacts are in place:

- **From M2** (`schema_model` + `catalog_reader`):
  - `src/schema_model/` with typed `Table`, `Function`, `View`, `Sequence`,
    `Type`, etc. (full list per architecture §Object types covered).
  - `pg_address` triple (`classid`, `objid`, `objsubid`) available on every
    typed object.
  - `pg/catalog_reader` streaming `pg_depend` rows with `deptype`
    filtering (excludes `'e'` extension-owned, `pg_catalog`,
    `information_schema`, `_topcat`).
- **From M4** (`body_parser`):
  - `src/pg/body_parser/` exposing a function that takes `pg_proc.prosrc`
    + resolved search_path and returns `Vec<InferredEdge { target,
    confidence, source_location }>` plus a `parse_status` (`ok`, `failed`,
    `dynamic_sql`, `dynamic_ddl`).
- **From existing `file_node`**:
  - `-- requires:` header parsing already extracts the declared-target
    name; M7 reuses the parsed output, does not re-parse.
  - `file_node` metadata carries `source_location` (file path + line
    number) for each declared dep — verify this is accessible; if not,
    extend `file_node` parsing before M7 starts.

If any of the above are missing or the shapes differ from what this plan
assumes, surface it at M7 kickoff and reconcile before writing code.

## Scope

### In scope

- New `src/object_dag/` module (green-field, mirrors the layout
  conventions of `src/file_dag/`).
- Node type with `node_id`, `pg_address`, owning source `file`,
  `parse_status`.
- Edge type with `kind` (`catalog` | `inferred` | `declared`),
  `confidence`, `source_location`.
- Builder that ingests all three edge sources and applies the
  edge-authority truth table from architecture §Edge authority.
- Stale declared-edge detection: warn in dev (sync) mode, fail-closed in
  migration mode.
- Structured diagnostics emitted as an event stream the caller drains
  (not println) so sync and migration generation can apply their own
  severity policies.
- Validation: cycle detection at the object level (reuses `stable_topo`
  patterns but on object nodes).

### Out of scope

- DDL emission (that's M3, and M3's emitter is called by M9 which
  consumes M7's DAG — not the other way around).
- `ChangeSet` computation (M8).
- Migration ordering/sectioning (M9).
- `-- exists:` handling: per architecture §`-- exists:` in the Object
  DAG, `exists` stays a File DAG concept and produces **no** Object DAG
  edges. M7 explicitly ignores `-- exists:` headers.
- Registry I/O (`_topcat.node_registry`): M7 consumes `node_id` from
  inputs, does not read or write the registry table. M5 owns
  registry population; M11 wires source-file `-- node_id:` parsing.
- Subgraph extraction APIs for sync worker pools (M12/M20 concern).

## Deliverables (files and modules)

New module `src/object_dag/` with these files:

- `mod.rs` — public surface: `ObjectDag`, `ObjectNode`, `ObjectEdge`,
  `EdgeKind`, `ParseStatus`, `BuildDiagnostic`, `BuilderMode`. Re-exports
  from submodules.
- `core.rs` — `ObjectDag` struct (petgraph-backed, matching
  `file_dag::core` conventions), node/edge accessors, topological walk
  helpers.
- `node.rs` — `ObjectNode { node_id: Uuid, pg_address: PgAddress, file:
  Option<PathBuf>, parse_status: ParseStatus }`. `ParseStatus` enum
  mirrors M4's enum (re-export, don't redefine).
- `edge.rs` — `ObjectEdge { kind: EdgeKind, confidence: Confidence,
  source_location: Option<SourceLocation> }`. `EdgeKind` is
  `Catalog | Inferred | Declared`. `Confidence` is `High | Low` for
  inferred edges per architecture §Supported plpgsql patterns; `High`
  for catalog and declared.
- `builder.rs` — `ObjectDagBuilder`:
  - `from_catalog_edges(reader: &CatalogReader) -> Vec<RawEdge>`
  - `from_inferred_edges(parser_out: &BodyParserOutput) -> Vec<RawEdge>`
  - `from_declared_edges(file_nodes: &[FileNode]) -> Vec<RawEdge>`
  - `build(mode: BuilderMode) -> Result<(ObjectDag,
    Vec<BuildDiagnostic>), BuildError>` — runs the authority truth
    table, emits diagnostics, returns the reconciled DAG.
- `authority.rs` — pure function `reconcile(catalog: Option<Edge>,
  inferred: Option<Edge>, declared: Option<Edge>) -> ReconcileOutcome`.
  Encodes the architecture §Edge authority table. Isolated for
  truth-table testing.
- `validation.rs` — cycle detection over the reconciled DAG (object-level,
  distinct from `file_dag::validation`), unresolved-target detection for
  declared edges (stale detection).
- `diagnostics.rs` — `BuildDiagnostic` variants: `CatalogOverridesDeclared`
  (warn), `DeclaredOverridesInferred` (info), `StaleDeclaredEdge`
  (warn in `BuilderMode::Sync`, error in `BuilderMode::Migration`),
  `InferredMiss` (debug, for parser-diagnostics logs),
  `ParseStatusNotOk` (warn, surfaces node-level parse failures).
- `tests.rs` or `tests/` dir — see Tests section.

### Edge authority rules (implementation-oriented)

From architecture §Edge authority, encoded in `authority.rs`:

| Catalog | Inferred | Declared | Result | Diagnostic |
|---|---|---|---|---|
| ✔ | — | — | include (catalog) | none |
| ✔ | ✔ | — | include (catalog) | none |
| ✔ | ✘ | — | include (catalog) | `InferredMiss` |
| ✔ | — | ✘ (header omits it) | include (catalog) | `CatalogOverridesDeclared` |
| ✘ | — | ✔ | include (declared) | none (but validation checks resolvability) |
| ✘ | ✔ | — | include (inferred) | none |
| ✘ | ✔ | ✔ | include (declared) | `DeclaredOverridesInferred` (info) |
| ✘ | ✘ | ✔ (unresolved target) | drop edge | `StaleDeclaredEdge` |
| ✔ | ✔ | ✔ | include (catalog, full confidence) | none |

"Catalog has edge, declared has edge disagreeing" is not a real state —
`declared` asserts an edge to a target; if the catalog also has that
same edge, they agree. Disagreement is only meaningful in presence/
absence, which the table above covers.

### Stale declared-edge detection

A declared edge whose target name does not resolve to any object in the
catalog read:

- `BuilderMode::Sync` → `BuildDiagnostic::StaleDeclaredEdge { severity:
  Warn, target, source_location }`. Sync continues.
- `BuilderMode::Migration` → `BuildDiagnostic::StaleDeclaredEdge {
  severity: Error, … }`. `build()` returns `Err(BuildError::
  StaleDeclaredEdges(Vec<_>))`. Migration generation fails-closed.

The policy lives in `authority.rs` / `validation.rs`; the caller picks
the mode. Both modes use the same data path — no mode-specific code in
the reconciler itself.

## Implementation phases

### Phase 1 — types and module skeleton

- Create `src/object_dag/` mirroring `src/file_dag/` layout.
- Add `pub mod object_dag;` to `src/lib.rs`.
- Define `ObjectNode`, `ObjectEdge`, `EdgeKind`, `ParseStatus`,
  `Confidence`, `SourceLocation`, `BuilderMode` with doc comments
  citing architecture sections.
- Stub `ObjectDag` wrapping a `petgraph::DiGraph<ObjectNode,
  ObjectEdge>` plus a `HashMap<NodeId, NodeIndex>` for lookup (same
  pattern `file_dag` uses).
- Add empty `ObjectDagBuilder` with method stubs returning `todo!()`.

Gate: compiles, no behavior yet.

### Phase 2 — ingest each edge source independently

- `from_catalog_edges`: iterate the catalog reader output, produce
  `RawEdge { source_node_id, target_node_id, kind: Catalog,
  confidence: High, source_location: None }`. Targets identified by
  `pg_address`, mapped to `node_id` via a lookup built during node
  insertion.
- `from_inferred_edges`: consume M4's output. Confidence `Low` for the
  low-confidence patterns (literal `EXECUTE`, `format()` templates),
  `High` for the rest. Targets resolved via search_path as M4
  determines; `object_dag` does **not** redo resolution — it trusts
  M4's output.
- `from_declared_edges`: iterate `file_node` metadata, produce
  `RawEdge { target: TargetRef::Name(String), kind: Declared,
  confidence: High, source_location: Some(...) }`. Target stays
  name-based until validation resolves it.

Gate: each source independently builds a list of raw edges. No
reconciliation yet. Unit-test each path in isolation.

### Phase 3 — reconciler and authority table

- Implement `authority::reconcile`. Pure function, no I/O.
- Implement `ObjectDagBuilder::build(mode)`:
  1. Ingest nodes from catalog reader (every object in the read becomes
     a node).
  2. Group edges by `(source_node_id, target)` where `target` is either
     `node_id` (catalog/inferred after resolution) or `name` (declared,
     pre-resolution).
  3. Resolve declared-edge names against the node set. Unresolved → feed
     to stale-detection.
  4. For each `(source, target)` group, call `reconcile(...)` → insert
     zero or one edge.
  5. Emit diagnostics as a `Vec<BuildDiagnostic>` returned alongside
     the DAG.

Gate: truth-table tests pass (see Tests).

### Phase 4 — validation and diagnostics wiring

- Cycle detection (`validation::detect_cycles`) on the assembled DAG.
- Stale declared-edge detection emits `BuildDiagnostic::StaleDeclaredEdge`
  with the appropriate severity for the mode.
- Node-level `ParseStatus != Ok` surfaces as `ParseStatusNotOk`
  diagnostics.
- Diagnostic formatting lives in `diagnostics.rs` with enough detail
  (file, line, target name, catalog alternatives) that sync and
  migration can surface actionable messages.

Gate: integration tests against fixture schemas pass.

### Phase 5 — CLI observability (optional, keep scope tight)

- No new top-level command. M7 is an internal module.
- Expose a debug-only `topcat diff --show-object-dag` flag (wire in M8,
  not here) — left as a note for the M8 session.

## Risks (copied and amplified)

### Risk: three-way edge reconciliation produces confusing diagnostics

From roadmap §M7: "Three-way edge reconciliation can produce confusing
diagnostics. Log conflicts with enough detail that `diff.edge.conflict`
is actionable."

Day-one mitigations:

- **Structured diagnostics from the start.** `BuildDiagnostic` is an enum
  with per-variant fields, not a `String`. The human-readable render
  lives in `Display` impls; machine consumers get the enum.
- **Include source_location on every diagnostic.** Declared-edge
  diagnostics carry file + line. Inferred-edge diagnostics carry the
  function's `pg_address` plus the AST span from M4. Catalog edges
  carry the `pg_depend` row.
- **One snapshot test per truth-table row.** Fixtures that produce
  each diagnostic kind assert the full rendered message. Diagnostics
  are user-facing output; regressions are UX bugs.
- **`--verbose` shows the full reconciliation trace.** Silent by default
  when everything agrees.

### Risk: declared edges that don't resolve

From roadmap §M7: "Declared edges referencing names that don't resolve:
warn in dev, fail-closed in migration. This policy must be consistent
between sync and migrate."

Day-one mitigations:

- **Single enum controls policy**: `BuilderMode::Sync` vs
  `BuilderMode::Migration`. Both sync (M12) and migration generator
  (M9) call the same `build()` with the appropriate mode. No
  mode-specific resolution code.
- **Target resolution happens exactly once.** Declared targets are
  name-based; resolution compares against the fully-populated node set
  after catalog ingest. A declared edge to `schemaA.foo` resolves to
  whichever node has that `pg_address.name` — not a textual match on
  source files.
- **Ambiguous resolution fails-loud.** Two nodes with the same
  qualified name (only possible if normalization is broken) produce
  `BuildError::AmbiguousDeclaredTarget`, not a silent pick. This is a
  guard against M2 normalization regressions.

### Risk: node_id not yet wired to source

M11 parses `-- node_id:` from source files. M7 ships before M11. The
node_id assignment for Object DAG nodes in M7 comes from the registry
rows (which M5 populates) — not from source headers. If M5 isn't fully
hot when M7 starts, `ObjectNode::node_id` accepts a synthetic `Uuid` for
fixtures, documented as "replaced by registry value at M11 integration".

## Tests

### Unit tests

**Edge-authority truth-table tests** — one test per row of the table in
§Deliverables. Each test constructs `Option<RawEdge>` triples, calls
`authority::reconcile`, and asserts:

- The returned `ReconcileOutcome` (include-with-kind or drop).
- The emitted `BuildDiagnostic` (exact variant + severity).

Tests live in `src/object_dag/authority.rs` under `#[cfg(test)] mod
tests` following `writing-rust-topcat` conventions.

Additional unit tests:

- `ObjectDagBuilder::from_catalog_edges` — single-table fixture, verify
  one `Catalog` edge per `pg_depend` row after filtering.
- `ObjectDagBuilder::from_inferred_edges` — stub `BodyParserOutput`
  with a function calling two other functions, verify two `Inferred`
  edges.
- `ObjectDagBuilder::from_declared_edges` — stub `FileNode` with two
  `-- requires:` headers, verify two `Declared` raw edges with
  source_location populated.
- `validation::detect_cycles` — fixture with a cycle (two tables with
  circular catalog-declared FKs), verify cycle reported.

### Integration tests with fixture schemas

Fixtures under `tests/fixtures/object_dag/`:

- `catalog_only/` — table + PK + FK, no plpgsql, no source headers.
  Expect: only `Catalog` edges. No diagnostics.
- `inferred_only/` — plpgsql function calling another function, no
  `-- requires:` headers, no catalog dep (because plpgsql bodies
  don't show in `pg_depend`). Expect: `Inferred` edges, no
  diagnostics.
- `declared_only/` — two source files with explicit `-- requires:`
  headers, no catalog FK, no plpgsql. Expect: `Declared` edges, no
  diagnostics.
- `mixed/` — every combination: table with FK (catalog), plpgsql
  function calling it (inferred), function's source file with
  `-- requires:` naming a third object (declared). Expect: one of
  each edge kind, no diagnostics.
- `stale_declared/` — source file `-- requires: nonexistent_thing`.
  Expect: `StaleDeclaredEdge` warning in `Sync` mode, error in
  `Migration` mode (build returns Err).
- `catalog_overrides_declared/` — catalog has FK, source file
  `-- requires:` omits it. Expect: `CatalogOverridesDeclared` warning.
- `declared_overrides_inferred/` — inferred edge to A, declared edge
  to B, both from same source. Expect: `Declared` edge included,
  `DeclaredOverridesInferred` diagnostic.
- `cycle/` — two tables with circular FKs. Expect: cycle detected,
  error returned with both nodes named.

Follow `creating-tests-topcat` for helpers and assertion styles.

### Property tests (optional)

- Commutativity: build order of the three edge sources must not affect
  the final DAG. Shuffle ingest order, assert identical output.

## Verification snapshot

Copy-pasteable commands run after M7 lands:

```bash
cd /Users/josha/oss/topcat
nix develop --command cargo build
nix develop --command cargo test --package topcat object_dag
nix develop --command cargo clippy --package topcat -- -D warnings
nix develop --command cargo test object_dag::authority
nix develop --command cargo test object_dag::validation
```

Expected:

- Build green.
- All `object_dag::` tests pass.
- Clippy clean on the new module.
- Fixture-based tests run under `cargo test --package topcat -- --nocapture object_dag::fixtures` produce no unexpected output.

Smoke check via a debug-only CLI (deferred to M8 wiring) — not blocking
for M7 completion.

## Open questions to resolve before starting

1. **`ParseStatus` location.** Does M4 ship `ParseStatus` in a crate
   that `object_dag` can depend on, or is it a type `object_dag`
   re-defines? Proposal: M4 owns it, `object_dag` re-exports.
2. **`node_id` source before M11.** Registry-populated vs. synthetic.
   Proposal: registry-populated (via M5) when available; synthetic
   UUIDv7 generated in-test for fixtures.
3. **Declared-target resolution algorithm.** Unqualified names need a
   search-path equivalent. Proposal: require qualified names
   (`schema.object`) for declared targets in M7; revisit
   shorthand at M11/M12 if users complain.
4. **Diagnostic emission API.** Return `Vec<BuildDiagnostic>` alongside
   the DAG, or call a callback? Proposal: return-vec for testability;
   sync/migration loop over and apply severity policy.
5. **Source-location representation for catalog edges.** `pg_depend`
   rows don't have line numbers. Proposal: `source_location = None`
   for catalog edges; `Some(file + line)` only for declared.
6. **Extension-owned object filtering location.** M2's catalog reader
   already filters `deptype = 'e'`. Confirm M7 gets a pre-filtered
   stream and doesn't need to re-filter.

Resolve each at the start of the M7 session or escalate.

## Session sizing notes

Roadmap estimate: **~1-2 sessions**.

Session 1 covers Phases 1-3 (types, ingest, reconciler with truth-table
tests). Session 2 covers Phases 4-5 (validation, diagnostics, fixture
integration tests, clippy polish).

If Phase 2 reveals the M4 or M2 contracts differ from what this plan
assumes, surface it before writing the reconciler — contract drift
spent on ingest code is wasted.

Single-session completion is viable if:

- `ParseStatus` is a clean re-export from M4.
- `pg_address` → `node_id` lookup lands in one pass.
- Fixture infrastructure from `file_dag` tests is reusable wholesale.

## Referenced architecture sections

- §Two DAGs: File DAG and Object DAG — granularity distinction.
- §Object DAG — three edge kinds and node attributes.
- §Edge authority — truth table encoded in `authority.rs`.
- §Stale declared edges — mode-dependent severity.
- §`-- exists:` in the Object DAG — confirms `exists` produces no
  Object DAG edges.
- §Body-dependency inference and §Supported plpgsql patterns —
  confidence levels for inferred edges.
- §CatalogReader, SchemaModel, and DDL emitter — context for the
  reader M7 consumes.
- §Conflict taxonomy — for the M11 hand-off on node_id identity.

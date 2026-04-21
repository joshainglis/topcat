# M4 — body_parser (libpg_query plpgsql)

> Self-contained execution plan for milestone M4 of the pg-dev-buddy
> roadmap. Covers the `pg/body_parser` module that extracts inferred
> dependency edges from plpgsql function bodies via `libpg_query`.
> Consumes M2's `SchemaModel` types as edge targets; produces
> `inferred_edges` and a `parse_status` that downstream milestones (M5
> event trigger, M9 migration generator) gate behavior on.

## Context

plpgsql function bodies are opaque text to `pg_depend` — PostgreSQL resolves
body references at execution time, so catalog-only introspection sees
zero edges out of a plpgsql function body. Topcat supplements with
`libpg_query`'s `pg_query_parse_plpgsql` entry point, parsing the
`pg_proc.prosrc` text that's actually installed in the shadow db (not
source text). M4 delivers this parser and emits typed inferred edges
against `SchemaModel` object identities.

Inferred edges are authoritative only as "best-effort" — `catalog` wins
disagreements and `declared` wins over `inferred` (architecture §Edge
authority). M4 is therefore allowed to miss edges (warn) but must never
assert an edge it cannot ground to a `SchemaModel` target.

## Prerequisites

- **M1** — `pg/client` lands first (shadow-db connection, needed by
  catalog reader in M2).
- **M2** — `schema_model` crate in tree. M4 consumes these
  `SchemaModel` types as edge targets and reads
  `pg_proc.proconfig` off the `Function` struct for search_path
  resolution:
  - `schema_model::Function` (owns `body: String`, `config:
    Option<Vec<ConfigSetting>>`, `inferred_edges: Vec<InferredEdge>`,
    `parse_status: ParseStatus`, plus `node_id`, `schema`, `name`).
  - `schema_model::Table`, `schema_model::View`,
    `schema_model::MaterializedView`, `schema_model::Sequence`,
    `schema_model::Type` (enum / composite / domain / range /
    multirange) — all valid edge-target variants.
  - `schema_model::NodeId` — the opaque identifier carried on every
    edge.
  - `schema_model::Schema` set — used to bound search-path
    resolution.
- **Not required**: M3 (`ddl_emitter`) is parallelizable; M4 does not
  consume DDL emission. M5 (event trigger) consumes M4 but does not
  block it.

## Scope

### In scope for this milestone

- New `pg/body_parser` module wrapping `pg_query_parse_plpgsql`.
- Extraction of **high-confidence** patterns (architecture §Supported
  plpgsql patterns):
  - Embedded SQL in `PERFORM`, `SELECT INTO`,
    `INSERT`/`UPDATE`/`DELETE`/`MERGE`.
  - Implicit query in `FOR rec IN SELECT … LOOP`.
  - Table references in `RETURN QUERY` / `RETURN NEXT`.
  - Function calls with statically-resolvable identifiers.
  - Type references in `DECLARE` blocks.
  - Static cursor declarations.
- Extraction of **low-confidence** patterns (warn):
  - Literal-string `EXECUTE 'SELECT …'`.
  - `format()` with literal-only templates.
- `parse_status` enum with variants: `ok`, `failed`, `dynamic_sql`,
  `dynamic_ddl`.
- Identifier resolution against M2's SchemaModel via
  `pg_proc.proconfig` `search_path` (if set) or cluster default,
  matching pg's runtime resolution.
- `libpg_query` (aka `pg_query` crate) dependency added to
  `Cargo.toml`; pinned to a specific version with a comment noting
  **plpgsql AST stability is weaker than SQL AST** — bump requires
  re-running golden tests.
- Golden-file tests on a fixture matrix covering every
  supported/unsupported pattern.

### Explicitly NOT in scope (deferred to M5, M9, M11)

- **Event-trigger integration** (M5): registering `body_parser`
  output against the live `_topcat.node_registry`.
- **`dynamic_ddl` capture at runtime** (M5): M4 only surfaces
  `parse_status = dynamic_ddl` when an `EXECUTE` in a body contains a
  parseable DDL statement; runtime DDL-in-body capture is M5's
  event-trigger job.
- **Migration generation gating** (M9): `--allow-unparseable` /
  `--allow-dynamic-ddl` escape hatches live in `migration_generator`.
  M4 only surfaces the status.
- **Non-plpgsql PL languages** (M11 or later): `plpython`, `plv8`,
  `plperl`, `plrust`, `c` — declared-only, no body parsing.
- **Header materialization** (M10): `topcat write-deps` turning
  inferred edges into `-- requires:` headers.

## Deliverables (files and modules)

| Path | Purpose |
|---|---|
| `Cargo.toml` | Add `pg_query = "x.y"` (pin version; comment rationale). Also add `thiserror` if not already present for parse errors. |
| `src/pg/mod.rs` | Create (or extend) the `pg` module tree; declare `pub mod body_parser;`. |
| `src/pg/body_parser/mod.rs` | Public API: `parse_function(fn_row: &schema_model::Function, schema: &SchemaModel) -> ParseOutcome`. |
| `src/pg/body_parser/parser.rs` | `pg_query_parse_plpgsql` invocation; AST-to-intermediate lowering; error mapping. |
| `src/pg/body_parser/patterns.rs` | Pattern matchers: `embedded_sql`, `return_query`, `for_loop`, `function_call`, `declare_type`, `static_cursor`, `literal_execute`, `format_literal`. One function per pattern; each yields a `Vec<InferredEdge>` or a non-edge diagnostic. |
| `src/pg/body_parser/resolve.rs` | Identifier resolution: `resolve_ref(unqualified_name, search_path: &[Schema], default_search_path: &[Schema], model: &SchemaModel) -> Option<NodeId>`. |
| `src/pg/body_parser/search_path.rs` | Parse `pg_proc.proconfig` for `search_path=…`; fall back to the cluster default. |
| `src/pg/body_parser/status.rs` | `enum ParseStatus { Ok, Failed { error: String }, DynamicSql { at: Span }, DynamicDdl { at: Span, stmt_tag: String } }`. Serde-serializable (matches architecture's `parse_status` field on `Function`). |
| `src/pg/body_parser/edge.rs` | `struct InferredEdge { target: NodeId, confidence: Confidence, source_location: Span }`. `enum Confidence { High, Low }`. |
| `src/pg/body_parser/errors.rs` | `enum BodyParseError` with `thiserror`; mapped to `ParseStatus::Failed` at the boundary. |
| `src/pg/body_parser/tests.rs` | Golden-file driver (reads fixtures/*, asserts snapshot). |
| `tests/fixtures/body_parser/` | Input plpgsql bodies, one file per pattern, paired with `.expected.json` snapshots (edges + parse_status). |
| `tests/body_parser_golden.rs` | Integration test wiring the fixture dir to the driver. |

`ParseStatus` variants (exact):

- `ok` — parse succeeded, all statements handled.
- `failed` — libpg_query returned an error; no edges emitted.
- `dynamic_sql` — a non-literal `EXECUTE` / `OPEN … FOR EXECUTE` /
  constructed `format()` encountered; partial edges may still be
  emitted for the statically-resolved portion of the body.
- `dynamic_ddl` — a parseable DDL statement (`CREATE TABLE …`,
  `ALTER …`, `DROP …`, etc.) was seen inside a body; emitted so M5 and
  M9 can fail-closed in migration generation.

## Implementation phases

1. **Dependency and skeleton**
   - Add `pg_query` crate to `Cargo.toml` with a pinned version and a
     `# M4: plpgsql AST stability` comment.
   - Create `src/pg/` tree and `src/pg/body_parser/` skeleton with
     public API stubs (`ParseOutcome`, `parse_function`, `ParseStatus`,
     `InferredEdge`, `Confidence`).
   - Wire `pub mod pg;` into `src/lib.rs`.
   - `cargo build` passes.

2. **Parser + status lifecycle**
   - Implement `parser.rs`: call `pg_query::parse_plpgsql(src)`, map
     panics / `Err` to `ParseStatus::Failed`.
   - Walk the returned AST once, classifying each statement into one
     of the 8 pattern buckets (6 high-confidence, 2 low-confidence) or
     the dynamic / ddl buckets.
   - At this phase, emit **no edges** — only produce a correct
     `ParseStatus`.
   - Golden tests for `parse_status` assertions only.

3. **Identifier resolution**
   - Implement `search_path.rs`: `pg_proc.proconfig` is a
     `text[]` of `key=value` strings; pluck `search_path=…`,
     comma-split, trim.
   - Implement `resolve.rs`: for each unqualified name, walk
     `search_path ++ default_search_path` and look up in
     `SchemaModel`'s (schema, name) indexes. First hit wins.
     Qualified names bypass search_path.
   - Unit-test resolution semantics against a small synthetic
     `SchemaModel`.

4. **Pattern extraction**
   - Implement `patterns.rs` modules one at a time, each with golden
     fixtures added as the matcher lands:
     1. `embedded_sql` (highest payoff — covers SELECT/INSERT/UPDATE/
        DELETE/MERGE/PERFORM).
     2. `for_loop`, `return_query`, `return_next`.
     3. `function_call` (needs resolve).
     4. `declare_type`, `static_cursor`.
     5. `literal_execute`, `format_literal` (low-confidence).
   - Each matcher produces `InferredEdge { target, confidence,
     source_location }`.
   - Dedupe edges at the outcome boundary (same `(node_id, kind)`
     collapses; retain lowest `source_location` for diagnostics).

5. **Golden-test harness and fixture matrix**
   - `tests/body_parser_golden.rs`: for each `*.sql` under
     `tests/fixtures/body_parser/`, parse the paired synthetic
     `SchemaModel` (`*.model.json`), run `parse_function`, snapshot
     `ParseOutcome` as JSON, diff against `*.expected.json`.
   - Covered-pattern matrix (≥1 fixture per row):
     - embedded `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `MERGE`,
       `PERFORM`.
     - `FOR rec IN SELECT … LOOP`.
     - `RETURN QUERY`, `RETURN NEXT`.
     - function call (qualified + unqualified + default search_path).
     - `DECLARE x some_schema.some_type`.
     - static cursor `DECLARE c CURSOR FOR SELECT …`.
     - literal `EXECUTE 'SELECT …'` (low-confidence).
     - `format('SELECT FROM %s', literal)` (low-confidence).
     - **Unsupported** (assert `dynamic_sql`): constructed dynamic SQL,
       non-literal `EXECUTE`, parameter-substituted cursor,
       `OPEN cursor_var FOR EXECUTE …`.
     - **DDL-in-body** (assert `dynamic_ddl`): `EXECUTE 'CREATE TABLE
       …'`, inline `CREATE INDEX`, `ALTER TABLE`, `DROP …`.
     - **Parse failure**: syntactically invalid body → `failed`.
     - **Empty body** (`BEGIN END`) → `ok` with zero edges.

## Risks (copied and amplified from roadmap)

| Risk | Amplification | Day-one mitigation |
|---|---|---|
| **libpg_query plpgsql AST is less stable than its SQL AST.** | Minor upstream versions have changed plpgsql node shapes historically. A silent AST shape change becomes a "body_parser emits no edges, differ silently reorders reapply" class of bug. | Pin `pg_query` crate to an exact `=x.y.z` version in `Cargo.toml` with a comment pointing at this plan. CI runs the full golden-fixture set; any drift in a `.expected.json` blocks the bump. Add a `tests/body_parser_version_probe.rs` that asserts a dozen known statement node types still deserialize — fails loud on shape change even if fixtures happen to miss it. |
| **Identifier resolution via `pg_proc.proconfig` search_path may diverge from pg's runtime.** | Unqualified references that resolve to the wrong target silently corrupt the inferred edge set. Catalog wins disagreements, so a wrong inferred edge is only dangerous when catalog has no edge on that statement (common for function-call patterns). | On day one, assert `resolve.rs` matches pg's runtime by writing a parallel test that (a) installs a fixture db in the shadow, (b) calls `SET search_path TO …; EXPLAIN (VERBOSE) SELECT …`, (c) parses the plan's resolved names, (d) compares to `resolve_ref`. Keep the test in the integration suite and gate merges on it. |
| **Escape from shadow truth.** | `body_parser` parses `pg_proc.prosrc` (what's installed), not source files. A caller that feeds raw file text will silently see different identifier bindings than pg will. | The public API takes `&schema_model::Function`, not `&str`. There is no `parse_raw(text)` escape hatch. Document the invariant in `body_parser/mod.rs`. |
| **Dedupe across catalog and inferred.** | M4 emits every inferred reference; the Object DAG layer dedupes against catalog. If M4 short-circuits by checking catalog, it hides parser miscoverage from diagnostics. | M4 does NOT consult the catalog edge set. It emits every inferred edge; the dedupe/authority decision is the Object DAG's job. |

## Tests

- **Golden-file tests** (primary): `tests/body_parser_golden.rs` drives
  `tests/fixtures/body_parser/*` through `parse_function` and asserts
  JSON-snapshot equality against `*.expected.json`. Fixture matrix
  covers every row in §Supported plpgsql patterns plus all
  `Unsupported` and edge cases. `cargo test body_parser_golden`
  refreshes with `TOPCAT_GOLDEN_REWRITE=1`.
- **Version probe** (shape guard): `tests/body_parser_version_probe.rs`
  asserts a set of AST node types round-trip through `pg_query`'s
  deserializer. Independent of fixture churn.
- **Resolve unit tests**: `src/pg/body_parser/resolve.rs#[cfg(test)]`
  — synthetic `SchemaModel` with a conflict pair (`public.foo` and
  `app.foo`), assert search_path ordering, qualified bypass, missing
  name → `None`.
- **Search-path parser**: `src/pg/body_parser/search_path.rs#[cfg(test)]`
  — `proconfig = None`, `proconfig = ["search_path=a,b"]`,
  `proconfig = ["other=x","search_path=a"]`, malformed entries.
- **Resolve-parity integration test** (deferred to M5 if shadow db
  isn't ready) — see Risks table, mitigation for identifier
  resolution.

## Verification snapshot (definition of done)

```bash
# Dependency is pinned (exact version, not caret):
grep -E '^pg_query\s*=\s*"=[0-9]' Cargo.toml

# Clean build + lints:
cargo build --release
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# All tests green:
cargo test

# Golden tests run and leave no modified fixtures:
cargo test body_parser_golden
git diff --exit-code tests/fixtures/body_parser/

# Version probe passes (AST shape guard):
cargo test body_parser_version_probe

# Public API imports cleanly from downstream (smoke):
cargo check -p topcat --tests

# Module tree sanity:
test -f src/pg/body_parser/mod.rs
test -d tests/fixtures/body_parser
```

Acceptance:

- Every row of architecture §Supported plpgsql patterns has ≥1
  passing fixture.
- Every `ParseStatus` variant (`ok`, `failed`, `dynamic_sql`,
  `dynamic_ddl`) has ≥1 passing fixture.
- `parse_function` never panics on any fixture; failures surface as
  `ParseStatus::Failed`.
- `InferredEdge.target` values are all `NodeId`s present in the
  provided `SchemaModel` (no dangling targets).

## Open questions to resolve before starting

1. **Crate choice**: `pg_query` (upstream, native libpg_query FFI) vs
   `pg_query_wrapper` (pure-Rust re-binding, if maintained). Roadmap
   notes pin the version either way. Recommend `pg_query` unless it's
   unmaintained; confirm latest release and license compatibility
   (topcat is MIT OR Apache-2.0).
2. **Cluster-default search_path**: hardcode `["$user", "public"]` or
   read from `shadow_db` at parse time? M4 can hardcode and leave a
   TODO; M5 can wire the runtime value through.
3. **Span representation**: `Span { byte_offset, len }` vs
   `{ line, col }`. libpg_query returns byte offsets; lean on that and
   compute line/col lazily at diagnostic render time.
4. **Edge dedupe key**: `(target_node_id,)` only, or
   `(target_node_id, statement_kind)`? The Object DAG ultimately only
   cares about target identity; recommend dedupe on `target_node_id`
   alone and keep the first-seen `source_location` for diagnostics.
5. **Fixture snapshot format**: JSON (easy diff, matches JCS elsewhere)
   vs `insta`-style `.snap` files. Recommend JSON + `git
   diff --no-index` to avoid pulling in `insta` just for this module.
6. **Is `MERGE` supported across every pg version we target?** `MERGE`
   landed in pg15; fixture matrix should tolerate absence or gate on
   the shadow version.

## Session sizing notes

Roadmap sizing: **~1–2 sessions**. Phases 1–3 (dep + skeleton + parser +
resolver) fit in one session; phases 4–5 (pattern extraction + golden
matrix) fit in a second. Each pattern matcher is ~50 lines plus 2–3
fixtures, so the second session is linear work.

**Parallelizable with M3** (ddl_emitter) and M19 — M4 touches none of
the emitter surface and is read-only against M2's SchemaModel. If two
agents are available, M3 and M4 can run concurrently after M2 lands.

## Referenced architecture sections

- `## Body-dependency inference` (docs/pg-dev-buddy-architecture.md
  L338) — language matrix, shadow-parse-not-source invariant.
- `### Supported plpgsql patterns` (L357) — canonical scope list for
  high-confidence, low-confidence, unsupported, and `dynamic_ddl`
  cases.
- `### Policy` (L386) — inferred-edges-are-registry-only-by-default,
  parse-failure policy, dynamic-SQL warn-and-skip.
- `## Object DAG` (L185) / `## Edge authority` (L200) — how
  `inferred` composes with `catalog` and `declared`.
- `## CatalogReader, SchemaModel, and DDL emitter` (L398) —
  `Function { …, inferred_edges, parse_status }` shape.
- `## Component map` (L1134) — `pg/body_parser` row.

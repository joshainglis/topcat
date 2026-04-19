# Postgres dev buddy — architecture

Design doc for extending topcat into a postgres schema-as-code workflow: dev-db
sync, catalog-based diffing, migration generation, and codegen.

Status: design, pre-implementation. Every design decision in this doc is
concrete; there are no deferrals. Non-goals are listed explicitly as
architectural choices, not as "later" work.

## Vision

Treat a postgres schema as source code. Files in a repo define postgres objects
with topcat headers for dependency metadata. As files change, a local dev db
stays in sync via drop-cascade-recreate through topcat's DAG. When ready to
ship, topcat generates a content-addressed migration by diffing two shadow
databases. Every forward migration produces a companion reverse migration.
Language bindings are generated from the same canonical schema model that
powers diffing.

## Scope

**In scope:**

- Local dev db kept in sync from files.
- Shadow-db-based diffing for migration generation.
- Migration verification: round-trip equivalence between two local shadow
  states.
- Codegen from the canonical schema model via plugins.
- Meta-schema self-migration (`_topcat` schema versioning).
- Reversible adoption: `topcat uninstall` restores pre-adoption state of both
  files and dev db.
- Automatic reverse migration generation for every forward migration.
- Automatic three-way resolution of node_id conflicts across merged branches.
- Split/merge table refactoring via source-file annotations.

**Non-goals** (architectural choices, not "later" work):

- Connection to staging/production databases. Topcat is a dev-time tool;
  migration application to remote environments is a migration runner's job.
- Shared dev db across multiple developers. Drop-cascade-recreate is
  inherently disruptive; teams provision per-developer dbs from a common
  template.
- Non-postgres databases.

### Cross-environment risks

Topcat's correctness claims are bounded to the developer's local postgres
cluster. Real deployments see drift that topcat cannot detect from a local
shadow:

- **pg version drift** between dev and target.
- **Collation version drift** (pg15+). Catalog rows carry collation version
  hashes; these can diverge without schema-intent changes.
- **Extension version pinning.** Topcat records observed extension versions in
  the migration header; enforcement on the target is the runner's job.
- **Manual ALTERs on staging/prod.** `shadow_main` reflects committed files
  applied to a fresh db, not the target's real state. Verification is
  necessary but not sufficient.

Migration headers record `pg_version` and `extension_versions` so a runner can
refuse on mismatch.

## Core concepts

**node_id** — stable identifier for a postgres object, UUIDv7, assigned by
topcat on first observation. Persisted both in source files (`-- node_id:`
header) and in `_topcat.node_registry`. Survives renames and ALTER statements.

**migration_id** — content hash (SHA-256 over canonical JSON per RFC 8785 JCS)
that identifies a migration artifact. Immutable. Different from node_id
because migrations are snapshots, not living entities.

**File DAG** — DAG whose nodes are source files and whose edges come from
`-- requires:` / `-- exists:` headers. Unit of *application*: topcat can only
apply whole files to a database. The canonical ordering model for
current-topcat commands (`concat`, `analyze`, `clean`, `update`, etc.).

**Object DAG** — DAG whose nodes are postgres objects (addressed via
`pg_identify_object_as_address`) and whose edges are dependencies from three
sources (catalog, inferred, declared). Unit of *analysis and migration
ordering*. Reconstructed from catalog after every apply.

**SchemaModel** — canonical, structured, typed representation of a database's
schema. Produced from any managed database by the CatalogReader. Comparable
pairwise. Consumed by diff, migration generation, codegen, and the DDL
emitter.

**Shadow database** — throwaway local postgres database created from a
template. `shadow_main` holds committed-file state; `shadow_head` holds
working-tree state; `shadow_verify` is a per-migration clone.

**Managed database** — any db topcat touches. Has the `_topcat` schema and the
event trigger installed. Topcat owns managed dbs unconditionally: drops
unmanaged objects on reconcile.

**ChangeSet** — set of per-object changes produced by comparing two
SchemaModels. Content hash of the ChangeSet is the migration_id.

## Two DAGs: File DAG and Object DAG

Topcat maintains two DAGs with distinct roles.

| | File DAG | Object DAG |
|---|---|---|
| Node | Source file | Postgres object |
| Edges | `-- requires:`, `-- exists:` headers | Catalog + inferred + declared |
| Built from | Parsed headers | Catalog read of an applied database |
| Persistence | Reconstructed each sync | Reconstructed each command |
| Granularity | File (unit of apply from source) | Object (unit of schema diff, codegen, DDL emission) |
| Consumers | `concat`, `analyze *`, `clean`, `update`, apply scheduler | `diff`, `migrate generate`, `codegen`, `sync` re-apply |

Both DAGs are first-class. Current-topcat users never touch the Object DAG;
pg-dev-buddy users touch both.

## Pipeline

```
  ┌─ source files ──────────────────────────────────┐
  │  *.sql with topcat headers                      │
  └──────────────────────┬──────────────────────────┘
                         │
                         ▼
           ┌──────────────────────────┐
           │  File DAG                │  initial apply order
           └─────────────┬────────────┘
                         │
                         ▼
  ┌──────────────────────────────────────────────────┐
  │   Shadow / dev db apply                           │
  │   (event trigger captures implicit objects)       │
  └──────────────────────┬───────────────────────────┘
                         │
                         ▼
  ┌──────────────────────────────────────────────────┐
  │   CatalogReader                                   │
  │     pg_depend          → catalog edges            │
  │     libpg_query parse  → inferred edges           │
  │     topcat headers     → declared edges           │
  │     per-object readers → structured fields        │
  └──────────────────────┬───────────────────────────┘
                         │
                         ▼
                    Object DAG
                         │
                         ▼
                   SchemaModel
                         │
     ┌───────────────────┼─────────────────┬────────────────┐
     ▼                   ▼                 ▼                ▼
   Differ           Codegen plugins   DDL emitter      Drift detector
     │                                     │
     ▼                                     ▼
  ChangeSet                          per-object DDL
     │                           (used by sync and
     ▼                            migration generator)
  Migration generator (forward + reverse, ordered via Object DAG)
     │
     ▼
  Migration files (topcat-headed, .sql) + registry entries
     │
     ▼
  Verification: apply forward to shadow_main clone, assert equals shadow_head
```

## Evolution from current topcat

Adding pg-dev-buddy is additive:

- **Current commands** (`concat`, `update`, `analyze`, `clean`, `schema`,
  `export`, `import`, `config`) operate on the File DAG at file granularity.
  No postgres connection needed. Behavior unchanged.
- **New commands** (`sync`, `shadow`, `diff`, `migrate`, `reconcile`,
  `write-deps`, `pause`, `resume`, `uninstall`, `meta-schema`) operate on the
  Object DAG and require a managed dev db. Opt-in via a `[postgres]` block in
  `topcat.toml`.

Adoption path:

1. Existing File DAG workflows work with no config change.
2. Add `[postgres] dev_db_url = "..."` to `topcat.toml`.
3. Run `topcat sync --adopt` to bootstrap node_ids against an existing dev db.
4. Subsequent commits produce migrations.
5. To leave: `topcat uninstall` strips node_id headers and drops `_topcat`.

## Object DAG

Three edge kinds:

| Edge kind | Source | Authority | Notes |
|---|---|---|---|
| `catalog` | `pg_depend` | Ground truth for DDL ordering | pg enforces at DDL time. |
| `inferred` | libpg_query parse of plpgsql bodies | Best-effort | Drives reapply ordering when function bodies change. |
| `declared` | User `-- requires:` header | User intent | Supplements catalog; wins over inferred. |

Node attributes: `node_id`, `pg_address` (classid/objid/objsubid), `file`,
`parse_status` (`ok` / `failed` / `dynamic_sql` / `dynamic_ddl`).

Edges carry `kind`, `confidence`, `source_location`.

## Edge authority

| Disagreement | Policy | Rationale |
|---|---|---|
| `catalog` has edge, `declared` does not | Use catalog. Warn if source has `-- requires:` but omits this target. | User may be unaware of an implicit dependency. |
| `catalog` lacks edge, `declared` asserts one | Use declared. Warn if the named target does not resolve. | User wants ordering for runtime semantics. |
| `catalog` has edge, `inferred` disagrees | Catalog wins. Log inferred-miss for parser diagnostics. | Catalog is ground truth. |
| `declared` and `inferred` disagree | Declared wins, no warning. | Declared is explicit user intent. |
| All three agree | Edge included, full confidence. | — |

### Stale declared edges

A declared edge whose target doesn't resolve to any known object: warn in
dev-mode sync, fail-closed in migration generation.

### `-- exists:` in the Object DAG

`-- exists:` remains a File DAG concept (ensure file inclusion, no ordering).
Produces no Object DAG edges — objects either exist in the catalog or they
don't.

## Identity and node_id

UUIDv7 per object. Assigned on first observation.

- **Source file header**: `-- node_id: 01924a...`. Survives in git, travels
  with the code.
- **Registry row**: `_topcat.node_registry`.

### Conflict taxonomy

Every sync runs a conflict scan before any DDL is applied.

| Case | Signal | Policy |
|---|---|---|
| **Clean rename** | node_id N in one source file with pg-name X; registry maps N to Y; no other file claims N | Emit `ALTER … RENAME` in the next migration. |
| **Copy-paste** | node_id N appears in two or more working-tree source files | Fail-loud. Error lists the files. |
| **Unknown node_id** | node_id N in source, no row in registry | Fail-loud. Error prompts: remove the header for reassignment, or run `topcat sync --adopt`. |
| **Deleted header** | Source file present with same CREATE body as registry row N; no `-- node_id:` header | Warn; registry row orphaned; sync drops the object and re-creates with a new node_id. |
| **Branch-merge collision** | Two source files with different node_ids creating the same pg-object | Auto-resolve per Three-way node_id resolution. |
| **Orphan registry row** | Registry row N with no source file claiming N | Sync drops the object. Protected via `--root-pattern` → reconcile flags for operator review. |

### Three-way node_id resolution

When a merge produces two source files with different node_ids for the same
schema-qualified object:

1. **Detection.** During post-merge `shadow_main` rebuild, the File DAG apply
   encounters two files creating the same pg-object. Topcat stops the apply
   and enters resolution.
2. **Automatic winner.** Chronological precedence. UUIDv7 encodes creation
   time in its high bits; the older node_id wins ("first observed").
3. **Source rewrite.** Topcat updates the losing file's `-- node_id:` header
   to the winner's value. Git shows a one-line diff on the next commit.
4. **Alias recording.** `_topcat.node_aliases (loser_node_id, winner_node_id,
   merged_at)` records the mapping. Historical migrations referencing the
   loser resolve through the alias table at read time; they remain immutable.
5. **Apply continues.** Any ChangeSet conflict on the object's shape
   (columns, constraints, body) is resolved at the normal diff step — the
   merged source's choice wins.

No user prompt. The conflict is metadata-only; real schema disagreements
resolve through the normal diff pipeline on the merged source.

## Shadow database lifecycle

- Template-based. `shadow_main` is built by applying all committed files to a
  fresh db. `shadow_head` is built by applying the working tree to a fresh
  db. `CREATE DATABASE ... TEMPLATE` makes reuse cheap.
- Cached per-input. Topcat hashes the sorted file-set (path + content hash)
  that produced a shadow. Hit → reuse the template. Miss → rebuild.
- Event trigger installed on both. Trigger role on shadows is observational.

## Event trigger

Installed on every managed db. Purpose:

1. Capture implicit objects (serial → sequence + index + constraint).
2. Atomic registry updates (same transaction as the DDL).
3. Drift detection on the dev db (out-of-band DDL → unmanaged flag).
4. Drop tracking via `pg_event_trigger_dropped_objects()`.

### Tag inventory

**Captured** (topcat handles):

- `CREATE/ALTER/DROP` of TABLE (incl. partitioned), VIEW, MATERIALIZED VIEW,
  SEQUENCE, INDEX, TYPE, DOMAIN, FUNCTION, PROCEDURE, AGGREGATE, OPERATOR,
  OPERATOR CLASS, OPERATOR FAMILY, CAST, COLLATION, TRIGGER, POLICY, RULE,
  EXTENSION, SCHEMA, TEXT SEARCH CONFIGURATION/DICTIONARY/PARSER/TEMPLATE,
  FOREIGN DATA WRAPPER, SERVER, USER MAPPING, STATISTICS, CONVERSION,
  PUBLICATION, ACCESS METHOD, EVENT TRIGGER.
- `REFRESH MATERIALIZED VIEW` (registry freshness).
- `SECURITY LABEL` (attribute).
- `COMMENT` (attribute).
- `ALTER FUNCTION … DEPENDS ON EXTENSION`.
- `IMPORT FOREIGN SCHEMA`.

**Out-of-list** (non-goals; cluster-global or environment-specific):

- `CREATE/DROP ROLE`, cluster-level `GRANT/REVOKE`.
- `CREATE/ALTER/DROP DATABASE`.
- `CREATE/DROP TABLESPACE`.
- `CREATE/ALTER/DROP SUBSCRIPTION` (environment-specific connection strings).
- `ALTER SYSTEM`.
- `VACUUM`, `ANALYZE`, `CLUSTER`, `REINDEX` (data-plane).

**Gap cases** (tag fires but response is special):

- **DDL inside function bodies.** A plpgsql function running `EXECUTE 'CREATE
  TABLE …'` creates catalog rows whose `pg_depend` doesn't link back to the
  originator. Parser flags any DDL AST node inside a plpgsql body; the
  function is marked `parse_status = dynamic_ddl`. Dev sync: warn. Migration
  generation: fail-closed unless `--allow-dynamic-ddl` is set. Generated
  catalog rows are captured as orphan-managed and flagged for reconcile.
- **Extension-owned objects** (`pg_depend.deptype = 'e'`). Not managed. Listed
  in the SchemaModel for dependency resolution but never diffed or migrated.
- **`pg_catalog` and `information_schema`.** Filtered from every SchemaModel.
- **Topcat's own `_topcat` schema.** Filtered. The `_topcat` schema
  self-manages via meta-migrations; it never appears in a user-facing
  ChangeSet.
- **Topcat's own event triggers** (`_topcat_ddl_end`, `_topcat_sql_drop`).
  Recognized by name and excluded from the managed set.
- **User-created event triggers** (not topcat's). Managed like any other
  schema object. The trigger fires on its own installation (topcat's trigger
  sees the user's DDL); the user's trigger is tracked in the registry.

### Bootstrap and pg_restore path

- `_topcat` schema must exist before the trigger is installed. The install
  statement sets `topcat.bootstrapping = true` for the transaction so the
  trigger ignores itself.
- Bulk restore: `topcat pause` runs `ALTER EVENT TRIGGER … DISABLE`, the
  restore proceeds, then `topcat resume` re-enables and runs a full-catalog
  scan to repopulate the registry.
- Event triggers require superuser. Topcat fails at install with a clear
  error if the connection lacks the privilege.

## Body-dependency inference

`pg_depend` is systematically incomplete for plpgsql — bodies are opaque text,
resolved at execution. Topcat supplements via `libpg_query`'s
`pg_query_parse_plpgsql`.

Language matrix:

| Language | Dependency source |
|---|---|
| `sql` | `pg_depend` only. |
| `plpgsql` | `pg_depend` + libpg_query parse of the body. |
| Views, matviews | `pg_depend` only. |
| `plpython`, `plv8`, `plperl`, `plrust`, `c` | `pg_depend` + declared-only. |

The parser runs on `pg_proc.prosrc` in the shadow db — parsing what's
installed, not source text. Unqualified references resolve via
`pg_proc.proconfig` `search_path` (if set) or cluster default.

### Supported plpgsql patterns

**High-confidence inferred edges:**

- Embedded SQL in `PERFORM`, `SELECT INTO`, `INSERT/UPDATE/DELETE/MERGE`.
- Implicit query in `FOR rec IN SELECT … LOOP`.
- Table references in `RETURN QUERY`, `RETURN NEXT`.
- Function calls with statically-resolvable identifiers.
- Type references in `DECLARE` blocks.
- Static cursor declarations.

**Low-confidence inferred edges** (warn):

- Literal-string `EXECUTE 'SELECT * FROM users'`.
- `format()` with literal templates only.

**Unsupported** (`parse_status = dynamic_sql`, no edge):

- Constructed dynamic SQL.
- Non-literal `EXECUTE` targets.
- Parameter-substituted cursor bodies.
- `OPEN cursor_var FOR EXECUTE …`.

Separately `parse_status = dynamic_ddl` for DDL-in-bodies (see Event trigger
Gap cases).

Record-field assignments, exception handlers, `RAISE … USING` — parsed for
structure but produce no edges.

### Policy

- **Inferred edges: registry-only** by default. `topcat write-deps`
  materializes them to `-- requires:` headers.
- **Parse failures:** warn in dev sync; fail-closed in migration generation.
  `--allow-unparseable` escape hatch.
- **Dynamic SQL:** warn and skip. User can declare with `-- requires:`.
- **Dynamic DDL:** warn in dev; fail-closed in migration generation.
  `--allow-dynamic-ddl` escape hatch.
- **Zero declared deps on non-parseable PL functions:** refuse in
  strict-mode, warn in default mode.

## CatalogReader, SchemaModel, and DDL emitter

Per-object-type structured readers, each emitting typed rows. The same typed
rows feed the DDL emitter, which generates DDL for any object — the single
authoritative path from SchemaModel to SQL, shared by both sync re-apply and
migration generation.

Examples:

```
Table {
  node_id, schema, name,
  columns: [{ name, type, nullable, default, identity, generated,
              collation, storage, comment }],
  constraints: [...], indexes: [...], rls_policies: [...],
  storage_params, inherits, partition_of, partition_strategy,
  partition_bounds, owner, acl, comment,
  rls_enabled, rls_forced
}

Function {
  node_id, schema, name,
  args: [{ name, type, mode, default }],
  returns, language, volatility, strict, security, parallel,
  cost, rows, body, owner, acl, comment, config,
  inferred_edges: [...], parse_status
}
```

### Normalization rules

Semantic equivalence, not textual. Applied by the CatalogReader before the
SchemaModel leaves the reader module.

| Hot spot | Canonical form |
|---|---|
| `varchar(n)` / `character varying(n)` / `varchar` (no length) | `character varying(n?)`; `varchar` without length → `text`. |
| serial / `GENERATED … AS IDENTITY` / `DEFAULT nextval(seq)` | Typed union: `legacy_serial`, `identity { mode, seq_opts }`, `default { expr }`. |
| View / matview definition | `pg_get_viewdef(…, true)` with explicit column qualification. |
| Generated-column expression | `pg_get_expr(attrdef, attrelid)`. |
| Auto-generated constraint names | Synthesized `<table>_<cols>_<kind>`. Excluded from content hash. |
| `reloptions` | Sorted key=value list; booleans as `true`/`false`; integers as decimal. |
| RLS | `{ enabled: bool, forced: bool, policies: [...] }`. Both flags load-bearing. |
| Comments | `COMMENT ON …` values captured per-object and per-column. Inline source `--` comments not captured. |
| Function body | Exact `pg_proc.prosrc` text. No whitespace normalization. |
| Sequence references in defaults | Schema-qualified. |
| Collation references | Schema-qualified; version hash stripped. |

### Partitioned tables and inheritance

Partition trees:

- `partition_strategy`: `range` | `list` | `hash` | null.
- `partition_of`: parent node_id, null for roots.
- `partition_bounds`: canonical `FOR VALUES …` text (or `DEFAULT`).

Inheritance: `inherits` array of parent node_ids.

ChangeSet kinds:

- `PartitionAttach { parent_node_id, child_node_id, bounds }`
- `PartitionDetach { parent_node_id, child_node_id }`
- Partition bound changes: automatic detach → data-move (if safe) → re-attach.
  Complex moves (bound overlaps requiring row redistribution) bounce with
  `kind=partition_bounds_complex`.

Sub-partitioning recurses.

### Object types covered

All of: tables (incl. partitioned), columns, constraints (PK, FK, UNIQUE,
CHECK, EXCLUDE), indexes, functions, procedures, views, materialized views,
sequences, types (enum, composite, domain, range, multirange), triggers,
policies, rules, schemas, extensions, text search
configs/dictionaries/parsers/templates, foreign data wrappers/servers/user
mappings, operators, operator classes/families, aggregates, casts,
conversions, statistics objects, publications, access methods, user-managed
event triggers.

Topcat's own event triggers (`_topcat_ddl_end`, `_topcat_sql_drop`) are
recognized by name and excluded — they belong to the `_topcat` schema, which
self-manages via meta-migrations.

## Diff and migration generation

Diff: catalog-row structural comparison between two SchemaModels.
Per-object-type equality via normalized structural comparison, not DDL text.

Output: `ChangeSet` = ordered set of `(node_id, change_kind, before_model?,
after_model?)`. `change_kind ∈ {Add, Drop, Alter, Rename, PartitionAttach,
PartitionDetach, SplitExtract, MergeAbsorb}`.

DDL statement order: topological walk of the Object DAG.
- Drops: leaves first.
- Creates: roots first.
- Alters: between dependency creates and dependents' creates.

## Migration file format

Multi-section SQL. Sections execute in order; the runner honors transaction
boundaries per-section.

```sql
-- topcat_migration
-- migration_id: <content_hash>
-- parents: <parent_migration_id>[, <other_parent_id>]
-- is_reverse: false
-- forward_id: null
-- generated_at: 2026-04-19T12:34:56Z
-- pg_version: 16.2
-- extension_versions: pgcrypto=1.3,uuid-ossp=1.1
-- requires_input: false
-- changes_summary:
--   rename: <node_id> users → customers
--   add:    <node_id> table orders
--   drop:   <node_id> table legacy_logs

-- section: pre_begin
-- (statements that must run outside a transaction)

-- section: transaction
BEGIN;
ALTER TABLE users RENAME TO customers;
CREATE TABLE orders (...);
DROP TABLE legacy_logs;
COMMIT;

-- section: post_commit
-- (statements that must run after the main transaction commits)
CREATE INDEX CONCURRENTLY orders_customer_idx ON orders(customer_id);
```

Sections:

- `pre_begin` — each statement in its own autocommit transaction before the
  main section. For `ALTER SYSTEM`, certain extension installs.
- `transaction` — single BEGIN/COMMIT wrapper. Bulk of DDL.
- `post_commit` — each statement in its own autocommit transaction after the
  main commits. For `CREATE INDEX CONCURRENTLY`, `REINDEX CONCURRENTLY`, and
  any statement that cannot run inside a transaction block.

Any section may be empty (omitted). The runner fails the migration if any
statement in any section fails; `post_commit` failures leave the schema
partially migrated, requiring a follow-up.

### Bounce markers

When the differ emits a migration that needs operator input (backfill,
destructive change, cast-incompatible type, complex refactor), the generator
sets `requires_input: true`, enumerates reasons in the header, and inserts
structured markers at each ambiguous statement:

```sql
-- topcat:bounce id=<uuid> kind=backfill object=<node_id>
-- topcat:bounce message="Column customers.email added with NOT NULL — backfill required."
-- topcat:bounce verification_query="SELECT count(*) FROM customers WHERE email IS NULL"
-- topcat:bounce placeholder_statement=true
ALTER TABLE customers ADD COLUMN email text NOT NULL;
```

Grammar: `-- topcat:bounce <key>=<value>`. Values with spaces use double
quotes. Consecutive `-- topcat:bounce` lines form one record scoped to the SQL
statement that follows. `placeholder_statement=true` signals the SQL is a
skeleton the operator must replace.

Runner contract:

- Migration with `requires_input: true` is refused unless a sidecar
  `<migration_id>.ack.sql` exists in the same directory.
- Ack file contains matching `-- topcat:ack id=<uuid>` lines for every bounce
  record, plus replacement DDL for `placeholder_statement=true` lines.
- `topcat migrate ack <migration_id>` generates a template; the operator
  edits it.
- Acks are validated at runner startup.

### Content-hash stability

`migration_id` = `sha256(canonical_json(ChangeSet))`. Canonical JSON per RFC
8785 (JCS): UTF-8, sorted keys, no insignificant whitespace.

**Excluded from the hash:**

- Catalog OIDs.
- Auto-generated constraint names.
- Runtime statistics (`last_analyzed`, `last_vacuumed`, `relpages`).
- Per-object comments (doc edits don't spawn new migrations).

**Included:**

- Schema-qualified names.
- Column types (canonicalized).
- Constraint expressions.
- Function bodies (exact text).
- Sequence parameters.
- RLS policy bodies.
- `pg_version` and `extension_versions`.

Same ChangeSet on two different pg versions → different migration_ids.
Formatting changes in the DDL emitter → same migration_id.

## Reverse migrations

Every forward migration produces a companion reverse migration, generated at
the same time and stored in the registry with `is_reverse = true` and
`forward_id = <forward_migration_id>`.

### Generation

Invert the ChangeSet and re-run the migration generator:

- `Add` ↔ `Drop`. The `before_model` and `after_model` captured in the
  ChangeSet provide the missing side.
- `Rename` forward → `Rename` reverse (names swapped).
- `Alter` forward → `Alter` reverse (old and new values swapped).
- `PartitionAttach` ↔ `PartitionDetach`.
- `SplitExtract` → reverse is a merge (recombining the extracted columns
  back into the source).
- `MergeAbsorb` → reverse is a split (re-extracting the absorbed table).

### Data-loss cases

Forward changes that lose information emit reverses with bounce markers:

- Forward drops a populated column → reverse emits `ALTER TABLE … ADD COLUMN`
  with `kind=data_restore`, `placeholder_statement=true`. Operator supplies
  restoration SQL (from backup, recomputation, or explicit null).
- Forward drops a table → reverse emits CREATE DDL with a
  `kind=data_restore` placeholder for row data.
- Forward changes a column type with lossy cast → reverse marks
  `kind=data_restore` for the column's pre-cast values.

### Storage and application

Reverse migrations are full artifacts, stored alongside forward migrations in
`_topcat.migration_registry`. Their content-hash is computed the same way as
forward migrations; the hash includes the forward's migration_id in the
ChangeSet header so the reverse is always distinguishable even when its DDL
matches another migration's DDL.

Operators apply reverses the same way as forwards. Topcat never applies
reverses automatically — rollback is a deliberate operator action.

## Migration verification

Every generated forward migration round-trips:

1. Clone `shadow_main` to `shadow_verify`.
2. Apply the forward migration to `shadow_verify`.
3. Read catalog. Assert structural equality with `shadow_head`.
4. On mismatch: fail, emit diff report, do not write the migration file.

Every generated reverse migration also round-trips:

5. Apply the reverse to `shadow_verify` (now at the post-forward state).
6. Read catalog. Assert structural equality with `shadow_main` (modulo
   `data_restore` bounce cases, which are excluded from the equality check).
7. On mismatch (outside excluded bounces): fail.

**Scope.** Verification proves equivalence between two local shadow states.
It does not prove the migration will succeed against staging or production,
which may have drifted. Verification is necessary but not sufficient.

## Migration DAG

Migrations form a DAG. Storage: `_topcat.migration_registry`.

- **Identity:** content hash.
- **Edges:** parent references. A merge produces a migration with two
  parents.
- **Reverses:** one per forward, identified by `is_reverse = true` and
  `forward_id`.
- **Application order:** topological walk from the common ancestor.

### Merge migrations

When the user merges two branches:

1. User merges git branches; working tree reflects merged source.
2. Topcat rebuilds `shadow_main` from the merged committed source. If two
   files collide on object identity, invoke Three-way node_id resolution.
3. Topcat rebuilds `shadow_head` from the working tree.
4. Diff → ChangeSet. Migration has `parents: [<branch_a_tip>,
   <branch_b_tip>]`.

Identical effects collapse: both branches producing the same ChangeSet
produce the same migration_id, already in the DAG.

## Sync (dev loop)

```
topcat sync
  ├─ parse working-tree files → candidate SchemaModel + File DAG
  ├─ conflict scan
  │    ├─ branch-merge collision → auto-resolve via alias + source rewrite
  │    └─ copy-paste / unknown / deleted → fail-loud
  ├─ read dev db SchemaModel via CatalogReader
  ├─ diff → dev_changeset
  ├─ for each changed object subgraph:
  │    ├─ acquire subgraph advisory lock, open transaction
  │    ├─ DROP … CASCADE (object-granular)
  │    ├─ re-apply via DDL emitter for each dropped node_id (object-granular)
  │    ├─ event trigger captures implicit objects, registry updated atomically
  │    ├─ commit, release lock, record `_topcat.sync_log` entry
  └─ final drift check: dev db SchemaModel equals candidate
```

### Apply granularity

Sync is **fully object-granular**. The DDL emitter (same one that drives
migration generation) produces per-object DDL from the SchemaModel; sync
re-applies only the dropped nodes, not their source files' siblings. This is
the minimum-work sync model.

The DDL emitter is the single SchemaModel→DDL path in the system, guaranteeing
that any DDL sync produces is identical to the DDL a migration would emit for
the same change.

### Transactions and failure recovery

- Many small transactions — one per subgraph. A single transaction over the
  whole sync would hold exclusive locks too long.
- `_topcat.sync_log (sync_id uuid, subgraph_root uuid, state text,
  started_at, finished_at, error)` checkpoints each subgraph. On crash, the
  next sync resumes: `state = in_progress` rows are retried from scratch
  (idempotent drop-cascade + DDL-emitted recreate).
- Session-level advisory lock `pg_advisory_lock(hashtext('topcat_sync'))`
  serializes concurrent topcat processes on the same dev db.

### Concurrency

Single developer, single local dev db. Multi-developer shared dev dbs are a
non-goal (drop-cascade-recreate would thrash); provision per-developer dbs.

## Rename detection

node_id persistence is the mechanism. Registry maps node_id N to pg object
`public.users`; a source file's `-- node_id: N` plus current `CREATE TABLE
public.customers (...)` gives `(node_id=N, new_name=customers)`. Registry
says `(node_id=N, current_name=users)`. Rename signal → `ALTER TABLE …
RENAME`.

No git dependence. No file-rename heuristics. Object identity is the file
header; the header moves with the object regardless of file location.

## Codegen

Plugin interface. Default plugins ship in-tree; projects register additional
plugins via config.

Plugin contract: consume a frozen SchemaModel, emit files to a configured
output directory. No access to shadow dbs or migrations — codegen is a pure
function of the schema.

Default plugins: Python, TypeScript, Rust, Go.

SchemaModel serializes to JSON (canonical JCS form) so plugins can be
implemented out-of-process.

## Bootstrapping

Three entry paths:

**Fresh project:** empty dev db → topcat installs `_topcat` schema and event
triggers → first `CREATE …` generates a node_id on apply.

**Project has schema-as-code files, no topcat history:**

1. Fresh dev db, event triggers installed.
2. `topcat update` generates `-- requires:` headers from SQL discovery.
3. File DAG topological sort, apply files in order. Event trigger captures
   every object, assigns node_ids, writes back to source files.
4. Steady state.

**Project has only migrations, no schema files:**

1. `pg_dump --schema-only` against a db at the desired state.
2. Fresh db, apply the dump, event trigger captures the object set.
3. Reverse-generate one source file per object (or per grouping) with topcat
   headers including node_ids.
4. Steady state.

**Adoption migration.** First diff after bootstrap is noise (every object is
new). `topcat sync --adopt` records the initial state without emitting a
migration.

## Data migration taxonomy

Every forward change is emitted deterministically. Cases that genuinely
cannot be inferred from catalog + annotations produce bounce markers for
operator input.

| Category | Behavior |
|---|---|
| Cast-compatible column type change | `ALTER TYPE`, no bounce. |
| Cast-incompatible column type change | Bounce: `kind=cast_incompatible`, `placeholder_statement=true`. Operator supplies `USING`. |
| NOT NULL added to populated column with no default | Bounce: `kind=backfill`, verification query included. |
| Rename (detected via node_id) | `ALTER … RENAME`. |
| Drop + add of different-shape objects with same node_id | Refuse to generate (bug or manual registry edit). |
| Table split (declared via `-- split_from:`) | Automated: create target, copy data, drop source columns. FK rewires bounce for operator review. |
| Table merge (declared via `-- merge_into:`) | Automated: add target columns, copy data, drop source table. |
| Undeclared table split/merge | Bounce: `kind=refactor`, `placeholder_statement=true`. Operator annotates with `-- split_from:` / `-- merge_into:` and re-runs. |
| CREATE INDEX CONCURRENTLY | Emit in `post_commit` section. No bounce. |
| Partition bound change (safe) | Automated: detach → in-place data-move → re-attach. |
| Partition bound change (overlapping redistribution) | Bounce: `kind=partition_bounds_complex`, verification query included. |
| Column drop of populated column | Bounce: `kind=destructive`, row count + sample query included. |
| Type-change on column referenced by index/FK/view | Drop dependents, alter, recreate dependents. Fully automated. |
| Replacement of PK with different columns | Bounce: `kind=identity_change`, includes the orphaned-FK query. |

### Split and merge annotations

To refactor tables without bouncing, declare intent in source files.

Split:

```sql
-- node_id: 01924b...
-- name: profiles
-- split_from: 01924a...
-- split_columns: first_name, last_name, avatar_url
-- split_key: user_id
```

Topcat emits:

1. `CREATE TABLE profiles (...)` (from the new file's SchemaModel).
2. `INSERT INTO profiles (user_id, first_name, last_name, avatar_url) SELECT user_id, first_name, last_name, avatar_url FROM user_profile`.
3. `ALTER TABLE user_profile DROP COLUMN first_name, DROP COLUMN last_name, DROP COLUMN avatar_url`.
4. For each FK referencing dropped columns: bounce with `kind=fk_rewire`
   (operator chooses whether FK should now target the new table).

Merge:

```sql
-- node_id: 01924c...
-- name: events
-- merge_from: 01924d...
-- merge_column_map: {"audit_log.ts":"events.occurred_at","audit_log.msg":"events.message"}
```

Emits:

1. Add any missing target columns.
2. `INSERT INTO events (occurred_at, message, ...) SELECT ts, msg, ... FROM audit_log`.
3. `DROP TABLE audit_log`.

Where the data-move can't be a column projection (filter predicates,
aggregation, type coercion beyond simple casts), the annotation falls back to
bounce with `kind=refactor_complex` and the operator supplies the data-move
DDL.

## Performance and caching

Binding targets:

| Workload | Object count | Target |
|---|---|---|
| `topcat sync` (incremental, ~5% changed) | 100 | < 3s |
| `topcat sync` (incremental, ~5% changed) | 1,000 | < 15s |
| `topcat sync` (from empty, first apply) | 1,000 | < 60s |
| `topcat migrate generate` (diff two shadows) | 1,000 | < 10s |
| Shadow rebuild (template reuse) | any | ~2s |
| Shadow rebuild (from scratch) | 1,000 | < 90s |

Caching:

- `shadow_main` template keyed by SHA-256 over the sorted file-set (path +
  content). Hit → `CREATE DATABASE … TEMPLATE`. Miss → rebuild, update cache.
- `shadow_head` not cached (working tree changes too frequently).
- CatalogReader queries parallelized per-object-type.
- DDL apply serialized within an object-family (pg safety) but independent
  subgraphs apply in parallel up to a configurable worker count (default 4).

## Testing strategy

- **Unit tests** for every Normalization rule (table-driven).
- **Integration tests** against real postgres across pg15, pg16, pg17, and
  current. Ephemeral containers; no mocked postgres.
- **Property tests** for round-trip: apply → read → re-emit → apply → read,
  assert catalog equality.
- **Migration tests** for each data-migration category: (before, after)
  fixture pair, assert forward + reverse both round-trip under verification.
- **Bounce tests** for each bounce category: migration emits correct markers,
  runner refuses without valid ack, accepts with valid ack.
- **Upgrade tests** for `_topcat` schema versioning: fixtures at each schema
  version, topcat upgrades correctly on startup.
- **Merge-resolution tests**: simulated git merges producing branch-merge
  collisions; alias table and source rewrite assertions.

## Observability

Structured JSON logs, one object per line.

Schema:

```json
{
  "ts": "2026-04-19T12:34:56.789Z",
  "level": "info" | "warn" | "error",
  "event": "<event_name>",
  "sync_id": "<uuid>" | null,
  "migration_id": "<hash>" | null,
  "node_id": "<uuid>" | null,
  "file": "<path>" | null,
  "pg_address": "<schema.name>" | null,
  "message": "<human-readable>",
  "details": { /* event-specific */ }
}
```

Event catalog:

- `sync.start`, `sync.complete`, `sync.subgraph.start`,
  `sync.subgraph.complete`, `sync.subgraph.failed`.
- `diff.edge.conflict`, `diff.stale_declared`, `diff.unknown_target`.
- `migration.generate.start`, `migration.generate.complete`,
  `migration.verify.pass`, `migration.verify.fail`.
- `reconcile.orphan`, `reconcile.drift.absorbed`, `reconcile.drift.reverted`.
- `parse.failed`, `parse.dynamic_sql`, `parse.dynamic_ddl`.
- `conflict.copy_paste`, `conflict.unknown_node_id`,
  `conflict.branch_merge_resolved`, `conflict.deleted_header`.
- `shadow.rebuild.start`, `shadow.rebuild.complete`, `shadow.cache.hit`,
  `shadow.cache.miss`.
- `meta.schema.upgrade.start`, `meta.schema.upgrade.complete`.

Levels:

- `info`: routine progress.
- `warn`: surfacing but non-blocking (stale declared, low-confidence parse,
  dynamic SQL).
- `error`: aborts (conflict scan fail, verify mismatch, missing extension).

CLI flags (global): `--log-format json|text` (default json),
`--log-file <path>`, `--log-level debug|info|warn|error` (default info).

## Uninstall and reversibility

`topcat uninstall`:

1. List every source file with `-- node_id:` headers. Strip those lines.
2. Drop the event triggers on the dev db.
3. Drop the `_topcat` schema.
4. User data and user-schema objects untouched.
5. Migration files on disk remain as plain SQL; no topcat runtime dependency.

`-- requires:` / `-- layer:` / `-- exists:` / `-- name:` headers remain —
they belong to the File DAG world that predates pg-dev-buddy.

Default mode: dry-run (reports what would happen). `--mode execute` performs
the teardown. `--force` skips the interactive confirmation.

## `_topcat` meta-schema versioning

`_topcat.schema_version (version int, applied_at timestamptz)`.

On every command that connects to a managed db:

1. Read `schema_version`. Missing → assume version 0 (pre-install).
2. Apply every built-in `_topcat` migration from version-in-db through
   version-in-tool.
3. Refuse to run if version-in-db > version-in-tool (downgrade unsupported).

Built-in `_topcat` migrations are SQL files embedded in the topcat binary.

### `_topcat` schema DDL (version 1)

```sql
CREATE SCHEMA _topcat;
SET LOCAL topcat.bootstrapping = 'true';

CREATE TABLE _topcat.schema_version (
  version int PRIMARY KEY,
  applied_at timestamptz NOT NULL DEFAULT now()
);

CREATE FUNCTION _topcat.gen_uuidv7() RETURNS uuid LANGUAGE sql AS $$
  -- UUIDv7: 48-bit unix-ms timestamp + version bits + random
  -- shipped by topcat at install time; body elided here
  SELECT gen_random_uuid();
$$;

CREATE TABLE _topcat.node_registry (
  node_id uuid PRIMARY KEY,
  classid oid NOT NULL,
  objid oid NOT NULL,
  objsubid int NOT NULL DEFAULT 0,
  address_type text NOT NULL,
  object_names text[] NOT NULL,
  object_args text[] NOT NULL,
  file text,
  created_at timestamptz NOT NULL DEFAULT now(),
  UNIQUE (classid, objid, objsubid)
);

CREATE TABLE _topcat.node_aliases (
  loser_node_id uuid PRIMARY KEY REFERENCES _topcat.node_registry(node_id),
  winner_node_id uuid NOT NULL REFERENCES _topcat.node_registry(node_id),
  merged_at timestamptz NOT NULL DEFAULT now(),
  CHECK (loser_node_id <> winner_node_id)
);

CREATE TABLE _topcat.sync_log (
  sync_id uuid NOT NULL,
  subgraph_root uuid NOT NULL,
  state text NOT NULL CHECK (state IN ('in_progress', 'complete', 'failed')),
  started_at timestamptz NOT NULL DEFAULT now(),
  finished_at timestamptz,
  error text,
  PRIMARY KEY (sync_id, subgraph_root)
);

CREATE TABLE _topcat.migration_registry (
  migration_id text PRIMARY KEY,
  parent_ids text[] NOT NULL DEFAULT '{}',
  is_reverse boolean NOT NULL DEFAULT false,
  forward_id text REFERENCES _topcat.migration_registry(migration_id),
  created_at timestamptz NOT NULL DEFAULT now(),
  changeset jsonb NOT NULL,
  ddl_text text NOT NULL,
  pg_version text NOT NULL,
  extension_versions jsonb NOT NULL DEFAULT '{}'::jsonb,
  requires_input boolean NOT NULL DEFAULT false,
  CHECK (is_reverse = (forward_id IS NOT NULL))
);

CREATE FUNCTION _topcat.on_ddl_command_end() RETURNS event_trigger
LANGUAGE plpgsql AS $$
DECLARE r record;
BEGIN
  IF current_setting('topcat.bootstrapping', true) = 'true' THEN RETURN; END IF;
  FOR r IN
    SELECT c.*, a.object_names, a.object_args
    FROM pg_event_trigger_ddl_commands() c,
         LATERAL pg_identify_object_as_address(c.classid, c.objid, c.objsubid) a
    WHERE c.schema_name NOT IN ('pg_catalog', 'information_schema', '_topcat')
  LOOP
    INSERT INTO _topcat.node_registry
      (node_id, classid, objid, objsubid, address_type, object_names, object_args)
    VALUES
      (_topcat.gen_uuidv7(), r.classid, r.objid, r.objsubid,
       r.object_type, r.object_names, r.object_args)
    ON CONFLICT (classid, objid, objsubid) DO NOTHING;
  END LOOP;
END $$;

CREATE EVENT TRIGGER _topcat_ddl_end ON ddl_command_end
  EXECUTE FUNCTION _topcat.on_ddl_command_end();

CREATE FUNCTION _topcat.on_sql_drop() RETURNS event_trigger
LANGUAGE plpgsql AS $$
DECLARE r record;
BEGIN
  IF current_setting('topcat.bootstrapping', true) = 'true' THEN RETURN; END IF;
  FOR r IN
    SELECT * FROM pg_event_trigger_dropped_objects()
    WHERE schema_name NOT IN ('pg_catalog', 'information_schema', '_topcat')
  LOOP
    DELETE FROM _topcat.node_registry
     WHERE classid = r.classid AND objid = r.objid AND objsubid = r.objsubid;
  END LOOP;
END $$;

CREATE EVENT TRIGGER _topcat_sql_drop ON sql_drop
  EXECUTE FUNCTION _topcat.on_sql_drop();

INSERT INTO _topcat.schema_version (version) VALUES (1);
```

Subsequent versions are shipped as SQL files embedded in the topcat binary;
each one starts with `SET LOCAL topcat.bootstrapping = 'true';`, applies its
changes, and ends with `INSERT INTO _topcat.schema_version (version) VALUES
(N);`.

## Config surface

`topcat.toml`:

```toml
[postgres]
dev_db_url = "postgres://localhost/myapp_dev"
shadow_db_url = "postgres://localhost"           # cluster; topcat creates shadow_main/head/verify
shadow_main_cache_dir = "~/.cache/topcat/shadows"
require_superuser = true
strict_mode = false                               # parse-failure / dynamic-SQL policy
worker_count = 4                                  # parallel DDL apply workers

[postgres.plugins]
python = { path = "./codegen/python.lua" }
typescript = { path = "./codegen/ts.lua" }
```

Environment variables: `TOPCAT_DEV_DB_URL`, `TOPCAT_SHADOW_DB_URL`.
Precedence (highest → lowest): CLI flag → `TOPCAT_*` env → `topcat.toml` →
`DATABASE_URL` (dev_db_url default only) → `.pgpass` defaults for the shadow
cluster.

Parse strictness: `--strict`, `--default`, `--permissive` flags (or
`strict_mode = true` in config) control fail-loud/warn boundary for parse
failures, dynamic SQL, and unresolved declared edges.

## CLI surface

Global flags on every command: `--log-format`, `--log-level`, `--log-file`,
`--config <path>`, `--mode dry-run|execute`.

**Sync and shadow:**

- `topcat sync [--adopt] [--force]` — sync dev db to working tree.
- `topcat shadow build [main|head|verify] [--rebuild]` — rebuild a shadow.
- `topcat shadow clean [--all]` — drop topcat-managed shadows.

**Diff and migrate:**

- `topcat diff [--from <ref>] [--to <ref>] [--format text|json]` — print
  the ChangeSet.
- `topcat migrate generate [--no-reverse] [--no-verify]` — produce next
  forward + reverse migrations.
- `topcat migrate verify <migration_id>` — verify locally.
- `topcat migrate ack <migration_id>` — template an ack file.
- `topcat migrate list [--since <migration_id>] [--format text|json]` —
  list registered migrations.
- `topcat migrate apply-reverse <forward_migration_id>` — apply the
  companion reverse locally (dev rollback).

**Reconcile and maintenance:**

- `topcat reconcile [--absorb|--revert] [--pattern <glob>]` — handle drift.
- `topcat write-deps [--confidence high|all]` — materialize inferred edges.
- `topcat pause` — disable event triggers (for bulk restore).
- `topcat resume` — re-enable and rescan.
- `topcat uninstall` — strip headers, drop `_topcat`.
- `topcat meta-schema upgrade [--force]` — force a `_topcat` upgrade
  (otherwise automatic).

**Current-topcat commands** (unchanged; no postgres required): `concat`,
`update`, `analyze`, `clean`, `schema`, `export`, `import`, `config`.

## Component map

| Module | Responsibility |
|---|---|
| `pg/client` | Connection management, DDL application. |
| `pg/event_trigger` | Trigger definition, install/uninstall, tag filter. |
| `pg/catalog_reader` | Per-object-type structured readers, normalization. |
| `pg/body_parser` | libpg_query integration for plpgsql. |
| `pg/ddl_emitter` | SchemaModel → DDL. Shared by sync re-apply and migration generation. |
| `schema_model` | Canonical in-memory types; JCS JSON serialization. |
| `object_dag` | Object-granularity DAG built from catalog + headers. |
| `shadow_db` | Template management, fresh builds, cleanup, caching. |
| `differ` | SchemaModel → SchemaModel → ChangeSet. |
| `migration_generator` | Forward + reverse ChangeSet → multi-section migration files with bounce markers. |
| `migration_verifier` | Round-trip verification against local shadows. |
| `migration_registry` | Migration DAG storage and traversal. |
| `node_alias_resolver` | Aliased node_id lookup for historical migration reads. |
| `sync` | Dev-db sync with advisory locking and checkpointing. |
| `reconcile` | Drift detection + absorb/revert actions. |
| `codegen/plugin` | Plugin interface and default plugins. |
| `bootstrap` | Entry-path workflows. |
| `uninstall` | Teardown of node_id headers and `_topcat`. |
| `meta_migrations` | `_topcat` schema versioning with embedded upgrade SQL. |
| `observability` | Structured JSON logging. |
| `cli` | Full subcommand surface. |

Existing topcat modules (`file_node`, `file_dag`, `stable_topo`, `sql_parser`,
`header_generator`) are kept unchanged; they serve current-topcat commands at
file granularity. The Object DAG and its tooling are additive.

# M11 — node_id identity + conflict scan + three-way resolution

> Self-contained execution plan for milestone M11 of the pg-dev-buddy
> roadmap. Ties the `_topcat.node_registry` (from M5) to source files by
> extending `file_node` parsing to recognize `-- node_id:` headers,
> implementing the six-case conflict scanner from architecture
> §Conflict taxonomy, and shipping automatic three-way resolution driven
> by UUIDv7 time-bits per architecture §Three-way node_id resolution.
> M11 unblocks M12 (MVP sync) by making rename, branch-merge, and
> adoption safe.

## Context

Every downstream pg-dev-buddy milestone treats node_id as the identity
primitive — M12 (sync) refuses to proceed while the conflict scanner is
dirty, M13 (reverse migrations) reads through the alias table to resolve
historical references, and M9/M10 (migration generation/verification)
stamp `node_id` into changes_summary headers. M11 is therefore the tier
that connects file-side identity (`-- node_id:` headers in source) with
db-side identity (`_topcat.node_registry` rows created by the M5 event
trigger). A wrong scanner silently corrupts rename detection and makes
branch merges unsafe, so fail-loud defaults are load-bearing.

M11 extends topcat's existing file-header parsing rather than introducing
a parallel parser. Source rewriting happens only during three-way
resolution (and only the loser's header), and the user sees it as a
one-line diff on the next commit per architecture §Three-way node_id
resolution, step 3. The `node_alias_resolver` module is green-field.

## Prerequisites

- **M5** — `_topcat` meta-schema with `node_registry` and `node_aliases`
  tables installed. Architecture §`_topcat` schema DDL (version 1)
  defines the concrete columns M11 reads and writes:
  - `_topcat.node_registry (node_id uuid PRIMARY KEY, classid, objid,
    objsubid, address_type, object_names[], object_args[], file,
    created_at)`.
  - `_topcat.node_aliases (loser_node_id PK, winner_node_id, merged_at,
    CHECK loser <> winner)`.
  - `_topcat.gen_uuidv7()` — UUIDv7 generator shipped at install time;
    M11 depends on its time-bit ordering property for "older wins".
- **Existing file-header parser** — `src/file_node.rs` (flat in this
  worktree; planned to modularize under `src/file_node/parsing.rs` per
  CLAUDE.md architecture table). M11 extends the existing parser with a
  new `node_id: Option<uuid::Uuid>` field and round-trip writer support.
  No fork, no parallel parser.
- **`uuid` crate** with the `v7` feature already present or added in
  `Cargo.toml` for parsing the header value and extracting
  timestamp bits.
- **M1 `pg/client`** for reading `_topcat.node_registry` /
  `_topcat.node_aliases` during the scan and writing alias rows during
  resolution.

Not required: M3 (ddl_emitter), M7 (object_dag), M8 (differ), M10
(migration_verifier). M11 is identity plumbing; it does not touch DDL
emission, diffing, or apply.

## Scope

### In scope for this milestone

- Extend `file_node` parsing to accept `-- node_id: <uuid>` headers and
  preserve them through the round-trip writer path used by `update` and
  `header_generator`.
- `node_scan` conflict scanner per architecture §Conflict taxonomy,
  emitting one typed variant per row of the table:
  - `CleanRename` (auto-handled downstream by M9).
  - `CopyPaste` — **fail-loud**.
  - `UnknownNodeId` — **fail-loud**.
  - `DeletedHeader` — **warn**, orphan registry row, drop-and-recreate.
  - `BranchMergeCollision` — dispatched to three-way resolution.
  - `OrphanRegistryRow` — drop by default; route through `reconcile`
    when the pg-name matches a configured `--root-pattern`.
- `node_alias_resolver` module: load `_topcat.node_aliases` at start of
  read, expose `resolve(node_id: Uuid) -> Uuid` that walks the alias
  chain to the winner. Used by any code that reads historical
  migrations (M13) or cross-references legacy registry rows.
- Three-way auto-resolution:
  - UUIDv7 time-bit extractor; older node_id wins.
  - Loser source file rewrite: update only the `-- node_id:` line,
    preserve every other byte of the file (including trailing newline,
    BOM, line endings).
  - `_topcat.node_aliases` row insert inside the same transaction as
    the shadow_main rebuild.
  - No user prompt; diff appears in the next `git commit`.
- `topcat sync --adopt`: bootstrap node_ids against an existing dev db.
  Reads the catalog via M5, registers every object in
  `_topcat.node_registry`, writes `-- node_id:` headers into matching
  source files. No migration emitted (architecture §Bootstrapping,
  "Adoption migration" paragraph).
- CLI flag `--mode dry-run|execute` on `sync --adopt` inherits the
  global convention; dry-run previews every source-file header write
  and every registry insert without touching disk or db.

### Explicitly NOT in scope (deferred to M12+)

- **Full `sync` pipeline** — object-granular drop-cascade-recreate,
  per-subgraph advisory locking, `sync_log` checkpointing. M11 ships
  only `--adopt` and the scanner invoked from `sync`; the main apply
  path lands in M12.
- **Rename emission** — the `CleanRename` variant is detected by M11
  but the actual `ALTER … RENAME` DDL is emitted by M9 from the
  resulting `ChangeSet`.
- **Reconcile UX** — M11 routes `OrphanRegistryRow` with protected
  pattern to `reconcile` as a flag; the `topcat reconcile` command
  itself is later (architecture §Component map: `reconcile`).
- **Header emission for schema files with no existing `-- name:`** —
  M11 assumes topcat-managed source files already carry a metadata
  header block. Bootstrapping from pure pg_dump output is M11 via
  `--adopt` against an already-topcat-shaped tree; pg_dump → topcat
  shape conversion is the separate `import pg-dump` command.

## Deliverables (files and modules)

| Path | Kind | Purpose |
|---|---|---|
| `src/file_node.rs` | extend | Add `node_id: Option<Uuid>` to `FileNode`; parse `-- node_id: <uuid>` header; round-trip through writer. |
| `src/file_node.rs` (tests mod) | extend | Unit tests for header parse/write, malformed UUIDs, missing / multiple headers. |
| `src/header_generator.rs` or equivalent | extend | Emit `-- node_id:` in the generated header block when the node_id is known; preserve existing ones when present. |
| `src/node_scan/mod.rs` | new | Public `scan(file_dag: &TCGraph, registry: &NodeRegistry) -> ScanReport` entry point. |
| `src/node_scan/report.rs` | new | `ScanReport { clean_renames, copy_paste, unknown, deleted, branch_collisions, orphans }` with `fn is_fatal(&self) -> bool` (copy_paste + unknown). |
| `src/node_scan/classify.rs` | new | Per-case classification rules matching the architecture §Conflict taxonomy table row-for-row. |
| `src/node_alias_resolver/mod.rs` | new | `AliasResolver::load(client) -> Self`, `resolve(&self, id: Uuid) -> Uuid` with cycle-break assertion (alias chain must terminate — DB CHECK `loser <> winner` + runtime hop budget). |
| `src/node_alias_resolver/tests.rs` | new | Chain resolution (A→B, B→C → resolve(A)==C), no-alias passthrough. |
| `src/resolve/three_way.rs` | new | UUIDv7 timestamp extractor + winner selection; loser-file header rewrite (in-place, byte-preserving); alias row insert. |
| `src/resolve/uuidv7_time.rs` | new | Pure function `uuidv7_timestamp(u: Uuid) -> u64` over the high 48 bits per RFC 9562 §5.7. No DB round-trip. |
| `src/commands/sync/mod.rs` | new (stub) | `sync` command skeleton; M11 only wires `--adopt` path plus the scan-before-apply hook; main apply loop `todo!()` until M12. |
| `src/commands/sync/adopt.rs` | new | `--adopt` implementation: read catalog, insert registry rows, write `-- node_id:` headers into source files, honors `--mode dry-run`. |
| `src/cli/sync.rs` | new | `clap` group for `topcat sync [--adopt] [--force] --mode dry-run\|execute`. |
| `src/main.rs` | extend | Register the `sync` subcommand. |
| `tests/node_scan.rs` | new | Integration tests per conflict case using fixture sqlite-ish `MockRegistry` + fixture SQL trees. |
| `tests/three_way_resolution.rs` | new | Property test: post-resolution apply to `shadow_main` yields no `BranchMergeCollision`. |
| `tests/input/node_id/` | new dir | Fixture SQL trees keyed by conflict case (`clean_rename/`, `copy_paste/`, `unknown/`, `deleted_header/`, `branch_merge/`, `orphan/`, `orphan_root_pattern/`). |
| `Cargo.toml` | extend | Enable `uuid` `v7` feature if not already; add `proptest` to dev-deps if not already pulled by an earlier milestone. |

## Implementation phases

### Phase 1 — file_node parsing extension

Extend `FileNode` and the header parser to recognize `-- node_id:
<uuid>`. Preserve through the writer path. Unit tests: parse valid
UUIDv7, reject non-UUID values as `FileNodeError::InvalidNodeId`,
round-trip preserves all other header lines byte-for-byte. Dry-run
writer must emit the expected serialized header. ~0.4 session.

### Phase 2 — node_scan + node_alias_resolver

Implement the six-case classifier against a `NodeRegistry` abstraction
(a thin struct over `_topcat.node_registry` read via pg/client). Ship
`ScanReport` with explicit fatal / non-fatal separation. Ship
`node_alias_resolver` with load-and-resolve; chain hop budget defaults
to 64. Fixture tree tests driving every variant — including the
**copy-paste day-one fixture** (`tests/input/node_id/copy_paste/`: two
files, same `-- node_id:`) that must produce a fatal `ScanReport` on
day one. ~0.7 session.

### Phase 3 — three-way resolution

Implement `uuidv7_timestamp` per RFC 9562 §5.7 (ms unix epoch in the
high 48 bits). Winner = older node_id. Loser-file rewriter preserves
every other byte via a targeted line replace keyed on the
`-- node_id:` prefix. `_topcat.node_aliases` insert. Property test:
start with two conflicting files → resolve → re-scan → no remaining
`BranchMergeCollision`, registry has exactly one alias row.
~0.6 session.

### Phase 4 — `topcat sync --adopt`

Wire a minimal `sync` command that runs the scan, applies three-way
resolution, and on `--adopt` additionally reads the catalog and emits
one `INSERT INTO _topcat.node_registry` + one header rewrite per
managed source file. Main apply path stays `todo!()` for M12. Dry-run
default per architecture §Risks (M11); `--mode execute` required to
write to disk or db. ~0.5 session.

### Phase 5 — fixture sweep + hardening

Build a fixture per row of architecture §Conflict taxonomy. Wire the
orphan-with-root-pattern case through `reconcile` flagging (the
command itself is deferred; M11 only sets the flag in the
`ScanReport`). Clippy + fmt + cargo test. ~0.3 session.

## Risks (copied and amplified)

Roadmap §M11 flags three risks; each gets a day-one mitigation.

- **Source-rewriting during sync is surprising UX.** Architecture says
  no user prompt — the loser file gets a one-line diff on next commit.
  Verify this meets the user's mental model.
  - *Day-one mitigation.* `sync --adopt` **defaults to `--mode
    dry-run`** (matches `clean` defaults per CLAUDE.md "Project-
    specific gotchas"). Every rewrite is previewed as a unified-diff
    fragment with the absolute path; nothing touches disk until
    `--mode execute`. The three-way resolver prints a one-line summary
    (`resolved 01924a... < 01924b...; rewrote path/to/loser.sql`)
    under default log level so the user sees the rewrite happen even
    when no prompt is issued.
  - *Amplification.* The rewriter must be byte-preserving outside the
    single header line. Test fixture `tests/input/node_id/
    branch_merge/loser_with_bom_and_crlf.sql` exercises a UTF-8 BOM +
    CRLF line endings; it lands in Phase 3 and must pass before the
    property test runs.

- **Orphan registry rows.** Sync drops the object by default, but
  protected patterns route to reconcile. Don't lose user data to an
  overzealous drop; dry-run must preview.
  - *Day-one mitigation.* `OrphanRegistryRow` classification reads the
    effective `--root-pattern` set from `settings`; any orphan whose
    pg-name matches a protected pattern is flagged `requires_review`
    and **never** emits a drop. Phase 5 ships the
    `tests/input/node_id/orphan_root_pattern/` fixture that asserts
    the flag path and asserts no drop statement is produced.
  - *Amplification.* The pattern set must match against
    schema-qualified names (`public.users`, not just `users`); bare
    names are ambiguous across schemas. The classifier uses the same
    `SchemaFilter` surface as `schema_utils.rs` for consistency.

- **Copy-paste detection** must be robust: node_id N in two
  working-tree files is **always fail-loud**, never auto-resolved.
  Don't confuse with branch-merge (which has different node_ids for
  the same pg-object).
  - *Day-one mitigation.* The classifier has two disjoint code paths:
    `group_by_node_id` (duplicate node_ids → copy_paste) runs before
    `group_by_pg_object` (duplicate pg-objects → branch_merge). A
    fixture `tests/input/node_id/copy_paste/` exists from Phase 2 and
    is the first test written; the entire branch-merge code path is
    dead until copy-paste is green.
  - *Amplification.* The failure message must list **every** file that
    claims the duplicated node_id (not just the first two) so the
    user can locate the rogue copy-paste in a large tree. `ScanReport`
    stores `Vec<PathBuf>` per copy-paste entry, not a fixed-size
    tuple.

## Tests

### Unit tests

- `FileNode` parse/write round-trips with and without `-- node_id:`.
- `uuidv7_timestamp` against known UUIDv7 samples (RFC 9562 appendix
  examples).
- `AliasResolver` chain resolution + hop-budget bail-out (malicious
  cycle assertion, should never occur in practice given DB CHECK, but
  defense-in-depth).
- `ScanReport::is_fatal` covers (copy_paste | unknown) correctly.

### Integration tests (`tests/node_scan.rs`)

One fixture directory per conflict case. Each fixture has a
`registry.json` describing the pre-existing `_topcat.node_registry`
rows and a directory of `.sql` files. The test loads both, runs
`node_scan::scan`, asserts the `ScanReport` shape.

Simulated git-merge fixtures: `branch_merge/a_then_b/` has a history
where branch A commits file with node_id X, branch B commits file with
node_id Y (both CREATE the same pg-object); the merged tree holds both
files. Assert: scanner emits `BranchMergeCollision`, resolver picks
older UUIDv7, writes the alias row, rewrites the loser.

### Property test (`tests/three_way_resolution.rs`)

Generator produces a bounded random set of conflicting file pairs
(schema + name + two UUIDv7s). Pipeline: `scan → resolve → re-scan`.
Assertion: the second scan's `branch_collisions` is empty. A second
assertion: applying the post-resolution file set to a fresh
`shadow_main` (via M5 event trigger + a `CREATE …` loop) yields zero
`copy_paste` and zero `branch_merge` in the scan.

### Fixture tree (`tests/input/node_id/`)

- `clean_rename/` — single file, node_id matches registry, pg-name
  differs → `CleanRename`.
- `copy_paste/` — two files, same node_id → fatal `CopyPaste`, must
  list both paths.
- `unknown/` — file carries node_id not in registry → fatal
  `UnknownNodeId`; error message must suggest `sync --adopt` or header
  removal (architecture §Conflict taxonomy row 3).
- `deleted_header/` — file has no `-- node_id:` but body matches a
  registry row by normalized CREATE → warn, orphan registry row
  scheduled for drop-and-recreate.
- `branch_merge/` — two files, different node_ids, same pg-object →
  dispatch to three-way.
- `branch_merge/loser_with_bom_and_crlf.sql` — byte-preserving rewrite.
- `orphan/` — registry row, no file claim → drop scheduled.
- `orphan_root_pattern/` — registry row, no file claim, pg-name
  matches `--root-pattern` → `requires_review`, no drop.

## Verification snapshot

```bash
nix develop --command cargo fmt --check
nix develop --command cargo clippy -- -D warnings
nix develop --command cargo test --lib                              # (new) file_node + node_scan + alias_resolver units
nix develop --command cargo test --test node_scan                   # (new) per-case integration
nix develop --command cargo test --test three_way_resolution        # (new) property test
nix develop --command cargo run -- sync --adopt --mode dry-run -i tests/input/node_id/adopt  # (new) dry-run preview smoke
nix develop --command cargo run -- sync --adopt --mode execute -i tests/input/node_id/adopt  # (new) end-to-end against ephemeral pg
```

All seven must pass. The scan must detect every row of architecture
§Conflict taxonomy on its corresponding fixture. Property test must
converge in `proptest`'s default 256 cases with no shrink failure.

## Open questions to resolve before starting

1. **Source-rewriting three-way resolution UX — silent vs prompt vs
   dry-run default.** Architecture §Three-way node_id resolution,
   final paragraph says "No user prompt." Roadmap §M11 risks question
   whether this meets user mental models. Recommend: honor architecture
   (no prompt) but make `sync --adopt` default to `--mode dry-run`
   (matches `clean` defaults in CLAUDE.md), and always log the rewrite
   at default log level. `sync` without `--adopt` (M12) inherits the
   same dry-run default.
2. **Hop budget for the alias resolver.** DB CHECK prevents
   `loser == winner`; chains are in practice 1 deep, but nothing in the
   schema prevents A→B and B→C across two merges months apart.
   Propose 64 as a budget with a log warning at >4 hops.
3. **node_id canonical form.** Architecture shows `01924a...` (26-char
   lowercase truncation) in prose but UUIDs are 36-char with hyphens.
   Enforce canonical lowercase hyphenated on parse/write; reject
   truncations. Confirm before Phase 1.
4. **`FileNode::node_id: Option<Uuid>` vs required.** Files newly
   adopted via `sync --adopt` will gain the header on first run, but
   pre-M11 commits may lack it entirely. Keep `Option<Uuid>`;
   `UnknownNodeId` is only triggered when the header is present and
   unresolvable, not when it's absent.
5. **CopyPaste error format — JSON vs human?** `log-format=json` global
   flag implies both. Emit structured via `observability` module
   (architecture §Observability) with `conflict.copy_paste` kind;
   human format derives from the same record.
6. **Where does `--root-pattern` come from during `sync`?** The CLI
   has `analysis.root_patterns` in `topcat.toml`; reuse that surface
   (architecture §Config surface). Don't invent a second config knob.
7. **Adoption ordering.** If `sync --adopt` runs against a dev db
   whose objects were created out-of-order vs the source tree's DAG,
   should the registry `created_at` follow catalog-order or DAG-order?
   Recommend catalog-order (what was actually observed) with a
   follow-up note: UUIDv7 time-bits may not match DAG topological
   order, which is fine because only pg-object-identity conflicts
   consult time-bits.

## Session sizing notes

Roadmap allocates **~2–3 sessions**. Suggested split:

- **Session 1** (Phase 1 + Phase 2): `file_node` extension, round-trip
  tests, `node_scan` classifier with all six variants, alias resolver.
  Green on copy-paste + unknown fatals + clean-rename detection.
- **Session 2** (Phase 3 + start of Phase 4): three-way resolver,
  UUIDv7 timestamp extractor, byte-preserving loser rewriter, property
  test wired. Begin `sync --adopt`.
- **Session 3** (finish Phase 4 + Phase 5): adopt command complete,
  dry-run preview path, orphan root-pattern fixture, full clippy pass,
  commit.

If Phase 3's byte-preserving rewriter balloons (encoding detection
edge cases around BOM + line endings), split it off into M11b with a
narrower supported encoding (UTF-8 LF only) and widen in a follow-up.
The property test must be green on the core set before M11 is
considered done.

## Referenced architecture sections

- `## Identity and node_id` (docs/pg-dev-buddy-architecture.md L221) —
  `-- node_id:` header contract and registry row layout.
- `### Conflict taxonomy` (L229) — six-case table; every row maps to a
  variant on `ScanReport`.
- `### Three-way node_id resolution` (L242) — five-step resolution
  algorithm including UUIDv7 time-bit winner, loser-file rewrite, and
  `_topcat.node_aliases` insertion.
- `## Bootstrapping` (L756) — adoption-migration paragraph (`topcat
  sync --adopt` records initial state without emitting a migration).
- `### `_topcat` schema DDL (version 1)` (L961) — `node_registry`,
  `node_aliases`, `gen_uuidv7` concrete schema.
- `## Component map` (L1134) — `node_alias_resolver` row defines this
  module's responsibility ("Aliased node_id lookup for historical
  migration reads.").
- `## CLI surface` (L1097) — `topcat sync [--adopt] [--force]`
  signature and global `--mode dry-run|execute`.

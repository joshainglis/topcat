# M18 — uninstall

> Reverse topcat's pg-dev-buddy adoption on a managed database: strip every
> `-- node_id:` header from source files, drop the event triggers, drop the
> `_topcat` schema, and leave user data plus the pre-existing File DAG headers
> (`-- name:`, `-- requires:`, `-- layer:`, `-- exists:`) untouched. Dry-run
> default; `--mode execute` performs the teardown; `--force` skips the
> interactive confirmation.

## Context

M18 is the reversibility contract topcat promises every adopter: "leaving is
cheap." Architecture §Uninstall and reversibility codifies the five-step
teardown, and §Bootstrapping lists `topcat uninstall` as the exit for all
three entry flows in M17. Without it, users cannot safely evaluate
pg-dev-buddy on a real schema — the adoption is one-way, which blocks trust
and blocks rollback after a failed pilot.

The scope is narrow and the mechanics mirror patterns topcat already ships
in `src/commands/clean/` (dry-run by default, `--mode execute`, `--force`,
confirmation prompt, deletion preview table). M18 extends `file_node/`
parsing with a header-strip path that preserves every other header line
verbatim, and extends `pg/client` with two small teardown routines: drop
topcat's event triggers and drop the `_topcat` schema with `CASCADE`.

M18 is parallelizable with M15, M16, M17. It depends on M5 having shipped
the event triggers and on `file_node/parsing.rs` existing (it does today,
pre-pg-dev-buddy). No changes to the migration_generator, differ, or
object_dag — uninstall is a file-and-catalog operation, not a schema
operation.

## Prerequisites

Depends on M5 (event triggers must be installable so there is something to
drop) and on the existing `file_node` parsing module. Concrete artifacts
expected to exist before M18 starts:

- `src/pg/client.rs` — async Postgres client wrapper from M1 with
  connection pooling, typed error taxonomy, and transaction helpers.
- `src/pg/event_trigger/mod.rs` from M5 — in particular the
  `uninstall(&Client)` routine that drops `_topcat_ddl_end` and
  `_topcat_sql_drop`. M18 calls this directly rather than reissuing the
  `DROP EVENT TRIGGER` statements itself.
- `_topcat` meta-schema v1 installed by M1's bootstrapper — includes
  `_topcat.schema_version`, `_topcat.node_registry`, `_topcat.node_aliases`,
  `_topcat.sync_log`, `_topcat.migration_registry`, and
  `_topcat.gen_uuidv7()`. M18 does not consult these; it drops the whole
  schema with `CASCADE`.
- `src/file_node/parsing.rs` — existing header reader (`get_file_headers`,
  `from_file`). M18 adds a sibling strip-headers entry point here; does not
  replace the existing parsers.
- `src/commands/clean/common.rs` — existing `show_deletion_preview`,
  `perform_deletion` patterns. M18 reuses the preview-table and
  confirmation-prompt shape; the actual "action" is header rewrite + DDL,
  not file deletion, so the helpers are copied-with-adjustment rather than
  called directly.
- `src/cli/execution.rs` — existing `ExecutionMode` (DryRun/Execute) and
  `--force` flag plumbing. Reused verbatim.
- Integration-test harness that spins up an ephemeral Postgres with
  superuser role (shipped by M1, reused by M5). M18 reuses it.

## Scope

### In scope

- `topcat uninstall [--mode dry-run|execute] [--force]` CLI command.
- List every source file in the configured input tree(s) whose headers
  contain a line matching `<comment_str> node_id:` and strip exactly those
  lines. Preserve all other header lines byte-for-byte, preserve trailing
  whitespace / blank separator line between header block and body, preserve
  line endings.
- Drop topcat's event triggers (`_topcat_ddl_end`, `_topcat_sql_drop`) on
  the dev database by delegating to M5's `event_trigger::uninstall`.
- Drop the `_topcat` schema via `DROP SCHEMA _topcat CASCADE`.
- Dry-run preview: emit a table of source files that would be modified
  (counting stripped lines) and a confirmation of which DDL statements
  would run on the dev db. No database or filesystem state changes.
- Execute path: prompt "About to strip N node_id headers from M files, drop
  2 event triggers, and drop _topcat schema on <db_url>. Continue? [y/N]"
  unless `--force` is set.
- Exit codes: 0 on success or clean no-op, 1 on partial failure (e.g.
  filesystem writes succeeded but DDL failed), 2 on user-cancelled.

### Out of scope

- Migration files on disk. The architecture explicitly says "migration
  files remain as plain SQL; no topcat runtime dependency" — leave them as
  they are. They are already valid SQL without `_topcat`.
- User data. No table contents touched.
- User-schema objects. Only `_topcat` schema and topcat's own event
  triggers get dropped.
- `-- requires:`, `-- layer:`, `-- exists:`, `-- name:` headers. These
  predate pg-dev-buddy and belong to the File DAG world. Preserved
  verbatim.
- Un-adoption of source files that never had node_id headers (e.g. files
  added after a previous uninstall). These are already in the File DAG
  world; no-op.
- Re-install. Once uninstalled, re-adoption goes through M17's three
  entry paths. M18 does not offer a "resume" path.
- Shadow databases. M6's shadow lifecycle is ephemeral; shadows are dropped
  by their own `topcat shadow clean`, not by `uninstall`.
- Multi-database fleets. M18 targets a single dev db configured in
  `topcat.toml`.

## Deliverables (files and modules)

- `src/commands/uninstall/mod.rs` — CLI handler. Parses args, loads
  settings, wires up: source-file scan → `_topcat` drop plan → preview →
  confirmation → execution.
- `src/commands/uninstall/plan.rs` — `UninstallPlan { files_to_strip:
  Vec<FileStripPlan>, drop_event_triggers: bool, drop_topcat_schema: bool }`
  and the planning logic that walks the input tree, reads each file's
  headers, and records the exact line ranges to remove. Planning is pure;
  no IO side effects except reads.
- `src/commands/uninstall/execute.rs` — applies an `UninstallPlan`: opens
  each file for rewrite (atomic temp-file + rename per file), then calls
  `pg::event_trigger::uninstall` and `DROP SCHEMA _topcat CASCADE` inside a
  single transaction. Ordering: source files first (reversible by git),
  then DDL. Rollback on DDL failure is not possible for file rewrites, so
  emit a clear recovery hint (`git checkout -- <files>` if user regrets).
- `src/commands/uninstall/preview.rs` — dry-run table formatting. Two
  tables: "Source files (headers stripped)" with columns `File`, `Lines
  removed`, `Sample stripped line`, and "Database teardown" listing `DROP
  EVENT TRIGGER _topcat_ddl_end`, `DROP EVENT TRIGGER _topcat_sql_drop`,
  `DROP SCHEMA _topcat CASCADE`. Reuses `comfy_table` patterns from
  `src/commands/clean/common.rs::show_deletion_preview`.
- `src/file_node/parsing.rs` — extend with
  `strip_node_id_header(path, comment_str) -> Result<StripReport>` that
  reads the file, locates every `<comment_str> node_id: ...` header line,
  rewrites the file without those lines atomically (temp file + rename),
  and returns a `StripReport { lines_removed: usize,
  stripped_line_samples: Vec<String> }`. Public, unit-tested. Shares the
  existing `get_file_headers` reader for the scan; does not re-invent
  header detection. Crucially: header detection stops at the first
  non-comment non-empty line (existing semantics), so only the header
  block is touched — in-body `-- node_id:` comments are left alone.
- `src/pg/client.rs` — extend with `drop_topcat_schema(&mut self) ->
  Result<()>` that issues `DROP SCHEMA _topcat CASCADE` inside an explicit
  transaction. Does not set `topcat.bootstrapping = 'true'` — the schema
  is going away, there is nothing for an event trigger to recursively
  capture. (The trigger is dropped first anyway.)
- `src/cli/mod.rs` — register `Uninstall` subcommand. Flags: `--mode`
  (inherits `ExecutionMode` from `src/cli/execution.rs`), `--force`
  (inherits from existing execution args group), plus inherited global
  flags (`--config`, `--log-level`, `--log-format`, `--log-file`).
- `src/main.rs` — route `Commands::Uninstall` to `commands::uninstall::run`.
- `tests/uninstall/roundtrip.rs` — end-to-end: install (M5) → apply some
  DDL so `node_registry` has rows → write node_id headers into source
  files (simulate M12 sync's writeback) → `topcat uninstall --mode execute
  --force` → assert zero `node_id` lines remain, assert other headers
  intact byte-for-byte, assert `_topcat` schema gone, assert both event
  triggers gone.
- `tests/uninstall/preserves_other_headers.rs` — fixture file with all
  five header kinds (`name`, `requires`, `layer`, `exists`, `node_id`);
  assert only `node_id` line is removed, other four preserved with exact
  original whitespace and ordering.
- `tests/uninstall/dry_run.rs` — `--mode dry-run` leaves filesystem and
  database untouched; stdout contains the preview tables; exit code 0.
- `tests/uninstall/confirmation.rs` — without `--force`, a piped "n" at
  the prompt aborts (exit code 2, no state changes); a piped "y" proceeds.
- `tests/uninstall/no_op.rs` — uninstall on a project that never adopted
  pg-dev-buddy (no `_topcat` schema, no node_id headers). Exits 0; no
  errors; preview tables report zero work.

## Implementation phases

1. **Phase 1 — plan + preview (read-only).** Ship
   `src/commands/uninstall/plan.rs` and `preview.rs`, plus the CLI
   registration. The plan module walks the input tree (reusing the
   existing discovery path from `commands/concat`), reads each file's
   headers via `file_node::parsing::get_file_headers`, and records line
   indices matching the `node_id:` prefix. Preview prints the two tables.
   No database connection yet — the plan stubs the DDL side with a literal
   list. Integration tests: dry-run against a fixture project; assert
   exact stdout shape.
2. **Phase 2 — source rewrite primitive.** Add
   `strip_node_id_header` to `src/file_node/parsing.rs`. Atomic rewrite via
   `tempfile::NamedTempFile` in the same directory plus rename. Unit tests
   in `src/file_node/tests.rs`: every-header-kind fixture, trailing
   newline preservation, CRLF preservation, file with only one `node_id`
   line, file with `node_id` plus other headers in varying orders. Then
   wire execute-path file rewrites in `commands/uninstall/execute.rs`;
   still no DDL yet.
3. **Phase 3 — DDL teardown.** Implement `drop_topcat_schema` on
   `pg::client` and wire the execute path to: (1) call
   `pg::event_trigger::uninstall`, (2) call `client.drop_topcat_schema`,
   each in its own transaction so a failure in step 2 doesn't leave the
   triggers alive referencing a gone schema. Add roundtrip integration
   test against ephemeral pg. Confirmation-prompt plumbing (reusing
   `Logger::prompt` from `commands/clean/common.rs`) lands here.
4. **Phase 4 — polish + error paths.** No-op case (nothing to do, exit
   cleanly, empty preview tables are ok), partial-failure recovery hints
   (if N/M files stripped then DDL fails, report which files were
   rewritten and suggest `git checkout --` for rollback), `--force`
   coverage. Final verification-snapshot walkthrough from a clean checkout
   to confirm the command is pleasant to run.

## Risks (copied and amplified)

Architecture §Uninstall and reversibility rates this low-risk because the
mechanics mirror existing topcat cleanup patterns in `src/commands/clean/`.
Concrete day-one mitigations:

- **Accidentally stripping non-header `-- node_id:` mentions.** A user
  might have a literal `-- node_id: 01924...` string in a function body or
  SQL comment past the header block. *Day-one mitigation*: reuse
  `get_file_headers`'s existing contract — it stops at the first
  non-comment non-empty line. `strip_node_id_header` operates only on that
  prefix slice; in-body occurrences are untouched. Unit test explicitly:
  file with `node_id:` in the header *and* inside a function body;
  assert only the header line is removed.
- **Atomic file rewrite interrupted mid-write.** Power loss or kill -9
  between file open and file close leaves a corrupt source file.
  *Day-one mitigation*: write to a sibling temp file, `fsync`, then rename
  over the original (POSIX atomic). Never truncate-in-place. This matches
  the `tempfile` crate's `persist` idiom topcat already uses in
  `header_generator/`.
- **`DROP SCHEMA _topcat CASCADE` hiding user dependencies.** If a user
  has mistakenly created a table in `_topcat` (treating it as "topcat's
  scratch space"), `CASCADE` silently drops their data. *Day-one
  mitigation*: before the drop, `SELECT count(*) FROM pg_class WHERE
  relnamespace = '_topcat'::regnamespace AND relname NOT IN (<known list
  from meta-schema v1>)`. If non-zero, refuse to uninstall without
  `--force` and surface the offending object names. Same protective shape
  as `clean dead-branches` with `--root-pattern`.
- **DDL-side partial failure after source rewrites.** File rewrites
  succeed, `DROP EVENT TRIGGER` succeeds, `DROP SCHEMA` fails (e.g.
  connection dropped). Filesystem and database are now divergent — source
  files have no node_ids but `_topcat` still thinks the adoption is live.
  *Day-one mitigation*: order of operations is source files → event
  trigger drop → schema drop. Log a durable "uninstall_attempt" record
  containing the list of stripped files before touching the db. On DDL
  failure, the error message enumerates: (a) which files were rewritten
  (`git checkout -- <files>` to roll back); (b) which DDL steps
  succeeded; (c) the exact failing statement. Resumable: re-running
  `topcat uninstall` on a half-uninstalled state is a no-op for already-
  stripped files and proceeds with remaining DDL.
- **Force flag misuse dropping a still-wanted `_topcat`.** A user in a
  multi-project monorepo might run `topcat uninstall --force` in the
  wrong directory and wipe the shared dev db's meta-schema. *Day-one
  mitigation*: the db url is resolved from `topcat.toml` + env, not
  inferred. The preview tables always print the target db url prominently
  in Phase 1. `--force` skips only the `y/N` prompt, not the
  dependency-safety check from the `DROP SCHEMA CASCADE` risk above.
- **Header-strip leaving an empty header block with a stray blank
  separator.** If a file's only header was `-- node_id:`, stripping it
  leaves a leading blank line before the body. *Day-one mitigation*:
  after strip, if the resulting header block is empty, also consume the
  trailing blank separator line so the file body starts at line 1. Unit
  test for this exact shape.

## Tests

All integration tests live under `tests/uninstall/` and run against an
ephemeral Postgres (superuser) wired via the M1 harness. Each test spins
up a fresh db, runs M1 bootstrap, runs M5 install, simulates `sync`
writeback by placing fixture source files with known node_id headers,
then exercises the scenario under test. Unit tests for the header-strip
primitive live under `src/file_node/tests.rs` alongside existing parsing
tests.

- **`roundtrip.rs`** — full install → uninstall cycle. Fixture project
  with 5 source files, 3 of which have `node_id` headers. After
  `uninstall --mode execute --force`: all 3 files show zero `node_id`
  lines (grep-level assertion), byte-for-byte equality on every other
  header line compared against the pre-sync fixture, `SELECT 1 FROM
  pg_namespace WHERE nspname = '_topcat'` returns zero rows, `SELECT
  evtname FROM pg_event_trigger WHERE evtname LIKE '_topcat_%'` returns
  zero rows.
- **`preserves_other_headers.rs`** — fixture file with all five header
  kinds, varied orderings (`node_id` first, last, middle), varied spacing
  (single space after colon, tab, multiple spaces). After strip: only
  `node_id` line removed, all other lines preserved exactly, ordering of
  remaining lines unchanged.
- **`dry_run.rs`** — `--mode dry-run` on a fully-adopted project.
  Filesystem diff: zero changes (hash every file before/after). Database
  state: `_topcat.schema_version` still equals v1, event triggers still
  `ENABLED`. Stdout contains the two preview tables.
- **`confirmation.rs`** — without `--force`, piped `n\n` aborts with
  exit code 2 and zero state changes; piped `y\n` proceeds to completion.
  Piped empty input (just newline) treated as `n`.
- **`no_op.rs`** — uninstall on a project that never adopted
  pg-dev-buddy. Exits 0. Preview tables report zero files to strip and
  list the DDL statements as `(no-op: _topcat schema absent)`. No error.
- **`partial_failure_recovery.rs`** — inject a `DROP SCHEMA` failure
  (e.g. revoke the superuser role between phases). Source files were
  already rewritten. Error message enumerates the rewritten file list.
  Re-running `uninstall` completes the DDL side without re-touching the
  already-stripped files.
- **`user_object_in_topcat_schema.rs`** — pathological case. User
  created `_topcat.my_table` by mistake. Uninstall without `--force`
  refuses and lists `my_table`. Uninstall with `--force` proceeds and
  drops it (with a loud warning line in stderr).
- **`header_strip_unit.rs`** (under `src/file_node/tests.rs`) — every
  whitespace variant of the node_id header line; CRLF vs LF; only-header
  file; no-header file; multiple `node_id:` lines in one header block
  (pathological but possible after a bad manual edit — all removed).

## Verification snapshot

```bash
# Prereq: M1 meta-schema + M5 event triggers installed; M12 sync has run
# at least once so source files carry -- node_id: headers.
topcat --config topcat.toml meta-schema upgrade
topcat --config topcat.toml sync

# Confirm adoption state before uninstall
psql "$DEV_DB_URL" -c "SELECT count(*) FROM _topcat.node_registry;"
grep -rc '^-- node_id:' sql/ | head

# Dry-run preview (default mode)
topcat uninstall                                  # (new)

# Actual teardown, still with confirmation
topcat uninstall --mode execute                   # (new)

# Non-interactive variant for CI / scripted rollback
topcat uninstall --mode execute --force           # (new)

# Post-uninstall assertions
grep -rc '^-- node_id:' sql/ | awk -F: '$2 > 0'   # expect zero output
psql "$DEV_DB_URL" -c "SELECT 1 FROM pg_namespace WHERE nspname='_topcat';"
psql "$DEV_DB_URL" -c "SELECT evtname FROM pg_event_trigger
                       WHERE evtname IN ('_topcat_ddl_end','_topcat_sql_drop');"

# Pre-existing File DAG headers still work
topcat concat -i sql/ -e sql /tmp/out.sql && head -5 /tmp/out.sql
```

## Open questions to resolve before starting

- **`--force` semantics around user objects in `_topcat`.** Should
  `--force` override the safety check for non-topcat objects in the
  `_topcat` schema? Option A: `--force` skips only the y/N prompt; the
  safety check always blocks. Option B: `--force` overrides both.
  Recommendation: **Option A**; require an explicit
  `--allow-user-data-in-topcat-schema` for Option B. Decide before Phase 3.
- **Per-input-root uninstall vs. whole-project uninstall.** If
  `topcat.toml` declares multiple input roots, does `uninstall` strip
  headers from all of them or from a filtered subset? Architecture says
  "every source file with `-- node_id:` headers" (whole project).
  Recommendation: follow the architecture; no partial uninstall in v1.
  Flagged here because M17 may want a `--scope <root>` flag eventually —
  do not build that into M18.
- **DDL transaction shape.** Two choices: (A) one transaction for
  `DROP EVENT TRIGGER … ; DROP EVENT TRIGGER … ; DROP SCHEMA _topcat
  CASCADE;`, or (B) separate transactions so each step is independently
  durable and retry-safe. Recommendation: **B**, with an explicit
  re-entrancy test in `partial_failure_recovery.rs`. Matches M5's
  pattern (install is multi-transactional by design). Decide before
  Phase 3.
- **Exit code for user-cancelled at confirmation.** Literature varies:
  some tools use 1, some 2, some 130 (SIGINT-adjacent). Topcat's
  existing `clean` commands use 0 for "cancelled" (documented in
  `commands/clean/common.rs::perform_deletion`). Recommendation:
  **match existing `clean` pattern → exit 0**, even though cancellation
  is not success. Consistency beats theoretical correctness. Lock
  before Phase 3.
- **Should migration files on disk be scanned too?** Architecture says
  migration files "remain as plain SQL; no topcat runtime dependency" —
  implying don't touch them. But they may contain `node_id` references
  in comments (e.g. `-- add: <node_id> table orders`). Recommendation:
  **don't touch migration files**; the architecture is explicit.
  Confirmed here to avoid scope creep in Phase 1.

## Session sizing notes

~1 session. Phases 1 + 2 are small (plan + preview + source rewrite
primitive, all local). Phase 3 adds two short DDL routines and the
roundtrip integration test. Phase 4 is polish. If the session lands
Phases 1–3 clean, Phase 4 rolls into the same session without
difficulty. Do not expand into M17 bootstrap flows or M16 reconcile
logic — both feel adjacent ("what if uninstall detected drift first?")
and both belong to their own milestones.

## Referenced architecture sections

- `docs/pg-dev-buddy-architecture.md` §Uninstall and reversibility —
  the five-step teardown and the preserved-header policy this plan
  implements verbatim.
- `docs/pg-dev-buddy-architecture.md` §Event trigger — install /
  uninstall API surface M18 calls; bootstrap flag discipline (relevant
  because the DDL teardown happens outside the bootstrap window).
- `docs/pg-dev-buddy-architecture.md` §Identity and node_id — why the
  `-- node_id:` header is the artifact that must be stripped, and why
  other File DAG headers are orthogonal.
- `docs/pg-dev-buddy-architecture.md` §`_topcat` meta-schema versioning
  — enumerates the tables, functions, and trigger bodies that
  `DROP SCHEMA _topcat CASCADE` takes down in one shot.
- `docs/pg-dev-buddy-architecture.md` §Bootstrapping — the three entry
  flows M17 implements; M18 is their shared exit.
- `docs/pg-dev-buddy-architecture.md` §CLI surface — `topcat uninstall`
  slot in the documented command set.
- `docs/pg-dev-buddy-roadmap.md` §M18 — scope, deliverables, risks,
  size (source of truth this plan expands). Confirms parallelizability
  with M15, M16, M17.

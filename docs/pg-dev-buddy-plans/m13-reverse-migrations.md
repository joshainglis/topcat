# M13 — reverse migrations + registry + data_restore bounces

> Every forward migration produced by the generator gains a companion
> reverse migration, persisted alongside the forward in
> `_topcat.migration_registry`, round-trip verified against the local
> shadows, and applyable via `topcat migrate apply-reverse
> <forward_migration_id>`. Changes that drop information produce reverses
> whose restoration statements are `kind=data_restore` placeholders; the
> generator fails closed if it would emit a silent-loss reverse.

## Context

M9 shipped the forward-only `migration_generator`. Forward-only migrations
are one-way; any dev-loop rollback today has to be hand-rolled, which
defeats the whole reversibility claim of the system and is the single
biggest gap between the M12 MVP and a usable dev tool.

M13 closes that gap. It teaches the generator to invert a ChangeSet, emits
a second migration artifact per forward, extends `migration_verifier` with
the reverse round-trip defined in the architecture, persists both artifacts
to the now-live `_topcat.migration_registry`, and adds a CLI verb so the
operator can actually apply a reverse during local development.

It is explicitly **not** a production rollback tool: `topcat` never applies
reverses automatically and reverses exist only for deliberate operator
action against a dev database. General (non-`data_restore`) bounces are
deferred to M14; split/merge refactors to M15.

Architecture authority for everything below:
`docs/pg-dev-buddy-architecture.md` §Reverse migrations and §Migration
verification (steps 5–7).

## Prerequisites

Hard dependencies (all already on `main`):

- **M9 — `migration_generator`**: provides the `ChangeSet` type, the
  topological walk, the multi-section emitter (`pre_begin` /
  `transaction` / `post_commit`), and the migration file header with
  `migration_id`, `parents`, `is_reverse`, `forward_id`,
  `requires_input`, `changes_summary`. M13 extends this module rather
  than forking it.
- **M10 — `migration_verifier`**: provides the shadow clone, forward
  apply, and structural-equality check (steps 1–4 of architecture
  §Migration verification). M13 adds steps 5–7.
- **M5 — `_topcat` meta-schema and registry**: provides live
  `_topcat.migration_registry` with columns `migration_id`,
  `parent_ids`, `is_reverse`, `forward_id`, `changeset`, `ddl_text`,
  `pg_version`, `extension_versions`, `requires_input`, plus the
  `CHECK (is_reverse = (forward_id IS NOT NULL))` constraint. M13
  persists both forward and reverse rows using these columns.

Soft dependencies:

- `pg/ddl_emitter` (M3) — unchanged, reused for reverse DDL rendering.
- `schema_model` — both `before_model` and `after_model` must already be
  captured on every `ChangeSet` entry (M8 contract); if they aren't,
  that's a prerequisite bug, not M13 scope.

## Scope

### In scope

- Pure-function inversion of a `ChangeSet` into a `ReverseChangeSet`.
- Reuse of the M9 generator to render the reverse as a multi-section
  migration file using the same section-routing rules.
- `kind=data_restore` bounce markers on reverses for the exhaustive list
  of information-losing forward change kinds.
- Hashing the forward's `migration_id` into the reverse's ChangeSet
  header so the reverse is globally distinguishable even when its DDL
  collides byte-for-byte with an unrelated forward's DDL.
- Persistence of both artifacts to `_topcat.migration_registry` in a
  single transaction, with `is_reverse`/`forward_id` populated
  correctly.
- Extension of `migration_verifier` with the reverse round-trip (steps
  5–7), including the `data_restore` exclusion rule on the structural
  equality check.
- `topcat migrate apply-reverse <forward_migration_id>` CLI verb that
  looks up the reverse by `forward_id`, rejects reverses whose
  `requires_input=true` unless ack'd (ack mechanism is M14; M13
  short-circuits to "refuse with a clear message" when any
  `data_restore` marker is present without an ack file).

### Out of scope (deferred)

- General bounce taxonomy beyond `data_restore` (M14).
- The ack grammar, templater, and runner-side ack validator (M14).
- Split/merge source-file annotations and the data-move DDL they emit
  (M15).
- Production rollback automation — reverses are operator-applied only.
- Parent/branch-merge migration wiring in the reverse's header beyond
  `forward_id` (M13 leaves `parents` on the reverse equal to the
  forward's `migration_id`; richer parentage is M16's concern).

## Deliverables (files and modules)

Paths below are proposed — align with existing `migration_generator`
layout chosen in M9. Create files only if they don't already exist;
prefer extending M9 modules.

**Extended modules:**

- `src/migration_generator/mod.rs` — adds `generate_forward_and_reverse`
  entry point (returns a pair), alongside the existing forward-only
  entry point from M9.
- `src/migration_generator/inverter.rs` (new) — the `ChangeSet`
  inversion logic, one function per `change_kind`:
  - `Add ↔ Drop` — swap sides using the `before_model` /
    `after_model` captured on the ChangeSet entry.
  - `Rename` — swap `from`/`to`.
  - `Alter` — swap `old_value`/`new_value`.
  - `PartitionAttach ↔ PartitionDetach`.
  - `SplitExtract ↔ merge` (M13 emits the merge DDL; the refactor
    annotation path is M15, but the inverse primitive lives here).
  - `MergeAbsorb ↔ split` (symmetric).
- `src/migration_generator/data_loss.rs` (new) — enumerates the
  information-losing forward kinds and emits the corresponding
  `kind=data_restore` bounce record with `placeholder_statement=true`.
  Exhaustive list per architecture §Reverse migrations §Data-loss
  cases: drop of populated column, drop of table with rows, lossy cast
  column type change. Add: drop of populated schema, drop of view that
  a materialized view depended on for seed data (follow-up if scope
  bleeds), drop of sequence whose nextval was consumed.
- `src/migration_generator/header.rs` — extend the migration file
  header builder to set `is_reverse=true`, `forward_id=<forward hash>`,
  and to include the forward's `migration_id` in the canonical JSON
  ChangeSet header so the reverse's content hash is distinct.
- `src/migration_verifier/mod.rs` — add `verify_reverse` implementing
  architecture §Migration verification steps 5–7. Reuses the clone
  already at post-forward state from the forward verification;
  `data_restore` node_ids are excluded from the equality check.
- `src/migration_registry/writer.rs` — single-transaction writer that
  inserts the forward row and the reverse row. Uses the live `M5`
  schema; no DDL changes to `_topcat.migration_registry`.
- `src/cli/migrate.rs` — add the `apply-reverse <forward_migration_id>`
  subcommand; looks up the reverse row, streams it to `pg/client` for
  application.

**New tests** (paths under `tests/` matching project convention):

- `tests/migration_generator/inversion_roundtrip.rs` — property tests.
- `tests/migration_verifier/reverse_roundtrip.rs` — end-to-end round
  trip against shadow databases.
- `tests/cli/migrate_apply_reverse.rs` — CLI smoke.

## Implementation phases

Three phases, with a natural session boundary between phase 2 and phase 3.

**Phase 1 — inversion engine (no IO).**

1. Define `ReverseChangeSet` (thin wrapper around `ChangeSet` with the
   forward's `migration_id` embedded for hashing).
2. Implement per-`change_kind` inversion in `inverter.rs`. One function
   per kind, exhaustive `match` on `ChangeKind` so the compiler
   catches un-inverted kinds.
3. Implement the fail-closed rule in `data_loss.rs`: if a forward kind
   is on the information-losing list, the inverter MUST emit a
   `kind=data_restore` bounce — unit test that asserts the generator
   panics (or returns a typed error) if any data-loss kind reaches the
   reverse renderer with zero bounces.
4. Re-run the M9 generator on the inverted ChangeSet to produce the
   reverse migration file; verify section routing still works.

**Phase 2 — verification and registry.**

1. Extend `migration_verifier` with steps 5–7. Exclude nodes touched by
   `data_restore` bounces from the equality check.
2. Wire both artifacts into a single-transaction write to
   `_topcat.migration_registry`. Enforce the
   `is_reverse = (forward_id IS NOT NULL)` invariant at the writer
   level (belt-and-braces with the DB check).
3. `topcat migrate generate` now produces both artifacts unless
   `--no-reverse` is passed (the flag already exists on the CLI
   surface per architecture §CLI surface).

**Phase 3 — CLI and round-trip guards.**

1. Implement `topcat migrate apply-reverse <forward_migration_id>`:
   lookup by `forward_id`, reject if the reverse has any unacked
   `data_restore` bounce, apply via `pg/client` in a single tx wrapper
   matching the migration's sections.
2. Add the migration-id hash collision test: synthesize a forward and
   a reverse whose raw DDL bodies are identical, assert their
   `migration_id` values differ because the forward's id was hashed
   into the reverse's ChangeSet header.
3. Run the full round-trip property test across every `change_kind`.

## Risks (copied and amplified)

- **`data_restore` bounces are the only silent-loss surface in the
  whole system.** A forward that drops a populated column, paired with
  a reverse that silently re-adds the column without a restoration
  placeholder, permanently erases the operator's data on round-trip
  with no diagnostic. **Day-one mitigation**: the inverter treats the
  information-losing `change_kind` list as an exhaustive allow-list,
  not a deny-list — if a reverse renderer is handed a data-loss
  forward and the accumulated bounce-record count is zero, it
  returns `Err(GeneratorError::SilentDataLoss { kind, node_id })`
  rather than writing anything. Unit test the panic/error for every
  listed kind. Second layer of defense: `verify_reverse` fails if the
  excluded-node set is non-empty but the reverse file contains zero
  `kind=data_restore` markers.
- **Hash collision between reverse DDL and unrelated forward DDL.**
  Two different migrations producing byte-identical DDL would collide
  at the content-hash layer and the second write would fail the
  registry's primary key, silently masking the bug. **Day-one
  mitigation**: the reverse's canonical-JSON ChangeSet header
  includes a `reverse_of: <forward_migration_id>` field. This is
  included in the hash, so the reverse is always distinct from any
  forward even when the rendered DDL matches. Add a regression test
  that constructs a synthetic forward and reverse with identical DDL
  bodies and asserts `forward.migration_id != reverse.migration_id`.
- **Reverse verification drift.** The reverse round-trip (steps 5–7)
  runs on the same shadow clone used for the forward verification
  (step 2). If step 5 forgets to confirm the clone is at the
  post-forward state, the reverse is effectively verified against
  `shadow_head` and step 6 becomes a no-op. **Day-one mitigation**:
  step 5 asserts the clone's catalog hash equals
  `shadow_head`'s before applying the reverse; test by deliberately
  starting from `shadow_main` and asserting the verifier rejects the
  run.

## Tests

- **Unit, per `change_kind`**: property test `invert(invert(cs)) == cs`
  modulo `data_restore` substitution. Every `ChangeKind` variant has a
  fixture; compiler-exhaustive match means adding a variant without a
  fixture fails the suite.
- **End-to-end round trip** (integration, against the shadow
  databases):
  1. Apply forward to `shadow_verify`.
  2. Apply reverse to the post-forward state.
  3. Read catalog, diff against `shadow_main`.
  4. Assert: empty diff except at node_ids covered by `data_restore`
     bounces.
- **Data-loss fail-closed**: for every entry in the information-losing
  `change_kind` list, assert the generator refuses to emit a reverse
  without at least one `kind=data_restore` bounce record.
- **Hash distinctness regression**: byte-identical DDL, different
  migration_ids.
- **CLI**: `topcat migrate apply-reverse` refuses on unacked
  `requires_input=true` reverses (exit non-zero, clear error);
  succeeds and applies on clean reverses; 404-equivalent when
  `forward_migration_id` has no registered reverse.
- **Golden migration files**: add forward+reverse pairs to the M9
  golden-fixture directory so future emitter changes surface as
  diffable artifacts.

## Verification snapshot

CLI surface delta after M13 (relative to architecture §CLI surface):

- `topcat migrate generate [--no-reverse] [--no-verify]` — unchanged;
  `--no-reverse` is now wired (was a placeholder in M9).
- `topcat migrate verify <migration_id>` — unchanged shape; body now
  runs steps 1–7 if a reverse is registered, 1–4 if not.
- `topcat migrate apply-reverse <forward_migration_id>` **(new)** —
  applies the companion reverse locally. Refuses on unacked
  `data_restore` bounces with a message pointing to `topcat migrate
  ack` (the command itself lands in M14; M13 leaves the message
  stub).
- `topcat migrate list [--since <id>] [--format text|json]` — unchanged
  signature; the JSON now surfaces `is_reverse` and `forward_id`
  columns so operators can see reverse rows.

Registry content after M13: every forward row in
`_topcat.migration_registry` has a matching reverse row with
`is_reverse=true` and `forward_id` pointing at the forward.

## Open questions to resolve before starting

1. Does `ChangeSet` as shipped by M8 carry both `before_model` and
   `after_model` on every entry, including `Alter`? If only one side is
   present for some kinds, the inverter can't reconstruct the reverse
   without walking the shadow catalog — which would couple the
   generator to `pg/client` in a way M9 avoided. Confirm before
   Phase 1.
2. Where does `SplitExtract`/`MergeAbsorb` live in the M8 ChangeSet
   enum today? If M8 left them as `TODO`/`unimplemented!()` kinds,
   M13 adds the inversion primitives but the forward side stays
   un-emittable until M15. Document that gap in the plan's "out of
   scope" summary rather than silently shipping half-wired code.
3. `apply-reverse` transaction boundary: reuse the forward runner's
   per-section handling (pre_begin / transaction / post_commit) as-is,
   or add a reverse-specific wrapper? Default assumption: reuse.
4. Should `--no-reverse` on `topcat migrate generate` still write the
   forward row to the registry? Default assumption: yes; the forward
   is durable, the reverse is optional.
5. What's the on-disk layout for the two files? Architecture shows a
   single migration_id-named file. If we're writing forward + reverse,
   either two files (`<id>.forward.sql`, `<id>.reverse.sql`) or one
   multi-document file. M14's `<migration_id>.ack.sql` sidecar
   convention argues for two files named by forward_id with suffixes.
   Pick before Phase 2 to avoid ack-path rework.

## Session sizing notes

Estimated ~2 sessions, matching the roadmap. Natural break between
Phase 2 (verifier + registry plumbing) and Phase 3 (CLI + the two
regression tests). If Phase 1's `ChangeSet` inversion surfaces an
M8-side gap (open question 1 or 2 above), treat that as a scope event
and raise before starting Phase 2 rather than extending the plan's
session count.

## Referenced architecture sections

All paths relative to `docs/pg-dev-buddy-architecture.md`:

- §Reverse migrations (line ~598)
- §Reverse migrations §Generation (line ~604)
- §Reverse migrations §Data-loss cases (line ~617)
- §Reverse migrations §Storage and application (line ~629)
- §Migration verification, steps 5–7 (lines ~640–658)
- §Migration file format (line ~495)
- §Bounce markers (line ~543) — used only for the `kind=data_restore`
  record shape in M13; general taxonomy is M14.
- §Content-hash stability (line ~573) — for the
  `reverse_of: <forward_migration_id>` hash-header contribution.
- §`_topcat` schema DDL (version 1) — `migration_registry` table
  definition (line ~1008).
- §CLI surface — `topcat migrate apply-reverse` (line ~1118).
- §Component map — `migration_generator` and `migration_verifier`
  responsibilities (lines ~1147–1148).

Roadmap cross-reference: `docs/pg-dev-buddy-roadmap.md` §M13 (line 514).

# M10 — migration_verifier

> Prove that each generated forward migration lands `shadow_main` at
> `shadow_head`. This milestone is the correctness claim of the whole
> pg-dev-buddy system: if `migration_verifier` is wrong, every later
> correctness bet (sync, reverse, merge) rests on sand.

## Context

`migration_generator` (M9) emits forward migration SQL from a `ChangeSet`.
Nothing in M1–M9 proves that executing that SQL actually transforms
`shadow_main` into the state captured by `shadow_head`. M10 closes that
loop.

Verification is a round-trip check between two *local* shadow databases:
clone `shadow_main` into a fresh `shadow_verify`, apply the candidate
forward migration, read the catalog, and assert structural equality with
`shadow_head`. On mismatch, topcat refuses to persist the migration file
and emits a human-readable diff report.

Per architecture §Migration verification, this proves equivalence between
two local shadow states. It does **not** prove the migration will succeed
against staging or production — those may have drifted. Verification is
necessary but not sufficient, and that boundary must be documented on
every diagnostic this milestone produces.

Verification runs by default on every `topcat migrate generate`. The
escape hatch is `--no-verify` (architecture §CLI surface). M10 owns the
cost budget that keeps the default on.

## Prerequisites

Hard blockers:

- **M2 — `schema_model` + `pg/catalog_reader`**. Verification reads the
  post-apply catalog and the reference catalog; both must normalize to the
  same `SchemaModel` shape or equality is meaningless.
- **M3 — `pg/ddl_emitter` + round-trip property tests**. The round-trip
  invariant (`apply → read → re-emit → apply → read` is stable) is what
  makes structural equality a legitimate oracle.
- **M6 — `shadow_db` lifecycle**. M10 consumes, does not build:
  - `shadow_db::clone_from_template(source, target)` — the
    `CREATE DATABASE target TEMPLATE source` path used to materialize
    `shadow_verify` from `shadow_main`.
  - `shadow_db::drop(name)` — teardown of the ephemeral `shadow_verify`
    on success or failure.
  - `shadow_db::connection(role)` — superuser-free routing to the shadow
    cluster (architecture §Config surface: separate from dev db).
  - `topcat shadow build verify [--rebuild]` — already ships in M6;
    M10 calls the same code path for ad-hoc rebuilds.
- **M8 — `differ`**. M10 diffs two `SchemaModel` values; the
  `differ::structural_diff(&SchemaModel, &SchemaModel) -> StructuralDiff`
  API (or equivalent) is reused so the mismatch report is identical in
  shape to the one `topcat diff` produces. No second differ.
- **M9 — `migration_generator`**. M10 is invoked by the generator *before*
  the migration file is written to disk. Concrete surface assumed:
  - `migration_generator::Candidate { migration_id, sections, changes_summary, .. }`
    — the in-memory migration value before `write_to_disk`.
  - A hook point in the generator that, when verification fails, returns
    the candidate plus the `VerificationFailure` to the caller instead of
    calling `write_to_disk`.

Soft blockers (not required for M10 to compile, but required for the
experience to match the roadmap):

- **M5 — event trigger on shadows** is expected to be installed by M6's
  shadow-build path. M10 does not write to the registry during verify, but
  the trigger firing on apply is part of the scenario under test.

## Scope

### In scope

- A new `src/migration_verifier/` module, green-field.
- The round-trip algorithm for the **forward** migration only (architecture
  §Migration verification steps 1–4).
- Diff report: structured (`VerificationFailure`) plus a human-readable
  rendering that names the mismatching `node_id`s and fields.
- The `topcat migrate verify <migration_id>` CLI path (architecture §CLI
  surface line 1114).
- Integration into `topcat migrate generate` as a pre-persist gate. The
  existing `--no-verify` flag (declared in M9) becomes meaningful here.
- A default-on cost budget: verification for a 1,000-object schema must
  complete in < 10s (architecture §Performance, `topcat migrate generate`
  row). Measure on day one.

### Out of scope

- Reverse-migration round-trip (steps 5–7 of architecture §Migration
  verification). Lands with the reverse generator milestone.
- Bounce-migration verification (`data_restore` exclusion rules). Same
  reason.
- Remote/staging verification. Architecture explicitly declares this
  non-goal in §Migration verification "Scope".
- Performance-regression harness across pg versions. M10 measures on the
  project's default pg version; the multi-version matrix is M-future.

### Explicitly deferred

- Registering successful verifications in `_topcat.migration_registry`.
  M10 produces an in-memory receipt; the registry write lands with M13.

## Deliverables (files and modules)

New files:

- `src/migration_verifier/mod.rs` — public API:
  - `pub fn verify_forward(candidate: &Candidate, ctx: &VerifyContext) -> Result<VerifyReceipt, VerificationFailure>`
  - `pub struct VerifyContext { shadow_main: DbName, shadow_head: DbName, shadow_cluster: ConnInfo, budget: Duration }`
  - `pub struct VerifyReceipt { migration_id, elapsed, objects_compared }`
  - `pub struct VerificationFailure { migration_id, elapsed, diff: StructuralDiff, stage: VerifyStage }`
  - `pub enum VerifyStage { Clone, Apply, Read, Compare }` — so the
    diagnostic names which step blew up, not just "failed".
- `src/migration_verifier/round_trip.rs` — the 4-step algorithm; pure
  orchestration, no SQL text.
- `src/migration_verifier/diff_report.rs` — renders `StructuralDiff` into
  a grouped, per-`node_id` human text block and a JSON variant for
  `--log-format json`.
- `src/migration_verifier/budget.rs` — wall-clock budget enforcement.
  Aborts with `VerifyStage::Clone|Apply|Read|Compare` and a
  `budget_exceeded=true` tag when the timer trips.
- `src/migration_verifier/tests/` — integration tests (see §Tests).

Modified files:

- `src/commands/migrate/mod.rs` (or wherever M9 lands the `migrate`
  subcommand dispatch): add the `verify` subcommand; wire `--no-verify`
  on `migrate generate` to the verifier call site.
- `src/commands/migrate/generate.rs`: on successful `Candidate` produced
  by `migration_generator`, call `verify_forward` before `write_to_disk`.
  On `VerificationFailure`, bail with exit code 1, render the diff
  report, do not touch the filesystem.
- `src/cli/` (global arg groups): no new groups expected; reuse the
  existing shadow-connection group from M6.
- `docs/pg-dev-buddy-architecture.md`: no doc change required — M10
  implements §Migration verification as already written.

## Implementation phases

Roughly 2 sessions split across 4 phases:

**Phase 1 — scaffolding and happy path.** Create the module, the public
types, and a `verify_forward` that clones `shadow_main → shadow_verify`,
applies the candidate migration via `pg/client`, reads the catalog via
`pg/catalog_reader`, compares via `differ`. Golden-path integration test
on a trivial `Add` ChangeSet. No budget yet, no diff report yet.

**Phase 2 — failure path and diff report.** Deliberately mis-apply (inject
a bad DDL into the candidate SQL), verify the comparison fires, and
implement `diff_report.rs`. Ensure the failure path tears down
`shadow_verify` and exits non-zero.

**Phase 3 — budget + CLI wiring.** Add the wall-clock budget. Wire
`topcat migrate verify <migration_id>` (reads the migration file, uses
its parent as `shadow_main`-equivalent, its target as `shadow_head`).
Wire the pre-persist gate into `topcat migrate generate`; ensure
`--no-verify` cleanly bypasses.

**Phase 4 — regression-injection tests.** Add the known-bad emitter
regressions required by the roadmap (see §Tests). Measure the 1,000-object
cost and record in the PR description; if > 10s, file a follow-up per
architecture §Performance.

## Risks (copied and amplified)

- **This is the correctness claim of the whole system.** If
  `migration_verifier` is wrong, every downstream correctness property
  collapses silently. **Day-one mitigation:** every test case must be
  bidirectional — assert pass on the good case *and* assert refuse on a
  deliberately-broken variant of the same fixture. No one-sided tests.
- **Default-on cost budget (< 10s for 1000 objects).** Verification runs
  on every `topcat migrate generate` unless `--no-verify`. If the cost
  blows the budget, the roadmap §Performance order needs revision — users
  will disable by default. **Day-one mitigation:** the Phase-1 scaffolding
  must time the four stages separately and log the split; a slow clone
  points at M6 template reuse, a slow compare points at `differ`
  normalization. Don't wait until Phase 4 to notice.
- **Flaky teardown leaves zombie `shadow_verify` databases.** Each run
  creates a fresh database; a panic between clone and drop leaks one.
  **Day-one mitigation:** wrap the clone in a `Drop`-guard struct
  (`ShadowVerifyHandle`) so teardown runs on every exit path, including
  panic unwind. Additionally, `topcat shadow clean` must cover
  `shadow_verify*` names (M6 already promises this — regression-test it).
- **`shadow_main` vs `shadow_head` drift during verification.** If the
  user edits the working tree mid-verify, `shadow_head` could rebuild
  under the verifier's feet. **Day-one mitigation:** snapshot the
  `shadow_head` identity (its content-hash per M6's cache key) at the
  start of verify and re-check at the end; fail with a distinct
  `VerifyStage::Read` + `head_drifted=true` diagnostic if it changed.
- **Equality oracle false positives.** Two catalogs that are
  *observationally* equivalent but textually differ (e.g., search_path
  order, whitespace in function bodies) must not trigger mismatch. This
  is a `SchemaModel` normalization responsibility, not a verifier bug —
  but M10 will surface any normalization gap. **Day-one mitigation:**
  when Phase-4 finds a false positive, fix it in `schema_model`
  normalization with a regression test and a changelog note; never
  paper over in the verifier.
- **Superuser requirements leak into user-facing error.** `CREATE
  DATABASE ... TEMPLATE` plus active-connection-count requirements can
  fail with cryptic pg errors. **Day-one mitigation:** wrap pg errors
  from the clone stage with a `VerifyStage::Clone` context string that
  names the template and the remediation
  ("close other connections to shadow_main").

## Tests

Positive fixtures (every one must pass verification):

- `Add table` with a single column.
- `Add column` to an existing table.
- `Alter type` on a column via a cast-compatible path.
- `Drop view` that depends on another view (cascade order matters).
- `Add function` with a body that depends on a type (M4 body-dep inference).
- A fixture pair covering every `change_kind` M9 supports (excluding the
  fail-closed ones: `SplitExtract`, `MergeAbsorb`, cast-incompatible,
  populated-NOT-NULL).

Negative fixtures (regressions injected into the emitter; verifier must
refuse):

- **Silent column drop.** M3 emitter patched to omit one column from
  `CREATE TABLE`. Verify: fail with a `Table.columns` diff on the
  affected `node_id`.
- **Wrong column type.** M3 patched to emit `int4` where the model says
  `int8`. Verify: fail with `Column.data_type` diff.
- **Missing index.** M3 patched to skip `CREATE INDEX`. Verify: fail
  with `Index` missing on the table.
- **Wrong function body.** M3 patched to emit a stale function body.
  Verify: fail with `Function.prosrc` diff.
- **Swapped constraint direction.** M3 patched to emit the FK backwards.
  Verify: fail with a `Constraint` diff naming the reversed columns.

CLI-level tests:

- `topcat migrate verify <migration_id>` on a registered migration
  produces exit 0 and a receipt.
- `topcat migrate generate` on a ChangeSet with a deliberately-broken
  generator exits non-zero, prints the diff, and leaves no file in
  `migrations/`.
- `topcat migrate generate --no-verify` skips the gate and persists the
  file (regression test: the flag actually works).
- Teardown test: kill -9 the process between clone and drop; subsequent
  `topcat shadow clean` removes the orphaned `shadow_verify*` database.

Performance test:

- 1,000-object fixture (reuse the M6 perf fixture). Assert total
  verification time < 10s on the project-standard CI runner. Record the
  four per-stage timings in the test output for trend tracking.

Unit tests (under `migration_verifier/tests/unit/`):

- `diff_report.rs` snapshot tests for each `StructuralDiff` shape.
- `budget.rs` timing tests (fake clock, not wall clock).

## Verification snapshot

After M10 ships, the CLI surface for migrate looks like:

- `topcat migrate generate [--no-reverse] [--no-verify]` — unchanged flag
  set, but `--no-verify` now has teeth.
- `topcat migrate verify <migration_id>` **(new)** — stand-alone
  verification of a persisted migration against the current shadows.
- `topcat migrate ack <migration_id>` — unchanged (ships with bounces).
- `topcat migrate list [--since <migration_id>] [--format text|json]` —
  unchanged.
- `topcat migrate apply-reverse <forward_migration_id>` — unchanged.

`topcat shadow build verify [--rebuild]` remains an M6 deliverable; M10
does not add shadow commands.

## Open questions to resolve before starting

1. **Where does `--no-verify` actually live in the CLI tree?** M9 declares
   it on `migrate generate`; confirm it is not also re-declared on
   `migrate verify` (that would be contradictory). Expect to remove it
   from any `verify` arg group M9 accidentally added.
2. **Budget source of truth.** Is the 10s budget a `VerifyContext`
   field passed from the caller, or a `Config` field in the project
   `topcat.toml`? Architecture §Performance lists it as a target, not a
   config knob. Decision before Phase 3.
3. **Does `shadow_head` need to be rebuilt inside `verify_forward`, or is
   it assumed already current?** For `migrate generate` the answer is
   "M9 rebuilt it already". For stand-alone `migrate verify <id>`, the
   answer is unclear — does the verifier trust the user's shadow, or
   force-rebuild? Default to "trust, but re-check content hash" per the
   Risks section.
4. **Exit code for verification failure.** Generate-then-fail should
   return a distinct exit code from "CLI misuse" and "pg unreachable" so
   CI can tell them apart. Align with the exit-code table if one exists
   in M1; otherwise, propose one in this milestone's PR.
5. **Diff report redaction.** Should column default values, check
   expressions, etc., be included verbatim in the diff, or redacted
   (they can contain literals that look sensitive)? Default: verbatim in
   local dev, because verification never leaves the machine. Document
   the assumption.
6. **Verification of an `_topcat`-only migration.** Are meta-migrations
   (architecture §`_topcat` meta-schema versioning) subject to this
   verifier, or do they have their own path? Confirm with architecture
   §Migration verification scope — probably out-of-scope for M10.

## Session sizing notes

Estimated ~1–2 sessions.

- Session 1: phases 1 + 2 (scaffold, happy path, failure path, diff
  report). Exit criterion: one positive and one negative fixture test
  pass end-to-end.
- Session 2: phases 3 + 4 (budget, CLI wiring, regression-injection
  fixtures, 1,000-object perf measurement). Exit criterion: the full
  test matrix in §Tests is green and the perf number is recorded.

If session 2 perf measurement blows the budget, expect a short session 3
scoped to the specific hot stage (clone / apply / read / compare) rather
than a blanket optimization pass.

## Referenced architecture sections

- §Migration verification (primary spec — all four forward steps).
- §Shadow database lifecycle (clone / template semantics consumed from M6).
- §CatalogReader, SchemaModel, and DDL emitter (the equality oracle).
- §CLI surface (`topcat migrate verify <migration_id>`,
  `topcat migrate generate [--no-verify]`).
- §Performance and caching (the < 10s budget for 1,000 objects).
- §Testing strategy (property-test round-trip discipline inherited here).
- §Component map (`migration_verifier` row — this milestone's module).
- §Observability (log events `migration.verify.pass`,
  `migration.verify.fail` — emit these from Phase 3 onward).

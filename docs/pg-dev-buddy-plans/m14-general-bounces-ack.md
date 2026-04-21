# M14 — general bounce markers + ack workflow

> Milestone M14 of the pg-dev-buddy roadmap. Generalize the bounce marker
> system beyond the M13 `data_restore` case so the M9 migration generator has
> a safe outlet for every change kind it cannot emit autonomously, and ship
> the complementary ack-file workflow operators use to discharge those
> bounces. Synchronous Rust, no runtime introduced. Self-contained briefing.

## Context

M9 ships a forward-only `migration_generator` that fails-closed on change
kinds it cannot emit deterministically (cast-incompatible, populated
NOT-NULL, undeclared split/merge, and so on). M13 adds reverse migrations
and introduces the first bounce kind — `data_restore` — for reverses of
data-losing forwards. M14 closes the loop by defining the full bounce
taxonomy, the grammar for bounce comments, the sidecar ack-file format, the
`topcat migrate ack` templater, and a reference validator that runners can
call before applying a migration marked `requires_input: true`.

Roadmap §Scope-cut candidates explicitly notes M14 cannot safely be cut:
without it, M9 has no outlet for any non-trivial forward change.
Architecture §Bounce markers and §Data migration taxonomy pin the grammar
and the case-to-kind mapping this plan must implement.

## Prerequisites

Concrete artifacts required before M14 starts:

- **M9 `migration_generator`**: must emit `requires_input: true` headers and
  already know how to refuse on the change kinds this milestone will replace
  with bounces. M14 swaps those refusals for marker emission.
- **M9 `ChangeSet` → multi-section migration file** pipeline (`pre_begin`,
  `transaction`, `post_commit`) — bounce records attach to individual
  statements inside those sections.
- **M13 reverse pipeline** and the existing `data_restore` bounce
  implementation — M14 must extend the same emitter, not fork it.
- **M13 `_topcat.migration_registry`** with `requires_input` column already
  persisted (architecture §Storage and application).
- **M1 UUIDv7 choice** resolved (roadmap §Open questions) — every bounce
  record needs a stable `id=<uuid>` and every ack must match it.
- **Architecture §Bounce markers** (lines 543–571) as the normative spec for
  grammar and runner contract.
- **Architecture §Data migration taxonomy** (lines 783–804) as the normative
  mapping from change case to bounce kind.

## Scope

### In scope

- Seven new bounce kinds: `cast_incompatible`, `backfill`, `destructive`,
  `refactor`, `partition_bounds_complex`, `fk_rewire`, `identity_change`.
- A formal grammar for `-- topcat:bounce key=value` lines with a real parser
  (no regex), including an EBNF the tests pin against.
- Ack file format (`<migration_id>.ack.sql`) with matching
  `-- topcat:ack id=<uuid>` records and replacement DDL for
  `placeholder_statement=true` bounces.
- `topcat migrate ack <migration_id>` CLI subcommand that emits a template
  ack file, idempotently, without clobbering operator edits.
- A reference validator (library + CLI flag) that cross-checks bounces
  against their acks. The runner is a separate concern (see M12/M20); M14
  only defines the contract and ships the reference implementation.
- Golden-file tests: for each bounce kind, a fixture ChangeSet emits a known
  migration, is refused without an ack, and accepted with an ack.

### Out of scope

- The runner itself (lives in M12/M20). M14 ships a reference validator
  callable from Rust, not a standalone migration applier.
- Split/merge annotations (M15) — `fk_rewire` is defined here but the DDL
  side that emits it for split/merge is M15's problem.
- Reconcile / drift handling (M16).
- Any change to `_topcat.migration_registry` schema beyond what M13 already
  persists.

## Deliverables (files and modules)

### New modules (all `(new)`, created in this milestone)

- `src/pg_dev_buddy/migration_generator/bounces/mod.rs` `(new)` — `BounceKind`
  enum, `BounceRecord` struct, `BounceSet` collection; emitter entry point
  called from `migration_generator` during ChangeSet → SQL lowering.
- `src/pg_dev_buddy/migration_generator/bounces/grammar.rs` `(new)` — parser
  and EBNF for bounce comment blocks. Implementation uses `nom` combinators
  over line-tokenized input; the EBNF is checked into a doc comment at the
  top of the file and exercised by a round-trip property test.
- `src/pg_dev_buddy/migration_generator/bounces/emit.rs` `(new)` — per-kind
  emitters that take a `Change` and produce a `BounceRecord` plus a
  (possibly placeholder) SQL statement. One function per kind keeps the
  case-analysis table from architecture §Data migration taxonomy explicit.
- `src/pg_dev_buddy/migration_generator/ack/mod.rs` `(new)` — ack file
  parser, writer, and validator. Validator consumes a migration file and an
  ack file and returns a `Result<(), AckError>` enumerating missing acks,
  extra acks, and unreplaced placeholders.
- `src/pg_dev_buddy/migration_generator/ack/templater.rs` `(new)` — emits a
  fresh `<migration_id>.ack.sql` from a migration that has
  `requires_input: true`. Idempotence rules below.
- `src/commands/migrate/ack.rs` `(new)` — CLI handler for
  `topcat migrate ack <migration_id>`. Thin wrapper around the templater.
- `src/commands/migrate/mod.rs` `(new)` — command router under `migrate`.
  (M13 creates this same module; M14 only adds the `ack` arm if M13 has not
  already.)
- `tests/pg_dev_buddy/bounces/` `(new)` — fixture ChangeSets, expected
  migration files, expected ack templates, and happy/refused-application
  tests for every bounce kind.

### Extend (paths that exist today)

- `docs/pg-dev-buddy-architecture.md` — amend §Bounce markers with the EBNF
  once the parser is written; keep the example block as-is.
- `src/cli/mod.rs` — register the `migrate ack` subcommand in the CLI
  surface. (`src/cli/` exists today as the shared CLI scaffolding.)
- `src/commands/mod.rs` — register the new `migrate` command module.

### Bounce kinds (seven, grounded in architecture §Data migration taxonomy)

| Kind | Triggering case |
|---|---|
| `cast_incompatible` | Cast-incompatible column type change; operator supplies `USING`. |
| `backfill` | NOT NULL added to populated column with no default; verification query included. |
| `destructive` | Column drop of populated column; row count and sample query included. |
| `refactor` | Undeclared table split/merge; operator annotates `-- split_from:` / `-- merge_into:` and re-runs. |
| `partition_bounds_complex` | Partition bound change with overlapping redistribution; verification query included. |
| `fk_rewire` | FK targets rewired by split/merge; reviewable by operator. |
| `identity_change` | PK replaced with different columns; orphaned-FK query included. |

`data_restore` is owned by M13; M14 does not redefine it but the grammar
and validator must accept it so reverse migrations continue to round-trip.

### Bounce grammar

```text
bounce_block ::= bounce_line { bounce_line } statement
bounce_line  ::= "--" ws "topcat:bounce" ws key "=" value newline
key          ::= ident
value        ::= bare_value | quoted_value
bare_value   ::= <chars except whitespace, quote, newline>
quoted_value ::= "\"" <chars except unescaped quote, newline> "\""
```

Consecutive `bounce_line`s with a shared `id=<uuid>` form one record and
scope to the SQL statement immediately following the block. `ident` is
`[a-z_][a-z0-9_]*`. The parser is line-tokenized (a bounce line is a SQL
comment, so `pg_query` cannot help); nom is the chosen combinator library.

### Example marker (from architecture §Bounce markers, verbatim)

```sql
-- topcat:bounce id=<uuid> kind=backfill object=<node_id>
-- topcat:bounce message="Column customers.email added with NOT NULL — backfill required."
-- topcat:bounce verification_query="SELECT count(*) FROM customers WHERE email IS NULL"
-- topcat:bounce placeholder_statement=true
ALTER TABLE customers ADD COLUMN email text NOT NULL;
```

### Ack file format

A sidecar `<migration_id>.ack.sql` sits next to the migration file.
One `-- topcat:ack id=<uuid>` line per bounce record, followed by the
operator's replacement DDL when the bounce had `placeholder_statement=true`,
or empty when it did not. Acks may appear in any order. Unmatched acks and
missing acks are both errors.

```sql
-- topcat:ack id=<uuid-from-bounce>
ALTER TABLE customers ADD COLUMN email text;
UPDATE customers SET email = lower(name) || '@example.invalid';
ALTER TABLE customers ALTER COLUMN email SET NOT NULL;
```

### CLI surface

- `topcat migrate ack <migration_id>` — template an ack file for a
  migration with `requires_input: true`. Writes
  `<migration_dir>/<migration_id>.ack.sql`. See idempotence rules below.
- `topcat migrate ack <migration_id> --stdout` — write the template to
  stdout instead of disk (for piping into editors).
- Reference validator is available via `topcat migrate verify
  <migration_id>` (M10) gaining an `ack` check arm when `requires_input:
  true`; no new top-level command.

## Implementation phases

1. **Grammar and parser.** Write the EBNF, implement the nom parser, and add
   round-trip property tests (`parse(emit(record)) == record`). Freeze the
   grammar before anything else depends on it.
2. **Per-kind emitter wiring.** Replace the M9 fail-closed arms for each of
   the seven kinds with `BounceRecord` emission via
   `bounces::emit::<kind>`. One PR-sized change per kind, each with its
   fixture ChangeSet and golden migration file. `data_restore` is not
   touched; its tests must still pass.
3. **Ack validator.** Build `ack::validate(migration, ack) -> Result<(),
   AckError>`. Exhaustively enumerate error cases (missing ack, extra ack,
   unreplaced placeholder, unknown kind, duplicated id).
4. **Ack templater + CLI.** Implement `topcat migrate ack`; enforce
   idempotence (see risks). Wire into `src/cli/mod.rs` and
   `src/commands/mod.rs`.
5. **Integration tests.** For every bounce kind: generate migration, assert
   refused without ack, template the ack, assert refused with placeholders
   unfilled, fill placeholders, assert accepted. Gate on exit codes.

## Risks (copied and amplified)

- **Grammar ambiguity — use a real parser, not regex.** Regex over SQL
  comments is the wrong tool; mixed quoting, embedded `=`, and multi-line
  records get pathological fast. Day-one mitigation: pin `nom` as the
  parsing library in phase 1, write the EBNF before the code, and ship a
  property test (`parse ∘ emit == id`) that runs on every CI build.
  Alternative (`pest`) considered and rejected because we only need line-level
  combinators and we want to avoid a grammar file out-of-tree.
- **Ack idempotence — don't clobber operator edits.** Re-running
  `topcat migrate ack` on an already-acked migration must not erase the
  operator's replacement DDL. Day-one mitigation: on existing
  `<migration_id>.ack.sql`, the templater *refuses by default* and prints a
  three-way diff between (existing ack, fresh template, migration bounces).
  `--force` overwrites, `--merge` appends missing ack stubs at the end of
  the file without touching existing ones. The refuse-and-diff default is
  the canonical mode; `--merge` is the convenience; `--force` is the escape
  hatch. No silent rewrites.
- **Bounce record / ack record drift.** If the operator edits the migration
  SQL but not the bounce comment block, validator correctness depends on
  the bounce id, not the SQL text. Keep `id=<uuid>` the only join key; do
  not hash statement text.
- **Placeholder detection.** `placeholder_statement=true` must cause the
  validator to check that the ack body is non-empty — otherwise a bare
  `-- topcat:ack id=…` with no DDL under it silently passes. Test this
  explicitly.

## Tests

For every bounce kind (`cast_incompatible`, `backfill`, `destructive`,
`refactor`, `partition_bounds_complex`, `fk_rewire`, `identity_change`):

- **Emission.** Fixture ChangeSet → generated migration file byte-equal to
  a golden file. Golden includes the full bounce record block and the
  surrounding migration header with `requires_input: true`.
- **Refused without ack.** Reference validator returns `Err(MissingAck {
  id })` when the ack file is absent or missing records.
- **Accepted with ack.** Validator returns `Ok(())` when the ack file
  contains matching ids and non-empty bodies for every placeholder.
- **Template idempotence.** Running `topcat migrate ack` twice without
  `--force` leaves the second invocation a no-op (exit 0, diff empty).
  Running with an operator edit in place refuses by default, succeeds with
  `--merge` by appending only the missing ack stubs.
- **Grammar property test.** Round-trip `parse(emit(record)) == record` for
  arbitrary records generated by `proptest`.

Plus `data_restore` regression: M13's tests must still pass unchanged.

## Verification snapshot

```text
$ topcat migrate generate
generated: 2026-04-21T12:00:00Z
migration: 019000a0-0000-7000-0000-000000000001
requires_input: true
  bounces:
    - kind=backfill object=public.customers id=…
    - kind=destructive object=public.legacy_notes id=…
next: topcat migrate ack 019000a0-0000-7000-0000-000000000001   (new)

$ topcat migrate ack 019000a0-0000-7000-0000-000000000001       (new)
wrote: ./migrations/019000a0-0000-7000-0000-000000000001.ack.sql
edit the placeholders, then topcat migrate verify …

$ topcat migrate verify 019000a0-0000-7000-0000-000000000001
ack: OK (2 records, 2 placeholders filled)
shadow verify: OK
```

## Open questions to resolve before starting

1. **Ack file co-location.** Sidecar in the same directory (per architecture
   §Bounce markers) or a parallel `acks/` tree? Default: sidecar; confirm.
2. **`topcat migrate ack` on existing ack.** Refuse-and-diff (this plan's
   default), always-overwrite, or always-merge? Default set to refuse-and-
   diff; confirm before writing tests.
3. **UUID source for `id=<uuid>`.** Reuse the M1 UUIDv7 decision? Yes,
   assuming roadmap §Open questions item 1 is answered; re-confirm at
   milestone start.
4. **`verification_query` execution.** Does the reference validator run
   verification queries against the dev DB, or is execution the runner's job
   exclusively? Default: validator is syntactic only; execution is runner.
5. **Grammar extensibility.** Do unknown keys in a bounce record warn or
   error? Default: warn, forward-compat; confirm.
6. **ACK for non-placeholder bounces.** A `backfill` with a verification
   query but `placeholder_statement=false` still needs operator sign-off —
   is a bare `-- topcat:ack id=…` sufficient, or does the ack need an
   explicit `accepted=true` key? Default: bare ack is sufficient.

## Session sizing notes

~2 sessions, matching roadmap §M14.

- Session 1: phases 1–2 (grammar, parser, per-kind emitters, golden files).
- Session 2: phases 3–5 (ack validator, templater, CLI wiring, integration
  tests, idempotence handling).

One session is the floor only if the grammar sticks the landing on the
first try; assume two.

## Referenced architecture sections

- §Bounce markers (lines 543–571) — normative grammar and runner contract.
- §Migration file format (lines 495–541) — `requires_input` header field
  and section routing (`pre_begin`, `transaction`, `post_commit`).
- §Data migration taxonomy (lines 783–804) — case-to-kind mapping for every
  bounce kind.
- §Reverse migrations (lines 598–638) — `data_restore` precedent and the
  storage model bounces live inside.
- §Component map (lines 1134–1162) — `migration_generator` is the module
  this milestone extends.

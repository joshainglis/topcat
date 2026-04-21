# M0 — Config and command scaffolding

> Source of truth: docs/pg-dev-buddy-roadmap.md §M0, docs/pg-dev-buddy-architecture.md §Config surface, §CLI surface, §Component map.

## Context

M0 lays the purely-additive surface that every later milestone plugs into: a
`[postgres]` config block, `TOPCAT_*` env plumbing, and CLI stubs for every
new subcommand. The roadmap's methodology puts this first because it is
"orchestration only" — no correctness bets are placed here, so later
milestones (M1 `pg/client`, M2 `schema_model`, etc.) can land behind a
stable CLI and config contract without ever churning the parser surface.
Landing this slice now also lets us start wiring integration tests that
assert "not implemented" exit codes, giving every downstream milestone a
built-in green→red signal when it replaces a stub.

## Prerequisites

No prior pg-dev-buddy milestones. Build against the existing topcat crate
as-is:

- `src/settings/mod.rs` — `Settings` struct and `CliOverrides` trait (extend).
- `src/settings/configs.rs` — sibling location for the new `PostgresConfig`.
- `src/settings/loading.rs` — existing `Settings::load` via the `config`
  crate; `TOPCAT_*` env prefix with `__` nested separator is already wired.
- `src/settings/validation.rs` — extend with URL sanity checks.
- `src/cli/mod.rs` — arg-group exports; no new group required but allowed.
- `src/commands/` — green-field subdirectories for every new command.
- `src/main.rs` — `Cli` / `Commands` enum; route new subcommands.
- `topcat.toml.example` — extend with a commented `[postgres]` section.
- `tests/` — existing `assert_cmd` + `predicates` harness pattern shown in
  `tests/cli_concat_tests.rs`.

No new Cargo dependencies are needed for M0. Later milestones will add
`tokio-postgres`, `pg_query`, etc.; do NOT add them here.

## Scope

Roadmap verbatim: *"`[postgres]` block in `topcat.toml`, `TOPCAT_*` env var
plumbing, CLI stubs for every new subcommand (return 'not implemented')."*

Amplification: every subcommand named under architecture §CLI surface gets a
parseable clap entry point. Each stub parses its flags, validates global
args, and returns a `TopCatError::NotImplemented(&'static str)` with a
milestone-reference message (e.g. `"sync will land in M12"`). This keeps
the CLI surface stable and makes later sessions a pure "replace the stub
body" edit.

### In scope for this milestone

- `PostgresConfig` struct under `src/settings/configs.rs`, registered on
  `Settings` and round-tripped through `serde` TOML + env.
- Config precedence already provided by `Settings::load` — verify it works
  for the new block with a unit test.
- New clap subcommands covering every row under architecture §CLI surface
  that is NOT already implemented:
  - `sync`, `shadow` (with nested `build`, `clean`), `diff`,
    `migrate` (with nested `generate`, `verify`, `ack`, `list`,
    `apply-reverse`), `reconcile`, `write-deps`, `pause`, `resume`,
    `uninstall`, `meta-schema` (with nested `upgrade`).
- `TopCatError::NotImplemented(&'static str)` error variant; each stub
  returns it with a milestone tag in the message.
- `topcat.toml.example` gains a fully commented `[postgres]` block.
- `topcat config show` surfaces the new block (automatic — the struct is
  serialized whole; verified by integration test).
- Integration tests assert: (1) each stub exits non-zero with the
  expected message; (2) each stub parses its documented flags without
  error; (3) `config show` contains `[postgres]`.

### Explicitly NOT in scope (deferred to later milestones)

- Any real PostgreSQL I/O — deferred to M1 (`pg/client`).
- `_topcat` meta-schema embedded SQL or migration application — M1.
- URL parsing beyond "is non-empty, is valid `postgres://` or empty
  string" — M1 owns the full URL parser and error taxonomy.
- `--strict` / `--default` / `--permissive` flags (architecture
  §Config surface) — those gate parse-failure policy that does not exist
  until M2+; defer to M2.
- `shadow_main_cache_dir` actually being used — M6.
- `worker_count` actually driving parallelism — M12/M20.
- `[postgres.plugins]` table — M19 owns it.
- Any async runtime or `tokio` in `Cargo.toml` — introduced by M1.

## Deliverables (files and modules)

| Kind        | Path                                            | Status | Description |
|-------------|-------------------------------------------------|--------|-------------|
| new module  | src/settings/configs/postgres.rs                | create | `PostgresConfig` struct (dev_db_url, shadow_db_url, shadow_main_cache_dir, require_superuser, strict_mode, worker_count). Serde defaults; no I/O. |
| extend      | src/settings/configs.rs                         | modify | Re-declare as module root (`pub mod postgres;` + `pub use postgres::PostgresConfig;`) or add struct inline — pick inline if the file stays readable. |
| extend      | src/settings/mod.rs                             | modify | Add `pub postgres: PostgresConfig` field to `Settings`; re-export `PostgresConfig`; include in `Default`. |
| extend      | src/settings/validation.rs                      | modify | Add URL shape check: if `dev_db_url`/`shadow_db_url` set, must start with `postgres://` or `postgresql://`. |
| extend      | src/settings/tests.rs                           | modify | Unit tests for postgres block loading + env-var precedence. |
| extend      | src/exceptions.rs                               | modify | Add `TopCatError::NotImplemented(&'static str)` variant with `Display` impl including the milestone tag. |
| new module  | src/commands/sync.rs                            | create | Stub: flatten `GlobalArgs`; optional `--adopt`, `--force`; returns `NotImplemented("sync lands in M12")`. |
| new module  | src/commands/shadow/mod.rs                      | create | Subcommand parent. |
| new module  | src/commands/shadow/build.rs                    | create | Stub for `shadow build [main\|head\|verify] [--rebuild]`. |
| new module  | src/commands/shadow/clean.rs                    | create | Stub for `shadow clean [--all]`. |
| new module  | src/commands/diff.rs                            | create | Stub for `diff [--from <ref>] [--to <ref>] [--format text\|json]`. |
| new module  | src/commands/migrate/mod.rs                     | create | Subcommand parent. |
| new module  | src/commands/migrate/generate.rs                | create | Stub for `migrate generate [--no-reverse] [--no-verify]`. |
| new module  | src/commands/migrate/verify.rs                  | create | Stub for `migrate verify <migration_id>`. |
| new module  | src/commands/migrate/ack.rs                     | create | Stub for `migrate ack <migration_id>`. |
| new module  | src/commands/migrate/list.rs                    | create | Stub for `migrate list [--since <migration_id>] [--format text\|json]`. |
| new module  | src/commands/migrate/apply_reverse.rs           | create | Stub for `migrate apply-reverse <forward_migration_id>`. |
| new module  | src/commands/reconcile.rs                       | create | Stub for `reconcile [--absorb\|--revert] [--pattern <glob>]`. |
| new module  | src/commands/write_deps.rs                      | create | Stub for `write-deps [--confidence high\|all]`. |
| new module  | src/commands/pause.rs                           | create | Stub for `pause`. |
| new module  | src/commands/resume.rs                          | create | Stub for `resume`. |
| new module  | src/commands/uninstall.rs                       | create | Stub for `uninstall` (distinct from existing `clean`). |
| new module  | src/commands/meta_schema/mod.rs                 | create | Subcommand parent. |
| new module  | src/commands/meta_schema/upgrade.rs             | create | Stub for `meta-schema upgrade [--force]`. |
| extend      | src/commands/mod.rs                             | modify | Register new modules. |
| extend      | src/main.rs                                     | modify | Extend `Commands` enum; route every new subcommand. |
| extend      | topcat.toml.example                             | modify | Append `[postgres]` block with every field documented, all commented out by default. |
| new test    | tests/cli_postgres_stubs_tests.rs               | create | Integration test: every new subcommand prints a `not implemented` message and exits non-zero; help text renders; documented flags parse. |
| new test    | tests/cli_config_postgres_tests.rs              | create | Integration test: `topcat config show` output contains `[postgres]`; env var `TOPCAT_POSTGRES__DEV_DB_URL` overrides config file. |

No Cargo dependency additions. If a future agent reviewing M0 sees the need
for `url` crate validation, defer to M1.

## Implementation phases

### Phase 1 — Config plumbing

**Goal**: `[postgres]` parses from TOML and env; `topcat config show`
displays it; validation runs.

**Tasks**:
- [ ] Create `src/settings/configs/postgres.rs` with `PostgresConfig`:
  - `dev_db_url: Option<String>` (default `None`)
  - `shadow_db_url: Option<String>` (default `None`)
  - `shadow_main_cache_dir: Option<PathBuf>` (default `None`)
  - `require_superuser: bool` (default `true`)
  - `strict_mode: bool` (default `false`)
  - `worker_count: u32` (default `4`)
  - `#[derive(Debug, Clone, Serialize, Deserialize)]`, `#[serde(default)]`,
    `Default` derive.
- [ ] Re-export from `src/settings/configs.rs` and `src/settings/mod.rs`.
  Add `postgres: PostgresConfig` field to `Settings` and wire
  `Default::default()`.
- [ ] Extend `src/settings/validation.rs` with URL prefix check for
  `postgres://` / `postgresql://` when present; empty/`None` is allowed.
- [ ] Extend `topcat.toml.example` with a `[postgres]` block; every field
  commented out. Document the env-var names (nested form:
  `TOPCAT_POSTGRES__DEV_DB_URL`, etc.).
- [ ] Add unit tests in `src/settings/tests.rs` covering load from TOML,
  env-var override, and URL validation failure.

**Verification**:

```bash
cargo build
cargo test --lib settings
cargo run -- config show | grep -F "[postgres]"
TOPCAT_POSTGRES__DEV_DB_URL="postgres://u@h/d" cargo run -- config show | grep -F "dev_db_url"
```

### Phase 2 — Error variant and stub contract

**Goal**: single mechanism for "not implemented" errors with consistent
exit code and message shape.

**Tasks**:
- [ ] Add `TopCatError::NotImplemented(&'static str)` in
  `src/exceptions.rs`. `Display` impl prefixes the message with
  `not yet implemented: ` followed by the stub's static message.
- [ ] Pick a conventional message form per subcommand:
  `"sync lands in M12"`, `"shadow build lands in M6"`, etc. Document
  expected wording in a constant or comment so tests can match.
- [ ] Confirm `main.rs`'s existing error path exits non-zero and prints
  the message — no change needed if `TopCatError: Display` is already
  forwarded.

**Verification**:

```bash
cargo build
cargo test --lib exceptions
```

### Phase 3 — Command stubs

**Goal**: every subcommand named under §CLI surface parses, flattens
`GlobalArgs`, and returns `NotImplemented`.

**Tasks**:
- [ ] Create single-file commands: `sync.rs`, `diff.rs`, `reconcile.rs`,
  `write_deps.rs`, `pause.rs`, `resume.rs`, `uninstall.rs` under
  `src/commands/`. Each:
  - Defines `<Name>Args` with `#[command(flatten)] pub global: GlobalArgs`
    and the documented flags (with correct `value_name`s and enum-kind
    `ValueEnum` where architecture specifies choices).
  - Exposes `pub fn execute(&self) -> Result<(), TopCatError>` that
    returns `Err(TopCatError::NotImplemented("<cmd> lands in M<n>"))`.
- [ ] Create subcommand-parent modules `src/commands/shadow/`,
  `src/commands/migrate/`, `src/commands/meta_schema/` each with a
  `mod.rs` containing `<Parent>Args` + `<Parent>Command` enum that routes
  to leaf modules. Mirror the existing `src/commands/analyze/mod.rs`
  pattern.
- [ ] Register every module in `src/commands/mod.rs` and wire each into
  `src/main.rs`'s `Commands` enum + match arm.
- [ ] Double-check help text reads cleanly for every new command:
  `cargo run -- <cmd> --help`.

**Verification**:

```bash
cargo build
for cmd in sync diff reconcile write-deps pause resume uninstall; do
  cargo run -- "$cmd" --help >/dev/null
done
cargo run -- shadow build main
cargo run -- migrate generate
```

(All should exit non-zero with a `not yet implemented:` line.)

### Phase 4 — Integration tests

**Goal**: lock the stub contract and the `[postgres]` block into CI.

**Tasks**:
- [ ] Create `tests/cli_postgres_stubs_tests.rs`. For each new subcommand,
  assert: exit status `failure()`, stderr contains
  `not yet implemented`, stderr contains the milestone tag (e.g. `M12`).
  Also assert each subcommand's `--help` prints each documented flag.
- [ ] Create `tests/cli_config_postgres_tests.rs`:
  - Write a temp `topcat.toml` with a `[postgres]` block, invoke
    `topcat config show --config <tmp>`, grep for the values.
  - Set `TOPCAT_POSTGRES__DEV_DB_URL` via `Command::env` and assert
    override precedence.
  - Invalid URL → `config validate` exits non-zero with an
    error referencing `dev_db_url` / `shadow_db_url`.
- [ ] Run the full suite; ensure no existing test regressed.

**Verification**:

```bash
cargo test
cargo test --test cli_postgres_stubs_tests
cargo test --test cli_config_postgres_tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Risks (copied and amplified from roadmap)

The roadmap rates M0 as **low risk** and the §Risk hotspots table has no M0
row. Two real design calls are hidden inside the "purely additive" framing;
flagging them here so they do not get silently deferred.

- **Env-var shape mismatch between architecture and existing convention.**
  Severity: low. Architecture §Config surface writes flat forms like
  `TOPCAT_DEV_DB_URL`, but every existing Settings block in this crate
  uses the nested `TOPCAT_<SECTION>__<FIELD>` form (see
  `src/settings/mod.rs` docstring and `TOPCAT_SQL_DISCOVERY__ENABLED` in
  tests). Day-one mitigation: adopt the nested form
  (`TOPCAT_POSTGRES__DEV_DB_URL`) for consistency; record the deviation
  from the architecture doc in `topcat.toml.example` comments and in
  this plan. Do not ship a second flat alias — one canonical form only.
- **Subcommand-depth modelling.** Severity: low. `shadow`, `migrate`, and
  `meta-schema` each have nested subcommands; getting the clap structure
  wrong makes later milestones harder to slot new leaves into. Day-one
  mitigation: mirror the existing `src/commands/analyze/mod.rs` pattern
  (enum-based subcommand routing, one file per leaf) for every nested
  parent.
- **"Not implemented" contract drift.** Severity: low. If each stub picks
  its own error message, integration tests become brittle. Day-one
  mitigation: introduce `TopCatError::NotImplemented(&'static str)` with
  a fixed prefix, and use a common string template (`"<cmd> lands in
  M<n>"`) documented in a module-level comment.
- **`topcat.toml.example` drifting from `PostgresConfig` field set.**
  Severity: low. Day-one mitigation: every field in `PostgresConfig`
  gets a commented line in the example; add an integration test that
  parses the example and asserts it round-trips.

## Tests

- **Unit tests** (`src/settings/tests.rs`, extended):
  - `test_postgres_config_defaults` — default values match architecture.
  - `test_postgres_config_from_toml` — all fields parse.
  - `test_postgres_config_env_override` — `TOPCAT_POSTGRES__*` beats
    config file values.
  - `test_postgres_url_validation_accepts_postgres_scheme`.
  - `test_postgres_url_validation_rejects_bad_scheme`.
- **Unit tests** (`src/exceptions.rs`):
  - `test_not_implemented_display` — message shape stable.
- **Integration tests** (`tests/cli_postgres_stubs_tests.rs`):
  - One test per subcommand, asserting exit failure + error message.
  - Help rendering smoke test per subcommand.
- **Integration tests** (`tests/cli_config_postgres_tests.rs`):
  - Postgres block appears in `config show`.
  - Env-var precedence.
  - Validation failure on malformed URL.
- **Fixtures**:
  - `tests/input/postgres_config_valid.toml` — minimal file with a
    `[postgres]` block.
  - `tests/input/postgres_config_invalid_url.toml` — bad scheme to prove
    validation triggers.
- **Property tests**: none required for M0. Property tests land with M2+
  where correctness bets exist.

## Verification snapshot (definition of done)

```bash
# Build and format gates
cargo build
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Full test suite
cargo test

# Existing commands unchanged
cargo run -- concat -i tests/input/sql /tmp/m00-smoke.sql
cargo run -- config show | grep -E "^\[postgres\]"                   # (new)
cargo run -- config validate

# New stubs — each must exit non-zero with "not yet implemented"
cargo run -- sync || true                                            # (new)
cargo run -- shadow build main || true                               # (new)
cargo run -- shadow clean || true                                    # (new)
cargo run -- diff || true                                            # (new)
cargo run -- migrate generate || true                                # (new)
cargo run -- migrate verify 00000000-0000-0000-0000-000000000000 || true  # (new)
cargo run -- migrate ack 00000000-0000-0000-0000-000000000000 || true     # (new)
cargo run -- migrate list || true                                    # (new)
cargo run -- migrate apply-reverse 00000000-0000-0000-0000-000000000000 || true  # (new)
cargo run -- reconcile || true                                       # (new)
cargo run -- write-deps || true                                      # (new)
cargo run -- pause || true                                           # (new)
cargo run -- resume || true                                          # (new)
cargo run -- uninstall || true                                       # (new)
cargo run -- meta-schema upgrade || true                             # (new)

# Env-var precedence over config file
TOPCAT_POSTGRES__DEV_DB_URL="postgres://u@h/d" cargo run -- config show | grep "dev_db_url"  # (new)
```

## Open questions to resolve before starting

None. From roadmap §Open questions, only Q4 (`shadow_db_url` cluster vs
db) lightly touches M0 and does not block: the field is a URL string
either way — semantic interpretation lands with M6 `shadow_db`. Q1, Q2,
Q3, Q5, Q6 all block later milestones and can be answered at the start
of M1/M5/M11/M12/M19 respectively.

## Session sizing notes

Roadmap estimate: **~1 session**. This is accurate. Natural split
points if it runs long: (1) Phase 1 (config) can land as one PR —
compilable, testable, and shipped even without the CLI stubs; (2) Phases
2–3 (error variant + stubs) land as a second PR; (3) Phase 4 (tests)
folds into Phase 3. A single session should comfortably cover all four.

## Referenced architecture sections

- `## Config surface` — dictates the `[postgres]` field set.
- `## CLI surface` — dictates the subcommand inventory.
- `## Component map` — names the modules that M1+ will populate behind
  the stubs shipped here.
- `## `_topcat` meta-schema versioning` — names the `meta-schema upgrade`
  command surface that M1 fills in (stub ships here).
- `## Scope` — scope boundaries that help distinguish "stub here" from
  "defer entirely."

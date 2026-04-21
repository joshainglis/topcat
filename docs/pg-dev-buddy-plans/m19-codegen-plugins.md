# M19 — codegen plugins

> Anchors: `docs/pg-dev-buddy-roadmap.md` §M19 (codegen plugins) and
> `docs/pg-dev-buddy-architecture.md` §Codegen, §CatalogReader, SchemaModel,
> and DDL emitter, §Config surface, §Component map.

## Context

M19 generates language bindings from a frozen `SchemaModel`. The contract
is deliberately narrow: a plugin consumes a `SchemaModel` (serialized as
canonical JCS JSON) and writes files to a configured output directory.
No shadow db, no migration registry, no event-trigger state — codegen is
a pure function of the schema.

This parallelizes cleanly with every milestone from M3 onward because it
depends only on M2's typed objects. The risk is not algorithmic — it's
configuration-surface design and plugin-count discipline. Four default
plugins (Python, TypeScript, Rust, Go) multiply the review and fixture
load; the roadmap sequences them as one plugin per session. This plan
lands the **plugin interface plus the Python default plugin as MVP**;
TypeScript, Rust, and Go are templated here but their implementations
ship in follow-up sessions.

## Prerequisites

- **M2 lands first.** Every type a plugin consumes lives in
  `crate::schema_model`. Concretely:
  - `crate::schema_model::Table`, `::Column`, `::Constraint`, `::Index`
  - `crate::schema_model::Function`, `::Procedure`
  - `crate::schema_model::View`, `::MaterializedView`
  - `crate::schema_model::Sequence`, `::Type` (enum / composite / domain /
    range / multirange)
  - `crate::schema_model::Trigger`, `::Policy`, `::Rule`
  - `crate::schema_model::Schema`, `::Extension`
  - Long-tail types (text search, FDW/server/user mapping,
    operator/class/family, aggregate, cast, conversion, statistics,
    publication, access method, user event trigger) per architecture
    §Object types covered.
- **M2 JCS serialization.** M19 assumes `SchemaModel` has a stable
  `serde` derivation plus a canonical-JSON writer (RFC 8785). If M2
  ships only `serde_json` default output, M19 adds a thin JCS wrapper;
  if M2 ships a dedicated crate (`serde_jcs` or similar), M19 reuses it.
  Open question #3 below.
- **No dependencies on M1, M3, M5, M6, M9, M10, M11.** Plugins never
  touch `pg/client`, `ddl_emitter`, `shadow_db`, `event_trigger`,
  `migration_*`, or the node_id registry. This is architecturally
  enforced — spawned subprocesses receive no `TOPCAT_DEV_DB_URL` /
  `TOPCAT_SHADOW_DB_URL` in their environment.
- Nix dev shell with `cargo` and `python3` (for the Python integration
  test toolchain check).

## Scope

### In scope for this milestone

- `src/codegen/` module tree with a `Plugin` trait, protocol envelope,
  out-of-process runner, and config-driven registry.
- Out-of-process plugin protocol: JCS-JSON envelope on stdin, JCS-JSON
  response on stdout, one request → one response per invocation.
- Versioned envelope (`{ protocol_version: "1", schema_model_version:
  "1", model: <SchemaModel>, options: { … } }`) so M2 evolution fails
  loudly at the boundary rather than silently corrupting output.
- `topcat codegen [--plugin <name>] [--output <dir>]` CLI command with
  per-plugin TOML config blocks.
- Python default plugin (MVP) — reads envelope, writes a `.py` file per
  table/enum, stubs a `Base` module for common imports.
- Integration test for the Python plugin: apply to a fixture
  `SchemaModel`, write to a tempdir, run `python -c "import …"` against
  the generated module to confirm syntactic validity.

### Explicitly NOT in scope (deferred)

- TypeScript, Rust, Go default plugins. Templated in §Deliverables;
  each ships in a dedicated follow-up session with its own fixture and
  toolchain check (`tsc --noEmit`, `cargo check`, `go build`).
- `node_id`-stable references in generated code. That's an M11 concern;
  the MVP emits schema-qualified names only.
- Inferred edges from plpgsql bodies (M4), bounce markers (M9), and any
  migration-aware codegen — plugins see the frozen model, nothing else.
- In-process Lua plugins. The architecture's `./codegen/python.lua`
  example predates the "out-of-process JCS JSON" roadmap commitment;
  this plan resolves the conflict in favor of out-of-process JSON. The
  config example in architecture §Config surface is amended in §Open
  questions.
- Hot-reload, watch-mode, or incremental regeneration. Single-shot
  invocation only; callers re-run `topcat codegen` as needed.

## Deliverables (files and modules)

| Path | Kind | Purpose |
|---|---|---|
| `src/lib.rs` | extend | Add `pub mod codegen;`. |
| `src/main.rs` | extend | Wire `topcat codegen` subcommand. |
| `src/codegen/mod.rs` | new | Module root; public `run(config, model, plugin_name) -> Result<CodegenReport>` entry point. |
| `src/codegen/plugin/mod.rs` | new | `Plugin` trait (`name()`, `invoke(envelope) -> response`), `PluginKind` enum (`Builtin`, `Subprocess`). |
| `src/codegen/plugin/protocol.rs` | new | Request/response envelope types; JCS serialization; `protocol_version` / `schema_model_version` fields; framing (length-prefixed or newline-delimited — pick in Phase 1). |
| `src/codegen/plugin/runner.rs` | new | Subprocess spawn with sanitized environment (strip `TOPCAT_DEV_DB_URL`, `TOPCAT_SHADOW_DB_URL`, `DATABASE_URL`, `PGPASSWORD`, etc.); timeout; stderr capture into structured log events. |
| `src/codegen/plugin/registry.rs` | new | Resolve plugin name → implementation from `[codegen.plugins.*]` config blocks. |
| `src/codegen/default/mod.rs` | new | Dispatch for bundled in-tree plugins (Python MVP; TS/Rust/Go follow-ups land here). |
| `src/codegen/default/python.rs` | new | Python default plugin: emits one module per table + enum, a `Base` module for shared imports. |
| `src/codegen/output.rs` | new | Output-directory writer with `--dry-run` support and overwrite-safety (writes to `<output>.tmp` then renames atomically). |
| `src/commands/codegen/mod.rs` | new | `topcat codegen` dispatch; reads model from JSON file or — with M2 catalog access — from a live db snapshot. MVP reads from file only. |
| `src/cli/codegen.rs` | new | `clap` arg group: `--plugin`, `--output`, `--model`, `--dry-run`. |
| `src/cli/mod.rs` | extend | Register the `codegen` arg group. |
| `src/commands/mod.rs` | extend | Register `codegen` command module. |
| `src/settings/configs.rs` | extend | Add `CodegenConfig { plugins: HashMap<String, PluginConfig> }`; `PluginConfig { command: Option<Vec<String>>, kind: PluginKind, options: toml::Table }`. |
| `src/settings/loading.rs` | extend | Load `[codegen.plugins.*]` blocks. |
| `src/settings/validation.rs` | extend | Reject plugin configs that specify neither `kind = "builtin"` nor a `command` array. |
| `tests/codegen_python.rs` | new | Integration test: fixture `SchemaModel` JSON → Python plugin → tempdir → `python -c "import generated; ..."`. |
| `tests/input/codegen_fixtures/` | new dir | Fixture `SchemaModel` JSON files keyed by hot spot (enum, composite, table with FK, generated column, RLS-bearing table). |
| `Cargo.toml` | extend | Dev-dep on `tempfile` (if not already present); runtime dep on JCS crate iff Open Question #3 resolves that way. |

## Implementation phases

### Phase 1 — plugin interface + protocol skeleton

Ship the trait, envelope types, JCS serialization, subprocess runner
with sanitized environment, registry, and config loading. No default
plugin beyond an inert `echo` test fixture that round-trips the envelope
and writes one file. The `topcat codegen` command is wired end-to-end
but has no useful output. ~1 session.

### Phase 2 — Python default plugin (MVP)

`src/codegen/default/python.rs` emits:

- One module per table with a `dataclass`/`TypedDict` (pick in Phase 2,
  default `dataclass`) mirroring columns.
- One module per enum type.
- `__init__.py` that re-exports every generated module.
- Shared `_base.py` with common imports.

Type-mapping table lives in code with TOML overrides per plugin config
(`[codegen.plugins.python.type_overrides]`). Integration test asserts
`python -c "import <pkg>"` succeeds and a sample dataclass instantiates.
~1 session.

### Phase 3 — TypeScript plugin (follow-up session)

Emits `.ts` files; integration test runs `tsc --noEmit`. Out of scope
for this plan's session; deliverable list and skeleton are in place.

### Phase 4 — Rust plugin (follow-up session)

Emits a `cargo` crate with `serde`-derived structs; integration test
runs `cargo check`.

### Phase 5 — Go plugin (follow-up session)

Emits a Go package with structs and enum constants; integration test
runs `go build`.

## Risks (copied and amplified)

Roadmap flags two risks under M19; each gets a day-one mitigation.

- **"Plugin configuration surface (architecture example uses `.lua`
  paths — odd for an out-of-process JSON protocol. Clarify before
  building)."**
  - *Day-one mitigation.* Open Question #1 below resolves before any
    code lands. Proposed answer is baked into §Deliverables — `[codegen
    .plugins.<name>]` tables with either `kind = "builtin"` or
    `command = [...]`. The architecture `.lua` example is amended in
    the same PR that lands Phase 1.
  - *Amplification.* Two separate traps live here. (a) Letting the
    config grow unbounded — every plugin gets its own TOML namespace
    for type overrides, so the settings loader must descend into
    plugin-specific tables it doesn't know about. Use `toml::Table` as
    a passthrough rather than typing every plugin's options upfront.
    (b) Silent environment leakage — the subprocess runner **must**
    strip every postgres-related env var before spawn. Enforce with a
    test that scans the child's `environ` for `TOPCAT_DEV_DB_URL`,
    `TOPCAT_SHADOW_DB_URL`, `DATABASE_URL`, `PGPASSWORD`, `PGHOST`,
    `PGUSER`. Omit any and the plugin silently acquires capabilities
    it's architecturally forbidden from having.

- **"Each default plugin is independent work; sequence them as separate
  sessions."**
  - *Day-one mitigation.* This plan executes Phase 1 + Phase 2 only.
    Phases 3/4/5 are declared for visibility but do not block M19's
    first merge. Each follow-up session lands with its own integration
    test and fixture set.
  - *Amplification.* The temptation is to pre-factor a shared "emit a
    struct" helper across all four plugins. Resist. Python
    `dataclass`, TypeScript `interface`, Rust `struct`, and Go `type
    X struct` diverge on naming (snake_case / camelCase / PascalCase),
    nullability (`Optional[T]` / `T | null` / `Option<T>` / pointer),
    and enum lowering (`Enum` subclass / string literal union / `enum`
    / `iota const`). A shared helper collapses those differences into
    flags and becomes the maintenance sink it was meant to prevent.
    Let each plugin own its full emission stack; dedupe only when three
    plugins repeat identical code verbatim.

- **Unstated: SchemaModel version drift across plugins.**
  - *Day-one mitigation.* Envelope carries `schema_model_version`.
    Plugins MUST echo it back or fail; the runner treats a missing /
    mismatched version as a hard error. First integration test
    asserts: bump `schema_model_version` in the fixture, plugin fails
    loudly, exit code non-zero.
  - *Amplification.* Out-of-process plugins can be written by third
    parties in languages topcat doesn't know. Silent acceptance of an
    unknown schema version means those plugins emit stale bindings
    that compile but misrepresent the database. Fail-closed is the
    only safe default.

## Tests

### Integration test (Python, this session)

- `tests/codegen_python.rs` — fixture `SchemaModel` JSON files under
  `tests/input/codegen_fixtures/`. Harness spawns the Python plugin in
  the same runner used at runtime, captures output to a `tempfile`
  tempdir, then invokes `python3 -c "import sys; sys.path.insert(0,
  '<tempdir>'); import generated"` and a follow-up `python3 -c "from
  generated.users import User; User(id=1, email='a@b')"` to confirm the
  dataclass is instantiable.
- Coverage targets:
  - `enum_basic.json` — enum type round-trips to a Python `Enum`.
  - `table_with_fk.json` — FK columns emit the right scalar type (the
    plugin does not materialize the referenced row; that's an ORM
    concern).
  - `generated_column.json` — stored generated columns emit the
    computed type; virtual generated columns emit the same (plugin
    does not track `generated` semantics in MVP).
  - `rls_enabled_table.json` — RLS flags are ignored by codegen (python
    dataclasses don't enforce RLS); emitted code is identical to
    `rls_disabled_table.json` on the same column shape.
  - `version_mismatch.json` — envelope with `schema_model_version:
    "999"`; harness asserts plugin exits non-zero.

### Environment-sanitization test

- `tests/codegen_env_sanitization.rs` — spawns a trivial plugin that
  dumps its own `environ` to stdout; asserts `TOPCAT_DEV_DB_URL`,
  `TOPCAT_SHADOW_DB_URL`, `DATABASE_URL`, `PGPASSWORD`, `PGHOST`,
  `PGUSER`, `PGPORT` are absent.

### Follow-up session tests (templated)

- `tests/codegen_typescript.rs` — `tsc --noEmit` on generated output.
- `tests/codegen_rust.rs` — `cargo check` on the generated crate.
- `tests/codegen_go.rs` — `go build` on the generated package.

Each is stubbed with `#[ignore]` or a `fn placeholder()` until the
corresponding plugin session lands, so the file tree stays stable.

## Verification snapshot

```bash
nix develop --command cargo fmt --check
nix develop --command cargo clippy -- -D warnings
nix develop --command cargo test --lib                              # (new) codegen unit tests
nix develop --command cargo test --test codegen_python              # (new) python integration test
nix develop --command cargo test --test codegen_env_sanitization    # (new) env-var leak test
nix develop --command cargo run -- codegen --plugin python --model tests/input/codegen_fixtures/enum_basic.json --output /tmp/topcat-codegen-smoke  # (new) end-to-end smoke
```

All six must pass. `cargo run … codegen` must write non-empty `.py`
files under `/tmp/topcat-codegen-smoke/` and the resulting package must
`import` cleanly under `python3`.

## Open questions to resolve before starting

1. **`.lua` vs out-of-process JCS JSON.** Architecture §Config surface
   shows `python = { path = "./codegen/python.lua" }`. Roadmap §M19
   commits to out-of-process JCS JSON. **Proposed resolution:** amend
   architecture §Config surface in the Phase 1 PR. New form:

   ```toml
   [codegen.plugins.python]
   kind = "builtin"                    # bundled Python default plugin

   [codegen.plugins.custom]
   command = ["./bin/my-codegen", "--flag"]
   options = { project_name = "myapp" }
   ```

   Confirm before writing any code; the resolution drives both
   `PluginConfig` shape and the runner's dispatch path.

2. **Default plugin delivery mechanism.** Three shapes to pick from:
   (a) bundled in-process — `kind = "builtin"` dispatches to
   `src/codegen/default/python.rs` directly, bypassing IPC; (b) hidden
   subcommand — `topcat __plugin python` reads the envelope from stdin
   and writes to stdout, reusing the subprocess path; (c) separate
   binaries shipped alongside `topcat` (`topcat-codegen-python`). Pick
   one in Phase 1 and document it in architecture §Codegen. **Bias
   toward (a)** — fewer moving parts, honest about the "one process"
   reality, and the subprocess protocol still exists for third-party
   plugins.

3. **JCS serialization source.** `serde_json` default output is not
   canonical (key order depends on insertion order). Options: (a)
   add a runtime dep on `serde_jcs` or equivalent; (b) hand-roll a
   thin `jcs` module using `serde_json::Value` with sorted keys; (c)
   piggyback on whatever M2 ships (the migration_id content hash in
   §Diff and migration generation already needs JCS — M2 or M8 must
   have produced something). Prefer (c); confirm the module name and
   public API before writing Phase 1.

4. **Protocol framing.** Length-prefixed binary (`<u32 LE len><JCS
   bytes>`) vs newline-delimited JSON (one JCS document per line).
   NDJSON is easier to debug manually (`cat envelope.json | plugin`);
   length-prefix is more robust against embedded newlines (none in
   JCS, but still). Pick NDJSON unless a concrete reason emerges.

5. **Output directory overwrite policy.** `topcat codegen` writing into
   a non-empty directory: refuse unless `--force`? Merge non-colliding
   files and overwrite colliding ones? Wipe first? **Bias toward**
   "wipe files matching a `.topcat-codegen` marker, leave everything
   else" — safe for users who check the output into source control and
   occasionally hand-edit.

6. **Catalog-backed model source.** Should `topcat codegen` optionally
   read the model from a live db (via M2 `catalog_reader`) instead of
   a JSON file? That would make M19 depend on M1 transitively, which
   the roadmap forbids. **Resolution:** file-only in MVP. A separate
   `topcat __debug dump-model` subcommand (owned by M3 or M12) can
   produce the JSON; `codegen` consumes it.

## Session sizing notes

Roadmap allocates ~3+ sessions. Suggested split:

- **Session 1 (this plan)** — Phases 1 + 2. Interface, protocol,
  runner, registry, config, Python default plugin, Python integration
  test, env-sanitization test, architecture docs amendment.
- **Session 2** — Phase 3 (TypeScript plugin + `tsc --noEmit` test).
- **Session 3** — Phase 4 (Rust plugin + `cargo check` test).
- **Session 4** — Phase 5 (Go plugin + `go build` test).

Each plugin session is self-contained: new module under
`src/codegen/default/<lang>.rs`, new integration test, new fixture
variants, no cross-plugin refactoring. If a Phase 2 design pressure
produces a shared helper candidate, it stays as a TODO comment until
at least three plugins independently want it.

## Referenced architecture sections

- §Codegen
- §CatalogReader, SchemaModel, and DDL emitter (for the authoritative
  list of `SchemaModel` types the plugin interface must accept)
- §Object types covered
- §Config surface (to be amended per Open Question #1)
- §Component map (to confirm the `codegen/plugin` module path)
- §Bootstrapping (context for why codegen stays orthogonal to the
  three entry paths — codegen runs against any valid `SchemaModel`
  regardless of how it was acquired)

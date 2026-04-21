# pg-dev-buddy — model assignments

Recommended Claude model per milestone. Machine-readable copy lives in
[`milestones.toml`](milestones.toml). The runner script
([`scripts/pg-dev-buddy/run-milestone.sh`](../../scripts/pg-dev-buddy/run-milestone.sh))
reads the manifest; this doc is the human-facing rationale.

If you change a model assignment, update **both** files.

## Rubric

| Tier   | When to pick it |
|--------|------------------|
| **haiku**  | Extremely simple, mechanical edits with no novel design decisions and low correctness risk. |
| **sonnet** | Bounded contract; mostly orchestration or extending an existing pattern; correctness load contained to a small surface. |
| **opus**   | Novel algorithms, multi-source reconciliation, identity primitives, transactional invariants, or correctness bets that propagate through every downstream milestone. |

## A note on the haiku tier

Under the rubric above, **no full milestone is haiku-tier**. Every plan in
this directory either touches a correctness-load-bearing surface or
demands enough cross-file consistency (M0 alone creates 30+ files) that
haiku risks errors a sonnet session would catch. Use haiku for *sub-tasks
within* a sonnet/opus session — emitting a single test fixture, drafting
one stub file from a clear template, or running mechanical refactors —
not for executing whole milestones.

If a future milestone genuinely lands in the haiku tier (e.g. a doc-only
update, a single-fixture addition), edit `milestones.toml` and this table
together.

## Assignments

| #   | Plan                                                                                  | Model   | Why this tier |
|-----|---------------------------------------------------------------------------------------|---------|---------------|
| M0  | [m00-scaffolding.md](m00-scaffolding.md)                                              | sonnet  | Purely additive surface but ~30 files (config, error variant, 15 stub commands, 2 integration test files) where consistency across them matters. |
| M1  | [m01-pg-client-meta-schema.md](m01-pg-client-meta-schema.md)                          | sonnet  | Bounded: thin `postgres` crate wrapper + an embedded-SQL migration runner with one DDL file. No multi-source reconciliation. |
| M2  | [m02-schema-model-catalog-reader.md](m02-schema-model-catalog-reader.md)              | opus    | Many object types and the **normalization rules become every later milestone's correctness floor**. Round-trip property tests in M3 expose any drift here. |
| M3  | [m03-ddl-emitter-round-trip.md](m03-ddl-emitter-round-trip.md)                        | opus    | Round-trip property tests across every object type are the central correctness bet — the emitter is the single `SchemaModel → DDL` path used by M9, M12, and M17. |
| M4  | [m04-body-parser.md](m04-body-parser.md)                                              | opus    | libpg_query AST traversal, search_path resolution, confidence levels for inferred edges. Subtle parser logic feeding M7. |
| M5  | [m05-event-trigger-node-registry.md](m05-event-trigger-node-registry.md)              | opus    | PL/pgSQL trigger functions plus the `node_id` identity primitive that every downstream milestone leans on. Must be atomic with DDL transactions. |
| M6  | [m06-shadow-db-lifecycle.md](m06-shadow-db-lifecycle.md)                              | sonnet  | Process management with clear contracts: cache-keyed template reuse, throwaway dbs. Borderline — bump to opus if cache invalidation surfaces unexpected race conditions. |
| M7  | [m07-object-dag.md](m07-object-dag.md)                                                | opus    | Three-way edge reconciliation (catalog / inferred / declared) with an authority truth table and a dual-mode (sync/migration) policy. Mistakes propagate to differ + migration generator. |
| M8  | [m08-differ.md](m08-differ.md)                                                        | opus    | Object-level change detection across every type in `SchemaModel`. Drives every migration emitted afterwards. |
| M9  | [m09-migration-generator-forward.md](m09-migration-generator-forward.md)              | opus    | Sectioning, ordering, and attaching DDL to a `ChangeSet` — every migration's correctness depends on this. |
| M10 | [m10-migration-verifier.md](m10-migration-verifier.md)                                | sonnet  | Largely orchestration of M3 (emit) + M6 (shadow) + M8 (diff). The hard work is in the modules it composes. |
| M11 | [m11-node-id-identity.md](m11-node-id-identity.md)                                    | opus    | UUIDv7 timestamp extraction, six-case conflict classifier, byte-preserving source rewrites with BOM/CRLF handling. Identity primitive — a wrong scanner silently corrupts rename detection. |
| M12 | [m12-sync-mvp.md](m12-sync-mvp.md)                                                    | opus    | The MVP capstone: per-subgraph transactions, session advisory locks, `_topcat.sync_log` checkpointing, crash recovery, post-condition drift assertion. |
| M13 | [m13-reverse-migrations.md](m13-reverse-migrations.md)                                | opus    | Reverse-migration derivation correctness plus `data_restore` bounce semantics. The reverse must apply cleanly against a snapshot of pre-forward state. |
| M14 | [m14-general-bounces-ack.md](m14-general-bounces-ack.md)                              | sonnet  | Workflow + UX layered onto the M13 reverse pipeline. Lower correctness load. |
| M15 | [m15-split-merge-annotations.md](m15-split-merge-annotations.md)                      | sonnet  | Annotation parsing + propagation through an existing change pipeline. Clear contract. |
| M16 | [m16-reconcile-drift.md](m16-reconcile-drift.md)                                      | opus    | Drift detection plus absorb/revert UX is subtle — the wrong default silently rewrites either the dev db or the source tree. |
| M17 | [m17-bootstrap-entry-paths.md](m17-bootstrap-entry-paths.md)                          | sonnet  | Mostly orchestration of pieces shipped earlier. The path-3 round-trip assertion is the only correctness load and it leans on M3. |
| M18 | [m18-uninstall.md](m18-uninstall.md)                                                  | sonnet  | Small surface; reuses the `clean` dry-run/execute/--force pattern. Header-strip must be byte-preserving but the rest is glue. |
| M19 | [m19-codegen-plugins.md](m19-codegen-plugins.md)                                      | sonnet  | Plugin protocol shape (envelope, framing, env-sanitization) plus one MVP Python plugin. Bounded surface; no multi-source reconciliation. |
| M20 | [m20-perf-observability.md](m20-perf-observability.md)                                | opus    | Parallelism is where correctness bugs hide. Observability schema must be stable across the whole event catalog before benchmarks become meaningful. |

Distribution: **opus 11 · sonnet 10 · haiku 0**.

## Using the runner

```bash
scripts/pg-dev-buddy/list-milestones.sh         # print the assignment table
scripts/pg-dev-buddy/run-milestone.sh m07       # launch an interactive session pre-wired with the right model
scripts/pg-dev-buddy/run-milestone.sh m07 --resume   # extra args pass through to claude
```

The runner sets `--model` and `--name`; everything else (permission mode,
agents, plugins) is yours to pass through or set in your settings.

### Worktree per milestone

Each milestone runs in its own git worktree on its own branch so multiple
milestone sessions can run in parallel without stepping on each other.
Defaults:

- Worktree path: `../topcat-pgdb-<id>` (sibling of the main checkout).
  Override with `TOPCAT_PGDB_WORKTREE_DIR`.
- Branch: `pgdb/<id>` off `main`. Override the base with
  `TOPCAT_PGDB_BASE_BRANCH` (e.g. set it to `pgdb/m07` if you want m08 to
  base off the in-flight m07 branch).

The worktree is created on first run and reused on subsequent runs, so
`--resume` works seamlessly. Inspect with `git worktree list`; clean up
with `git worktree remove ../topcat-pgdb-<id>` when the branch has merged.

## Per-milestone closing protocol

The bootstrap prompt instructs the session to run the `/simplify` skill
after the milestone-completing commit, and — if `/simplify` produces edits —
to commit them as a separate follow-up (`refactor(<id>): simplify per
/simplify pass`). Empty `/simplify` passes do not create empty commits.

This keeps cleanup tied to the milestone that introduced the code while it
is still fresh in the session's context, rather than deferring it to a
generic post-merge sweep.

# Import Module Refactoring - Follow-up Cleanup (Completed)

## Status

**Completed on:** 2026-02-19  
**Scope:** PR1-PR6 cleanup series on top of the refactored import pipeline

## What Landed

1. Added orchestrator behavior coverage for attachment/security/dependency/path handling.
2. Removed runtime trait-wiring self-check logic.
3. Routed primary categorization and rendering through handler dispatch helpers.
4. Switched `requires:` header generation to source-populated `RawObject::extracted_deps`.
5. Removed dead handler helper APIs and tightened header builder usage in orchestrator.
6. Improved parser dependency extraction fidelity:
   - `INDEX` now extracts parent table from `INDEX_PATTERN`.
   - `POLICY` now extracts parent table from `POLICY_PATTERN`.
   - `DEFAULT` now extracts parent table via `ALTER_TABLE_PATTERN`.
   - `CAST` now extracts both source and target type dependencies.

## Verification Snapshot

- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --test cli_import_tests`
- Targeted unit tests:
  - `commands::import::sources::pg_dump::parser::tests::`
  - `commands::import::sources::pg_dump::orchestrator::tests::`
  - `commands::import::output::header_builder::tests::`

## Follow-up Guidance

- Keep new parser dependency extraction tests in sync when adding new attachment types.
- Prefer source-level `extracted_deps` updates over reintroducing handler-driven fallback extraction.

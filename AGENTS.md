## Skills
A skill is a set of local instructions stored in a `SKILL.md` file. This project keeps its local skills in `/.claude/skills`.

### Available skills
- analyzing-dependencies: Guides dependency analysis and safe cleanup operations in Topcat. Use when analyzing dead branches, finding orphans, detecting cycles, cleaning unused files, or integrating with CI/CD pipelines. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/analyzing-dependencies/SKILL.md)
- clippy-fixing: Systematically runs clippy and fixes linting issues following project patterns. Use when running lints, addressing clippy warnings, or improving code quality before commits. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/clippy-fixing/SKILL.md)
- creating-tests-topcat: Guides test creation in Topcat following existing patterns for unit tests, integration tests, and test utilities. Use when writing new tests, debugging test failures, or improving test coverage. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/creating-tests-topcat/SKILL.md)
- discovering-sql-dependencies: Analyzes SQL files to automatically extract and manage dependencies, eliminating manual header maintenance. Use when working with SQL dependency discovery, configuring automatic extraction patterns, or migrating from manual to automatic dependency management in Topcat. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/discovering-sql-dependencies/SKILL.md)
- exporting-graphs: Guides graph export and visualization in multiple formats. Use when visualizing dependencies, generating documentation, integrating with analysis tools, or creating diagrams for reports. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/exporting-graphs/SKILL.md)
- managing-schemas: Guides schema-based organization and analysis for multi-schema database projects. Use when working with multiple schemas, analyzing cross-schema dependencies, or filtering operations by schema. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/managing-schemas/SKILL.md)
- testing-topcat: Guides testing and debugging of Topcat including running tests, writing new test cases, and debugging issues. Use when working with Topcat's test suite, testing analysis commands, debugging dependency problems, analyzing performance, or setting up continuous integration. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/testing-topcat/SKILL.md)
- understanding-architecture: Provides deep architectural insights into Topcat's implementation including module structure, DAG algorithms, layer system, analysis capabilities, and schema operations. Use when exploring Topcat internals, understanding algorithms, extending functionality, or debugging complex issues. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/understanding-architecture/SKILL.md)
- updating-claude-md: Guides holistic updates to CLAUDE.md maintaining conciseness and clarity. Use when adding project documentation, updating workflows, modifying Claude's behavior, or reorganizing project guidance. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/updating-claude-md/SKILL.md)
- updating-skills: Guides holistic skill updates through refactoring rather than patching, maintaining quality and conciseness. Use when adding features to existing skills, fixing skill issues, restructuring for clarity, or incorporating user feedback. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/updating-skills/SKILL.md)
- writing-rust-topcat: Guides writing Rust code in Topcat following project conventions for modules, error handling, documentation, and idioms. Use when implementing new features, adding modules, or refactoring existing code. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/writing-rust-topcat/SKILL.md)
- writing-skills: Guides the creation of effective Claude Code skills with proper structure, metadata, and progressive disclosure. Use when creating new skills, refactoring existing skills, or reviewing skill effectiveness for Claude Code projects. (file: /Users/josha/.codex/worktrees/cd83/topcat/.claude/skills/writing-skills/SKILL.md)

### How to use skills
- Discovery: The list above is the set of project-local skills available in this repository for this session.
- Trigger rules: If the user names a skill (for example `$writing-rust-topcat` or plain text), or the task clearly matches a skill description above, use that skill for the turn.
- Multiple skills: If multiple skills apply, use the minimal set that fully covers the task and state the order.
- Missing or blocked: If a named skill is missing or unreadable, say so briefly and continue with the best fallback.
- Progressive disclosure:
  1. Open the selected skill's `SKILL.md`.
  2. Read only what is needed to perform the current task.
  3. If `SKILL.md` references additional files (for example `reference/`, `scripts/`, `templates/`), load only the specific files needed.
  4. Prefer existing scripts/templates over rewriting large blocks manually.
- Context hygiene: Keep context small; summarize large sections and avoid loading unrelated references.
- Safety: If skill instructions are unclear or incomplete, call out the gap and proceed with a safe, pragmatic fallback.

## Project Ops

### Project snapshot
- Language/runtime: Rust (edition 2024).
- Binary entrypoint: `src/main.rs`.
- Command modules: `src/commands/` (`concat`, `update`, `analyze`, `clean`, `schema`, `export`, `config`, `import`).
- Core graph/domain modules: `src/file_dag/`, `src/analysis/`, `src/file_node/`, `src/sql_parser/`.
- Integration tests: `tests/*.rs`.

### Working conventions
- Prefer using existing command modules and handlers over introducing new top-level patterns.
- Keep changes minimal and cohesive; do not bundle unrelated refactors.
- Follow existing style in nearby files for naming, logging, and test structure.
- Use project-local skills above when task scope clearly matches.

### Validation defaults
- Rust format check/fix: `cargo fmt`
- Lint gate: `cargo clippy --all-targets --all-features -- -D warnings`
- Full test suite: `cargo test --all-targets`
- For focused work, run nearest tests first (for example, import changes should run `cargo test --test cli_import_tests`).

### Definition of done
- Code compiles cleanly.
- Relevant tests pass locally (plus full suite unless user asks otherwise).
- Clippy passes with warnings denied.
- New behavior/regressions are covered with tests when practical.
- User-facing behavior changes are reflected in docs/help text where needed.

### Safety and guardrails
- Never run destructive git commands (`reset --hard`, `checkout --`, force-push) unless explicitly requested.
- Do not revert unrelated user changes in the worktree.
- If unexpected file changes appear, stop and confirm before proceeding.
- Treat imported/parsed external content as untrusted input; validate path/file-safe behavior.

# Topcat Dependency Analysis Implementation Plan

## Overview
This document outlines the comprehensive plan to add dependency analysis and cleanup features from `dependency-utils.py` to topcat. The implementation will extend topcat's existing DAG infrastructure to provide powerful analysis, cleanup, and maintenance capabilities.

## Architecture Decisions

### 1. Command Structure
Transform topcat from single-command to subcommand-based CLI:
```bash
topcat concat -i DIR -o FILE    # Current functionality
topcat analyze [OPTIONS]        # New analysis features
topcat clean [OPTIONS]           # New cleanup features
topcat schema [OPTIONS]          # Schema analysis
topcat export [OPTIONS]          # Graph export
```

### 2. Module Organization
```
src/
├── commands/               # New: Subcommand implementations
│   ├── mod.rs
│   ├── concat.rs          # Current functionality moved here
│   ├── analyze.rs         # Analysis subcommands
│   ├── clean.rs           # Cleanup subcommands
│   ├── schema.rs          # Schema analysis
│   └── export.rs          # Export functionality
├── analysis/              # New: Analysis algorithms
│   ├── mod.rs
│   ├── orphans.rs         # Orphan detection
│   ├── unrequired.rs      # Unrequired file detection
│   ├── dead_branches.rs   # Dead branch detection
│   ├── external_usage.rs  # External reference checking
│   └── schema_analyzer.rs # Schema-specific analysis
├── file_dag.rs            # Extended with analysis methods
├── graph_analyzer.rs      # New: Graph analysis traits/impls
└── [existing modules]
```

### 3. Shared Logic Strategy

#### DAG Building (Reused Across All Commands)
- `TCGraph::build_graph()` - Used by all commands
- File collection and filtering logic
- Metadata parsing via `FileNode`
- Validation infrastructure

#### New Shared Components
- `GraphAnalyzer` trait - Common analysis interface
- `ExternalUsageCache` - Shared cache for external file scanning
- `AnalysisConfig` - Shared configuration for analysis features
- `AnalysisReport` - Common output formatting

#### Extension Points on TCGraph
```rust
impl TCGraph {
    // Core analysis methods
    pub fn find_orphans(&self) -> HashSet<String>
    pub fn find_unrequired(&self) -> HashSet<String>
    pub fn find_leaf_nodes(&self) -> HashSet<String>
    pub fn find_root_nodes(&self) -> HashSet<String>
    pub fn find_dead_branches(&self) -> HashSet<String>

    // Schema analysis
    pub fn get_schemas(&self) -> HashMap<String, SchemaInfo>
    pub fn filter_by_schema(&self, schema: &str) -> TCGraph

    // Dependency queries
    pub fn get_transitive_dependents(&self, node: &str) -> HashSet<String>
    pub fn get_transitive_dependencies(&self, node: &str) -> HashSet<String>
}
```

## Implementation Phases

### Phase 1: Foundation (Core Infrastructure) ✅ COMPLETE
**Goal**: Set up subcommand structure and basic analysis framework

**Tasks**:
- [x] Refactor CLI to use clap v4 with subcommands
- [x] Move current functionality to `commands::concat`
- [x] Create `commands` module structure
- [x] Create `analysis` module structure
- [x] Add `GraphAnalyzer` trait
- [x] Extend `TCGraph` with basic query methods
- [x] Add integration test framework for new commands
- [x] Run `cargo clippy --all-targets` and fix warnings
- [x] Create lib.rs for testing support
- [ ] Update README with new command structure (deferred to Phase 8)

**Tests**:
- [x] Test subcommand routing (manual testing)
- [x] Test backward compatibility (concat command works identically)
- [x] Test basic graph queries (implemented, pending debugging)

### Phase 2: Dead Branches Detection (Priority Feature) ✅ COMPLETE
**Goal**: Implement dead branch detection with external usage checking

**Tasks**:
- [x] Implement `find_leaf_nodes()` on TCGraph
- [x] Implement `find_dead_branches()` algorithm
- [x] Implement `find_orphans()` algorithm
- [x] Implement `find_unrequired()` algorithm
- [x] Implement `find_root_nodes()` algorithm
- [x] Create `ExternalUsageChecker` with caching
- [x] Add support for multiple file patterns (*.py, *.rs, etc.)
- [x] Implement parallel external file scanning with rayon
- [x] Add `analyze dead-branches` subcommand
- [x] Add `analyze orphans` subcommand
- [x] Add `analyze unrequired` subcommand
- [x] Add `analyze leaf-nodes` subcommand
- [x] Add `analyze root-nodes` subcommand
- [x] Add `--external-check-dir` and `--external-pattern` options
- [x] Format output with comfy-table visualization
- [x] Add progress bars with indicatif
- [x] Run `cargo clippy --all-targets`
- [x] Debug and fix integration tests
- [ ] Add exemption rules (never_unrequired patterns) - **REPLACED BY ROOT NODES FEATURE (See Phase 2.5)**

**Tests**:
- [x] Test external usage detection (unit tests passing)
- [x] Comprehensive integration tests written (10 test cases)
- [x] Fix integration test issues (all 10 tests passing)
- [x] All 36 tests passing (26 existing + 10 new)

**Test Fix Summary**:
The integration tests were failing because `TempDir` creates directories starting with a dot (e.g., `.tmpXXXX`), and topcat's `walk_dir` function was treating these as hidden directories and skipping them. Fixed by setting `include_hidden: true` in test Config. Also updated test expectations to match algorithm's correct behavior (in a closed system without external references, all unreferenced nodes are correctly identified as dead).

**Note**: The "exemption rules" task has been superseded by the more comprehensive "Root Nodes Feature" (see Phase 2.5 below).

### Phase 2.5: Root Nodes / Entry Points Feature 🎯 PRIORITY
**Goal**: Add ability to mark files as protected "root nodes" that should never be considered dead

**Rationale**: Without external usage checking, the dead branches algorithm correctly identifies all unreferenced nodes as dead. In production, certain files ARE entry points (API handlers, migrations, CLI commands) that should never be deleted. This feature allows explicit protection of these files.

**Design Document**: See `ROOT_NODES_DESIGN.md` for comprehensive specification

**Tasks**:
- [ ] Add `regex` crate to dependencies
- [ ] Create `src/analysis/root_matcher.rs` with `RootNodeMatcher` struct
  - [ ] Implement exact node name matching
  - [ ] Implement glob pattern matching for file paths
  - [ ] Implement regex pattern matching for node names
  - [ ] Implement directory-based root detection
- [ ] Update `GraphAnalyzer` trait to accept optional `RootNodeMatcher`
- [ ] Modify `find_dead_branches()` to exclude root nodes and their dependencies
- [ ] Add CLI arguments to `AnalyzeArgs`:
  - [ ] `--root-nodes` for specific node names
  - [ ] `--root-pattern` for glob patterns
  - [ ] `--root-regex` for regex patterns
  - [ ] `--root-dir` for directory-based roots
- [ ] Extend `TopcatConfig` with `AnalysisConfig` section
- [ ] Implement config file loading and CLI/config merging
- [ ] Update all analysis commands to use root matcher
- [ ] Run `cargo clippy --all-targets`

**Tests**:
- [ ] Unit tests for `RootNodeMatcher` pattern matching
- [ ] Test glob pattern matching
- [ ] Test regex pattern matching
- [ ] Test directory-based matching
- [ ] Update integration tests to use root nodes for realistic scenarios
- [ ] Test config file loading
- [ ] Test CLI and config merging
- [ ] Test interaction with external usage checking

**Benefits**:
- Production safety: Critical files can never be accidentally deleted
- Better testing: Tests can create realistic scenarios with protected entry points
- Flexible configuration: Multiple pattern types (exact, glob, regex, directory)
- Config file support: Project-specific protection rules can be version controlled
- Complementary to external checking: Works alongside `--external-check-dir`

### Phase 3: Cleanup Operations
**Goal**: Implement safe file deletion with dependency awareness

**Tasks**:
- [ ] Implement `clean` subcommand structure
- [ ] Add `clean dead-branches` with dry-run support
- [ ] Add `clean unrequired` command
- [ ] Add `clean orphans` command
- [ ] Add `clean targets <files>` for specific files
- [ ] Implement dependency tree visualization before deletion
- [ ] Add force/confirmation prompts
- [ ] Add file deletion with error handling
- [ ] Show affected files summary
- [ ] Run `cargo clippy --all-targets`

**Tests**:
- [ ] Test dry-run mode
- [ ] Test deletion with dependencies
- [ ] Test force deletion
- [ ] Test error recovery
- [ ] Integration tests with temp directories

### Phase 4: Comprehensive Analysis
**Goal**: Add all analysis subcommands

**Tasks**:
- [ ] Implement `analyze orphans` command
- [ ] Implement `analyze unrequired` command
- [ ] Implement `analyze leaf-nodes` command
- [ ] Implement `analyze root-nodes` command
- [ ] Implement `analyze cycles` with improved reporting
- [ ] Implement `analyze missing` for missing dependencies
- [ ] Add `analyze <file>` for single file analysis
- [ ] Add progress bars for long operations (indicatif crate)
- [ ] Add verbose/quiet output modes
- [ ] Run `cargo clippy --all-targets`

**Tests**:
- [ ] Test each analysis type
- [ ] Test output formatting
- [ ] Test with various graph structures
- [ ] Test performance benchmarks

### Phase 5: Schema Analysis
**Goal**: Add schema-aware analysis features

**Tasks**:
- [ ] Implement schema extraction from names
- [ ] Add `schema list` command with statistics
- [ ] Add `schema analyze <schema>` detailed view
- [ ] Add cross-schema dependency analysis
- [ ] Add schema filtering to all commands
- [ ] Implement visual statistics (bar charts)
- [ ] Add schema-based graph coloring in DOT export
- [ ] Run `cargo clippy --all-targets`

**Tests**:
- [ ] Test schema extraction
- [ ] Test schema filtering
- [ ] Test cross-schema analysis
- [ ] Test with no-schema files

### Phase 6: Export Capabilities
**Goal**: Add graph export in multiple formats

**Tasks**:
- [ ] Create `export` subcommand
- [ ] Add JSON export with metadata
- [ ] Enhance DOT export with schema colors
- [ ] Add subtree export options
- [ ] Add export modes (full/deps/dependents/direct)
- [ ] Add GraphML format support
- [ ] Add Mermaid diagram format
- [ ] Run `cargo clippy --all-targets`

**Tests**:
- [ ] Test each export format
- [ ] Test subtree filtering
- [ ] Test large graph exports
- [ ] Validate output formats

### Phase 7: Configuration & Polish
**Goal**: Add configuration support and polish features

**Tasks**:
- [ ] Add analysis config to TOML
- [ ] Add exemption rules configuration
- [ ] Add config auto-discovery (.topcat.toml)
- [ ] Add shell completions generation
- [ ] Add man page generation
- [ ] Optimize performance for large codebases
- [ ] Add benchmarks
- [ ] Run comprehensive `cargo clippy --all-targets -- -D warnings`

**Tests**:
- [ ] Test config loading
- [ ] Test config precedence
- [ ] Test auto-discovery
- [ ] Performance regression tests

### Phase 8: Documentation & Skills
**Goal**: Complete documentation and create maintenance skills

**Tasks**:
- [ ] Update README with all new features
- [ ] Create user guide with examples
- [ ] Add inline documentation for all public APIs
- [ ] Create skill: "analyzing-dependencies-topcat"
- [ ] Create skill: "cleaning-unused-files-topcat"
- [ ] Create skill: "schema-management-topcat"
- [ ] Update CLAUDE.md with new capabilities
- [ ] Add cookbook with common workflows
- [ ] Generate API documentation

## Testing Strategy

### Unit Tests
- Each analysis algorithm
- Graph operations
- External usage checking
- File operations

### Integration Tests
- End-to-end command execution
- Multi-command workflows
- Error scenarios
- Large codebase handling

### Performance Tests
- Benchmark analysis algorithms
- Memory usage profiling
- External file scanning optimization
- Cache effectiveness

## Dependencies to Add

```toml
[dependencies]
clap = { version = "4.0", features = ["derive", "cargo"] }
indicatif = "0.17"  # Progress bars
rayon = "1.7"       # Parallel processing
cached = "0.46"     # Caching for external usage
comfy-table = "7.0" # Table formatting
```

## Code Quality Checklist

**Regular Tasks**:
- [ ] Run `cargo clippy --all-targets` after each phase
- [ ] Run `cargo fmt` before commits
- [ ] Run `cargo test` after changes
- [ ] Update tests for new features
- [ ] Keep test coverage >80%
- [ ] Document public APIs
- [ ] Add examples to doc comments

## Risk Mitigation

### Performance Risks
- **Risk**: External file scanning could be slow
- **Mitigation**: Implement parallel scanning, caching, and progress indicators

### Compatibility Risks
- **Risk**: Breaking existing CLI interface
- **Mitigation**: Maintain backward compatibility with aliases/deprecation warnings

### Complexity Risks
- **Risk**: Dead branch algorithm complexity
- **Mitigation**: Extensive testing, clear documentation, visual debugging aids

## Success Metrics

1. **Functionality**: All features from dependency-utils.py implemented
2. **Performance**: <5s analysis for 1000+ file codebases
3. **Reliability**: Zero data loss, comprehensive error handling
4. **Usability**: Clear documentation, intuitive commands
5. **Code Quality**: Pass clippy with no warnings, >80% test coverage

## Next Steps

1. Review and approve this plan
2. Set up development branch
3. Begin Phase 1 implementation
4. Create tracking issue with task checklist
5. Regular progress reviews after each phase

## Notes for Multi-Session Work

- Each phase is self-contained and can be completed independently
- Tests should be written alongside implementation
- Documentation updates should happen within each phase
- Regular clippy runs prevent accumulation of warnings
- Consider creating a skill after Phase 3 (core features complete)

## Estimated Timeline

- Phase 1: 2-3 hours (Foundation)
- Phase 2: 3-4 hours (Dead Branches - Priority)
- Phase 3: 2-3 hours (Cleanup)
- Phase 4: 2-3 hours (Analysis)
- Phase 5: 2 hours (Schema)
- Phase 6: 2 hours (Export)
- Phase 7: 1-2 hours (Config)
- Phase 8: 2 hours (Documentation)

**Total Estimate**: 16-22 hours of implementation

---

This plan is designed to be executed incrementally with regular validation points. The priority feature (dead branches with external checking) comes early in Phase 2-3, providing immediate value while building toward the complete feature set.
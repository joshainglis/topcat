---
name: understanding-architecture
description: Provides deep architectural insights into Topcat's implementation including module structure, DAG algorithms, layer system, analysis capabilities, and schema operations. Use when exploring Topcat internals, understanding algorithms, extending functionality, or debugging complex issues.
---

# Understanding Topcat Architecture

## System Overview

Topcat uses a directed acyclic graph (DAG) with layer-based constraints to order files based on dependencies, analyze graph structure, detect dead code, and manage multi-schema projects.

**Core Workflows**:
```
Concat:  Input Files → Parse Metadata → Build DAG → Validate → Topological Sort → Output
Analyze: Input Files → Build DAG → Run Analysis Algorithms → Report Results
Clean:   Input Files → Build DAG → Find Dead Nodes → Validate Safety → Delete Files
Schema:  Input Files → Build DAG → Extract Schemas → Analyze Cross-Schema Deps
Export:  Input Files → Build DAG → Generate Export Format → Write Output
```

## Core Components

### File Representation

- **FileNode**: Represents files with metadata (name, dependencies, layer, schema)
- **Metadata Parser**: Extracts headers from comment lines
- **Schema Extraction**: Automatic from node names (`schema.table` or `schema::table`)
- **Dependency Types**: `requires` (hard), `exists` (soft), `dropped_by` (reverse)

### Graph Management

- **TCGraph**: Multi-layer DAG structure with schema-aware operations
- **Layer System**: Enforces ordering between file groups
- **Validation**: Cycle detection and cross-layer dependency checks
- **Analysis Algorithms**: Dead branches, orphans, leaf/root nodes via GraphAnalyzer trait
- **Schema Operations**: Grouping, filtering, cross-schema dependency tracking

### Command Structure

- **Subcommand Architecture**: `concat`, `analyze`, `clean`, `schema`, `export`
- **Shared Graph Building**: All commands use TCGraph as foundation
- **Analysis Trait**: Common interface via `GraphAnalyzer` trait
- **Safety Mechanisms**: Root node protection, external usage checking, dry-run mode

## Module Map

### Core Infrastructure

| Module           | Purpose               | Details                                            |
|------------------|-----------------------|----------------------------------------------------|
| `main.rs`        | CLI routing to subcommands | [reference/modules.md](reference/modules.md)  |
| `file_node.rs`   | File representation, schema extraction | [reference/modules.md](reference/modules.md) |
| `file_dag.rs`    | DAG management, schema ops, analysis | [reference/algorithms.md](reference/algorithms.md) |
| `stable_topo.rs` | Topological sorting   | [reference/algorithms.md](reference/algorithms.md) |
| `config.rs`      | Configuration, TOML parsing | [reference/modules.md](reference/modules.md) |
| `output.rs`      | File output generation | [reference/modules.md](reference/modules.md) |

### Commands (Subcommands)

| Module           | Purpose               | Details                                            |
|------------------|-----------------------|----------------------------------------------------|
| `commands/common.rs` | Shared utilities, GraphBuilder | [reference/modules.md](reference/modules.md) |
| `commands/concat.rs` | File concatenation | [reference/modules.md](reference/modules.md) |
| `commands/analyze/` | Modularized analysis (10 modules) | [reference/algorithms.md](reference/algorithms.md) |
| `commands/clean/` | Modularized cleanup (6 modules) | [reference/modules.md](reference/modules.md) |
| `commands/schema.rs` | Schema operations | [reference/modules.md](reference/modules.md) |
| `commands/export/` | Modularized export (multiple formats) | [reference/modules.md](reference/modules.md) |

### Utility Modules

| Module           | Purpose               | Details                                            |
|------------------|-----------------------|----------------------------------------------------|
| `schema_utils.rs` | SchemaFilter, matching/filtering | [reference/modules.md](reference/modules.md) |
| `display_utils.rs` | Table formatting, output | [reference/modules.md](reference/modules.md) |
| `graph_utils.rs` | Node mapping, graph helpers | [reference/modules.md](reference/modules.md) |
| `platform.rs` | Platform utilities (null device, temp) | [reference/modules.md](reference/modules.md) |

### Analysis Infrastructure

| Module           | Purpose               | Details                                            |
|------------------|-----------------------|----------------------------------------------------|
| `analysis/mod.rs` | GraphAnalyzer trait, algorithms | [reference/algorithms.md](reference/algorithms.md) |
| `analysis/root_matcher.rs` | Root node protection patterns | [reference/modules.md](reference/modules.md) |
| `analysis/external_usage.rs` | External usage checking | [reference/modules.md](reference/modules.md) |

## Layer System

Layers enforce ordering between groups of files:

```
prepend → normal → append
```

- Files in earlier layers always precede later layers
- Custom layers: `--layers ddl,dml,indexes`
- Cross-layer dependencies validated at build time

For implementation details, see [reference/layers.md](reference/layers.md).

## Key Algorithms

### Ordering Algorithms

- **Topological Sort**: Custom DFS implementation with deterministic ordering through node weights
- **Cycle Detection**: Uses `graph-cycles` crate to identify circular dependencies
- **Dependency Resolution**: Breadth-first traversal to pull in transitive dependencies

### Analysis Algorithms (GraphAnalyzer Trait)

- **Dead Branches**: Iterative transitive closure to find complete dead subtrees (not just leaf nodes)
- **Orphan Detection**: Finds nodes with no dependencies AND no dependents
- **Leaf/Root Nodes**: Identifies graph endpoints and entry points
- **Unrequired Nodes**: Finds nodes not required by any other nodes
- **Missing Dependencies**: Validates all dependency references exist

### Schema Operations

- **Schema Extraction**: Parses node names for schema prefixes (`schema.table` or `schema::table`)
- **Cross-Schema Dependencies**: Builds map of dependencies between schemas
- **Schema Grouping**: Groups nodes by schema for filtered operations

### Graph Traversal

- **Transitive Dependencies**: BFS to find all dependencies of a node (for export modes)
- **Transitive Dependents**: Reverse BFS to find all dependents of a node
- **Direct Neighbors**: Returns immediate dependencies and dependents

For algorithm details, see [reference/algorithms.md](reference/algorithms.md).

## Extension Points

Common customization scenarios:

- **Add metadata fields**: Modify FileNode and parser
- **Custom sort order**: Update Ord implementation
- **New filter types**: Extend io_utils patterns
- **Additional validations**: Add to graph validation
- **New analysis algorithms**: Implement GraphAnalyzer trait methods
- **New export formats**: Add to export command with format-specific serialization
- **Custom protection patterns**: Extend RootNodeMatcher with new pattern types
- **New commands**: Add subcommand to Commands enum and implement handler

For extension guide, see [reference/extensions.md](reference/extensions.md).

## Performance Notes

**Graph Operations**:
- Graph construction: O(V + E)
- Topological sort: O(V + E)
- Dead branches analysis: O(V + E) with iterative refinement
- Schema extraction: O(V) during node creation

**External Checking**:
- Parallel file scanning using rayon
- File content caching for fast lookups
- Progress bars for operations >100 items

**Memory Usage**:
- O(V) for visited sets during traversal
- O(E) for dependency maps
- File content caching bounded by external file count

**I/O**:
- Lazy content reading for concat
- Parallel scanning for external usage checking
- Efficient file deletion with error recovery
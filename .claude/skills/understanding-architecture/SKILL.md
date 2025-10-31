---
name: understanding-architecture
description: Provides deep architectural insights into Topcat's implementation including module structure, DAG algorithms, and layer system. Use when exploring Topcat internals, understanding the topological sort implementation, extending functionality, or debugging complex dependency issues.
---

# Understanding Topcat Architecture

## System Overview

Topcat uses a directed acyclic graph (DAG) with layer-based constraints to order files based on dependencies.

```
Input Files → Parse Metadata → Build DAG → Validate → Topological Sort → Output
```

## Core Components

### File Representation

- **FileNode**: Represents files with metadata (name, dependencies, layer)
- **Metadata Parser**: Extracts headers from comment lines
- **Dependency Types**: `requires` (hard), `exists` (soft), `dropped_by` (reverse)

### Graph Management

- **TCGraph**: Multi-layer DAG structure
- **Layer System**: Enforces ordering between file groups
- **Validation**: Cycle detection and cross-layer dependency checks

### Ordering Algorithm

- **Stable Topological Sort**: Deterministic DFS-based sorting
- **Node Weighting**: Ensures consistent output across runs
- **Layer Constraints**: Maintains strict inter-layer ordering

## Module Map

| Module           | Purpose               | Details                                            |
|------------------|-----------------------|----------------------------------------------------|
| `main.rs`        | CLI and orchestration | [reference/modules.md](reference/modules.md)       |
| `file_node.rs`   | File representation   | [reference/modules.md](reference/modules.md)       |
| `file_dag.rs`    | DAG management        | [reference/algorithms.md](reference/algorithms.md) |
| `stable_topo.rs` | Topological sorting   | [reference/algorithms.md](reference/algorithms.md) |

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

### Topological Sort

Custom DFS implementation with deterministic ordering through node weights.

### Cycle Detection

Uses `graph-cycles` crate to identify circular dependencies.

### Dependency Resolution

Breadth-first traversal to pull in transitive dependencies.

For algorithm details, see [reference/algorithms.md](reference/algorithms.md).

## Extension Points

Common customization scenarios:

- **Add metadata fields**: Modify FileNode and parser
- **Custom sort order**: Update Ord implementation
- **New filter types**: Extend io_utils patterns
- **Additional validations**: Add to graph validation

For extension guide, see [reference/extensions.md](reference/extensions.md).

## Performance Notes

- Graph construction: O(V + E)
- Topological sort: O(V + E)
- Memory usage: O(V) for visited sets
- File I/O: Lazy content reading
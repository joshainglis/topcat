# Algorithm Reference

## Stable Topological Sort

### Problem

DAGs can have multiple valid topological orderings. We need deterministic output.

### Solution

1. **Node Weighting**: Sort nodes by name and path
2. **Ordered DFS**: Visit neighbors in sorted order
3. **Post-order Collection**: Build result in reverse

```rust
// Simplified algorithm
fn stable_topo_sort(graph: &DiGraph) -> Vec<NodeIndex> {
    let mut visited = HashSet::new();
    let mut result = Vec::new();

    // Sort nodes by weight
    let sorted_nodes = nodes.sorted_by_key(|n| n.weight());

    for node in sorted_nodes {
        if !visited.contains(node) {
            dfs(node, &mut visited, &mut result);
        }
    }

    result.reverse();
    result
}
```

### Complexity

- Time: O(V + E) for DFS + O(V log V) for sorting
- Space: O(V) for visited set

## Cycle Detection

### Implementation

Uses `graph-cycles` crate for efficient cycle detection:

```rust
use graph_cycles::is_cyclic_directed;

if is_cyclic_directed( & graph) {
let cycle = find_cycle( & graph);
return Err(TopCatError::CycleDetected { cycle });
}
```

### Breaking Cycles

- Use layers to separate conflicting nodes
- Review dependency directions
- Consider if cycle represents actual circular dependency

## Cross-Layer Dependency Validation

### Algorithm

```rust
fn validate_cross_layer(source: &str, target: &str) -> Result<()> {
    let source_idx = get_layer_index(source)?;
    let target_idx = get_layer_index(target)?;

    if target_idx > source_idx {
        return Err(TopCatError::InvalidCrossLayerDep {
            source,
            target
        });
    }

    Ok(())
}
```

### Rules

- Dependencies can only point backward or within same layer
- Layer N cannot depend on Layer M where M > N
- Validated during graph construction

## Dependency Resolution

### Transitive Dependencies

When using `--subdir-filter`, pulls in dependencies from outside:

```rust
fn resolve_dependencies(node: &str, graph: &TCGraph) -> HashSet<String> {
    let mut deps = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(node);

    while let Some(current) = queue.pop_front() {
        for dep in graph.dependencies(current) {
            if deps.insert(dep) {
                queue.push_back(dep);
            }
        }
    }

    deps
}
```

## Performance Optimizations

### Graph Construction

- HashMap for O(1) name lookups
- Lazy dependency resolution
- Batch edge additions

### Memory Management

- String interning for repeated names
- Arena allocation for graph nodes
- Buffered file I/O
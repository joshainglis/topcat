# Layer System Reference

## Concept

Layers enforce strict ordering between groups of files:

```
Layer 0 (prepend)  ──┐
                     ├─→ All files in Layer 0 before Layer 1
Layer 1 (normal)   ──┤
                     ├─→ All files in Layer 1 before Layer 2
Layer 2 (append)   ──┘
```

## Data Structure

```rust
pub struct TCGraph {
    layers: Vec<String>,                           // Ordered layer names
    graphs: HashMap<String, DiGraph<NodeIndex>>,   // Per-layer DAGs
    node_map: HashMap<String, (String, NodeIndex)> // name → (layer, index)
}
```

## Configuration

### Default Layers
```bash
# Default configuration
prepend → normal → append
```

### Custom Layers
```bash
# Define custom layers
topcat --layers "extensions,tables,views,functions,data"

# In file headers
-- name: create_users
-- layer: tables
```

### Backward Compatibility
- `is_initial: true` → `layer: prepend`
- `is_final: true` → `layer: append`

## Constraints

### Intra-layer Dependencies
Files within same layer follow dependency order:
```sql
-- name: table_a
-- layer: tables
-- requires: table_b  -- OK if table_b is also in 'tables'
```

### Cross-layer Dependencies
Dependencies can only point backward:
```sql
-- name: view_a
-- layer: views
-- requires: table_a  -- OK if 'tables' comes before 'views'
```

Invalid cross-layer dependency:
```sql
-- name: table_a
-- layer: tables
-- requires: view_a  -- ERROR: 'views' comes after 'tables'
```

## Implementation

### Layer Assignment
```rust
impl FileNode {
    pub fn get_layer(&self, default: &str) -> String {
        if let Some(layer) = &self.layer {
            layer.clone()
        } else if self.is_initial {
            "prepend".to_string()
        } else if self.is_final {
            "append".to_string()
        } else {
            default.to_string()
        }
    }
}
```

### Validation
```rust
fn validate_layer_deps(&self) -> Result<()> {
    for (name, (source_layer, _)) in &self.node_map {
        let deps = self.get_dependencies(name);

        for dep in deps {
            let (target_layer, _) = self.node_map.get(dep)?;
            if layer_index(target_layer) > layer_index(source_layer) {
                return Err(TopCatError::CrossLayerViolation);
            }
        }
    }
    Ok(())
}
```

## Use Cases

### Database Migrations
```bash
topcat --layers "extensions,schemas,tables,constraints,indexes,data"
```

### Application Deployment
```bash
topcat --layers "infrastructure,database,application,configuration"
```

### Test Data
```bash
topcat --layers "setup,fixtures,tests,cleanup"
```
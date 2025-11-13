---
name: exporting-graphs
description: Guides graph export and visualization in multiple formats. Use when visualizing dependencies, generating documentation, integrating with analysis tools, or creating diagrams for reports.
---

# Exporting Graphs

## When to Use This Skill

- Visualizing dependency graphs (GraphViz, Gephi)
- Generating documentation (Mermaid in markdown)
- API integration (JSON export)
- Analyzing graph structure in external tools
- Creating reports and diagrams

## Quick Command Reference

```bash
# Export formats
topcat export -i sql/ -e sql -o graph.json json          # JSON (API)
topcat export -i sql/ -e sql -o graph.dot dot            # GraphViz
topcat export -i sql/ -e sql -o graph.graphml graphml    # Gephi/yEd
topcat export -i sql/ -e sql -o graph.md mermaid         # Markdown

# Export modes
topcat export -i sql/ -e sql -o full.json json                    # Full graph
topcat export -i sql/ -e sql --mode deps --node my_node -o deps.json json    # Dependencies
topcat export -i sql/ -e sql --mode dependents --node my_node -o dependents.json json  # Dependents
topcat export -i sql/ -e sql --mode direct --node my_node -o direct.json json   # Direct neighbors

# With schema filtering
topcat export -i sql/ -e sql --schema auth -o auth.dot dot
```

## Export Formats

### JSON - API Integration

Full metadata export for programmatic access:

```bash
topcat export -i sql/ -e sql -o graph.json json
```

**Contains**:
- All node metadata (name, path, layer, schema, deps, dependents)
- Edge information (source, target, type)
- Schema groupings with statistics
- Graph metadata (version, timestamp, counts)

**Use cases**: API consumption, custom analysis tools, data processing pipelines

### DOT - GraphViz Visualization

Visual graph format with schema coloring:

```bash
topcat export -i sql/ -e sql -o graph.dot dot

# Generate image
dot -Tpng graph.dot -o graph.png
dot -Tsvg graph.dot -o graph.svg
```

**Features**:
- Schema-based node coloring
- Cross-schema edges highlighted (red dashed)
- Supports all GraphViz layouts (dot, neato, fdp, circo)

**Use cases**: Visual exploration, architecture documentation, identifying bottlenecks

### GraphML - Standard Format

XML format compatible with graph analysis tools:

```bash
topcat export -i sql/ -e sql -o graph.graphml graphml
```

**Compatible with**: Gephi, yEd, Cytoscape, NetworkX

**Use cases**: Advanced graph analysis, interactive exploration, community detection

### Mermaid - Markdown Diagrams

Lightweight diagrams embeddable in markdown:

```bash
topcat export -i sql/ -e sql -o graph.md mermaid
```

**Features**:
- Renders in GitHub, GitLab, VS Code
- Schema-based subgraphs
- Cross-schema deps as dotted lines

**Use cases**: README documentation, PR descriptions, wiki pages

## Export Modes

### Full Graph (Default)

Export entire dependency graph:

```bash
topcat export -i sql/ -e sql -o full.json json
```

### Dependencies Mode

Export node and all transitive dependencies:

```bash
topcat export -i sql/ -e sql \
  --mode deps \
  --node my_schema.c \
  -o deps.json json
```

**Useful for**: Understanding what a node needs, minimal subgraph for execution

### Dependents Mode

Export node and all transitive dependents:

```bash
topcat export -i sql/ -e sql \
  --mode dependents \
  --node my_schema.a \
  -o dependents.json json
```

**Useful for**: Impact analysis, understanding downstream effects

### Direct Neighbors Mode

Export node and immediate dependencies/dependents only:

```bash
topcat export -i sql/ -e sql \
  --mode direct \
  --node my_schema.b \
  -o direct.json json
```

**Useful for**: Quick relationship view, simplified visualization

## Common Workflows

### Visual Exploration Workflow

```
Visualization Checklist:
- [ ] Step 1: Export to DOT format
      topcat export -i sql/ -e sql -o graph.dot dot

- [ ] Step 2: Generate PNG image
      dot -Tpng graph.dot -o graph.png

- [ ] Step 3: View image
      open graph.png

- [ ] Step 4: If too complex, use mode filtering
      topcat export -i sql/ -e sql --mode deps --node critical_node -o subgraph.dot dot

- [ ] Step 5: Try different layouts
      neato -Tpng graph.dot -o graph_neato.png
      circo -Tpng graph.dot -o graph_circo.png
```

### Documentation Workflow

```
Documentation Checklist:
- [ ] Step 1: Export Mermaid diagram
      topcat export -i sql/ -e sql -o DEPENDENCIES.md mermaid

- [ ] Step 2: Add to repository
      git add DEPENDENCIES.md
      git commit -m "docs: add dependency diagram"

- [ ] Step 3: Link from README
      echo "See [DEPENDENCIES.md](DEPENDENCIES.md)" >> README.md

- [ ] Step 4: Verify rendering on GitHub
      git push && open https://github.com/user/repo/blob/main/DEPENDENCIES.md
```

### Per-Schema Visualization

```bash
# Export each schema separately
for schema in auth billing public; do
    topcat export -i sql/ -e sql \
        --schema $schema \
        -o ${schema}.dot dot
    dot -Tpng ${schema}.dot -o ${schema}.png
done
```

## Schema Filtering in Exports

Export specific schemas:

```bash
# Single schema
topcat export -i sql/ -e sql --schema auth -o auth.json json

# Multiple schemas
topcat export -i sql/ -e sql \
  --schema auth \
  --schema billing \
  -o core.dot dot
```

## GraphViz Layouts

Different layout engines for different graph types:

```bash
# Hierarchical (default) - good for DAGs
dot -Tpng graph.dot -o graph.png

# Spring model - good for small graphs
neato -Tpng graph.dot -o graph.png

# Force-directed - good for clustered graphs
fdp -Tpng graph.dot -o graph.png

# Circular - good for showing cycles
circo -Tpng graph.dot -o graph.png
```

## Advanced Examples

### Impact Analysis

```bash
# Find all dependents of a critical node
topcat export -i sql/ -e sql \
  --mode dependents \
  --node auth.users \
  -o users_impact.dot dot

dot -Tpng users_impact.dot -o users_impact.png
```

### Dependency Footprint

```bash
# Find complete dependency tree for a node
topcat export -i sql/ -e sql \
  --mode deps \
  --node api_main \
  -o api_deps.dot dot

dot -Tpng api_deps.dot -o api_deps.png
```

### JSON Querying with jq

```bash
# Export JSON
topcat export -i sql/ -e sql -o graph.json json

# Find all leaf nodes
jq '.nodes[] | select(.classification=="leaf") | .name' graph.json

# Find nodes in specific schema
jq '.nodes[] | select(.schema=="auth") | {name, path}' graph.json

# Count dependencies per schema
jq '.schemas | map({schema: .name, deps: (.dependencies | length)})' graph.json

# Find cross-schema edges
jq '.edges[] | select(.source | startswith("auth")) | select(.target | startswith("billing"))' graph.json
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| DOT file too large to render | Use `--mode` filtering or `--schema` filtering |
| GraphViz crashes | Try different layout engine (neato, fdp) |
| Mermaid not rendering | Check syntax, try Mermaid Live Editor |
| JSON file too large | Use mode filtering or export subgraphs |
| Schema colors not distinct | Only 6 colors available, use schema filtering |

## Tool Integration

See [reference/integrations.md](reference/integrations.md) for detailed examples:

- **Python/NetworkX** - Programmatic graph analysis, centrality metrics, community detection
- **Gephi** - Interactive visualization, modularity analysis, statistical reports
- **yEd** - Diagram editing, automatic layouts, property mapping
- **GraphViz Advanced** - Custom styling, subgraph layouts, enhanced visualization
- **Mermaid Dynamic** - Python generation scripts, custom formatting
- **CI/CD Integration** - GitHub Actions, GitLab CI, automated reports
- **Custom D3.js** - Web-based interactive visualization
- **Database Storage** - PostgreSQL integration for graph data

## Format Comparison

| Format | Size | Visual | Programmatic | Editable | Compatible |
|--------|------|--------|--------------|----------|------------|
| JSON | Medium | No | Yes | Yes | Universal |
| DOT | Small | Yes | Limited | Yes | GraphViz |
| GraphML | Large | Yes | Yes | Limited | Many tools |
| Mermaid | Small | Yes | Limited | Yes | Markdown |

## Related Skills

- `analyzing-dependencies` - Analyze before visualizing
- `managing-schemas` - Export schema-specific graphs
- `understanding-architecture` - How graph export works

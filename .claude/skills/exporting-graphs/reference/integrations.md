# Graph Export Integrations

## Overview

This reference provides detailed examples for integrating Topcat's graph exports with various tools and frameworks for analysis, visualization, and documentation.

## Python Integration

### NetworkX Analysis

NetworkX is a Python library for network analysis and graph algorithms.

```python
#!/usr/bin/env python3
"""
Analyze Topcat graph export with NetworkX
"""
import json
import networkx as nx
from collections import Counter

def load_topcat_graph(json_path):
    """Load Topcat JSON export into NetworkX DiGraph"""
    with open(json_path, 'r') as f:
        data = json.load(f)

    G = nx.DiGraph()

    # Add nodes with attributes
    for node in data['nodes']:
        G.add_node(
            node['name'],
            schema=node.get('schema'),
            layer=node.get('layer'),
            path=node.get('path'),
            classification=node.get('classification')
        )

    # Add edges
    for edge in data['edges']:
        G.add_edge(
            edge['source'],
            edge['target'],
            edge_type=edge.get('type', 'hard')
        )

    return G

def analyze_graph(G):
    """Perform various graph analyses"""
    print(f"Graph Statistics:")
    print(f"  Nodes: {G.number_of_nodes()}")
    print(f"  Edges: {G.number_of_edges()}")
    print(f"  Density: {nx.density(G):.4f}")
    print()

    # Strongly connected components
    scc = list(nx.strongly_connected_components(G))
    print(f"Strongly Connected Components: {len(scc)}")
    if len(scc) > 1:
        print(f"  Largest: {len(max(scc, key=len))} nodes")
    print()

    # Centrality metrics
    print("Top 10 Nodes by PageRank:")
    pagerank = nx.pagerank(G)
    for node, score in sorted(pagerank.items(), key=lambda x: x[1], reverse=True)[:10]:
        print(f"  {node}: {score:.4f}")
    print()

    print("Top 10 Nodes by Betweenness Centrality:")
    betweenness = nx.betweenness_centrality(G)
    for node, score in sorted(betweenness.items(), key=lambda x: x[1], reverse=True)[:10]:
        print(f"  {node}: {score:.4f}")
    print()

    # Schema distribution
    schemas = [G.nodes[n].get('schema') for n in G.nodes()]
    schema_counts = Counter(s for s in schemas if s is not None)
    print("Schema Distribution:")
    for schema, count in schema_counts.most_common():
        print(f"  {schema}: {count} nodes")
    print()

    # Find critical nodes (high in-degree)
    in_degrees = G.in_degree()
    print("Top 10 Most Depended Upon Nodes:")
    for node, degree in sorted(in_degrees, key=lambda x: x[1], reverse=True)[:10]:
        print(f"  {node}: {degree} dependents")

if __name__ == '__main__':
    import sys
    if len(sys.argv) != 2:
        print("Usage: python analyze.py <graph.json>")
        sys.exit(1)

    G = load_topcat_graph(sys.argv[1])
    analyze_graph(G)
```

**Usage**:
```bash
# Export graph
topcat export -i sql/ -e sql -o graph.json json

# Analyze with NetworkX
python analyze.py graph.json
```

### Dependency Impact Analysis

```python
#!/usr/bin/env python3
"""
Analyze impact of changing a node
"""
import json
import networkx as nx
import sys

def load_graph(json_path):
    with open(json_path, 'r') as f:
        data = json.load(f)
    G = nx.DiGraph()
    for node in data['nodes']:
        G.add_node(node['name'], **node)
    for edge in data['edges']:
        G.add_edge(edge['source'], edge['target'])
    return G

def impact_analysis(G, node_name):
    """Analyze impact of modifying a node"""
    if node_name not in G:
        print(f"Error: Node '{node_name}' not found")
        sys.exit(1)

    # Direct dependencies
    deps = list(G.predecessors(node_name))
    print(f"Direct Dependencies ({len(deps)}):")
    for dep in deps:
        print(f"  - {dep}")
    print()

    # Direct dependents
    dependents = list(G.successors(node_name))
    print(f"Direct Dependents ({len(dependents)}):")
    for dependent in dependents:
        print(f"  - {dependent}")
    print()

    # Transitive closure (all affected nodes)
    all_dependents = nx.descendants(G, node_name)
    print(f"Total Transitive Dependents: {len(all_dependents)}")
    if all_dependents:
        # Group by schema
        by_schema = {}
        for node in all_dependents:
            schema = G.nodes[node].get('schema', 'none')
            by_schema.setdefault(schema, []).append(node)

        print("\nAffected Nodes by Schema:")
        for schema, nodes in sorted(by_schema.items()):
            print(f"  {schema}: {len(nodes)} nodes")
    print()

    # Critical path (longest path to any leaf)
    try:
        leaves = [n for n in G.nodes() if G.out_degree(n) == 0]
        max_path_len = 0
        max_path = []
        for leaf in leaves:
            if nx.has_path(G, node_name, leaf):
                paths = nx.all_simple_paths(G, node_name, leaf)
                for path in paths:
                    if len(path) > max_path_len:
                        max_path_len = len(path)
                        max_path = path

        if max_path:
            print(f"Longest Dependency Chain ({len(max_path)} nodes):")
            for i, node in enumerate(max_path):
                print(f"  {i+1}. {node}")
    except Exception as e:
        print(f"Error computing longest path: {e}")

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: python impact.py <graph.json> <node_name>")
        sys.exit(1)

    G = load_graph(sys.argv[1])
    impact_analysis(G, sys.argv[2])
```

**Usage**:
```bash
# Analyze impact of changing auth.users_table
topcat export -i sql/ -e sql -o graph.json json
python impact.py graph.json auth.users_table
```

## Gephi Integration

Gephi is a graph visualization and analysis tool.

### Import GraphML

```bash
# Export to GraphML
topcat export -i sql/ -e sql -o graph.graphml graphml
```

**In Gephi**:
1. File → Open → Select `graph.graphml`
2. Import Report → Click "OK"
3. Layout → Choose "Force Atlas 2" or "Fruchterman Reingold"
4. Statistics → Run "Modularity" for community detection
5. Appearance → Nodes → Color → Partition → Select "schema"
6. Appearance → Nodes → Size → Ranking → Select "Degree"

### Analysis Workflows in Gephi

**1. Find Communities**:
- Statistics → Modularity → Run
- Appearance → Nodes → Color → Partition → "Modularity Class"
- Shows natural groupings (may align with schemas)

**2. Find Central Nodes**:
- Statistics → PageRank → Run
- Appearance → Nodes → Size → Ranking → "PageRank"
- Larger nodes = more central to graph

**3. Identify Bottlenecks**:
- Statistics → Betweenness Centrality → Run
- Appearance → Nodes → Size → Ranking → "Betweenness Centrality"
- High betweenness = critical connector nodes

**4. Filter by Schema**:
- Filters → Attributes → Partition → "schema"
- Select specific schemas
- Apply filter

## yEd Integration

yEd is a diagram editor with automatic layout capabilities.

### Import and Layout

```bash
# Export to GraphML
topcat export -i sql/ -e sql -o graph.graphml graphml
```

**In yEd**:
1. File → Open → Select `graph.graphml`
2. Layout → Hierarchical (good for DAGs)
   - Orientation: Top to Bottom
   - Layer Assignment: Hierarchical
3. Tools → Fit Node to Label
4. Edit → Properties Mapper → Create color mapping for schema attribute

### Advanced Layouts

**Organic Layout**:
- Good for showing clusters
- Layout → Organic → Settings:
  - Preferred Edge Length: 100
  - Activate Deterministic Mode

**Orthogonal Layout**:
- Clean, grid-based layout
- Layout → Orthogonal → Settings:
  - Grid: 20
  - Edge Length: Automatic

## GraphViz Advanced

### Custom Styling

Create a custom DOT template:

```bash
# Export base DOT file
topcat export -i sql/ -e sql -o base.dot dot

# Enhance with custom styling
cat > styled.dot << 'EOF'
digraph dependencies {
    # Global settings
    graph [
        rankdir=LR,
        bgcolor="#f8f9fa",
        fontname="Arial",
        fontsize=12
    ];

    node [
        shape=box,
        style="rounded,filled",
        fontname="Arial",
        fontsize=10
    ];

    edge [
        color="#666666",
        fontname="Arial",
        fontsize=8
    ];

    # Include Topcat output
EOF

# Append nodes and edges from base.dot
tail -n +2 base.dot | head -n -1 >> styled.dot
echo "}" >> styled.dot

# Generate image
dot -Tpng styled.dot -o styled.png
```

### Subgraph Layouts

```bash
# Export with schema filtering
topcat export -i sql/ -e sql --schema auth -o auth.dot dot
topcat export -i sql/ -e sql --schema billing -o billing.dot dot

# Combine into single graph with subgraphs
cat > combined.dot << 'EOF'
digraph dependencies {
    rankdir=TB;

    subgraph cluster_auth {
        label="Auth Schema";
        style=filled;
        color=lightblue;
EOF

# Add auth nodes
grep "^  " auth.dot >> combined.dot

echo "  }" >> combined.dot
echo "" >> combined.dot
echo "  subgraph cluster_billing {" >> combined.dot
echo "    label=\"Billing Schema\";" >> combined.dot
echo "    style=filled;" >> combined.dot
echo "    color=lightgreen;" >> combined.dot

# Add billing nodes
grep "^  " billing.dot >> combined.dot

echo "  }" >> combined.dot
echo "}" >> combined.dot

# Generate
dot -Tpng combined.dot -o combined.png
```

## Mermaid Advanced

### Dynamic Mermaid Generation

```python
#!/usr/bin/env python3
"""
Generate custom Mermaid diagrams from Topcat JSON
"""
import json
import sys

def generate_mermaid(json_path, focus_schema=None):
    """Generate Mermaid flowchart with optional schema focus"""
    with open(json_path, 'r') as f:
        data = json.load(f)

    print("```mermaid")
    print("flowchart TD")
    print()

    # Filter nodes
    nodes = data['nodes']
    if focus_schema:
        nodes = [n for n in nodes if n.get('schema') == focus_schema]

    # Node definitions
    for node in nodes:
        node_id = node['name'].replace('.', '_').replace('::', '_')
        label = node['name']
        schema = node.get('schema', 'none')

        # Different shapes by classification
        classification = node.get('classification', 'intermediate')
        if classification == 'root':
            print(f"  {node_id}[{label}]")
            print(f"  style {node_id} fill:#e1f5e1")
        elif classification == 'leaf':
            print(f"  {node_id}[{label}]")
            print(f"  style {node_id} fill:#ffe1e1")
        else:
            print(f"  {node_id}[{label}]")

    print()

    # Edges
    for edge in data['edges']:
        if focus_schema:
            # Check if both nodes are in schema
            source_node = next((n for n in nodes if n['name'] == edge['source']), None)
            target_node = next((n for n in nodes if n['name'] == edge['target']), None)
            if not (source_node and target_node):
                continue

        source_id = edge['source'].replace('.', '_').replace('::', '_')
        target_id = edge['target'].replace('.', '_').replace('::', '_')
        edge_type = edge.get('type', 'hard')

        if edge_type == 'soft':
            print(f"  {source_id} -.-> {target_id}")
        else:
            print(f"  {source_id} --> {target_id}")

    print("```")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python mermaid_gen.py <graph.json> [schema]")
        sys.exit(1)

    json_path = sys.argv[1]
    focus_schema = sys.argv[2] if len(sys.argv) > 2 else None

    generate_mermaid(json_path, focus_schema)
```

**Usage**:
```bash
# Generate Mermaid for specific schema
topcat export -i sql/ -e sql -o graph.json json
python mermaid_gen.py graph.json auth > auth.md
```

## CI/CD Integration

### GitHub Actions Report Generation

```yaml
# .github/workflows/dependency-check.yml
name: Dependency Analysis

on: [pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Topcat
        run: |
          curl -L https://github.com/user/topcat/releases/latest/download/topcat-linux -o topcat
          chmod +x topcat

      - name: Generate Dependency Graph
        run: |
          ./topcat export -i sql/ -e sql -o graph.md mermaid
          ./topcat export -i sql/ -e sql -o graph.json json

      - name: Analyze Changes
        run: |
          # Check for cycles
          if ! ./topcat analyze -i sql/ -e sql --quiet cycles; then
            echo "❌ Cycle detected!"
            exit 1
          fi

          # Check for missing dependencies
          if ! ./topcat analyze -i sql/ -e sql --quiet missing; then
            echo "❌ Missing dependencies detected!"
            exit 1
          fi

      - name: Generate Report
        run: |
          cat > dependency-report.md << 'EOF'
          # Dependency Analysis Report

          ## Graph Visualization

          EOF
          cat graph.md >> dependency-report.md

          echo "" >> dependency-report.md
          echo "## Schema Statistics" >> dependency-report.md
          echo "" >> dependency-report.md
          ./topcat schema -i sql/ -e sql list >> dependency-report.md

      - name: Comment PR
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const report = fs.readFileSync('dependency-report.md', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: report
            });
```

### GitLab CI Report

```yaml
# .gitlab-ci.yml
dependency-analysis:
  stage: test
  image: rust:latest
  script:
    - cargo install topcat
    - topcat export -i sql/ -e sql -o graph.json json
    - topcat export -i sql/ -e sql -o graph.md mermaid
    - topcat schema -i sql/ -e sql list > schema-stats.txt
    - topcat analyze -i sql/ -e sql --quiet cycles || exit 1
    - topcat analyze -i sql/ -e sql --quiet missing || exit 1
  artifacts:
    reports:
      dotenv: schema-stats.txt
    paths:
      - graph.json
      - graph.md
    when: always
```

## Custom Visualization

### D3.js Web Visualization

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Topcat Dependency Graph</title>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <style>
        body { margin: 0; font-family: Arial, sans-serif; }
        #graph { width: 100vw; height: 100vh; }
        .node { stroke: #fff; stroke-width: 1.5px; }
        .link { stroke: #999; stroke-opacity: 0.6; }
        .node-label { font-size: 12px; pointer-events: none; }
    </style>
</head>
<body>
    <div id="graph"></div>
    <script>
        // Load Topcat JSON export
        d3.json('graph.json').then(data => {
            const width = window.innerWidth;
            const height = window.innerHeight;

            const svg = d3.select('#graph')
                .append('svg')
                .attr('width', width)
                .attr('height', height);

            // Create force simulation
            const simulation = d3.forceSimulation(data.nodes)
                .force('link', d3.forceLink(data.edges)
                    .id(d => d.name)
                    .distance(100))
                .force('charge', d3.forceManyBody().strength(-300))
                .force('center', d3.forceCenter(width / 2, height / 2));

            // Draw edges
            const link = svg.append('g')
                .selectAll('line')
                .data(data.edges)
                .enter().append('line')
                .attr('class', 'link')
                .attr('stroke-width', 2);

            // Draw nodes
            const node = svg.append('g')
                .selectAll('circle')
                .data(data.nodes)
                .enter().append('circle')
                .attr('class', 'node')
                .attr('r', 8)
                .attr('fill', d => schemaColor(d.schema))
                .call(drag(simulation));

            // Node labels
            const label = svg.append('g')
                .selectAll('text')
                .data(data.nodes)
                .enter().append('text')
                .attr('class', 'node-label')
                .text(d => d.name);

            // Update positions on tick
            simulation.on('tick', () => {
                link
                    .attr('x1', d => d.source.x)
                    .attr('y1', d => d.source.y)
                    .attr('x2', d => d.target.x)
                    .attr('y2', d => d.target.y);

                node
                    .attr('cx', d => d.x)
                    .attr('cy', d => d.y);

                label
                    .attr('x', d => d.x + 10)
                    .attr('y', d => d.y + 3);
            });

            // Schema color mapping
            function schemaColor(schema) {
                const colors = {
                    'auth': '#ff7f7f',
                    'billing': '#7f7fff',
                    'public': '#7fff7f',
                };
                return colors[schema] || '#cccccc';
            }

            // Drag behavior
            function drag(simulation) {
                function dragstarted(event) {
                    if (!event.active) simulation.alphaTarget(0.3).restart();
                    event.subject.fx = event.subject.x;
                    event.subject.fy = event.subject.y;
                }

                function dragged(event) {
                    event.subject.fx = event.x;
                    event.subject.fy = event.y;
                }

                function dragended(event) {
                    if (!event.active) simulation.alphaTarget(0);
                    event.subject.fx = null;
                    event.subject.fy = null;
                }

                return d3.drag()
                    .on('start', dragstarted)
                    .on('drag', dragged)
                    .on('end', dragended);
            }
        });
    </script>
</body>
</html>
```

**Usage**:
```bash
# Export JSON
topcat export -i sql/ -e sql -o graph.json json

# Serve with Python
python3 -m http.server 8000

# Open browser to http://localhost:8000
```

## Database Integration

### Store Graph in PostgreSQL

```sql
-- Create tables for graph storage
CREATE TABLE topcat_nodes (
    name TEXT PRIMARY KEY,
    schema TEXT,
    layer TEXT,
    path TEXT,
    classification TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE topcat_edges (
    id SERIAL PRIMARY KEY,
    source TEXT REFERENCES topcat_nodes(name),
    target TEXT REFERENCES topcat_nodes(name),
    edge_type TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_edges_source ON topcat_edges(source);
CREATE INDEX idx_edges_target ON topcat_edges(target);
CREATE INDEX idx_nodes_schema ON topcat_nodes(schema);
```

```python
#!/usr/bin/env python3
"""
Import Topcat graph into PostgreSQL
"""
import json
import psycopg2
import sys

def import_graph(json_path, conn_string):
    """Import graph into PostgreSQL"""
    with open(json_path, 'r') as f:
        data = json.load(f)

    conn = psycopg2.connect(conn_string)
    cur = conn.cursor()

    # Clear existing data
    cur.execute("DELETE FROM topcat_edges")
    cur.execute("DELETE FROM topcat_nodes")

    # Insert nodes
    for node in data['nodes']:
        cur.execute("""
            INSERT INTO topcat_nodes (name, schema, layer, path, classification)
            VALUES (%s, %s, %s, %s, %s)
        """, (
            node['name'],
            node.get('schema'),
            node.get('layer'),
            node.get('path'),
            node.get('classification')
        ))

    # Insert edges
    for edge in data['edges']:
        cur.execute("""
            INSERT INTO topcat_edges (source, target, edge_type)
            VALUES (%s, %s, %s)
        """, (
            edge['source'],
            edge['target'],
            edge.get('type', 'hard')
        ))

    conn.commit()
    cur.close()
    conn.close()

    print(f"Imported {len(data['nodes'])} nodes and {len(data['edges'])} edges")

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print("Usage: python import_db.py <graph.json> <connection_string>")
        sys.exit(1)

    import_graph(sys.argv[1], sys.argv[2])
```

**Usage**:
```bash
topcat export -i sql/ -e sql -o graph.json json
python import_db.py graph.json "host=localhost dbname=mydb user=user password=pass"
```

## Summary

This reference covers:
- Python/NetworkX for programmatic analysis
- Gephi/yEd for interactive visualization
- GraphViz for custom styling
- Mermaid for dynamic generation
- CI/CD integration examples
- Custom D3.js web visualization
- Database storage for graph data

Choose the integration that best fits your workflow and requirements.

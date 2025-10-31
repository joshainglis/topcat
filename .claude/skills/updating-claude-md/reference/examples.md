# CLAUDE.md Refactoring Examples

## Example 1: Feature Addition

### Scenario
Adding new authentication feature to project

### ❌ Wrong: Just Append

```markdown
[... existing content ...]

## New Authentication Feature

We've added authentication to the project. To use authentication,
you first need to set up your credentials. Create a file called
.env and add your API_KEY. Then you can authenticate by calling
the auth endpoint. The authentication token will be valid for
24 hours. If you need to refresh, use the refresh endpoint...
[30 more lines]
```

### ✅ Right: Refactor Holistically

```markdown
## Quick Start

```bash
topcat auth --setup  # Set up authentication (if needed)
topcat -i input/ -o output.sql
```

## Available Skills
- **configuring-authentication**: Setup and management
[other skills...]
```

## Example 2: Growing Configuration Section

### ❌ Before: Verbose Configuration (80 lines)

```markdown
## Configuration

You can configure Topcat using a TOML file. The configuration
file should be named topcat.toml and placed in the root directory.

### Basic Configuration

The basic configuration includes the following options:
- input_dir: The directory containing input files
- output_file: Where to write the output
- comment_prefix: The comment style (default: "--")

Here's an example:
```toml
input_dir = "sql/"
output_file = "output.sql"
comment_prefix = "--"
```

### Advanced Configuration

For SQL discovery, you can add:
[40 more lines of configuration details]
```

### ✅ After: Concise with Skill Reference (15 lines)

```markdown
## Configuration

Basic: `topcat.toml` in project root

```toml
[basic]
input_dir = "sql/"
output_file = "output.sql"

[sql_discovery]
enabled = true
schema_pattern = "app_\\w+"
```

Details: See `configuring-topcat` skill
```

## Example 3: Consolidating Redundant Sections

### ❌ Before: Multiple Similar Sections

```markdown
## Building for Development
```bash
cargo build
```
This builds a debug version...

## Building for Production
```bash
cargo build --release
```
This builds an optimized version...

## Building with Features
```bash
cargo build --features sql_discovery
```
This enables additional features...

## Testing the Build
```bash
cargo test
```
Run this after building...
```

### ✅ After: Unified Commands Section

```markdown
## Development

| Task | Command | Notes |
|------|---------|-------|
| Build (debug) | `cargo build` | For development |
| Build (release) | `cargo build --release` | For production |
| Build (features) | `cargo build --features sql_discovery` | Enable features |
| Test | `cargo test` | Run all tests |
```

## Example 4: Extracting Workflows

### ❌ Before: Detailed Workflow in CLAUDE.md

```markdown
## Database Migration Workflow

1. First, prepare your migration files
   - Ensure all files have proper headers
   - Check dependencies are correct
   - Validate no circular dependencies

2. Run the discovery process
   ```bash
   topcat -i migrations/ -o temp.sql --enable-sql-discovery --dry
   ```

3. Review the output
   - Check the order is correct
   - Verify all files are included
   - Look for any warnings

4. Generate the final migration
   ```bash
   topcat -i migrations/ -o migration.sql --enable-sql-discovery
   ```

5. Apply the migration
   [20 more lines...]
```

### ✅ After: Quick Reference with Skill

```markdown
## Database Migrations

```bash
topcat -i migrations/ -o migration.sql --enable-sql-discovery
```

For workflows: Use `migrating-database` skill
```

## Example 5: Complete CLAUDE.md Refactor

### ❌ Before: 600 lines, Everything Mixed

```markdown
# MyProject

## Introduction
[50 lines of history and philosophy]

## Installation
[80 lines of detailed platform-specific instructions]

## How It Works
[100 lines of implementation details]

## Configuration
[100 lines of every possible option]

## Usage
[150 lines of examples]

## Troubleshooting
[100 lines of problems and solutions]

## Contributing
[20 lines of contribution guidelines]
```

### ✅ After: 250 lines, Clear Structure

```markdown
# MyProject

Fast, reliable tool for X. Built with Rust.

## Quick Start

```bash
install-command
myproject run input.txt
```

## Core Concepts

- **Concept A**: One-line explanation
- **Concept B**: One-line explanation

## Available Skills

### Usage
- **getting-started**: Installation and setup
- **common-workflows**: Typical usage patterns
- **troubleshooting-myproject**: Problem resolution

### Development
- **contributing-guide**: How to contribute
- **understanding-internals**: Architecture details

## Commands

| Task | Command |
|------|---------|
| Run | `myproject run <file>` |
| Test | `myproject test` |
| Config | `myproject config set <key> <value>` |

## Configuration

```yaml
# config.yml
setting1: default_value
setting2: default_value
```

Details: See `configuring-myproject` skill

## Development

```bash
cargo build          # Build
cargo test           # Test
cargo fmt            # Format
```

## Project Conventions

1. Always use X for Y
2. Never do Z without W
3. Follow pattern P for Q
```

## Example 6: Adding Emergency Information

### Scenario
Critical bug discovered, needs immediate documentation

### ❌ Wrong: Panic Addition

```markdown
## URGENT BUG FIX NEEDED

THERE IS A CRITICAL BUG IN VERSION 2.3.1 THAT CAUSES DATA LOSS
IF YOU RUN THE COMMAND WITH THE --DELETE FLAG. DO NOT USE THIS
FLAG UNTIL FIXED. THE BUG HAPPENS WHEN...
[30 lines of bug details]
```

### ✅ Right: Concise Warning with Skill

```markdown
## ⚠️ Known Issues

- **v2.3.1**: Data loss with `--delete` flag. See `critical-issues` skill.

## Quick Start
[rest of content unchanged]
```

## Refactoring Workflow

### Step-by-Step Process

1. **Measure Current State**
   ```bash
   wc -l CLAUDE.md  # Check line count
   ```

2. **Identify Extraction Candidates**
   - Sections over 20 lines
   - Detailed procedures
   - Troubleshooting guides

3. **Create/Update Skills**
   ```bash
   # Extract detailed content to skills
   claude "Create skill for [extracted topic]"
   ```

4. **Refactor CLAUDE.md**
   - Replace extracted content with references
   - Consolidate similar sections
   - Convert prose to tables/lists

5. **Verify Quality**
   - Under 300 lines?
   - No redundancy?
   - Clear structure?
   - Skills referenced?

### Before/After Metrics

Good refactor example:
- **Before**: 650 lines, 15 sections, 3000 tokens
- **After**: 280 lines, 8 sections, 1400 tokens
- **Extracted**: 5 new skills created
- **Result**: 53% token reduction per interaction
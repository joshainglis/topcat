# CLAUDE.md Structure Patterns

## Ideal Structure

```markdown
# Project Name

## Project Overview
[2-3 sentences - what it is and primary use case]

## Quick Start
[Essential commands to get started - minimal explanation]

## Key Concepts
[Only concepts needed to understand the project - brief]

## Available Skills
[Directory of skills with one-line descriptions]

## Common Workflows
[Quick reference for frequent tasks - commands only]

## Project Conventions
[Critical rules Claude must always follow]

## Development
[Basic commands for development work]
```

## Size Guidelines

### Section Limits

- **Project Overview**: 3-5 lines
- **Quick Start**: 10-20 lines
- **Key Concepts**: 20-30 lines
- **Available Skills**: 2 lines per skill
- **Common Workflows**: 30-50 lines
- **Conventions**: 10-20 lines
- **Development**: 10-20 lines

**Total Target**: 200-300 lines

## Hierarchy Rules

### Keep It Shallow

```markdown
# Good - Two levels
## Main Section
### Subsection

# Bad - Too deep
## Main Section
### Subsection
#### Sub-subsection
##### Too deep
```

### Logical Grouping

Group by:

1. **Frequency** - Most used first
2. **Workflow** - In order of typical use
3. **Category** - Similar items together
4. **Importance** - Critical before optional

## Refactoring Examples

### Example 1: Verbose to Concise

**Before** (15 lines):

```markdown
## Building the Project

To build this project, you'll need to have Rust installed.
Once Rust is installed, you can build the project by running
the cargo build command. This will compile the source code
and create an executable. For production use, you should
build with the release flag for optimizations...
```

**After** (3 lines):

```markdown
## Build
`cargo build` (debug) or `cargo build --release` (production)
```

### Example 2: Detailed to Reference

**Before** (50 lines of SQL instructions):

```markdown
## SQL Processing
[Detailed SQL processing instructions...]
[Configuration examples...]
[Troubleshooting guide...]
```

**After** (5 lines):

```markdown
## SQL Processing
- Basic: `topcat -i sql/ -o output.sql`
- Discovery: Add `--enable-sql-discovery`
- Details: Use `discovering-sql-dependencies` skill
```

## Anti-Patterns

### ❌ The Growing Blob

CLAUDE.md that keeps accumulating sections:

- Started: 200 lines
- After "just one more section": 300 lines
- After another update: 400 lines
- Eventually: 600+ lines

**Fix**: Periodic refactoring to extract to skills

### ❌ The Duplicate

Same information in CLAUDE.md and skills:

```markdown
# CLAUDE.md
## Testing
[30 lines about testing]

# testing-skill
[Same 30 lines about testing]
```

**Fix**: Keep overview in CLAUDE.md, details in skill

### ❌ The Tutorial

CLAUDE.md trying to teach:

```markdown
## How Topological Sort Works
A topological sort is an algorithm that...
[Academic explanation]
```

**Fix**: Just state what it does, not how

## Maintenance Patterns

### Pattern: Regular Audits

Every major update:

1. Count total lines
2. Check for redundancy
3. Identify extraction candidates
4. Refactor if over 300 lines

### Pattern: Skill Extraction

When a section grows beyond 20 lines:

1. Create a skill for that topic
2. Replace with 2-3 line summary
3. Add skill reference

### Pattern: Command Tables

Replace verbose instructions with tables:

```markdown
| Task | Command |
|------|---------|
| Build | `cargo build` |
| Test | `cargo test` |
| Run | `cargo run` |
```

## Template Structure

```markdown
# [Project Name]

Brief description of what this project does.

## Quick Start

```bash
# Minimal commands to start
command1
command2
```

## Core Concepts

- **Concept 1**: One line explanation
- **Concept 2**: One line explanation

## Available Skills

### Category 1

- **skill-name**: One line description

### Category 2

- **skill-name**: One line description

## Common Tasks

```bash
# Task 1
command for task 1

# Task 2
command for task 2
```

## Development

Essential development commands and workflows.

## Conventions

Project-specific rules Claude must follow.

```
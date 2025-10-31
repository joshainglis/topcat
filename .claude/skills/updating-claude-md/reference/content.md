# Content Guidelines for CLAUDE.md

## Content Decision Matrix

| Content Type       | CLAUDE.md   | Skill           | Example                          |
|--------------------|-------------|-----------------|----------------------------------|
| Project purpose    | ✅ Brief     | ❌               | "CLI for topological sorting"    |
| Quick commands     | ✅ Essential | ✅ All           | `cargo build`, `cargo test`      |
| Detailed workflows | ❌           | ✅               | Multi-step procedures            |
| Troubleshooting    | ❌           | ✅               | Error resolution guides          |
| Examples           | ✅ Minimal   | ✅ Comprehensive | Basic vs advanced usage          |
| Configuration      | ✅ Critical  | ✅ All options   | Required vs optional settings    |
| Architecture       | ✅ Overview  | ✅ Details       | Component list vs implementation |
| Best practices     | ✅ Top 5     | ✅ Comprehensive | Critical vs nice-to-have         |

## Writing Style

### For CLAUDE.md

**Principle**: Maximum information density

```markdown
# Bad - Verbose

To run the tests for this project, you'll need to use
the cargo test command. This will execute all the test
suites and show you the results.

# Good - Concise

Test: `cargo test` or `cargo test specific_test`
```

### Conciseness Techniques

1. **Use tables over paragraphs**
2. **Commands over explanations**
3. **Lists over prose**
4. **Links over duplication**
5. **Examples over descriptions**

## What to Extract

### Always Extract

1. **Procedures > 10 steps**

- Move to workflow skill
- Keep 2-line summary

2. **Domain-specific knowledge**

- Move to specialized skill
- Keep skill reference

3. **Troubleshooting guides**

- Move to skill reference
- Keep "See skill for errors"

4. **Advanced features**

- Move to skill
- Keep basic usage only

### Never Extract

1. **Project identity**

- What the project is
- Primary purpose
- Key differentiator

2. **Orientation information**

- Available skills list
- Basic commands
- Project structure

3. **Critical constraints**

- Must-follow rules
- Security requirements
- Performance limits

## Content Priorities

### Level 1: Essential (Always include)

- What the project does
- How to run it
- Available skills
- Critical conventions

### Level 2: Important (Include if < 250 lines)

- Common workflows
- Key concepts
- Development setup
- Basic examples

### Level 3: Nice-to-have (Include if < 200 lines)

- Architecture overview
- Best practices
- Quick troubleshooting

### Level 4: Extract to skills (Never include)

- Detailed procedures
- Comprehensive guides
- Edge cases
- Historical context

## Redundancy Check

### With Skills

Before adding to CLAUDE.md, check:

```markdown
Questions:

1. Does a skill already cover this?
   → Yes: Reference the skill
   → No: Continue

2. Would this fit better in existing skill?
   → Yes: Update the skill
   → No: Continue

3. Is this worth creating a skill for?
   → Yes: Create skill, add reference
   → No: Add concisely to CLAUDE.md
```

### Within CLAUDE.md

Look for:

- Similar sections that could merge
- Repeated information
- Overlapping examples
- Duplicate commands

## Examples of Good Content

### Project Overview

```markdown
## Overview

Topcat is a CLI tool for topologically sorting files based on
dependencies declared in header comments. Primary use: ordering
SQL migrations.
```

### Quick Reference

```markdown
## Commands

| Action | Command |
|--------|---------|
| Build | `cargo build --release` |
| Run | `topcat -i input/ -o output.sql` |
| Test | `cargo test` |
```

### Skill Directory

```markdown
## Available Skills

- **discovering-sql-dependencies**: Automatic dependency extraction
- **testing-topcat**: Test writing and debugging
- **understanding-architecture**: Internal implementation details
```

## Examples of Bad Content

### Too Detailed

```markdown
## How It Works

First, Topcat reads all files from the input directory. Then it
parses the header comments looking for metadata. The metadata is
used to build a directed acyclic graph. The graph is then...
[50 more lines]
```

### Tutorial Style

```markdown
## Learning Topological Sort

To understand how Topcat works, you need to understand
topological sorting. This is a graph algorithm that...
```

### Redundant with Skills

```markdown
## SQL Discovery (full guide here)

[Same content as in discovering-sql-dependencies skill]
```

## Refactoring Checklist

When updating CLAUDE.md content:

- [ ] Is this needed for most interactions?
- [ ] Can this be said in fewer words?
- [ ] Would a table be clearer?
- [ ] Does a skill already cover this?
- [ ] Am I explaining or just stating?
- [ ] Would an example be better?
- [ ] Is this the right section?
- [ ] Does this duplicate existing content?

## Token Cost Awareness

Remember: CLAUDE.md loads EVERY time

### Cost Calculation

- Average interaction: ~50-100 tokens for user query
- CLAUDE.md at 300 lines: ~1500 tokens
- CLAUDE.md at 600 lines: ~3000 tokens
- Efficiency lost: 50% of context on documentation

### Optimization Goals

- Keep under 300 lines (1500 tokens)
- Extract details to skills (load on demand)
- Use information density techniques
- Remove redundancy ruthlessly
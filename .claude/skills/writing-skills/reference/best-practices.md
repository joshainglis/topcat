# Best Practices for Writing Skills

## Core Principles

### 1. Concise is Key

The context window is shared with:

- System prompt
- Conversation history
- Other skills' metadata
- User requests

**Before adding content, ask:**

- Does Claude really need this?
- Can I assume Claude knows this?
- Does this justify its token cost?

### 2. Progressive Disclosure

```
Level 1: Metadata (always loaded) ~100 tokens
Level 2: SKILL.md (when triggered) <5k tokens
Level 3: Reference files (as needed) unlimited
```

Only loaded content costs tokens.

### 3. Clear Triggering

**Good description:**

```yaml
description: Analyzes Python code for security vulnerabilities and generates remediation reports. Use when auditing code security, reviewing dependencies, or preparing security documentation.
```

**Bad description:**

```yaml
description: Helps with security stuff
```

## Structural Best Practices

### Naming Conventions

Use gerund form (verb + -ing):

- ✅ `analyzing-data`
- ✅ `building-interfaces`
- ✅ `testing-apis`
- ❌ `data-analyzer`
- ❌ `interface-builder`

### File Organization

Keep references one level deep:

```
skill/
├── SKILL.md           # Links to all references
├── reference/
│   ├── guide1.md      # Linked from SKILL.md
│   ├── guide2.md      # Linked from SKILL.md
│   └── guide3.md      # Linked from SKILL.md
```

Avoid:

```
SKILL.md → guide.md → details.md → more.md  # Too deep
```

### Content Distribution

**SKILL.md** (100-200 lines):

- Quick start
- Essential workflows
- Common commands
- Links to references

**Reference files**:

- Detailed explanations
- Comprehensive examples
- Edge cases
- Troubleshooting

## Writing Effective Descriptions

### Include Both What and When

```yaml
# Good - Clear what and when
description: Processes CSV files to generate statistical analyses and visualizations. Use when analyzing datasets, creating reports from CSV data, or performing statistical calculations on tabular data.

# Bad - Missing context
description: Works with CSV files
```

### Use Third Person

```yaml
# Good - Third person
description: Converts Markdown documents to various formats

# Bad - First/second person
description: I can help you convert Markdown documents
description: You can use this to convert documents
```

## Workflow Patterns

### Checklist Pattern

Effective for complex tasks:

```markdown
## Migration Workflow

Copy this checklist:
\`\`\`
Migration Progress:

- [ ] Backup current data
- [ ] Validate schema changes
- [ ] Run migration script
- [ ] Verify data integrity
- [ ] Update documentation
  \`\`\`
```

### Conditional Pattern

Guide through decision trees:

```markdown
## Processing Workflow

1. Determine file type:
  - **CSV?** → Use pandas workflow
  - **JSON?** → Use json workflow
  - **XML?** → Use xml workflow

2. Follow appropriate workflow below...
```

## Script Integration

### Execution vs Reading

Be explicit about intent:

```markdown
# Execute the script (preferred for deterministic operations)

Run: `python scripts/validate.py input.json`

# Read for reference (only when logic understanding needed)

See implementation in `scripts/validate.py`
```

### Utility Scripts

Provide scripts for:

- Complex validations
- Repetitive operations
- Error-prone tasks
- Performance-critical code

## Testing Skills

### With Claude A (Designer)

```markdown
1. Work through problem without skill
2. Note what context you provide
3. Ask Claude to create skill from that context
4. Refine for conciseness
```

### With Claude B (User)

```markdown
1. Fresh Claude instance with skill
2. Give realistic task
3. Observe:
  - Does skill trigger?
  - Are instructions followed?
  - Is output correct?
```

## Common Optimizations

### Reduce Redundancy

Before:

```markdown
Python is a programming language. To read files in Python,
you need to import the appropriate modules...
```

After:

```markdown
Read files with: `Path('file').read_text()`
```

### Use Examples Over Explanation

Before:

```markdown
The configuration file should have a section called database
with fields for host, port, username, and password...
```

After:

```toml
[database]
host = "localhost"
port = 5432
username = "user"
password = "pass"
```

## Quality Checklist

Before publishing a skill:

- [ ] Description includes what AND when to use
- [ ] SKILL.md under 200 lines
- [ ] References properly linked
- [ ] Tested with real scenarios
- [ ] No time-sensitive information
- [ ] Consistent terminology throughout
- [ ] Forward slashes in all paths
- [ ] Third-person descriptions
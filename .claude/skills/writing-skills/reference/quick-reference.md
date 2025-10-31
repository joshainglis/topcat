# Quick Reference for Skill Creation

## Metadata Requirements

```yaml
---
name: lowercase-with-hyphens  # Max 64 chars, no spaces
description: What it does and when to use it  # Max 1024 chars
---
```

## Directory Structure

```
skill-name/
├── SKILL.md                 # Main file (<200 lines)
├── reference/               # Detailed guides
│   ├── guide.md
│   └── examples.md
├── scripts/                 # Executable utilities
│   └── helper.py
└── templates/               # Reusable templates
    └── template.md
```

## Line Count Guidelines

- **SKILL.md**: 100-200 lines ideal, 500 max
- **Metadata**: ~100 tokens (always loaded)
- **Reference files**: No limit (loaded on demand)

## Naming Conventions

### Skill Names (Gerund Form)

- ✅ `analyzing-data`
- ✅ `building-interfaces`
- ✅ `testing-applications`
- ❌ `data-analyzer`
- ❌ `test-runner`

### File Names

- Use lowercase
- Separate words with hyphens
- `.md` extension for documentation
- `.py` for Python scripts

## Description Formula

```yaml
description: |
  [What it does] including [key capabilities].
  Use when [scenario 1], [scenario 2], or [scenario 3].
```

### Examples

```yaml
description: |
  Processes CSV files to generate statistical analyses.
  Use when analyzing datasets, creating reports, or calculating metrics.
```

## Content Priorities

### Must Have in SKILL.md

1. Quick start / common usage
2. Essential commands/code
3. Basic workflow
4. Links to references

### Move to References

1. Detailed explanations
2. Edge cases
3. Comprehensive examples
4. Troubleshooting
5. Advanced features

## Common Patterns

### Checklist Pattern

```markdown
\`\`\`
Task Checklist:

- [ ] Step 1: Action
- [ ] Step 2: Action
- [ ] Step 3: Action
  \`\`\`
```

### Reference Pattern

```markdown
For details, see:

- **Topic**: See [reference/topic.md](reference/topic.md)
- **Examples**: See [reference/examples.md](reference/examples.md)
```

### Command Pattern

```markdown
## Quick Commands

| Action | Command |
|--------|---------|
| Create | \`cmd create\` |
| Update | \`cmd update\` |
| Delete | \`cmd delete\` |
```

## Testing Checklist

- [ ] Description triggers skill appropriately
- [ ] SKILL.md provides enough context
- [ ] References load only when needed
- [ ] Works with target model (Haiku/Sonnet/Opus)
- [ ] No redundant information
- [ ] Clear action steps

## Common Mistakes to Avoid

1. **Too verbose** - Remove obvious context
2. **Wrong person** - Use third person
3. **No triggers** - Include "Use when..."
4. **Deep nesting** - Keep references one level
5. **Platform specific** - Use forward slashes
6. **Time sensitive** - Avoid dates/versions
7. **Too many options** - Provide sensible defaults

## Validation Rules

### Name Field

- Max 64 characters
- Lowercase letters, numbers, hyphens only
- No XML tags
- No reserved words ("anthropic", "claude")

### Description Field

- Non-empty
- Max 1024 characters
- No XML tags
- Third person
- Includes triggers

## Performance Tips

1. **Frontload common tasks** in SKILL.md
2. **Defer details** to reference files
3. **Execute scripts** rather than reading them
4. **Use examples** over explanations
5. **Link directly** to relevant references

## File Size Targets

- **Minimal**: 50-100 lines (simple tools)
- **Standard**: 100-150 lines (most skills)
- **Complex**: 150-200 lines (workflows)
- **Maximum**: 500 lines (split if larger)
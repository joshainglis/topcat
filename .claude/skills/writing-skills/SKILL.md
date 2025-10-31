---
name: writing-skills
description: Guides the creation of effective Claude Code skills with proper structure, metadata, and progressive disclosure. Use when creating new skills, refactoring existing skills, or reviewing skill effectiveness for Claude Code projects.
---

# Writing Skills

## Quick Start

Create a new skill following this workflow:

```
Skill Creation Checklist:
- [ ] Step 1: Identify the reusable pattern
- [ ] Step 2: Create skill directory with SKILL.md
- [ ] Step 3: Write YAML frontmatter (name, description)
- [ ] Step 4: Draft concise main content (<200 lines)
- [ ] Step 5: Extract details to reference files
- [ ] Step 6: Test with real usage scenarios
- [ ] Step 7: Iterate based on Claude's behavior
```

## Skill Structure

```
skill-name/
├── SKILL.md              # Main file (loaded when triggered)
├── reference/            # Detailed guides (loaded as needed)
│   ├── guide1.md
│   └── guide2.md
└── scripts/              # Utility scripts (executed, not loaded)
    └── helper.py
```

## Required Metadata

```yaml
---
name: skill-name          # lowercase, hyphens only, max 64 chars
description: What this skill does and when to use it. # max 1024 chars
---
```

## Writing Principles

### Be Concise
- Keep SKILL.md under 200 lines (ideally ~100)
- Assume Claude already knows basics
- Challenge every paragraph's value

### Use Progressive Disclosure
- Metadata → Main instructions → Reference files → Scripts
- Only loaded content consumes tokens
- Bundle comprehensive resources without penalty

### Set Appropriate Freedom
- **High freedom**: Text instructions for flexible tasks
- **Medium freedom**: Pseudocode with parameters
- **Low freedom**: Exact scripts for fragile operations

## Common Patterns

### Workflow Pattern
```markdown
## Task Workflow

Copy this checklist:
\`\`\`
Progress:
- [ ] Step 1: Clear action
- [ ] Step 2: Next action
- [ ] Step 3: Final action
\`\`\`
```

### Reference Pattern
```markdown
## Quick Guide
[Essential information here]

**Details**: See [reference/details.md](reference/details.md)
**Examples**: See [reference/examples.md](reference/examples.md)
```

For templates, see [templates/](templates/).
For best practices, see [reference/best-practices.md](reference/best-practices.md).
For anti-patterns, see [reference/anti-patterns.md](reference/anti-patterns.md).

## Testing Your Skill

1. **Test discovery**: Does Claude trigger the skill appropriately?
2. **Test effectiveness**: Does it complete the task successfully?
3. **Test efficiency**: Is unnecessary content being loaded?
4. **Test models**: Works with Haiku, Sonnet, and Opus?

## Iteration Workflow

Work with Claude to refine skills:

```
Iteration Checklist:
- [ ] Complete task without skill (note what context you provide)
- [ ] Create skill capturing that context
- [ ] Test skill with fresh Claude instance
- [ ] Observe behavior and note issues
- [ ] Refine based on observations
- [ ] Repeat until effective
```
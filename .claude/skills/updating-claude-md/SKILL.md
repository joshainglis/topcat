---
name: updating-claude-md
description: Guides holistic updates to CLAUDE.md maintaining conciseness and clarity. Use when adding project documentation, updating workflows, modifying Claude's behavior, or reorganizing project guidance. Remember CLAUDE.md loads for every interaction.
---

# Updating CLAUDE.md

## Critical Principle

**CLAUDE.md is loaded for EVERY interaction** - every token counts!

**Refactor, don't patch.** When updating CLAUDE.md, restructure holistically to maintain quality and conciseness.

## Update Workflow

```
CLAUDE.md Update Checklist:
- [ ] Step 1: Assess current size and structure
- [ ] Step 2: Determine if content belongs in CLAUDE.md or skill
- [ ] Step 3: Plan holistic restructure if adding to CLAUDE.md
- [ ] Step 4: Refactor content maintaining conciseness
- [ ] Step 5: Verify no redundancy with skills
- [ ] Step 6: Check total size (<300 lines ideal)
- [ ] Step 7: Test clarity with fresh read-through
```

## What Belongs in CLAUDE.md

### ✅ Include
- Project overview and purpose
- Quick start commands
- Core workflow guidance
- Skill directory (what skills exist)
- Project-specific conventions
- Critical behavioral instructions

### ❌ Move to Skills
- Detailed procedures
- Comprehensive examples
- Troubleshooting guides
- Advanced features
- Domain-specific knowledge
- Optional workflows

## Content Decision Tree

```
Should this go in CLAUDE.md?
├─ Is it needed for EVERY interaction?
│  ├─ Yes → CLAUDE.md (keep concise)
│  └─ No → Continue
├─ Is it project overview/orientation?
│  ├─ Yes → CLAUDE.md (summarize)
│  └─ No → Continue
├─ Is it a common workflow?
│  ├─ Yes → Basic in CLAUDE.md, details in skill
│  └─ No → Skill only
```

## Refactoring Patterns

### Pattern: Extract to Skills

**Before**: Detailed instructions in CLAUDE.md
```markdown
## Database Migrations
[50 lines of detailed migration instructions]
```

**After**: Overview with skill reference
```markdown
## Database Migrations
Use `migrating-database` skill for detailed workflows.
Basic command: `migrate up`
```

### Pattern: Consolidate Sections

**Before**: Multiple similar sections
**After**: Single unified section with clear subsections

### Pattern: Replace Examples with Commands

**Before**: Long code examples
**After**: Quick reference commands or tables

## Quality Metrics

After updates, verify:
- [ ] Under 300 lines (ideal) or 500 (maximum)
- [ ] No duplicate information
- [ ] Clear section hierarchy
- [ ] Skills referenced, not duplicated
- [ ] Quick commands prominent
- [ ] No verbose explanations

## Common Mistakes

1. **Adding "just one more section"** - Restructure instead
2. **Duplicating skill content** - Reference the skill
3. **Over-explaining basics** - Trust Claude's knowledge
4. **Deep nesting** - Keep hierarchy shallow
5. **Verbose descriptions** - Be concise

For detailed guidance:
- **Structure patterns**: See [reference/structure.md](reference/structure.md)
- **Content guidelines**: See [reference/content.md](reference/content.md)
- **Migration examples**: See [reference/examples.md](reference/examples.md)
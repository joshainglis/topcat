---
name: updating-skills
description: Guides holistic skill updates through refactoring rather than patching, maintaining quality and conciseness. Use when adding features to existing skills, fixing skill issues, restructuring for clarity, or incorporating user feedback.
---

# Updating Skills

## Core Principle

**Refactor, don't patch.** When updating a skill, review and restructure the entire skill to maintain quality and coherence.

## Update Workflow

Follow this process for every skill update:

```
Update Checklist:
- [ ] Step 1: Assess current state (document issues)
- [ ] Step 2: Gather new requirements
- [ ] Step 3: Plan new information architecture
- [ ] Step 4: Refactor skill holistically
- [ ] Step 5: Validate conciseness (<200 lines main)
- [ ] Step 6: Test updated skill
- [ ] Step 7: Document changes made
```

## Assessment Phase

Before making changes, evaluate:

1. **Current size**: Is SKILL.md already too large?
2. **Structure**: Is progressive disclosure working?
3. **Redundancy**: Any duplicate information?
4. **Clarity**: Are workflows clear?
5. **Effectiveness**: Does it trigger appropriately?

## Refactoring Patterns

### Pattern 1: Information Overload

**Symptom**: SKILL.md > 300 lines

**Solution**:
1. Extract details to reference files
2. Keep only essential workflows
3. Use links for advanced topics

### Pattern 2: Feature Addition

**Wrong approach**: Append new section
**Right approach**:
1. Reconsider entire structure
2. Integrate feature naturally
3. Update description to include trigger

### Pattern 3: Fixing Ineffectiveness

**Process**:
1. Test current skill behavior
2. Identify why it fails
3. Restructure for clarity
4. Retest with fresh Claude

## Quality Checklist

After updates, verify:

- [ ] SKILL.md still under 200 lines
- [ ] Description updated if scope changed
- [ ] No duplicate information added
- [ ] Progressive disclosure maintained
- [ ] References properly organized
- [ ] Examples updated if needed
- [ ] Third-person voice maintained

## Common Update Mistakes

1. **Just appending content** - Integrate properly instead
2. **Ignoring line limits** - Extract to references
3. **Forgetting description** - Update if scope changes
4. **Creating redundancy** - Check for existing similar content
5. **Breaking structure** - Maintain consistent organization

For detailed strategies:
- **Refactoring patterns**: See [reference/refactoring.md](reference/refactoring.md)
- **Migration guide**: See [reference/migration.md](reference/migration.md)
- **Testing updates**: See [reference/testing-updates.md](reference/testing-updates.md)

## Key Questions

Before committing updates, ask:

1. Is this the right place for this information?
2. Can existing content be consolidated?
3. Does this maintain or improve clarity?
4. Will this trigger when needed?
5. Have I removed outdated information?
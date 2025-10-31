# Skill Migration Guide

## Migration Scenarios

### Scenario 1: Adding Major Feature

**Situation**: Need to add significant new capability to existing skill

**Migration Process**:

```
Migration Steps:
- [ ] Document new feature requirements
- [ ] Review current skill structure
- [ ] Determine if feature warrants separate skill
- [ ] If keeping in same skill:
  - [ ] Plan integrated structure
  - [ ] Update description to include new triggers
  - [ ] Refactor SKILL.md to incorporate feature
  - [ ] Add reference files if needed
- [ ] If creating separate skill:
  - [ ] Extract common elements to shared references
  - [ ] Create new focused skill
  - [ ] Update original skill to reference new one
```

### Scenario 2: Fixing Poor Performance

**Situation**: Skill doesn't trigger or works incorrectly

**Migration Process**:

1. **Diagnose Issue**
```markdown
Testing Questions:
- Does skill trigger at all?
- Does it trigger inappropriately?
- Are instructions unclear?
- Is critical info in references instead of main?
```

2. **Plan Fix**
```markdown
Fix Strategy:
- Update description with better keywords
- Move essential info to SKILL.md
- Clarify ambiguous instructions
- Add concrete examples
```

3. **Implement and Test**
```markdown
- Refactor skill with fixes
- Test with fresh Claude instance
- Verify improvement
- Document what was fixed
```

### Scenario 3: Splitting Oversized Skill

**Situation**: Skill has grown too large and covers too much

**Before**: One large skill
```
mega-skill/
├── SKILL.md (600 lines)
└── reference/ (15 files)
```

**After**: Focused skills
```
skill-part-1/
├── SKILL.md (150 lines)
└── reference/ (5 files)

skill-part-2/
├── SKILL.md (150 lines)
└── reference/ (5 files)

skill-part-3/
├── SKILL.md (150 lines)
└── reference/ (5 files)
```

**Split Strategy**:
1. Identify natural boundaries
2. Group related functionality
3. Create separate skills
4. Update descriptions for clear differentiation
5. Cross-reference where appropriate

### Scenario 4: Consolidating Multiple Skills

**Situation**: Several small related skills should be combined

**Consolidation Process**:

```markdown
Consolidation Checklist:
- [ ] Identify skills to merge
- [ ] Plan unified structure
- [ ] Create new consolidated skill
- [ ] Merge unique content
- [ ] Eliminate redundancy
- [ ] Update description to cover all functions
- [ ] Test consolidated skill
- [ ] Remove old skills
```

## Version Control Strategy

### Tracking Changes

```bash
# Before making changes
git checkout -b update-skillname
git add .claude/skills/skillname
git commit -m "Snapshot skill before update"

# After refactoring
git add .claude/skills/skillname
git commit -m "Refactor skillname: [summary of changes]"
```

### Change Documentation

Create a CHANGES.md in skill directory:

```markdown
# Skill Change Log

## Version 2.0 (2024-10-31)
- Refactored for clarity and conciseness
- Extracted advanced topics to reference/
- Added new feature: X
- Updated examples to modern patterns
- Reduced SKILL.md from 400 to 150 lines

## Version 1.0 (2024-09-15)
- Initial skill creation
```

## Migration Patterns

### Pattern: Progressive Enhancement

Instead of big-bang rewrites, progressively improve:

```markdown
Phase 1: Quick Wins
- [ ] Fix description
- [ ] Remove obvious redundancy
- [ ] Update outdated examples

Phase 2: Structure
- [ ] Reorganize sections logically
- [ ] Extract verbose content to references
- [ ] Consolidate similar sections

Phase 3: Polish
- [ ] Refine workflows
- [ ] Add missing examples
- [ ] Improve cross-references
```

### Pattern: Backward Compatibility

When updating, consider existing users:

```markdown
## Migration Considerations
- Maintain same skill name if possible
- Keep same basic structure
- Document any breaking changes
- Provide migration guide if needed
```

## Common Migration Tasks

### Extracting Verbose Sections

```python
# Script to identify verbose sections
def analyze_skill(skill_path):
    with open(f"{skill_path}/SKILL.md") as f:
        lines = f.readlines()

    sections = {}
    current_section = "header"

    for line in lines:
        if line.startswith("##"):
            current_section = line.strip()
            sections[current_section] = 0
        sections[current_section] += 1

    # Sections over 50 lines are candidates for extraction
    for section, count in sections.items():
        if count > 50:
            print(f"Extract '{section}' ({count} lines) to reference/")
```

### Updating Examples

```markdown
Example Update Checklist:
- [ ] Review all code examples
- [ ] Update deprecated syntax
- [ ] Use modern best practices
- [ ] Ensure consistency across examples
- [ ] Test that examples actually work
```

### Reorganizing References

```bash
# Before: Flat structure
reference/
├── advanced-topic-1.md
├── advanced-topic-2.md
├── basics-1.md
├── basics-2.md
├── troubleshooting-1.md
├── troubleshooting-2.md

# After: Organized structure
reference/
├── basics/
│   ├── getting-started.md
│   └── common-tasks.md
├── advanced/
│   ├── topic-1.md
│   └── topic-2.md
└── troubleshooting.md
```

## Success Metrics

### Quantitative Metrics

Before and after comparison:
- SKILL.md line count
- Number of reference files
- Total skill size
- Load time in Claude

### Qualitative Metrics

- Improved clarity
- Better organization
- Easier navigation
- Higher success rate
- Fewer user questions

## Recovery Strategy

If update causes problems:

```markdown
Recovery Steps:
1. Revert to previous version in git
2. Analyze what went wrong
3. Create smaller incremental changes
4. Test each change individually
5. Proceed more cautiously
```
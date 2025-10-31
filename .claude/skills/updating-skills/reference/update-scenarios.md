# Common Update Scenarios

## Scenario: User Feedback Integration

### Feedback: "Skill doesn't work for my use case"

**Analysis Process**:

```markdown
1. Understand the gap:
   - What specifically doesn't work?
   - Is it a triggering issue?
   - Missing functionality?
   - Unclear instructions?

2. Determine scope:
   - Is this within skill's intended purpose?
   - Would it fit better in different skill?
   - Is it an edge case or common need?

3. Plan integration:
   - If in scope: Plan holistic update
   - If edge case: Add to reference/edge-cases.md
   - If out of scope: Document limitation or create new skill
```

### Feedback: "Skill is confusing"

**Clarity Improvements**:

```markdown
Refactoring for Clarity:
- [ ] Simplify language
- [ ] Add more examples
- [ ] Restructure workflow order
- [ ] Extract complexity to references
- [ ] Add step-by-step checklist
- [ ] Improve section headings
```

## Scenario: Technology Updates

### Handling Deprecations

```markdown
## Old Method (deprecated)

<details>
<summary>Legacy approach (pre-2024)</summary>
[Old method kept for reference]
</details>

## Current Method

[New recommended approach]
```

### Adding Support for New Versions

**Wrong way**: Adding version-specific sections

```markdown
## For Version 2.x
[Instructions]

## For Version 3.x
[Different instructions]

## For Version 4.x
[More instructions]
```

**Right way**: Current with compatibility notes

```markdown
## Setup

[Current best practice]

**Compatibility**: Works with v3.0+. For older versions, see [reference/legacy.md]
```

## Scenario: Scope Creep

### Signs of Scope Creep

- Skill description keeps growing
- Unrelated features being added
- Multiple workflows in one skill
- Confused triggering

### Managing Scope

```markdown
Scope Decision Tree:
1. Is new feature closely related?
   Yes → Integrate holistically
   No → Continue to 2

2. Would users expect it here?
   Yes → Add with clear separation
   No → Continue to 3

3. Create separate skill
   - Reference from original if related
   - Clear differentiation in descriptions
```

## Scenario: Performance Issues

### Skill Too Slow

**Optimization Strategies**:

```markdown
Speed Improvements:
1. Move details to references
2. Simplify complex workflows
3. Use tables instead of prose
4. Provide quick commands upfront
5. Defer examples to references
```

### Too Many Files Loading

**Progressive Disclosure Fix**:

```markdown
# Before: Everything linked from SKILL.md
See [guide1.md], [guide2.md], [guide3.md], [guide4.md]

# After: Hierarchical loading
For details, see [reference/index.md]
(index.md then links to specific guides)
```

## Scenario: Merging Duplicate Content

### Finding Duplicates

```bash
# Find similar content across skills
grep -r "similar phrase" .claude/skills/

# Find duplicate files
fdupes -r .claude/skills/
```

### Consolidation Strategy

```markdown
Consolidation Steps:

1. Identify all duplicates
2. Determine authoritative source
3. Create single reference
4. Update all skills to point to it
5. Remove duplicates
6. Test all affected skills
```

## Scenario: Breaking Changes

### When Breaking Changes are Necessary

- Fundamental restructuring needed
- Old approach was wrong
- Security or safety issues
- Major simplification possible

### Managing Breaking Changes

```markdown
Breaking Change Process:

1. Document what will break
2. Provide migration path
3. Keep old version available temporarily
4. Clear deprecation notice
5. Update version in description
```

### Example Breaking Change Notice

```markdown
> ⚠️ **Breaking Change in v2.0**
>
> This skill has been restructured for clarity.
> - Old structure: See [reference/v1-legacy.md]
> - Migration guide: See [reference/migration-v2.md]
> - Key changes: Workflow order, new required parameters
```

## Scenario: Emergency Fixes

### Critical Bug Found

```markdown
Emergency Fix Process:
- [ ] Immediate: Add warning to SKILL.md
- [ ] Quick fix: Patch the specific issue
- [ ] Test fix: Verify it works
- [ ] Deploy: Update skill
- [ ] Follow-up: Plan proper refactoring
```

### Example Warning

```markdown
> ⚠️ **Known Issue**: Step 3 may fail with large files.
> Workaround: Process in batches of 100 items.
> Fix coming in next update.
```

## Update Frequency Guidelines

### Batch Updates

Better to batch multiple improvements:

- Less disruption for users
- Holistic refactoring opportunity
- Single testing cycle
- One documentation update

### Update Cadence

- **Critical fixes**: Immediately
- **Minor improvements**: Batch weekly/monthly
- **Major refactoring**: Quarterly or as needed
- **Feature additions**: When complete and tested

## Change Communication

### In-Skill Documentation

```markdown
<!-- At top of SKILL.md -->
> 📝 **Recent Updates** (2024-10-31)
> - Added support for new format
> - Improved error handling
> - See [reference/changelog.md] for details
```

### Changelog Format

```markdown
# Changelog

## [2.0.0] - 2024-10-31

### Added
- New feature X
- Support for Y

### Changed
- Restructured workflow for clarity
- Improved examples

### Removed
- Deprecated method Z

### Fixed
- Issue with large files
- Incorrect error messages
```
# Refactoring Patterns for Skill Updates

## Pattern 1: Extracting to References

### Before (Bloated SKILL.md)
```markdown
# SKILL.md (450 lines)
## Overview
[50 lines of explanation]

## Basic Usage
[100 lines of examples]

## Advanced Usage
[150 lines of complex scenarios]

## Troubleshooting
[150 lines of edge cases]
```

### After (Refactored)
```markdown
# SKILL.md (150 lines)
## Overview
[10 lines - essential only]

## Quick Start
[40 lines - most common usage]

## Common Tasks
[80 lines - frequent operations]

## Advanced Topics
- **Complex scenarios**: See [reference/advanced.md](reference/advanced.md)
- **Troubleshooting**: See [reference/troubleshooting.md](reference/troubleshooting.md)
```

## Pattern 2: Consolidating Redundancy

### Identifying Redundancy

Look for:
- Similar instructions in multiple places
- Repeated examples with minor variations
- Multiple explanations of same concept
- Overlapping reference files

### Consolidation Strategy

```markdown
# Before: Multiple similar sections
## Using with CSV
[Instructions for CSV]

## Using with TSV
[Similar instructions for TSV]

## Using with PSV
[Similar instructions for PSV]

# After: Single parameterized section
## Using with Delimited Files
\`\`\`python
# Works with CSV, TSV, PSV
pandas.read_csv(file, delimiter=delimiter)
# delimiter: ',' for CSV, '\t' for TSV, '|' for PSV
\`\`\`
```

## Pattern 3: Restructuring for Clarity

### Information Hierarchy

Reorganize content by:

1. **Frequency of use**
   - Most common → SKILL.md
   - Occasional → reference/
   - Rare → reference/advanced/

2. **User journey**
   - Setup → Quick start → Common tasks → Advanced

3. **Complexity**
   - Simple examples → Complex scenarios → Edge cases

### Example Restructure

```markdown
# Before: Mixed complexity
## Examples
[Simple example]
[Complex example]
[Simple example]
[Edge case]
[Common example]

# After: Progressive complexity
## Quick Examples
[Most common case]

## Standard Usage
[Typical scenarios]

## Advanced Examples
See [reference/advanced-examples.md](reference/advanced-examples.md)
```

## Pattern 4: Updating Descriptions

### When to Update Description

Update when:
- New major feature added
- Scope significantly changed
- New triggers needed
- Use cases expanded

### Description Evolution

```yaml
# Original
description: Processes JSON files

# After adding features (bad - just appending)
description: Processes JSON files and now also does XML and YAML

# After adding features (good - holistic rewrite)
description: Processes structured data formats including JSON, XML, and YAML for validation, transformation, and schema operations. Use when working with configuration files, API responses, or data interchange formats.
```

## Pattern 5: Integrating New Features

### Wrong Way: Append

```markdown
# SKILL.md
[Original content]

## New Feature Added in Update
[New content tacked on end]
```

### Right Way: Integrate

```markdown
# SKILL.md
## Core Features
- Original capability
- **New capability** (integrated naturally)

## Workflows
[Updated workflow including new feature where relevant]
```

## Pattern 6: Modernizing Examples

### Update Examples Holistically

When adding new capabilities:

1. Review all existing examples
2. Update to show best current practices
3. Remove outdated approaches
4. Ensure consistency across examples

```python
# Before: Old example
data = json.load(open('file.json'))

# After: Modernized with new capability
from pathlib import Path
data = json.loads(Path('file.json').read_text())
```

## Pattern 7: Reference Consolidation

### Merging Similar References

```
# Before: Fragmented
reference/
├── setup-linux.md
├── setup-mac.md
├── setup-windows.md
├── troubleshooting-linux.md
├── troubleshooting-mac.md
├── troubleshooting-windows.md

# After: Consolidated
reference/
├── setup.md (with platform sections)
├── troubleshooting.md (with platform sections)
```

## Refactoring Workflow

### 1. Audit Phase

```markdown
Audit Checklist:
- [ ] List all current sections
- [ ] Note line counts for each
- [ ] Identify redundancies
- [ ] Find outdated content
- [ ] Check reference organization
```

### 2. Planning Phase

```markdown
Planning Questions:
- What's the new ideal structure?
- What moves to references?
- What can be consolidated?
- What needs updating?
- What should be removed?
```

### 3. Execution Phase

```markdown
Execution Order:
1. Create new structure (don't delete yet)
2. Migrate content to new structure
3. Consolidate and refactor as you go
4. Update examples and workflows
5. Update description if needed
6. Remove old structure
7. Test thoroughly
```

## Code Smells in Skills

Signs that refactoring is needed:

1. **Length smell**: SKILL.md > 300 lines
2. **Duplication smell**: Same info in multiple places
3. **Navigation smell**: Hard to find information
4. **Inconsistency smell**: Different styles/patterns
5. **Obsolescence smell**: Outdated examples/approaches
6. **Patch smell**: Obviously appended sections
7. **Description mismatch**: Description doesn't match content

## Measuring Refactoring Success

### Before/After Metrics

- Line count reduction in SKILL.md
- Number of consolidated sections
- Improved navigation (fewer levels)
- Faster skill triggering
- Better test results

### Quality Indicators

- Clear information hierarchy
- No redundant content
- Consistent style throughout
- Appropriate progressive disclosure
- Updated, modern examples
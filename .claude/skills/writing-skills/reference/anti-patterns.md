# Anti-Patterns to Avoid

## Structural Anti-Patterns

### ❌ Missing YAML Frontmatter

**Bad:**
```markdown
# My Skill

This skill does things...
```

**Good:**
```markdown
---
name: doing-things
description: Performs specific operations on data files. Use when processing data or generating reports.
---

# Doing Things
```

### ❌ Too Verbose

**Bad:**
```markdown
## Introduction

PDF files, which stands for Portable Document Format, are a type of
file format created by Adobe Systems in 1993. They are widely used
for documents because they preserve formatting across different
platforms...
```

**Good:**
```markdown
## PDF Processing

Extract text: `pdfplumber.open(file).pages[0].extract_text()`
```

### ❌ Deeply Nested References

**Bad:**
```
SKILL.md → overview.md → details.md → implementation.md
```

**Good:**
```
SKILL.md → implementation.md (direct reference)
```

## Content Anti-Patterns

### ❌ Time-Sensitive Information

**Bad:**
```markdown
If running before December 2024, use old API.
After December 2024, use new API.
```

**Good:**
```markdown
## Current API
Use v2 endpoint: `/api/v2/data`

## Legacy (deprecated)
Old v1 endpoint: `/api/v1/data` (removed 2024-12)
```

### ❌ Offering Too Many Options

**Bad:**
```markdown
You can use pandas, or numpy, or polars, or dask, or...
```

**Good:**
```markdown
Use pandas for data processing:
\`\`\`python
import pandas as pd
\`\`\`

For large datasets (>10GB), consider Dask instead.
```

### ❌ Windows Path Separators

**Bad:**
```markdown
Load the file from `scripts\helpers\utils.py`
```

**Good:**
```markdown
Load the file from `scripts/helpers/utils.py`
```

## Description Anti-Patterns

### ❌ Vague Descriptions

**Bad:**
```yaml
description: Helps with stuff
description: Does things with files
description: Utility functions
```

**Good:**
```yaml
description: Validates JSON schemas and generates TypeScript interfaces. Use when working with API contracts or data validation.
```

### ❌ Wrong Person

**Bad:**
```yaml
description: I can help you process your Excel files
description: You can use this for data analysis
```

**Good:**
```yaml
description: Processes Excel files and generates statistical reports
```

### ❌ Missing Triggers

**Bad:**
```yaml
description: Processes images
```

**Good:**
```yaml
description: Processes images for resizing, format conversion, and optimization. Use when working with image files, preparing web assets, or batch processing photos.
```

## Workflow Anti-Patterns

### ❌ No Clear Steps

**Bad:**
```markdown
Just run the validation and fix any issues that come up.
```

**Good:**
```markdown
## Validation Workflow

1. Run: `python validate.py input.json`
2. If errors, check `errors.log`
3. Fix issues in source file
4. Re-run validation
5. Continue only when validation passes
```

### ❌ Missing Feedback Loops

**Bad:**
```markdown
1. Process the file
2. Generate output
3. Done
```

**Good:**
```markdown
1. Process the file
2. Generate output
3. Validate output: `python verify.py output.json`
4. If validation fails, return to step 1
5. Done when validation passes
```

## Testing Anti-Patterns

### ❌ No Real-World Testing

**Bad:**
- Creating skill from theory
- Not testing with actual Claude instance
- Assuming it will work

**Good:**
- Test with real tasks
- Iterate based on Claude's behavior
- Verify with multiple scenarios

### ❌ Testing Only Happy Path

**Bad:**
```markdown
This skill handles file processing
(No mention of error cases)
```

**Good:**
```markdown
## Error Handling

- File not found: Create default file
- Invalid format: Show specific error and example
- Permission denied: Suggest fixes
```

## Performance Anti-Patterns

### ❌ Loading Everything Upfront

**Bad:**
```markdown
# SKILL.md (2000 lines)
[All content in one file]
```

**Good:**
```markdown
# SKILL.md (150 lines)
[Overview and links]

Details in:
- reference/guide.md
- reference/examples.md
```

### ❌ Including Unnecessary Context

**Bad:**
```markdown
Claude is an AI assistant created by Anthropic.
Python is a programming language.
Files can be read and written.
```

**Good:**
(Skip obvious context - Claude already knows this)

## Common Mistakes

1. **Forgetting the user can't see internal reasoning**
   - Don't reference "the above analysis"
   - Don't assume context from previous skills

2. **Not considering token costs**
   - Every line in SKILL.md costs tokens when loaded
   - Reference files only cost when accessed

3. **Mixing concerns**
   - Keep skills focused on one domain
   - Don't combine unrelated functionality

4. **Inconsistent terminology**
   - Pick one term and stick with it
   - Don't mix "field", "attribute", "property"

5. **Platform-specific assumptions**
   - Use forward slashes for paths
   - Don't assume specific OS tools
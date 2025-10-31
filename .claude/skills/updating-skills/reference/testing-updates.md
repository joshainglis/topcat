# Testing Skill Updates

## Pre-Update Testing

Before making changes, establish baseline:

```markdown
Baseline Testing:
- [ ] Test current skill with typical task
- [ ] Note any failures or inefficiencies
- [ ] Document current behavior
- [ ] Save test cases for comparison
```

## Update Validation Process

### 1. Structural Validation

```markdown
Structure Checklist:
- [ ] SKILL.md under 200 lines
- [ ] YAML frontmatter valid
- [ ] Description under 1024 chars
- [ ] Name follows conventions
- [ ] References properly linked
- [ ] No broken internal links
```

### 2. Content Validation

```markdown
Content Checklist:
- [ ] No duplicate information
- [ ] Consistent terminology
- [ ] Third-person voice maintained
- [ ] Examples are current
- [ ] No time-sensitive content
- [ ] Forward slashes in paths
```

### 3. Functional Testing

Test the updated skill with fresh Claude instance:

```markdown
Functional Tests:
- [ ] Primary use case works
- [ ] New features work as intended
- [ ] Old features still work
- [ ] Skill triggers appropriately
- [ ] References load when needed
```

## Test Scenarios

### Scenario 1: Basic Functionality

```markdown
Test: Original core feature
Expected: Works as before or better
```

### Scenario 2: New Feature

```markdown
Test: New capability added in update
Expected: Feature works correctly
```

### Scenario 3: Edge Cases

```markdown
Test: Boundary conditions
Expected: Graceful handling or clear errors
```

### Scenario 4: Non-Triggering

```markdown
Test: Unrelated task
Expected: Skill doesn't trigger inappropriately
```

## A/B Testing Updates

Compare old vs new versions:

```markdown
Comparison Test:
1. Task A with old skill → Note: time, tokens, success
2. Task A with new skill → Note: time, tokens, success
3. Compare results:
   - Which was faster?
   - Which used fewer tokens?
   - Which was more successful?
   - Which had clearer output?
```

## Regression Testing

Ensure updates don't break existing functionality:

```markdown
Regression Checklist:
- [ ] All previously working examples still work
- [ ] No removed features still referenced
- [ ] Dependencies between files intact
- [ ] Cross-references still valid
- [ ] No orphaned reference files
```

## Performance Testing

### Token Usage

```python
# Rough token estimation
def estimate_tokens(text):
    # Approximate: 1 token ≈ 4 characters
    return len(text) / 4

# Compare before/after
old_tokens = estimate_tokens(old_skill_content)
new_tokens = estimate_tokens(new_skill_content)
print(f"Token change: {new_tokens - old_tokens:+.0f}")
```

### Load Testing

```markdown
Load Test:
1. Count which files get loaded for common task
2. Note total characters loaded
3. Compare with previous version
4. Aim for reduction or no increase
```

## User Acceptance Testing

### Test with Different User Styles

```markdown
User Personas:
1. Beginner: "How do I [basic task]?"
2. Intermediate: "[Standard task] with [specific requirement]"
3. Expert: "[Complex scenario] optimizing for [criterion]"
```

### Test with Variations

```markdown
Phrasing Variations:
- "I need to [task]"
- "Help me [task]"
- "Can you [task]"
- "[Task] please"
- Direct: "[specific technical term]"
```

## Integration Testing

Test skill interactions:

```markdown
Integration Tests:
- [ ] Skill works alongside related skills
- [ ] No naming conflicts
- [ ] Clear differentiation from similar skills
- [ ] Cross-references work if present
```

## Update Testing Workflow

```
Complete Testing Flow:
- [ ] Run structural validation
- [ ] Run content validation
- [ ] Test with Claude (fresh instance)
- [ ] Test primary use cases
- [ ] Test new features
- [ ] Run regression tests
- [ ] Check token usage
- [ ] Test with variations
- [ ] Document test results
```

## Common Testing Failures

### Skill Doesn't Trigger

**Diagnosis**:
```markdown
- Check description keywords
- Verify description includes use cases
- Test with explicit mention: "use the [skill-name] skill"
```

### Wrong Behavior

**Diagnosis**:
```markdown
- Check if instructions are ambiguous
- Verify examples match instructions
- Ensure critical info isn't in references
```

### Performance Degradation

**Diagnosis**:
```markdown
- Check if SKILL.md grew too large
- Look for redundant content
- Verify progressive disclosure working
```

## Test Documentation

Document test results:

```markdown
# Test Report: skill-name Update

## Summary
- Date: YYYY-MM-DD
- Version: Before 1.0 → After 2.0
- Tester: [Name]

## Structural Changes
- SKILL.md: 400 lines → 150 lines
- References: 3 files → 5 files
- Total size: Reduced by 30%

## Functional Testing
✅ Primary use case: Pass
✅ New feature X: Pass
✅ Regression tests: Pass
⚠️ Edge case Y: Minor issue (documented)

## Performance
- Token usage: -20% for common tasks
- Triggering: Improved accuracy
- Reference loading: More selective

## Recommendations
- Monitor edge case Y
- Consider extracting section Z in future
```

## Rollback Criteria

Consider rolling back if:

```markdown
Rollback Triggers:
- [ ] Skill fails to trigger for primary use case
- [ ] Critical functionality broken
- [ ] Token usage increased >50%
- [ ] Multiple regression failures
- [ ] Confusing or incorrect behavior
```

## Continuous Improvement

After deployment:

```markdown
Monitoring:
- [ ] Collect user feedback
- [ ] Note any confusion points
- [ ] Track success/failure patterns
- [ ] Plan next iteration
```
# Testing Skills Effectively

## Testing Workflow

### 1. Baseline Test (Without Skill)

Work through your task with Claude without the skill:

```markdown
User: [Give Claude the task]
Claude: [Attempts task, may ask for context]
User: [Provide necessary context]
Claude: [Completes task]
```

Note what context you had to provide - this becomes skill content.

### 2. Create Initial Skill

Ask Claude to create a skill from the interaction:

```markdown
"Create a skill that captures the context and workflow we just used"
```

### 3. Test Skill Discovery

With a fresh Claude instance:

```markdown
User: [Same task as before]
[Check: Does the skill trigger?]
```

If skill doesn't trigger, refine the description:
- Add more specific keywords
- Include common task variations
- Clarify when to use it

### 4. Test Skill Effectiveness

Observe Claude using the skill:

```markdown
Questions to answer:
- Does Claude follow the workflow?
- Are all necessary steps completed?
- Is the output correct?
- Are there unnecessary steps?
```

### 5. Test Progressive Disclosure

Monitor which files get loaded:

```markdown
- Check if SKILL.md is sufficient
- Note which reference files are accessed
- Identify unused reference files
- Look for missing information
```

## Testing Scenarios

### Scenario 1: Basic Task

Test the most common use case:
```markdown
"[Primary task the skill is designed for]"
```

Expected: Skill triggers, completes successfully

### Scenario 2: Edge Case

Test boundary conditions:
```markdown
"[Unusual but valid use case]"
```

Expected: Skill handles gracefully or indicates limitations

### Scenario 3: Adjacent Task

Test something similar but outside scope:
```markdown
"[Related task the skill shouldn't handle]"
```

Expected: Skill doesn't trigger inappropriately

## Testing with Different Models

### Haiku Testing

Focus on clarity:
- Are instructions explicit enough?
- Does it need more guidance?
- Are examples clear?

### Sonnet Testing

Balance check:
- Is the skill efficient?
- Good balance of freedom vs structure?

### Opus Testing

Avoid over-explaining:
- Remove unnecessary context
- Trust Opus's reasoning
- Focus on domain-specific knowledge

## Iteration Based on Testing

### Common Refinements

1. **Skill doesn't trigger**
   - Add keywords to description
   - Make triggers more explicit
   - Include "Use when..." phrases

2. **Missing information**
   - Add to SKILL.md if essential
   - Create reference file if detailed
   - Include examples if unclear

3. **Too verbose**
   - Remove obvious explanations
   - Extract details to references
   - Use examples over descriptions

4. **Wrong behavior**
   - Clarify ambiguous instructions
   - Add validation steps
   - Provide explicit workflows

## Testing Checklist

Before considering skill complete:

- [ ] Tested with primary use case
- [ ] Tested with edge cases
- [ ] Tested with fresh Claude instance
- [ ] Verified progressive disclosure works
- [ ] Checked with target model (Haiku/Sonnet/Opus)
- [ ] Refined based on observations
- [ ] Re-tested after refinements

## A/B Testing Pattern

Compare skill versions:

```markdown
1. Create variant A and variant B
2. Test same tasks with each
3. Measure:
   - Task completion success
   - Token usage
   - Time to complete
   - Error frequency
4. Choose better performer
```

## Debugging Failed Skills

### Skill Not Triggering

```markdown
Debug steps:
1. Check description keywords
2. Verify name format (lowercase, hyphens)
3. Test with explicit mention: "use the [skill-name] skill"
4. Review description length (<1024 chars)
```

### Incorrect Behavior

```markdown
Debug steps:
1. Read through SKILL.md as Claude would
2. Check for ambiguous instructions
3. Verify examples match instructions
4. Test each workflow step individually
```

### Performance Issues

```markdown
Debug steps:
1. Check SKILL.md line count (<200 ideal)
2. Monitor which references load
3. Look for redundant content
4. Consider splitting into multiple skills
```
# CI/CD Integration

## Quiet Mode

Use `--quiet` flag to suppress all output and rely on exit codes:

```bash
topcat analyze -i sql/ -e sql --quiet cycles
echo $?  # 0 = no cycles, 1 = cycles found
```

**Exit codes**:
- `0` - Success (no issues found)
- `1` - Issues found (cycles, missing deps, etc.)

## Pipeline Examples

### GitHub Actions

```yaml
name: Check SQL Dependencies

on: [push, pull_request]

jobs:
  check-cycles:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Topcat
        run: cargo install --git https://github.com/your/topcat

      - name: Check for cycles
        run: topcat analyze -i sql/ -e sql --quiet cycles

      - name: Check for missing dependencies
        run: topcat analyze -i sql/ -e sql --quiet missing
```

### GitLab CI

```yaml
sql-dependency-check:
  stage: test
  script:
    - cargo install --git https://github.com/your/topcat
    - topcat analyze -i sql/ -e sql --quiet cycles
    - topcat analyze -i sql/ -e sql --quiet missing
  only:
    changes:
      - sql/**/*
```

### Jenkins

```groovy
pipeline {
    agent any

    stages {
        stage('Check SQL Dependencies') {
            steps {
                sh '''
                    cargo install --git https://github.com/your/topcat
                    topcat analyze -i sql/ -e sql --quiet cycles
                    topcat analyze -i sql/ -e sql --quiet missing
                '''
            }
        }
    }
}
```

### CircleCI

```yaml
version: 2.1

jobs:
  check-dependencies:
    docker:
      - image: rust:latest
    steps:
      - checkout
      - run:
          name: Install Topcat
          command: cargo install --git https://github.com/your/topcat
      - run:
          name: Check for cycles
          command: topcat analyze -i sql/ -e sql --quiet cycles
      - run:
          name: Check for missing dependencies
          command: topcat analyze -i sql/ -e sql --quiet missing

workflows:
  version: 2
  test:
    jobs:
      - check-dependencies
```

## Pre-commit Hooks

### Git Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash

echo "Checking SQL dependencies..."

# Check for cycles
if ! topcat analyze -i sql/ -e sql --quiet cycles; then
    echo "❌ Commit rejected: Circular dependencies detected!"
    echo "Run: topcat analyze -i sql/ -e sql cycles"
    exit 1
fi

# Check for missing dependencies
if ! topcat analyze -i sql/ -e sql --quiet missing; then
    echo "❌ Commit rejected: Missing dependencies detected!"
    echo "Run: topcat analyze -i sql/ -e sql missing"
    exit 1
fi

echo "✅ SQL dependencies check passed"
exit 0
```

Make executable:
```bash
chmod +x .git/hooks/pre-commit
```

### pre-commit Framework

Create `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: check-sql-cycles
        name: Check SQL cycles
        entry: topcat analyze -i sql/ -e sql --quiet cycles
        language: system
        pass_filenames: false
        files: \.sql$

      - id: check-sql-missing
        name: Check SQL missing deps
        entry: topcat analyze -i sql/ -e sql --quiet missing
        language: system
        pass_filenames: false
        files: \.sql$
```

## Automated Cleanup

### Scheduled Cleanup Job

GitHub Actions example:

```yaml
name: Cleanup Dead SQL Files

on:
  schedule:
    - cron: '0 2 * * 0'  # Weekly at 2 AM Sunday
  workflow_dispatch:  # Allow manual trigger

jobs:
  cleanup:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Topcat
        run: cargo install --git https://github.com/your/topcat

      - name: Clean orphaned files
        run: |
          topcat clean -i sql/ -e sql \
            --root-pattern "**/api/*.sql" \
            --root-pattern "**/migrations/*.sql" \
            --external-check-dir src/ \
            --external-check-pattern "*.py" \
            orphans --no-dry-run --force

      - name: Create Pull Request
        uses: peter-evans/create-pull-request@v5
        with:
          commit-message: "chore: remove orphaned SQL files"
          title: "Automated SQL cleanup"
          body: "Automatically removed orphaned SQL files that are not referenced."
          branch: automated-sql-cleanup
```

### Cron Job

```bash
#!/bin/bash
# /etc/cron.weekly/cleanup-sql

cd /path/to/project

# Clean orphans with protection
topcat clean -i sql/ -e sql \
  --sql-config topcat.toml \
  orphans --no-dry-run --force

# Commit changes
git add -A
git commit -m "chore: automated SQL cleanup"
git push
```

## Pull Request Checks

### GitHub Actions PR Comment

```yaml
name: SQL Dependency Report

on:
  pull_request:
    paths:
      - 'sql/**'

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Topcat
        run: cargo install --git https://github.com/your/topcat

      - name: Analyze dependencies
        id: analyze
        run: |
          echo "## SQL Dependency Analysis" > report.md
          echo "" >> report.md

          echo "### Dead Branches" >> report.md
          topcat analyze -i sql/ -e sql dead-branches >> report.md || true

          echo "" >> report.md
          echo "### Orphans" >> report.md
          topcat analyze -i sql/ -e sql orphans >> report.md || true

      - name: Comment PR
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const report = fs.readFileSync('report.md', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: report
            });
```

## Validation Scripts

### Comprehensive Check Script

```bash
#!/bin/bash
# validate-sql.sh

set -e

echo "🔍 Running SQL dependency validation..."

# Check for cycles
echo "Checking for circular dependencies..."
if ! topcat analyze -i sql/ -e sql --quiet cycles; then
    echo "❌ FAILED: Circular dependencies found"
    topcat analyze -i sql/ -e sql cycles
    exit 1
fi
echo "✅ No cycles detected"

# Check for missing dependencies
echo "Checking for missing dependencies..."
if ! topcat analyze -i sql/ -e sql --quiet missing; then
    echo "❌ FAILED: Missing dependencies found"
    topcat analyze -i sql/ -e sql missing
    exit 1
fi
echo "✅ No missing dependencies"

# Report dead branches (informational only)
echo "Checking for dead branches..."
topcat analyze -i sql/ -e sql \
  --root-pattern "**/api/*.sql" \
  --external-check-dir src/ \
  --external-check-pattern "*.py" \
  dead-branches || true

echo "✅ SQL validation complete"
```

### Makefile Integration

```makefile
.PHONY: check-sql clean-sql validate-sql

check-sql:
	@echo "Checking SQL dependencies..."
	@topcat analyze -i sql/ -e sql --quiet cycles
	@topcat analyze -i sql/ -e sql --quiet missing
	@echo "✅ SQL checks passed"

clean-sql:
	@echo "Cleaning unused SQL files (dry-run)..."
	@topcat clean -i sql/ -e sql \
		--sql-config topcat.toml \
		dead-branches

clean-sql-force:
	@echo "Cleaning unused SQL files..."
	@topcat clean -i sql/ -e sql \
		--sql-config topcat.toml \
		dead-branches --no-dry-run --force
	@echo "✅ Cleanup complete"

validate-sql: check-sql
	@echo "✅ SQL validation complete"

# Add to CI target
ci: test lint validate-sql
```

## Docker Integration

### Dockerfile

```dockerfile
FROM rust:latest as builder
RUN cargo install --git https://github.com/your/topcat

FROM debian:bullseye-slim
COPY --from=builder /usr/local/cargo/bin/topcat /usr/local/bin/topcat
WORKDIR /workspace
ENTRYPOINT ["topcat"]
```

### Usage in CI

```yaml
check-dependencies:
  image: your-registry/topcat:latest
  script:
    - topcat analyze -i sql/ -e sql --quiet cycles
    - topcat analyze -i sql/ -e sql --quiet missing
```

## Monitoring and Alerts

### Slack Notifications

```bash
#!/bin/bash

# Run analysis
if ! topcat analyze -i sql/ -e sql --quiet cycles; then
    # Send Slack alert
    curl -X POST -H 'Content-type: application/json' \
        --data '{"text":"⚠️ SQL circular dependencies detected!"}' \
        $SLACK_WEBHOOK_URL
    exit 1
fi
```

### Email Alerts

```bash
#!/bin/bash

# Weekly dead branches report
REPORT=$(topcat analyze -i sql/ -e sql dead-branches)

if [ -n "$REPORT" ]; then
    echo "$REPORT" | mail -s "Weekly SQL Dead Branches Report" team@example.com
fi
```

## Best Practices

### 1. Fail Fast

Check for blocking issues (cycles, missing deps) before non-blocking issues:

```bash
# These must pass
topcat analyze -i sql/ -e sql --quiet cycles || exit 1
topcat analyze -i sql/ -e sql --quiet missing || exit 1

# These are informational
topcat analyze -i sql/ -e sql dead-branches || true
```

### 2. Use Config Files

Store protection rules in version control:

```toml
# topcat.toml (committed to repo)
[analysis]
root_patterns = ["**/api/*.sql", "**/migrations/*.sql"]
```

```bash
# CI uses config file
topcat analyze -i sql/ -e sql --sql-config topcat.toml dead-branches
```

### 3. Separate Validation from Cleanup

- **CI**: Run validation checks (cycles, missing)
- **Scheduled jobs**: Run cleanup (weekly/monthly)
- **Never** cleanup on every commit (too aggressive)

### 4. Use Dry-Run in CI

Preview cleanup changes before executing:

```bash
# Show what would be cleaned
topcat clean -i sql/ -e sql dead-branches

# Create PR for review
git checkout -b automated-cleanup
topcat clean -i sql/ -e sql dead-branches --no-dry-run --force
git commit -am "chore: cleanup dead SQL files"
# Create PR (requires review before merge)
```

### 5. Cache Dependencies

Speed up CI by caching Topcat installation:

```yaml
# GitHub Actions
- name: Cache Topcat
  uses: actions/cache@v3
  with:
    path: ~/.cargo/bin/topcat
    key: topcat-${{ runner.os }}

- name: Install Topcat
  run: |
    if [ ! -f ~/.cargo/bin/topcat ]; then
      cargo install --git https://github.com/your/topcat
    fi
```

## Exit Code Reference

| Command | Exit 0 | Exit 1 |
|---------|--------|--------|
| `analyze cycles --quiet` | No cycles | Cycles found |
| `analyze missing --quiet` | No missing | Missing deps |
| `analyze dead-branches` | Always 0 | N/A |
| `analyze orphans` | Always 0 | N/A |
| `clean --no-dry-run` | Success | Failed to delete |

## Troubleshooting CI

### False Positives

If CI reports issues incorrectly:

```bash
# Run locally with verbose mode
topcat analyze -i sql/ -e sql cycles

# Check if protection patterns are applied
topcat analyze -i sql/ -e sql \
  --sql-config topcat.toml \
  --external-check-dir src/ \
  dead-branches
```

### Performance Issues

For large codebases:

```bash
# Use schema filtering
topcat analyze -i sql/ -e sql --schema critical_schema cycles

# Limit external checking scope
topcat analyze -i sql/ -e sql \
  --external-check-dir src/api \
  --external-check-pattern "*.py" \
  dead-branches
```

### Timeout Issues

```yaml
# Increase timeout in CI
- name: Check dependencies
  run: topcat analyze -i sql/ -e sql --quiet cycles
  timeout-minutes: 10
```

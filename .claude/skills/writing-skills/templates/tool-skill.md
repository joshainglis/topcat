---
name: using-toolname
description: Provides comprehensive guidance for using [tool] to [accomplish what]. Use when [working scenarios], [debugging situations], or [other use cases].
---

# Using [Tool Name]

## Installation

```bash
# Install command
install-command toolname
```

## Quick Reference

| Command | Purpose | Example |
|---------|---------|---------|
| `tool init` | Initialize project | `tool init myproject` |
| `tool run` | Execute operation | `tool run --input file` |
| `tool test` | Run tests | `tool test --coverage` |

## Common Operations

### Basic Usage

```bash
# Most common command pattern
tool [action] [target] [options]

# Example
tool process input.txt --output result.txt
```

### Configuration

```yaml
# config.yml
setting1: value
setting2: value
options:
  - option1
  - option2
```

## Workflows

### Development Workflow

```
Development Checklist:
- [ ] Initialize: tool init
- [ ] Configure: Edit config.yml
- [ ] Develop: Make changes
- [ ] Test: tool test
- [ ] Build: tool build
- [ ] Deploy: tool deploy
```

### Debugging Workflow

```bash
# Enable debug mode
tool --debug command

# Check logs
tool logs --tail 100

# Validate configuration
tool validate config.yml
```

## Advanced Usage

**Performance tuning**: See [reference/performance.md](reference/performance.md)
**Custom plugins**: See [reference/plugins.md](reference/plugins.md)
**API reference**: See [reference/api.md](reference/api.md)

## Troubleshooting

| Error               | Solution                |
|---------------------|-------------------------|
| "Command not found" | Check PATH or reinstall |
| "Invalid config"    | Run `tool validate`     |
| "Permission denied" | Check file permissions  |

For more issues, see [reference/troubleshooting.md](reference/troubleshooting.md)
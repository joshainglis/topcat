# SQL Pattern Matching Reference

## Pattern Categories

### DDL Statement Patterns

Patterns for extracting object names from DDL:

```regex
# Tables
CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?(\S+)
ALTER\s+TABLE\s+(\S+)
DROP\s+TABLE\s+(?:IF\s+EXISTS\s+)?(\S+)

# Views
CREATE\s+(?:OR\s+REPLACE\s+)?VIEW\s+(\S+)
CREATE\s+MATERIALIZED\s+VIEW\s+(\S+)

# Functions/Procedures
CREATE\s+(?:OR\s+REPLACE\s+)?FUNCTION\s+(\S+)\s*\(
CREATE\s+(?:OR\s+REPLACE\s+)?PROCEDURE\s+(\S+)\s*\(
```

### Reference Patterns

Patterns for finding dependencies in queries:

```regex
# Schema.object references
(?:FROM|JOIN)\s+([a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*)

# Function calls
([a-z_][a-z0-9_]*\.[a-z_][a-z0-9_]*)\s*\(

# Type casts
::\s*([A-Z_][A-Z0-9_]*)
```

## Pattern Customization

### Model Generation Patterns

For code generation procedures:

```toml
# Custom model generation pattern
model_pattern = "CALL\\s+(\\w+\\.generate_\\w+_model)\\s*\\("
```

### Excluding Patterns

Patterns to ignore during discovery:

```toml
# Ignore temporary tables
exclude_pattern = "(?:tmp|temp)_\\w+"

# Ignore system schemas
exclude_schemas = ["pg_catalog", "information_schema", "pg_toast"]
```

## Complex Scenarios

### Multi-line Statements

```sql
-- Discovery handles multi-line CREATE statements
CREATE TABLE schema.users (
    id SERIAL PRIMARY KEY,
    role_id INTEGER REFERENCES schema.roles(id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

Extracted dependencies:
- `schema.users` (object being created)
- `schema.roles` (foreign key reference)

### Nested Subqueries

```sql
-- Nested references are discovered
SELECT * FROM (
    SELECT u.* FROM app.users u
    JOIN app.departments d ON u.dept_id = d.id
    WHERE EXISTS (
        SELECT 1 FROM app.permissions p
        WHERE p.user_id = u.id
    )
) AS filtered_users;
```

Extracted dependencies:
- `app.users`
- `app.departments`
- `app.permissions`

### Dynamic SQL

```sql
-- Dynamic SQL requires manual override
EXECUTE format('CREATE TABLE %I.%I (...)', schema_name, table_name);
-- Use: -- requires: !expected_schema.expected_table
```
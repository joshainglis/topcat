# Schema Naming Patterns

## Supported Separators

### Dot Notation (`.`)

Most common in SQL, especially PostgreSQL:

```sql
-- name: auth.users_table
CREATE TABLE auth.users (...);

-- name: billing.invoices_table
CREATE TABLE billing.invoices (...);
```

**Extraction**:
- Node name: `auth.users_table`
- Schema: `auth`

### Double Colon (`::`)

Common in some SQL dialects and programming languages:

```sql
-- name: auth::create_user_function
CREATE FUNCTION auth.create_user(...);

-- name: billing::calculate_invoice
CREATE FUNCTION billing.calculate_invoice(...);
```

**Extraction**:
- Node name: `auth::create_user_function`
- Schema: `auth`

### No Separator

Files without schema qualification:

```sql
-- name: standalone_util
CREATE FUNCTION standalone_util(...);
```

**Extraction**:
- Node name: `standalone_util`
- Schema: `None`

## Multi-Segment Names

Only the **first segment** is used as schema:

### Dot Notation
```sql
-- name: auth.users.table
-- Schema: "auth" (first segment)
-- Node: "auth.users.table"

-- name: my_app.v2.api.handler
-- Schema: "my_app"
-- Node: "my_app.v2.api.handler"
```

### Double Colon
```sql
-- name: auth::users::create
-- Schema: "auth"
-- Node: "auth::users::create"
```

## Naming Conventions

### PostgreSQL Schema Pattern

```sql
-- Schema definition
-- name: auth
CREATE SCHEMA IF NOT EXISTS auth;

-- Tables in schema
-- name: auth.users
CREATE TABLE auth.users (...);

-- name: auth.sessions
CREATE TABLE auth.sessions (...);

-- Functions in schema
-- name: auth.create_user
CREATE FUNCTION auth.create_user(...);
```

### Microservice Pattern

Each service gets its own schema:

```sql
-- User service
-- name: users.accounts_table
-- name: users.profiles_table

-- Order service
-- name: orders.orders_table
-- name: orders.items_table

-- Billing service
-- name: billing.invoices_table
-- name: billing.payments_table
```

### Feature-Based Pattern

Group by feature/domain:

```sql
-- Authentication feature
-- name: auth.login_function
-- name: auth.logout_function

-- Reporting feature
-- name: reports.generate_daily
-- name: reports.generate_monthly
```

## Best Practices

### 1. Be Consistent

Choose one separator style and stick with it:

**Good**:
```sql
-- name: auth.users
-- name: auth.sessions
-- name: billing.invoices
```

**Bad** (mixed):
```sql
-- name: auth.users
-- name: auth::sessions
-- name: billing.invoices
```

### 2. Use Descriptive Schema Names

**Good**:
```sql
auth.users
billing.invoices
analytics.reports
```

**Bad**:
```sql
a.users
db1.invoices
s1.reports
```

### 3. Match SQL Schema Names

Align node names with actual SQL schemas:

```sql
-- If your SQL uses:
CREATE SCHEMA auth;
CREATE TABLE auth.users (...);

-- Then use:
-- name: auth.users_table
```

### 4. Avoid Deeply Nested Names

**Good**:
```sql
-- name: auth.users
-- name: auth.api_login
```

**Avoid**:
```sql
-- name: auth.api.v2.handlers.login.create
-- (Only "auth" is schema, rest is just part of node name)
```

## Schema Filtering Examples

### Single Schema

```bash
# Only analyze auth schema
topcat analyze -i sql/ -e sql --schema auth dead-branches
```

Matches:
- `auth.users`
- `auth.sessions`
- `auth::anything` (if using `::`)

Does NOT match:
- `billing.users`
- `standalone_function`
- `public.auth_log` (different schema)

### Multiple Schemas

```bash
topcat analyze -i sql/ -e sql \
  --schema auth \
  --schema billing \
  orphans
```

Matches:
- `auth.users`
- `billing.invoices`

Does NOT match:
- `public.users`
- `analytics.reports`

### Schema with Root Protection

```bash
topcat clean -i sql/ -e sql \
  --schema auth \
  --root-pattern "auth.api_*" \
  dead-branches --no-dry-run
```

Only considers files in `auth` schema, and protects any matching `auth.api_*` pattern.

## Migration Patterns

### Adding Schema to Existing Files

If you have files without schemas:

```sql
-- Old:
-- name: users_table

-- New:
-- name: auth.users_table
```

### Splitting Schemas

Moving from monolithic to multi-schema:

```sql
-- Old (no schema):
-- name: users_table
-- name: invoices_table
-- name: reports_table

-- New (with schemas):
-- name: auth.users_table
-- name: billing.invoices_table
-- name: analytics.reports_table
```

Steps:
1. Add schema prefixes to node names
2. Update dependencies in headers
3. Run `topcat analyze` to verify
4. Update SQL CREATE statements to use schemas

## SQL Discovery Integration

When using SQL discovery with schemas:

### Type Mappings

```toml
[[sql_discovery.type_mappings]]
from = "JSONB"
to = "pg_catalog.jsonb"
# Schema-qualified type mapping
```

### Schema Pattern

```toml
[sql_discovery]
schema_pattern = "(?:auth|billing|public)_\\w+"
```

This helps discovery recognize schema-qualified references in SQL code.

## Troubleshooting

### Schema Not Extracted

**Symptom**: File has schema in name but `schema list` doesn't show it.

**Check**:
```bash
# View node names
topcat analyze -i sql/ -e sql root-nodes

# If node name is "auth_users" (underscore, not dot)
# No schema will be extracted
```

**Solution**: Use `.` or `::` separator:
```sql
-- name: auth.users  (not auth_users)
```

### Wrong Schema Extracted

**Symptom**: Multi-segment name extracts wrong schema.

**Example**:
```sql
-- name: v1.auth.users
-- Schema: "v1" (not "auth")
```

**Solution**: Put schema first:
```sql
-- name: auth.v1_users
-- Schema: "auth"
```

### Schema Filter Not Working

**Symptom**: `--schema auth` doesn't filter expected files.

**Debug**:
```bash
# Check schema names
topcat schema -i sql/ -e sql list

# Check if nodes have schema
topcat schema -i sql/ -e sql analyze auth
```

**Common causes**:
- Typo in schema name
- Using underscore instead of dot
- Schema defined differently in node names

# SQL Discovery Configuration Reference

## TOML Configuration Structure

```toml
[sql_discovery]
enabled = true
schema_pattern = "(?:schema1|schema2)_\\w+"
merge_strategy = "discovery-only"

# Type mappings for custom types
[[sql_discovery.type_mappings]]
from = "TSTZRANGE"
to = "c_tmf.t_time_period"

[[sql_discovery.type_mappings]]
from = "JSONB"
to = "pg_catalog.jsonb"

# Extension mappings for PostgreSQL
[[sql_discovery.extension_mappings]]
object = "digest"
extension = "pgcrypto"

[[sql_discovery.extension_mappings]]
object = "uuid_generate_v4"
extension = "uuid-ossp"
```

## CLI Arguments Reference

| Argument | Description | Example |
|----------|-------------|---------|
| `--enable-sql-discovery` | Enable automatic discovery | |
| `--sql-config PATH` | Path to TOML config file | `--sql-config topcat.toml` |
| `--schema-pattern REGEX` | Schema name pattern | `--schema-pattern "app_\\w+"` |
| `--merge-strategy STRATEGY` | Dependency merge strategy | `--merge-strategy union` |
| `--update-headers` | Update files in-place | |
| `--generate-headers DIR` | Generate updated files to directory | `--generate-headers /tmp/updated` |

## Schema Pattern Examples

```toml
# Single schema prefix
schema_pattern = "myapp_\\w+"

# Multiple schema prefixes
schema_pattern = "(?:app|test|staging)_\\w+"

# Complex pattern with specific schemas
schema_pattern = "(?:c|d[pio]|codegen|md)_\\w+"

# All schemas (be careful with this)
schema_pattern = "\\w+"
```

## Type Mappings

Map SQL type casts to their definitions:

```toml
[[sql_discovery.type_mappings]]
from = "UUID"
to = "pg_catalog.uuid"

[[sql_discovery.type_mappings]]
from = "TSTZRANGE"
to = "temporal.time_range"

[[sql_discovery.type_mappings]]
from = "GEOMETRY"
to = "postgis.geometry"
```

## Extension Mappings

Map functions to required extensions:

```toml
# PostGIS functions
[[sql_discovery.extension_mappings]]
object = "ST_Transform"
extension = "postgis"

# UUID functions
[[sql_discovery.extension_mappings]]
object = "uuid_generate_v4"
extension = "uuid-ossp"

# Cryptographic functions
[[sql_discovery.extension_mappings]]
object = "digest"
extension = "pgcrypto"

# Full text search
[[sql_discovery.extension_mappings]]
object = "to_tsvector"
extension = "pg_trgm"
```
-- This file was generated by topcat. To regenerate run:
--
-- topcat -i tests/input/sql -o tests/output/sql/output.sql

------------------------------------------------------------------------------------------------------------------------
-- tests/input/sql/my_other_schema/schema.sql
-- name: my_schema

DROP SCHEMA IF EXISTS my_schema CASCADE;
CREATE SCHEMA IF NOT EXISTS my_schema;

------------------------------------------------------------------------------------------------------------------------
-- tests/input/sql/my_other_schema/functions/a.sql
-- name: my_schema.a
-- dropped_by: my_schema

CREATE FUNCTION my_schema.a() RETURNS INT AS
$$
SELECT 1;
$$ LANGUAGE SQL;

------------------------------------------------------------------------------------------------------------------------
-- tests/input/sql/my_other_schema/functions/b.sql
-- name: my_schema.b
-- dropped_by: my_schema
-- requires: my_schema.a

CREATE FUNCTION my_schema.b() RETURNS INT AS
$$
SELECT my_schema.a() + 1
$$ LANGUAGE SQL;

------------------------------------------------------------------------------------------------------------------------
-- tests/input/sql/my_schema/schema.sql
-- name: my_other_schema

DROP SCHEMA IF EXISTS my_other_schema CASCADE;
CREATE SCHEMA IF NOT EXISTS my_other_schema;

------------------------------------------------------------------------------------------------------------------------
-- tests/input/sql/my_schema/functions/a.sql
-- name: my_other_schema.a
-- dropped_by: my_other_schema
-- requires: my_schema.b

CREATE FUNCTION my_other_schema.a() RETURNS INT AS
$$
SELECT my_schema.b() + 1
$$ LANGUAGE SQL IMMUTABLE
                PARALLEL SAFE;

------------------------------------------------------------------------------------------------------------------------
-- tests/input/sql/my_other_schema/functions/c.sql
-- name: my_schema.c
-- dropped_by: my_schema
-- requires: my_schema.b
-- requires: my_other_schema.a

CREATE FUNCTION my_schema.c() RETURNS INT AS
$$
SELECT my_schema.b() + my_other_schema.a() + 1
$$ LANGUAGE SQL;


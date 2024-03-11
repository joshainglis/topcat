-- name: my_other_schema.a
-- dropped_by: my_other_schema
-- requires: my_schema.b

CREATE FUNCTION my_other_schema.a() RETURNS INT AS
$$
SELECT my_schema.b() + 1
$$ LANGUAGE SQL IMMUTABLE
                PARALLEL SAFE;

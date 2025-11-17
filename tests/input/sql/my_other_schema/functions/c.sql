-- name: my_schema.c
-- dropped_by: my_schema
-- requires: my_schema.b
-- requires: my_other_schema.a

CREATE FUNCTION my_schema.c() RETURNS INT AS
$$
SELECT my_schema.b() + my_other_schema.a() + 1
$$ LANGUAGE SQL;

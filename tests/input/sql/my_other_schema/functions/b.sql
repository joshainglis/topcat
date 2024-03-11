-- name: my_schema.b
-- dropped_by: my_schema
-- requires: my_schema.a

CREATE FUNCTION my_schema.b() RETURNS INT AS
$$
SELECT my_schema.a() + 1
$$ LANGUAGE SQL;

-- name: my_schema.a
-- dropped_by: my_schema

CREATE FUNCTION my_schema.a() RETURNS INT AS
$$
SELECT 1;
$$ LANGUAGE SQL;

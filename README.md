# topcat

**top**ological con**cat**enation of files

## Description

`topcat` is a simple tool to concatenate files in a topological order. It is useful when you have a set of files that
depend on each other and you want to concatenate them in the right order.

For my use case this is SQL files.

I like to treat my SQL files as a set of functions and views that depend on each other. I like to keep them in separate
files and concatenate them in the right order to create a single file that I can run in my database.

## Installation

pip:

```sh
pip install topcat
```

poetry:

```sh
poetry add topcat
```

## Usage

### The quick version

```sh
topcat -i /path/to/input -o /path/to/output.sql
```

Where `/path/to/input` is the directory containing the files to concatenate and `/path/to/output.sql` will be where the
concatenated file will be written.

### The long version

```sh
USAGE:
    topcat [FLAGS] [OPTIONS] --output <FILE>

FLAGS:
        --dry        Only print the output, do not write to file.
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v, --verbose    Print debug information

OPTIONS:
        --comment-str <comment-str>
            The string used to denote a comment. eg '--' [default: --]

        --ensure-each-file-ends-with <ensure-each-file-ends-with-str>
            Add this string to the end of files if it does not exist. eg ';' [default: ;]

    -x, --exclude <PATTERN>...                                           Exclude files matching given glob pattern
        --file-separator-str <file-separator-str>
            Add this between each concatenated file in the output. eg '---' [default:
            ------------------------------------------------------------------------------------------------------------------------]
    -n, --include <PATTERN>...                                           Only include files matching glob pattern
    -i, --input_dir <DIR>...
            Path to directory containing files to be concatenated

    -o, --output <FILE>                                                  Path to generate combined output file
```

Some quirks here:

- `-i` is the input directory. You can have multiple input directories. This is useful if you have a set of files in
  different directories that depend on each other.
- `-o` is the output file. This is where the concatenated file will be written.
- `-x` and `-n` are used to exclude and include files respectively. These are glob patterns. For example `-x
  **/tests/*` will exclude all files in any `tests` directory. `-n **/functions/*` will **only** include files in the
  `functions` directory. You can use these together to include and exclude files as you need. You can use these multiple
  times.
- `--comment-str` is the string used to denote a comment. This is used to find the `name`, `requires`, `dropped_by` and
  `exists` comments in the files. The default is `--`. In SQL this is `--` but in other languages it might be `//`
  or `#`.
- `--ensure-each-file-ends-with` is the string to add to the end of each file if it doesn't exist. This is useful for
  SQL
  files where you might want to ensure each file ends with a `;`. The default is `;`.
- `--file-separator-str` is the string to add between each concatenated file in the output. The default is a long line
  of
  dashes. This is just visually useful to see where one file ends and the next begins.
- `--dry` will only print the output, it will not write to the output file.
- `-v` will print debug information and a `.dot` format of the dependency graph.

## What a file needs to include to be concatenated

### `name`

The only requirement for a file to be included in the concatenation is that it needs to have a `name` comment at the top
of the file.

This can be anything you want, but it needs to be unique. This is used to define a node in the dependency graph.

For example:

```postgresql
-- name: my_schema
```

### `requires`

If a file requires another file to be concatenated before it, you can add a `requires` comment to the file.
An alias for `requires` is `dropped_by`. I use `dropped_by` in SQL files for clarity to show that the DDL in the file
gets dropped so I don't need to use `CREATE OR REPLACE FUNCTION` or the like.

For example:

```postgresql
-- name: my_schema.b
-- dropped_by: my_schema
-- requires: my_schema.a
```

### `exists`

`exists` is for soft dependencies. For example in plpgsql functions, the body isn't parsed until the function is called.
So any dependent objects you can't use `requires` for, you can use `exists` to ensure the file is included in the
concatenated file but order of creation doesn't matter.

For example:

```postgresql
-- name: my_schema.b
-- dropped_by: my_schema
-- requires: my_schema.a
-- exists: my_schema.c
```

## Example

Lets say you have a directory with the following files:

```

sql
├── my_other_schema
│ ├── functions
│ │ ├── a.sql
│ │ ├── b.sql
│ │ └── c.sql
│ └── schema.sql
└── my_schema
├── functions
│ └── a.sql
└── schema.sql

```

And the content of the files is:

`sql/my_schema/schema.sql`:

```postgresql
-- name: my_schema

DROP SCHEMA IF EXISTS my_schema CASCADE;
CREATE SCHEMA IF NOT EXISTS my_schema;
```

`sql/my_schema/functions/a.sql`:

```postgresql
-- name: my_schema.a
-- dropped_by: my_schema

CREATE FUNCTION my_schema.a() RETURNS INT AS
$$
SELECT 1;
$$ LANGUAGE SQL IMMUTABLE
                PARALLEL SAFE;
```

`sql/my_schema/functions/b.sql`:

```postgresql
-- name: my_schema.b
-- dropped_by: my_schema
-- requires: my_schema.a

CREATE FUNCTION my_schema.b() RETURNS INT AS
$$
SELECT my_schema.a() + 1
$$ LANGUAGE SQL;
```

`sql/my_schema/functions/c.sql`:

```postgresql
-- name: my_schema.c
-- dropped_by: my_schema
-- requires: my_schema.b

CREATE FUNCTION my_schema.c() RETURNS INT AS
$$
SELECT my_schema.b() + 1
$$ LANGUAGE SQL IMMUTABLE
                PARALLEL SAFE;
```

`sql/my_other_schema/schema.sql`:

```postgresql
-- name: my_other_schema

DROP SCHEMA IF EXISTS my_schema CASCADE;
CREATE SCHEMA IF NOT EXISTS my_schema;
```

`sql/my_other_schema/functions/a.sql`:

```postgresql
-- name: my_other_schema.a
-- dropped_by: my_other_schema
-- requires: my_schema.b

CREATE FUNCTION my_other_schema.a() RETURNS INT AS
$$
SELECT my_schema.b() + 1
$$ LANGUAGE SQL IMMUTABLE
                PARALLEL SAFE;
```

So the dependency graph looks like:
![](https://github.com/joshainglis/topcat/raw/main/docs/assets/graph.png)

Now you can run `topcat` to concatenate the files in the right order:

```sh
topcat -i tests/input/sql -o tests/output/sql/output.sql
```

The content of `output.sql` will be:

```postgresql
-- This file was generated by topcat. To regenerate run:
--
-- topcat -i tests/input/sql -o tests/output/sql/output.sql -v

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
```


//! Regex patterns for parsing PostgreSQL dump files.
//!
//! These patterns are used to identify and extract object metadata from pg_dump output.
//! Covers all PostgreSQL object types that can appear in pg_dump output.
//!
//! Patterns are used in three contexts:
//! - Primary parsing: METADATA_PATTERN, EXTENSION_PATTERN, ALTER_TABLE_PATTERN, TRIGGER_PATTERN
//! - Security/ACL: GRANT_PATTERN, REVOKE_PATTERN, OWNER_PATTERN, DEFAULT_ACL_PATTERN
//! - Dependency extraction: SERVER_PATTERN, FOREIGN_TABLE_PATTERN, SUBSCRIPTION_PATTERN, etc.

use std::sync::LazyLock;

use regex::Regex;

/// Pattern for matching pg_dump metadata comments.
/// Example: `-- Name: TABLE my_table; Type: TABLE; Schema: public;`
/// Example: `-- Name: users; Type: TABLE; Schema: public; Owner: postgres`
pub static METADATA_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        ^--\s+Name:\s+(?P<tgt_type>[A-Z]+(?:\s+[A-Z]+)*\s+)?(?P<identity>"?(?P<name>[^(";]+)"?[^;]*);
        \s+Type:\s+(?P<type>[^;]+);\s+Schema:\s+(?P<schema>[^;]+);
        (?:\s+Owner:\s+(?P<owner>[^;]+))?
        "#,
    )
    .expect("Invalid METADATA_PATTERN regex")
});

/// Pattern for matching COMMENT statements.
/// Example: `COMMENT ON TABLE "schema"."table" IS 'description';`
pub static COMMENT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xs)
        COMMENT\s+ON\s+(?P<tgt_type>[A-Z]+)\s+
        (?:"(?P<tgt_schema>[^"]+)"\.)?
        "(?P<tgt_name>[^"]+)"
        (?:\."(?P<tgt_column>[^"]+)"|[^)]*\))?
        \s+IS\s+(?P<cmt>.+)"#,
    )
    .expect("Invalid COMMENT_PATTERN regex")
});

/// Pattern for matching CREATE EXTENSION statements.
/// Example: `CREATE EXTENSION IF NOT EXISTS "pgcrypto" WITH SCHEMA "public";`
pub static EXTENSION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        CREATE\s+EXTENSION(?:\s+IF\s+NOT\s+EXISTS)?\s+
        "(?P<ext_name>[^"]+)"
        (?:\s+WITH\s+SCHEMA\s+"(?P<ext_schema>[^"]+)")?"#,
    )
    .expect("Invalid EXTENSION_PATTERN regex")
});

/// Pattern for matching ALTER TABLE and related statements.
/// Example: `ALTER TABLE ONLY "schema"."table" ADD CONSTRAINT ...`
pub static ALTER_TABLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        ^(ALTER\s+TABLE(?:\s+ONLY)?|CREATE\s+(?:(?:UNIQUE\s+)?INDEX|POLICY|CONSTRAINT)\s+.+?\s+ON)
        \s+"(?P<tbl_schema>[^"]+)"\."(?P<tbl_name>[^"]+)".*"#,
    )
    .expect("Invalid ALTER_TABLE_PATTERN regex")
});

/// Pattern for matching CREATE TRIGGER statements.
/// Example: `CREATE TRIGGER trigger_name ... EXECUTE FUNCTION "schema"."function"()`
pub static TRIGGER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^CREATE TRIGGER .+? EXECUTE FUNCTION "(?P<fn_schema>[^"]+)"\."(?P<fn_name>[^"]+)"\(\)"#,
    )
    .expect("Invalid TRIGGER_PATTERN regex")
});

/// Builder for creating configurable CAST patterns.
///
/// The schema pattern allows matching specific schema prefixes in your database.
#[allow(clippy::uninlined_format_args)]
pub fn build_cast_pattern(schema_pattern: &str) -> Regex {
    let pattern = format!(
        r#"(?x)
        ^CREATE\s+CAST\s+
        \(
            ((?:"{sp}"\.)?
            "?(?P<from_name>\w+)"?(?:\[\])?)
            \s+AS\s+
            ((?:"{sp}"\.)?
            "?(?P<to_name>\w+)"?(?:\[\])?)
        \)
        \s+WITH\s+
        (?:
            (?:FUNCTION\s+
                ((?:"{sp}"\.)?
                "?(?P<func_name>\w+)"?))
            |
            INOUT
        )"#,
        sp = schema_pattern
    );
    Regex::new(&pattern).expect("Invalid CAST pattern regex")
}

/// Builder for creating configurable OPERATOR patterns.
#[allow(clippy::uninlined_format_args)]
pub fn build_operator_pattern(schema_pattern: &str) -> Regex {
    let pattern = format!(
        r#"(?x)
        ^CREATE\s+OPERATOR\s+
        (?:"{sp}"\.)?
        "?(?P<operator_name>\S+)"?
        \s+\(
        \s*FUNCTION\s*=\s*
            (?:"{sp}"\.)?
            "?(?P<procedure_name>\w+)"?,?\s*
        \s*LEFTARG\s*=\s*
            (?:"{sp}"\.)?
            "?(?P<left_arg_name>\w+)"?(?:\[\])?,?\s+
        \s*RIGHTARG\s*=\s*
            (?:"{sp}"\.)?
            "?(?P<right_arg_name>\w+)"?(?:\[\])?,?\s*"#,
        sp = schema_pattern
    );
    Regex::new(&pattern).expect("Invalid OPERATOR pattern regex")
}

/// Default schema pattern matching common PostgreSQL schema naming conventions.
pub const DEFAULT_SCHEMA_PATTERN: &str = r"(?:e|c|d[pio]|codegen|md)_\w+";

// =============================================================================
// Security and ACL Patterns
// =============================================================================

/// Pattern for matching GRANT statements.
/// Example: `GRANT SELECT ON TABLE "schema"."table" TO "role";`
/// Example: `GRANT EXECUTE ON FUNCTION "schema"."func"() TO "role";`
pub static GRANT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^GRANT\s+
        (?P<privileges>[^O]+)\s+
        ON\s+
        (?P<obj_type>TABLE|SEQUENCE|FUNCTION|PROCEDURE|SCHEMA|TYPE|DOMAIN|
                     FOREIGN\s+DATA\s+WRAPPER|FOREIGN\s+SERVER|LANGUAGE|
                     LARGE\s+OBJECT|TABLESPACE|DATABASE)?\s*
        (?:"(?P<obj_schema>[^"]+)"\.)?
        "?(?P<obj_name>[^"(\s]+)"?
        (?:\([^)]*\))?\s+
        TO\s+
        (?P<grantee>.+?)
        (?:\s+WITH\s+GRANT\s+OPTION)?
        \s*;"#,
    )
    .expect("Invalid GRANT_PATTERN regex")
});

/// Pattern for matching REVOKE statements.
/// Example: `REVOKE ALL ON TABLE "schema"."table" FROM PUBLIC;`
pub static REVOKE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^REVOKE\s+
        (?P<privileges>[^O]+)\s+
        ON\s+
        (?P<obj_type>TABLE|SEQUENCE|FUNCTION|PROCEDURE|SCHEMA|TYPE|DOMAIN|
                     FOREIGN\s+DATA\s+WRAPPER|FOREIGN\s+SERVER|LANGUAGE|
                     LARGE\s+OBJECT|TABLESPACE|DATABASE)?\s*
        (?:"(?P<obj_schema>[^"]+)"\.)?
        "?(?P<obj_name>[^"(\s]+)"?
        (?:\([^)]*\))?\s+
        FROM\s+
        (?P<revokee>.+?)
        \s*;"#,
    )
    .expect("Invalid REVOKE_PATTERN regex")
});

/// Pattern for matching ALTER ... OWNER TO statements.
/// Example: `ALTER TABLE "schema"."table" OWNER TO "postgres";`
pub static OWNER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^ALTER\s+
        (?P<obj_type>TABLE|VIEW|MATERIALIZED\s+VIEW|SEQUENCE|FUNCTION|PROCEDURE|
                     SCHEMA|TYPE|DOMAIN|AGGREGATE|FOREIGN\s+TABLE|
                     TEXT\s+SEARCH\s+(?:CONFIGURATION|DICTIONARY|PARSER|TEMPLATE)|
                     OPERATOR(?:\s+CLASS|\s+FAMILY)?|COLLATION|CONVERSION|
                     EVENT\s+TRIGGER|PUBLICATION|SUBSCRIPTION)\s+
        (?:"(?P<obj_schema>[^"]+)"\.)?
        "?(?P<obj_name>[^"(\s]+)"?
        (?:\([^)]*\))?\s+
        OWNER\s+TO\s+
        "?(?P<owner>[^";\s]+)"?
        \s*;"#,
    )
    .expect("Invalid OWNER_PATTERN regex")
});

/// Pattern for matching DEFAULT ACL statements.
/// Example: `ALTER DEFAULT PRIVILEGES FOR ROLE "user" IN SCHEMA "public" GRANT SELECT ON TABLES TO "role";`
pub static DEFAULT_ACL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^ALTER\s+DEFAULT\s+PRIVILEGES\s+
        (?:FOR\s+ROLE\s+"?(?P<grantor>[^"\s]+)"?\s+)?
        (?:IN\s+SCHEMA\s+"?(?P<schema>[^"\s]+)"?\s+)?
        (?P<action>GRANT|REVOKE)\s+
        (?P<privileges>[^O]+)\s+
        ON\s+(?P<obj_type>TABLES|SEQUENCES|FUNCTIONS|TYPES|SCHEMAS)\s+
        (?:TO|FROM)\s+
        "?(?P<target>[^";\s]+)"?
        "#,
    )
    .expect("Invalid DEFAULT_ACL_PATTERN regex")
});

/// Pattern for matching SECURITY LABEL statements.
/// Example: `SECURITY LABEL FOR "provider" ON TABLE "schema"."table" IS 'label';`
pub static SECURITY_LABEL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^SECURITY\s+LABEL\s+
        FOR\s+"?(?P<provider>[^"\s]+)"?\s+
        ON\s+
        (?P<obj_type>\w+(?:\s+\w+)?)\s+
        (?:"(?P<obj_schema>[^"]+)"\.)?
        "?(?P<obj_name>[^";\s]+)"?\s+
        IS\s+
        '(?P<label>[^']*)'
        "#,
    )
    .expect("Invalid SECURITY_LABEL_PATTERN regex")
});

// =============================================================================
// Full Text Search Patterns
// =============================================================================

/// Pattern for matching CREATE TEXT SEARCH CONFIGURATION statements.
/// Example: `CREATE TEXT SEARCH CONFIGURATION "schema"."name" (PARSER = pg_catalog.default);`
pub static FTS_CONFIG_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+TEXT\s+SEARCH\s+CONFIGURATION\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \("#,
    )
    .expect("Invalid FTS_CONFIG_PATTERN regex")
});

/// Pattern for matching CREATE TEXT SEARCH DICTIONARY statements.
/// Example: `CREATE TEXT SEARCH DICTIONARY "schema"."name" (TEMPLATE = snowball, ...);`
pub static FTS_DICTIONARY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+TEXT\s+SEARCH\s+DICTIONARY\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \("#,
    )
    .expect("Invalid FTS_DICTIONARY_PATTERN regex")
});

// =============================================================================
// Foreign Data Wrapper Patterns
// =============================================================================

/// Pattern for matching CREATE FOREIGN DATA WRAPPER statements.
/// Example: `CREATE FOREIGN DATA WRAPPER "name" HANDLER handler_func;`
pub static FDW_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+FOREIGN\s+DATA\s+WRAPPER\s+
        "?(?P<name>[^"\s]+)"?\s*
        (?:HANDLER\s+"?(?P<handler>[^"\s,;]+)"?)?
        "#,
    )
    .expect("Invalid FDW_PATTERN regex")
});

/// Pattern for matching CREATE SERVER statements.
/// Example: `CREATE SERVER "name" FOREIGN DATA WRAPPER "fdw_name" OPTIONS (...);`
pub static SERVER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+SERVER\s+
        "?(?P<name>[^"\s]+)"?\s+
        FOREIGN\s+DATA\s+WRAPPER\s+
        "?(?P<fdw>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid SERVER_PATTERN regex")
});

/// Pattern for matching CREATE USER MAPPING statements.
/// Example: `CREATE USER MAPPING FOR "user" SERVER "server_name" OPTIONS (...);`
pub static USER_MAPPING_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+USER\s+MAPPING\s+FOR\s+
        "?(?P<user>[^"\s]+)"?\s+
        SERVER\s+
        "?(?P<server>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid USER_MAPPING_PATTERN regex")
});

/// Pattern for matching CREATE FOREIGN TABLE statements.
/// Example: `CREATE FOREIGN TABLE "schema"."name" (...) SERVER "server_name";`
pub static FOREIGN_TABLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+FOREIGN\s+TABLE\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \([^)]*\)\s*
        SERVER\s+
        "?(?P<server>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid FOREIGN_TABLE_PATTERN regex")
});

// =============================================================================
// Replication Patterns
// =============================================================================

/// Pattern for matching CREATE PUBLICATION statements.
/// Example: `CREATE PUBLICATION "name" FOR TABLE "schema"."table";`
pub static PUBLICATION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+PUBLICATION\s+
        "?(?P<name>[^"\s]+)"?\s+
        (?:FOR\s+(?P<target>ALL\s+TABLES|TABLE\s+.+))?
        "#,
    )
    .expect("Invalid PUBLICATION_PATTERN regex")
});

/// Pattern for matching CREATE SUBSCRIPTION statements.
/// Example: `CREATE SUBSCRIPTION "name" CONNECTION '...' PUBLICATION "pub_name";`
pub static SUBSCRIPTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+SUBSCRIPTION\s+
        "?(?P<name>[^"\s]+)"?\s+
        CONNECTION\s+'[^']+'\s+
        PUBLICATION\s+
        "?(?P<publication>[^";\s]+)"?
        "#,
    )
    .expect("Invalid SUBSCRIPTION_PATTERN regex")
});

// =============================================================================
// Other Object Patterns
// =============================================================================

/// Pattern for matching CREATE AGGREGATE statements.
/// Example: `CREATE AGGREGATE "schema"."name" (basetype) (...);`
pub static AGGREGATE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+(?:OR\s+REPLACE\s+)?AGGREGATE\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \("#,
    )
    .expect("Invalid AGGREGATE_PATTERN regex")
});

/// Pattern for matching CREATE COLLATION statements.
/// Example: `CREATE COLLATION "schema"."name" (LOCALE = 'en_US.utf8');`
pub static COLLATION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+COLLATION\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        "#,
    )
    .expect("Invalid COLLATION_PATTERN regex")
});

/// Pattern for matching CREATE CONVERSION statements.
/// Example: `CREATE CONVERSION "name" FOR 'encoding1' TO 'encoding2' FROM func;`
pub static CONVERSION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+(?:DEFAULT\s+)?CONVERSION\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s]+)"?\s+
        FOR\s+'(?P<from_enc>[^']+)'\s+
        TO\s+'(?P<to_enc>[^']+)'
        "#,
    )
    .expect("Invalid CONVERSION_PATTERN regex")
});

/// Pattern for matching CREATE EVENT TRIGGER statements.
/// Example: `CREATE EVENT TRIGGER "name" ON ddl_command_end EXECUTE FUNCTION func();`
/// Note: Uses non-greedy matching to handle WHEN clauses containing any characters
pub static EVENT_TRIGGER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xis)
        ^CREATE\s+EVENT\s+TRIGGER\s+
        "?(?P<name>[^"\s]+)"?\s+
        ON\s+(?P<event>\w+)\s+
        (?:WHEN\s+.+?\s+)?
        EXECUTE\s+(?:FUNCTION|PROCEDURE)\s+
        (?:"(?P<fn_schema>[^"]+)"\.)?
        "?(?P<fn_name>[^"(\s]+)"?
        "#,
    )
    .expect("Invalid EVENT_TRIGGER_PATTERN regex")
});

/// Pattern for matching CREATE LANGUAGE statements.
/// Example: `CREATE PROCEDURAL LANGUAGE "plpgsql";`
pub static LANGUAGE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+(?:OR\s+REPLACE\s+)?
        (?:TRUSTED\s+)?
        (?:PROCEDURAL\s+)?LANGUAGE\s+
        "?(?P<name>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid LANGUAGE_PATTERN regex")
});

/// Pattern for matching CREATE OPERATOR CLASS statements.
/// Example: `CREATE OPERATOR CLASS "schema"."name" FOR TYPE type USING index_method AS ...;`
pub static OPERATOR_CLASS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+OPERATOR\s+CLASS\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s]+)"?\s+
        (?:DEFAULT\s+)?
        FOR\s+TYPE\s+
        (?:"(?P<type_schema>[^"]+)"\.)?
        "?(?P<type_name>[^"\s]+)"?\s+
        USING\s+(?P<method>\w+)
        "#,
    )
    .expect("Invalid OPERATOR_CLASS_PATTERN regex")
});

/// Pattern for matching CREATE OPERATOR FAMILY statements.
/// Example: `CREATE OPERATOR FAMILY "schema"."name" USING index_method;`
pub static OPERATOR_FAMILY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+OPERATOR\s+FAMILY\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s]+)"?\s+
        USING\s+(?P<method>\w+)
        "#,
    )
    .expect("Invalid OPERATOR_FAMILY_PATTERN regex")
});

/// Pattern for matching CREATE ACCESS METHOD statements.
/// Example: `CREATE ACCESS METHOD "name" TYPE INDEX HANDLER handler_func;`
pub static ACCESS_METHOD_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+ACCESS\s+METHOD\s+
        "?(?P<name>[^"\s]+)"?\s+
        TYPE\s+(?P<type>\w+)\s+
        HANDLER\s+
        (?:"(?P<handler_schema>[^"]+)"\.)?
        "?(?P<handler>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid ACCESS_METHOD_PATTERN regex")
});

/// Pattern for matching CREATE TRANSFORM statements.
/// Example: `CREATE TRANSFORM FOR "type" LANGUAGE "lang" (...);`
pub static TRANSFORM_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+(?:OR\s+REPLACE\s+)?TRANSFORM\s+FOR\s+
        (?:"(?P<type_schema>[^"]+)"\.)?
        "?(?P<type_name>[^"\s]+)"?\s+
        LANGUAGE\s+
        "?(?P<language>[^"\s]+)"?
        "#,
    )
    .expect("Invalid TRANSFORM_PATTERN regex")
});

/// Pattern for matching CREATE STATISTICS statements.
/// Example: `CREATE STATISTICS "schema"."name" ON col1, col2 FROM "schema"."table";`
pub static STATISTICS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+STATISTICS\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        (?:\([^)]*\)\s+)?
        ON\s+
        (?P<columns>[^F]+)\s+
        FROM\s+
        (?:"(?P<table_schema>[^"]+)"\.)?
        "?(?P<table_name>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid STATISTICS_PATTERN regex")
});

/// Pattern for matching CREATE RULE statements.
/// Example: `CREATE RULE "name" AS ON INSERT TO "schema"."table" DO ...;`
pub static RULE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+(?:OR\s+REPLACE\s+)?RULE\s+
        "?(?P<name>[^"\s]+)"?\s+
        AS\s+ON\s+(?P<event>\w+)\s+
        TO\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<table_name>[^"\s]+)"?
        "#,
    )
    .expect("Invalid RULE_PATTERN regex")
});

/// Pattern for matching CREATE MATERIALIZED VIEW statements.
/// Example: `CREATE MATERIALIZED VIEW "schema"."name" AS SELECT ...;`
pub static MATERIALIZED_VIEW_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+MATERIALIZED\s+VIEW\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        "#,
    )
    .expect("Invalid MATERIALIZED_VIEW_PATTERN regex")
});

/// Pattern for matching standalone CREATE SEQUENCE statements.
/// Example: `CREATE SEQUENCE "schema"."name" START 1 INCREMENT 1;`
pub static SEQUENCE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+SEQUENCE\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid SEQUENCE_PATTERN regex")
});

// =============================================================================
// Schema and Table Patterns
// =============================================================================

/// Pattern for matching CREATE SCHEMA statements.
/// Example: `CREATE SCHEMA "public";`
pub static SCHEMA_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+SCHEMA\s+
        (?:IF\s+NOT\s+EXISTS\s+)?
        "?(?P<name>[^"\s;]+)"?
        "#,
    )
    .expect("Invalid SCHEMA_PATTERN regex")
});

/// Pattern for matching CREATE TABLE statements.
/// Example: `CREATE TABLE "schema"."name" (...);`
pub static TABLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+
        (?:UNLOGGED\s+)?
        TABLE\s+
        (?:IF\s+NOT\s+EXISTS\s+)?
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        "#,
    )
    .expect("Invalid TABLE_PATTERN regex")
});

/// Pattern for matching CREATE VIEW statements.
/// Example: `CREATE VIEW "schema"."name" AS SELECT ...;`
pub static VIEW_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+
        (?:OR\s+REPLACE\s+)?
        (?:TEMP(?:ORARY)?\s+)?
        VIEW\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        "#,
    )
    .expect("Invalid VIEW_PATTERN regex")
});

// =============================================================================
// Routine Patterns (Functions, Procedures)
// =============================================================================

/// Pattern for matching CREATE FUNCTION statements.
/// Example: `CREATE FUNCTION "schema"."name"(...) RETURNS ...;`
pub static FUNCTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+
        (?:OR\s+REPLACE\s+)?
        FUNCTION\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \(
        "#,
    )
    .expect("Invalid FUNCTION_PATTERN regex")
});

/// Pattern for matching CREATE PROCEDURE statements.
/// Example: `CREATE PROCEDURE "schema"."name"(...);`
pub static PROCEDURE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+
        (?:OR\s+REPLACE\s+)?
        PROCEDURE\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \(
        "#,
    )
    .expect("Invalid PROCEDURE_PATTERN regex")
});

// =============================================================================
// Type Patterns
// =============================================================================

/// Pattern for matching CREATE TYPE statements.
/// Example: `CREATE TYPE "schema"."name" AS ENUM (...);`
pub static TYPE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+TYPE\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        "#,
    )
    .expect("Invalid TYPE_PATTERN regex")
});

/// Pattern for matching CREATE DOMAIN statements.
/// Example: `CREATE DOMAIN "schema"."name" AS type ...;`
pub static DOMAIN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+DOMAIN\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s]+)"?\s+
        AS\s+
        "#,
    )
    .expect("Invalid DOMAIN_PATTERN regex")
});

// =============================================================================
// Text Search Patterns (Parser, Template)
// =============================================================================

/// Pattern for matching CREATE TEXT SEARCH PARSER statements.
/// Example: `CREATE TEXT SEARCH PARSER "name" (START = func, ...);`
pub static FTS_PARSER_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+TEXT\s+SEARCH\s+PARSER\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \(
        "#,
    )
    .expect("Invalid FTS_PARSER_PATTERN regex")
});

/// Pattern for matching CREATE TEXT SEARCH TEMPLATE statements.
/// Example: `CREATE TEXT SEARCH TEMPLATE "name" (INIT = func, LEXIZE = func);`
pub static FTS_TEMPLATE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+TEXT\s+SEARCH\s+TEMPLATE\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<name>[^"\s(]+)"?\s*
        \(
        "#,
    )
    .expect("Invalid FTS_TEMPLATE_PATTERN regex")
});

// =============================================================================
// Attachment Patterns (Index, Constraint, Policy, etc.)
// =============================================================================

/// Pattern for matching CREATE INDEX statements.
/// Example: `CREATE INDEX "name" ON "schema"."table" (...);`
pub static INDEX_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+
        (?:UNIQUE\s+)?
        INDEX\s+
        (?:CONCURRENTLY\s+)?
        (?:IF\s+NOT\s+EXISTS\s+)?
        "?(?P<name>[^"\s]+)"?\s+
        ON\s+
        (?:ONLY\s+)?
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<table>[^"\s(]+)"?
        "#,
    )
    .expect("Invalid INDEX_PATTERN regex")
});

/// Pattern for matching ADD CONSTRAINT statements (non-FK).
/// Example: `ALTER TABLE "schema"."table" ADD CONSTRAINT "name" CHECK (...);`
pub static CONSTRAINT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^ALTER\s+TABLE\s+
        (?:ONLY\s+)?
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<table>[^"\s]+)"?\s+
        ADD\s+CONSTRAINT\s+
        "?(?P<name>[^"\s]+)"?\s+
        (?P<type>CHECK|UNIQUE|PRIMARY\s+KEY|EXCLUDE)
        "#,
    )
    .expect("Invalid CONSTRAINT_PATTERN regex")
});

/// Pattern for matching ADD CONSTRAINT ... FOREIGN KEY statements.
/// Example: `ALTER TABLE "schema"."table" ADD CONSTRAINT "name" FOREIGN KEY ...;`
pub static FK_CONSTRAINT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^ALTER\s+TABLE\s+
        (?:ONLY\s+)?
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<table>[^"\s]+)"?\s+
        ADD\s+CONSTRAINT\s+
        "?(?P<name>[^"\s]+)"?\s+
        FOREIGN\s+KEY\s*\([^)]+\)\s*
        REFERENCES\s+
        (?:"(?P<ref_schema>[^"]+)"\.)?
        "?(?P<ref_table>[^"\s(]+)"?
        "#,
    )
    .expect("Invalid FK_CONSTRAINT_PATTERN regex")
});

/// Pattern for matching ALTER TABLE ... SET DEFAULT statements.
/// Example: `ALTER TABLE ONLY "schema"."table" ALTER COLUMN "col" SET DEFAULT ...;`
pub static DEFAULT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^ALTER\s+TABLE\s+
        (?:ONLY\s+)?
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<table>[^"\s]+)"?\s+
        ALTER\s+COLUMN\s+
        "?(?P<column>[^"\s]+)"?\s+
        SET\s+DEFAULT\s+
        "#,
    )
    .expect("Invalid DEFAULT_PATTERN regex")
});

/// Pattern for matching CREATE POLICY statements.
/// Example: `CREATE POLICY "name" ON "schema"."table" ...;`
pub static POLICY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+POLICY\s+
        "?(?P<name>[^"\s]+)"?\s+
        ON\s+
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<table>[^"\s]+)"?
        "#,
    )
    .expect("Invalid POLICY_PATTERN regex")
});

/// Pattern for matching ALTER TABLE ... ENABLE ROW LEVEL SECURITY statements.
/// Example: `ALTER TABLE "schema"."table" ENABLE ROW LEVEL SECURITY;`
pub static ROW_SECURITY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^ALTER\s+TABLE\s+
        (?:ONLY\s+)?
        (?:"(?P<schema>[^"]+)"\.)?
        "?(?P<table>[^"\s]+)"?\s+
        ENABLE\s+ROW\s+LEVEL\s+SECURITY
        "#,
    )
    .expect("Invalid ROW_SECURITY_PATTERN regex")
});

// =============================================================================
// Operator Patterns (Static versions)
// =============================================================================

/// Pattern for matching CREATE CAST statements.
/// Example: `CREATE CAST (source AS target) WITH FUNCTION func;`
pub static CAST_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+CAST\s+\(
        (?:"(?P<from_schema>[^"]+)"\.)?
        "?(?P<from_type>[^"\s\[\]]+)"?(?:\[\])?\s+
        AS\s+
        (?:"(?P<to_schema>[^"]+)"\.)?
        "?(?P<to_type>[^"\s\[\])]+)"?(?:\[\])?
        \)
        "#,
    )
    .expect("Invalid CAST_PATTERN regex")
});

/// Pattern for matching CREATE OPERATOR statements.
/// Example: `CREATE OPERATOR "schema".@@ (FUNCTION = func, ...);`
pub static OPERATOR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?xi)
        ^CREATE\s+OPERATOR\s+
        (?:"(?P<schema>[^"]+)"\.)?
        (?:"?(?P<name>\S+)"?)\s*
        \(
        "#,
    )
    .expect("Invalid OPERATOR_PATTERN regex")
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_pattern() {
        let input = "-- Name: users; Type: TABLE; Schema: public;";
        let caps = METADATA_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("name").unwrap().as_str(), "users");
        assert_eq!(caps.name("type").unwrap().as_str(), "TABLE");
        assert_eq!(caps.name("schema").unwrap().as_str(), "public");
    }

    #[test]
    fn test_metadata_pattern_with_target_type() {
        let input = "-- Name: COLUMN id; Type: DEFAULT; Schema: public;";
        let caps = METADATA_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("tgt_type").unwrap().as_str().trim(), "COLUMN");
    }

    #[test]
    fn test_metadata_pattern_with_multi_word_target_type() {
        let input =
            "-- Name: MATERIALIZED VIEW report_summary; Type: MATERIALIZED VIEW; Schema: public;";
        let caps = METADATA_PATTERN.captures(input).unwrap();
        assert_eq!(
            caps.name("tgt_type").unwrap().as_str().trim(),
            "MATERIALIZED VIEW"
        );
        assert_eq!(caps.name("name").unwrap().as_str(), "report_summary");
    }

    #[test]
    fn test_extension_pattern() {
        let input = r#"CREATE EXTENSION IF NOT EXISTS "pgcrypto" WITH SCHEMA "public";"#;
        let caps = EXTENSION_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("ext_name").unwrap().as_str(), "pgcrypto");
        assert_eq!(caps.name("ext_schema").unwrap().as_str(), "public");
    }

    #[test]
    fn test_trigger_pattern() {
        let input = r#"CREATE TRIGGER my_trigger AFTER INSERT ON "public"."users" FOR EACH ROW EXECUTE FUNCTION "public"."notify_insert"()"#;
        let caps = TRIGGER_PATTERN.captures(input).unwrap();
        assert_eq!(caps.name("fn_schema").unwrap().as_str(), "public");
        assert_eq!(caps.name("fn_name").unwrap().as_str(), "notify_insert");
    }
}

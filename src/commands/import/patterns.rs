//! Regex patterns for parsing PostgreSQL dump files.
//!
//! These patterns are used to identify and extract object metadata from pg_dump output.

use std::sync::LazyLock;

use regex::Regex;

/// Pattern for matching pg_dump metadata comments.
/// Example: `-- Name: TABLE my_table; Type: TABLE; Schema: public;`
pub static METADATA_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?x)
        ^--\s+Name:\s+(?P<tgt_type>[A-Z]+\s+)?(?P<identity>"?(?P<name>[^(";]+)"?[^;]*);
        \s+Type:\s+(?P<type>[^;]+);\s+Schema:\s+(?P<schema>[^;]+);"#,
    )
    .expect("Invalid METADATA_PATTERN regex")
});

/// Pattern for matching COMMENT statements.
/// Example: `COMMENT ON TABLE "schema"."table" IS 'description';`
#[allow(dead_code)]
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

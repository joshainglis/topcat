//! Tests for SQL content analyzer.

use super::*;
use crate::sql_config::SqlDiscoveryConfig;

fn make_simple_config() -> SqlDiscoveryConfig {
    SqlDiscoveryConfig {
        enabled: true,
        schema_pattern: Some(r"(?:test|e|c|d[pio])_\w+".to_string()),
        ..Default::default()
    }
}

#[test]
fn test_extract_create_statement() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = "CREATE TABLE c_test.my_table (id INT);";
    let result = analyzer.analyze(sql);

    assert_eq!(result.node_name, Some("c_test.my_table".to_string()));
}

#[test]
fn test_extract_dependencies() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = "SELECT * FROM c_test.table1 JOIN c_test.table2 ON true;";
    let result = analyzer.analyze(sql);

    assert!(result.dependencies.contains("c_test.table1"));
    assert!(result.dependencies.contains("c_test.table2"));
}

#[test]
fn test_subobjects() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = r#"
CREATE SCHEMA c_test;
CREATE TABLE c_test.table1 (id INT);
CREATE TABLE c_test.table2 (id INT);
"#;
    let result = analyzer.analyze(sql);

    assert_eq!(result.node_name, Some("c_test".to_string()));
    assert!(result.subobjects.contains("c_test.table1"));
    assert!(result.subobjects.contains("c_test.table2"));
}

#[test]
fn test_type_mappings() {
    let mut config = make_simple_config();
    config
        .type_mappings
        .insert("TSTZRANGE".to_string(), "c_tmf.t_time_period".to_string());

    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = "SELECT '2024-01-01'::TSTZRANGE;";
    let result = analyzer.analyze(sql);

    assert!(result.dependencies.contains("c_tmf.t_time_period"));
}

#[test]
fn test_strip_suffixes() {
    let mut config = make_simple_config();
    config.strip_suffixes.push("_or_ref".to_string());

    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = "CREATE TABLE c_test.my_table_or_ref (id INT);";
    let result = analyzer.analyze(sql);

    // The suffix should be stripped from the node name
    assert_eq!(result.node_name, Some("c_test.my_table".to_string()));
}

#[test]
fn test_ignore_comments() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = r#"
-- This references c_test.fake_table but it's in a comment
SELECT * FROM c_test.real_table;
"#;
    let result = analyzer.analyze(sql);

    assert!(!result.dependencies.contains("c_test.fake_table"));
    assert!(result.dependencies.contains("c_test.real_table"));
}

#[test]
fn test_model_generation() {
    let mut config = make_simple_config();
    config
        .model_gen_patterns
        .push(r"codegen_tmf\.proc_(?:make_model|combine_enums)".to_string());

    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = r#"
CALL codegen_tmf.proc_make_model(
    p_schema := 'c_test',
    p_name := 'my_model'
);
"#;
    let result = analyzer.analyze(sql);

    assert_eq!(result.node_name, Some("c_test.my_model".to_string()));
}

#[test]
fn test_extension_mappings() {
    let mut config = make_simple_config();
    // Configure extension mappings similar to the Python script
    config
        .extension_mappings
        .insert("nlevel".to_string(), "ltree".to_string());
    config
        .extension_mappings
        .insert("lca".to_string(), "ltree".to_string());
    config
        .extension_mappings
        .insert("index".to_string(), "ltree".to_string());
    config
        .extension_mappings
        .insert("digest".to_string(), "pgcrypto".to_string());

    let analyzer = SqlAnalyzer::new(config).unwrap();

    // Test that e_extensions.nlevel gets mapped to e_extensions.ltree
    let sql = "SELECT e_extensions.nlevel(path) FROM my_table;";
    let result = analyzer.analyze(sql);

    assert!(
        result.dependencies.contains("e_extensions.ltree"),
        "Expected dependency on e_extensions.ltree, got {:?}",
        result.dependencies
    );
    assert!(
        !result.dependencies.contains("e_extensions.nlevel"),
        "Should not have dependency on e_extensions.nlevel, got {:?}",
        result.dependencies
    );

    // Test digest -> pgcrypto mapping
    let sql2 = "SELECT e_extensions.digest('test', 'sha256');";
    let result2 = analyzer.analyze(sql2);

    assert!(
        result2.dependencies.contains("e_extensions.pgcrypto"),
        "Expected dependency on e_extensions.pgcrypto, got {:?}",
        result2.dependencies
    );
}

#[test]
fn test_implicit_schema_dependency() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    // Test that creating a function in a schema adds the schema as a dependency
    let sql = r#"
CREATE OR REPLACE FUNCTION c_test.my_function(param1 INT)
RETURNS INT AS $$
BEGIN
    RETURN param1 + 1;
END;
$$ LANGUAGE plpgsql;
"#;
    let result = analyzer.analyze(sql);

    assert_eq!(
        result.node_name,
        Some("c_test.my_function".to_string()),
        "Node name should be c_test.my_function"
    );
    assert!(
        result.dependencies.contains("c_test"),
        "Should have implicit dependency on c_test schema, got {:?}",
        result.dependencies
    );
}

#[test]
fn test_implicit_schema_dependency_for_table() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    // Test that creating a table in a schema adds the schema as a dependency
    let sql = "CREATE TABLE c_test.my_table (id INT, name TEXT);";
    let result = analyzer.analyze(sql);

    assert_eq!(
        result.node_name,
        Some("c_test.my_table".to_string()),
        "Node name should be c_test.my_table"
    );
    assert!(
        result.dependencies.contains("c_test"),
        "Should have implicit dependency on c_test schema, got {:?}",
        result.dependencies
    );
}

#[test]
fn test_create_cast_detection() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = r#"
CREATE CAST (text AS c_test.my_type)
WITH FUNCTION c_test.text_to_my_type(text);
"#;
    let result = analyzer.analyze(sql);

    assert!(
        result.has_implicit,
        "Should detect CREATE CAST as implicit object"
    );
}

#[test]
fn test_create_operator_detection() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    let sql = r#"
CREATE OPERATOR === (
    LEFTARG = text,
    RIGHTARG = text,
    FUNCTION = texteq
);
"#;
    let result = analyzer.analyze(sql);

    assert!(
        result.has_implicit,
        "Should detect CREATE OPERATOR as implicit object"
    );
}

#[test]
fn test_create_cast_case_insensitive() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    // Test with lowercase
    let sql_lower = "create cast (text as c_test.my_type) with function c_test.converter;";
    let result_lower = analyzer.analyze(sql_lower);
    assert!(
        result_lower.has_implicit,
        "Should detect lowercase CREATE CAST"
    );

    // Test with mixed case
    let sql_mixed = "Create Cast (text as c_test.my_type) With Function c_test.converter;";
    let result_mixed = analyzer.analyze(sql_mixed);
    assert!(
        result_mixed.has_implicit,
        "Should detect mixed case CREATE CAST"
    );
}

#[test]
fn test_no_implicit_for_regular_create() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    // Regular CREATE TABLE should not be marked as implicit
    let sql = "CREATE TABLE c_test.my_table (id INT);";
    let result = analyzer.analyze(sql);

    assert!(
        !result.has_implicit,
        "Regular CREATE TABLE should not be marked as implicit"
    );
}

#[test]
fn test_ignore_dependencies_in_comment_statements() {
    let config = make_simple_config();
    let analyzer = SqlAnalyzer::new(config).unwrap();

    // COMMENT ON statements should not have their string content parsed for dependencies
    let sql = r#"
CREATE TABLE c_test.my_table (
    id INT,
    data TEXT
);

COMMENT ON TABLE c_test.my_table IS 'This table references data from do_tmf.entity_index';
COMMENT ON COLUMN c_test.my_table.data IS 'Stores ((do_tmf).entity_index).entity_json column data';
"#;
    let result = analyzer.analyze(sql);

    // Should have the table itself
    assert_eq!(result.node_name, Some("c_test.my_table".to_string()));

    // Should have implicit schema dependency
    assert!(result.dependencies.contains("c_test"));

    // Should NOT have dependencies found in COMMENT statements
    assert!(
        !result.dependencies.contains("do_tmf.entity_index"),
        "Should not extract dependencies from COMMENT ON statements, got {:?}",
        result.dependencies
    );
    assert!(
        !result.dependencies.contains("do_tmf"),
        "Should not extract schema dependencies from COMMENT ON statements, got {:?}",
        result.dependencies
    );
}

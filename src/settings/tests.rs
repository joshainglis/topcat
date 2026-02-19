//! Tests for settings configuration.

use super::*;

#[test]
fn test_default_settings() {
    let settings = Settings::default();
    assert_eq!(settings.layers.names, vec!["prepend", "normal", "append"]);
    assert_eq!(settings.layers.fallback, "normal".to_string());
    assert!(!settings.sql_discovery.enabled);
    assert!(!settings.behavior.verbose);
}

#[test]
fn test_validation() {
    let mut settings = Settings::default();
    assert!(settings.validate().is_ok());

    // Invalid fallback layer
    settings.layers.fallback = "invalid".to_string();
    assert!(settings.validate().is_err());

    // Empty layers
    settings.layers.names.clear();
    assert!(settings.validate().is_err());
}

#[test]
fn test_validation_rename_requires_header_update() {
    // rename_files without header updates should fail
    let settings = Settings {
        rename_files: true,
        header_update_mode: HeaderUpdateMode::Never,
        ..Default::default()
    };
    let result = settings.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("rename files"));

    // Should pass with header updates enabled
    let settings = Settings {
        rename_files: true,
        header_update_mode: HeaderUpdateMode::InPlace,
        ..Default::default()
    };
    assert!(settings.validate().is_ok());
}

#[test]
fn test_validation_invalid_regex_patterns() {
    // Invalid root_regex
    let settings = Settings {
        analysis: AnalysisConfig {
            root_regex: vec!["[invalid".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };
    let result = settings.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("root_regex"));

    // Invalid schema_pattern
    let settings = Settings {
        sql_discovery: SqlDiscoveryConfig {
            schema_pattern: Some("[invalid".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let result = settings.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("schema_pattern"));
}

#[test]
fn test_merge_strategy_from_str() {
    assert_eq!(
        "header-only".parse::<MergeStrategy>().unwrap(),
        MergeStrategy::HeaderOnly
    );
    assert_eq!(
        "discovery-only".parse::<MergeStrategy>().unwrap(),
        MergeStrategy::DiscoveryOnly
    );
    assert!("invalid".parse::<MergeStrategy>().is_err());
}

#[test]
fn test_export_mode_from_str() {
    assert_eq!(
        "dependencies".parse::<ExportMode>().unwrap(),
        ExportMode::Dependencies
    );
    assert_eq!(
        "deps".parse::<ExportMode>().unwrap(),
        ExportMode::Dependencies
    );
    assert!("invalid".parse::<ExportMode>().is_err());
}

#[test]
fn test_env_var_parsing() {
    // Test that environment variables are properly parsed
    // Note: We test by loading with env vars, not by setting them directly
    // to avoid test interference in parallel test execution

    // Create a test config file in a temp directory
    use std::io::Write;
    use tempfile::Builder;

    let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
    writeln!(
        temp_file,
        r#"
[behavior]
verbose = true

[filters]
include_extensions = ["sql", "ddl"]
"#
    )
    .unwrap();

    // Load settings from the temp file
    let settings = Settings::load(Some(temp_file.path().to_str().unwrap()))
        .expect("Failed to load settings from temp file");

    assert!(settings.behavior.verbose);
    assert_eq!(settings.filters.include_extensions, vec!["sql", "ddl"]);
}

#[test]
fn test_config_file_precedence() {
    // Test that config files are loaded and merged correctly
    use std::io::Write;
    use tempfile::Builder;

    // Create a config file with specific settings
    let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
    writeln!(
        temp_file,
        r#"
[layers]
names = ["first", "second", "third"]
fallback = "second"

[behavior]
verbose = true
quiet = false
"#
    )
    .unwrap();

    let settings =
        Settings::load(Some(temp_file.path().to_str().unwrap())).expect("Failed to load settings");

    assert_eq!(settings.layers.names, vec!["first", "second", "third"]);
    assert_eq!(settings.layers.fallback, "second".to_string());
    assert!(settings.behavior.verbose);
    assert!(!settings.behavior.quiet);
}

#[test]
fn test_config_validation_with_custom_layers() {
    use std::io::Write;
    use tempfile::Builder;

    // Create a config with valid custom layers
    let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
    writeln!(
        temp_file,
        r#"
[layers]
names = ["alpha", "beta", "gamma"]
fallback = "beta"
"#
    )
    .unwrap();

    let settings =
        Settings::load(Some(temp_file.path().to_str().unwrap())).expect("Failed to load settings");

    assert!(settings.validate().is_ok());

    // Test invalid fallback layer
    let mut temp_file2 = Builder::new().suffix(".toml").tempfile().unwrap();
    writeln!(
        temp_file2,
        r#"
[layers]
names = ["alpha", "beta"]
fallback = "gamma"
"#
    )
    .unwrap();

    let settings2 =
        Settings::load(Some(temp_file2.path().to_str().unwrap())).expect("Failed to load settings");

    assert!(settings2.validate().is_err());
}

#[test]
fn test_sql_discovery_settings() {
    use std::io::Write;
    use tempfile::Builder;

    let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
    writeln!(
        temp_file,
        r#"
[sql_discovery]
enabled = true
schema_pattern = "myapp_\\w+"
merge_strategy = "union"
"#
    )
    .unwrap();

    let settings =
        Settings::load(Some(temp_file.path().to_str().unwrap())).expect("Failed to load settings");

    assert!(settings.sql_discovery.enabled);
    assert_eq!(
        settings.sql_discovery.schema_pattern,
        Some("myapp_\\w+".to_string())
    );
    assert_eq!(settings.sql_discovery.merge_strategy, MergeStrategy::Union);
}

#[test]
fn test_analysis_settings() {
    use std::io::Write;
    use tempfile::Builder;

    let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
    writeln!(
        temp_file,
        r#"
[analysis]
root_patterns = ["**/api/*.sql", "**/*_init.sql"]
external_check_dirs = ["/app/src", "/app/lib"]
external_check_patterns = ["*.py", "*.ts", "*.js"]
"#
    )
    .unwrap();

    let settings =
        Settings::load(Some(temp_file.path().to_str().unwrap())).expect("Failed to load settings");

    assert_eq!(
        settings.analysis.root_patterns,
        vec!["**/api/*.sql", "**/*_init.sql"]
    );
    assert_eq!(
        settings.analysis.external_check_dirs,
        vec!["/app/src", "/app/lib"]
    );
    assert_eq!(
        settings.analysis.external_check_patterns,
        vec!["*.py", "*.ts", "*.js"]
    );
}

#[test]
fn test_filters_configuration() {
    use std::io::Write;
    use tempfile::Builder;

    let mut temp_file = Builder::new().suffix(".toml").tempfile().unwrap();
    writeln!(
        temp_file,
        r#"
[filters]
include_extensions = ["sql", "ddl", "dml"]
exclude_extensions = ["bak", "tmp"]
include_globs = ["**/*.sql"]
exclude_globs = ["**/test/**", "**/tmp/**"]
include_hidden = true
"#
    )
    .unwrap();

    let settings =
        Settings::load(Some(temp_file.path().to_str().unwrap())).expect("Failed to load settings");

    assert_eq!(
        settings.filters.include_extensions,
        vec!["sql", "ddl", "dml"]
    );
    assert_eq!(settings.filters.exclude_extensions, vec!["bak", "tmp"]);
    assert_eq!(settings.filters.include_globs, vec!["**/*.sql"]);
    assert_eq!(
        settings.filters.exclude_globs,
        vec!["**/test/**", "**/tmp/**"]
    );
    assert!(settings.filters.include_hidden);
}

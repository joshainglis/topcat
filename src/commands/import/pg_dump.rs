//! PostgreSQL dump file parser and splitter.
//!
//! This module provides functionality to parse pg_dump output files and split them
//! into organized per-object SQL files with topcat-compatible headers.
//!
//! # Example
//!
//! ```bash
//! topcat import pg-dump database.sql ./output/ --schema-pattern "app_\\w+"
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use clap::Args;
use regex::Regex;

use topcat::cli::GlobalArgs;
use topcat::exceptions::TopCatError;
use topcat::logging::{Logger, init_logging};
use topcat::settings::Settings;

use super::patterns::{
    ALTER_TABLE_PATTERN, DEFAULT_SCHEMA_PATTERN, EXTENSION_PATTERN, METADATA_PATTERN,
    TRIGGER_PATTERN, build_cast_pattern, build_operator_pattern,
};

/// Command-line arguments for the pg-dump import subcommand.
#[derive(Debug, Args, Clone)]
pub struct PgDumpArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    /// Path to the pg_dump SQL file to import
    #[arg(value_name = "DUMP_FILE")]
    pub dump_file: PathBuf,

    /// Output directory for the split SQL files
    #[arg(value_name = "OUTPUT_DIR")]
    pub output_dir: PathBuf,

    /// Regex pattern for matching schema names (for CAST/OPERATOR parsing)
    #[arg(long, value_name = "PATTERN")]
    pub schema_pattern: Option<String>,

    /// Preview changes without writing files
    #[arg(long)]
    pub dry_run: bool,
}

/// Basic object information extracted from pg_dump metadata comments.
#[derive(Debug, Clone)]
struct ObjInfo {
    schema: String,
    name: String,
    identity: String,
    obj_type: String,
    target_type: Option<String>,
}

/// Hydrated object with content and categorization.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct HydratedObjInfo {
    schema: String,
    name: String,
    identity: String,
    obj_type: String,
    target_type: Option<String>,
    category: String,
    content: String,
}

impl HydratedObjInfo {
    fn from_info(info: ObjInfo, category: String, content: String) -> Self {
        Self {
            schema: info.schema,
            name: info.name,
            identity: info.identity,
            obj_type: info.obj_type,
            target_type: info.target_type,
            category,
            content,
        }
    }
}

/// Collection of related objects that belong to a primary object.
#[derive(Debug, Default)]
struct RelatedObjects {
    primary_obj: Vec<HydratedObjInfo>,
    cast: Vec<HydratedObjInfo>,
    operator: Vec<HydratedObjInfo>,
    comment: Vec<HydratedObjInfo>,
    trigger: Vec<HydratedObjInfo>,
    constraint: Vec<HydratedObjInfo>,
    fk_constraint: Vec<HydratedObjInfo>,
    policy: Vec<HydratedObjInfo>,
    row_security: Vec<HydratedObjInfo>,
    index: Vec<HydratedObjInfo>,
    sequence: Vec<HydratedObjInfo>,
}

impl RelatedObjects {
    /// Render all related objects into a single SQL string.
    fn render(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();

        for obj in &self.primary_obj {
            parts.push(&obj.content);
        }
        for obj in &self.sequence {
            parts.push(&obj.content);
        }
        for obj in &self.index {
            parts.push(&obj.content);
        }
        for obj in &self.constraint {
            parts.push(&obj.content);
        }
        for obj in &self.fk_constraint {
            parts.push(&obj.content);
        }
        for obj in &self.row_security {
            parts.push(&obj.content);
        }
        for obj in &self.policy {
            parts.push(&obj.content);
        }
        for obj in &self.trigger {
            parts.push(&obj.content);
        }
        for obj in &self.cast {
            parts.push(&obj.content);
        }
        for obj in &self.operator {
            parts.push(&obj.content);
        }
        for obj in &self.comment {
            parts.push(&obj.content);
        }

        parts.join("\n\n")
    }
}

/// Key for looking up objects: (obj_type, schema, name)
type ObjectKey = (String, String, String);

/// Global object entry for database-level objects (casts, operators, etc.)
#[derive(Debug)]
struct GlobalObject {
    category: String,
    name: String,
    content: String,
}

/// Main parser for PostgreSQL dump files.
pub struct PgDumpParser {
    dump_file: PathBuf,
    output_dir: PathBuf,
    #[allow(dead_code)]
    schema_pattern: String,
    cast_pattern: Regex,
    operator_pattern: Regex,
    /// schema -> category -> name -> list of (obj_type, schema, name) keys
    schemas: HashMap<String, HashMap<String, HashMap<String, Vec<ObjectKey>>>>,
    /// type -> schema -> name -> RelatedObjects
    objects: HashMap<String, HashMap<String, HashMap<String, RelatedObjects>>>,
    /// Database-level objects that don't belong to a schema
    global_objects: Vec<GlobalObject>,
    dry_run: bool,
    logger: Logger,
}

impl PgDumpParser {
    /// Create a new parser instance.
    pub fn new(
        dump_file: PathBuf,
        output_dir: PathBuf,
        schema_pattern: Option<String>,
        dry_run: bool,
        logger: Logger,
    ) -> Self {
        let pattern = schema_pattern.unwrap_or_else(|| DEFAULT_SCHEMA_PATTERN.to_string());
        let cast_pattern = build_cast_pattern(&pattern);
        let operator_pattern = build_operator_pattern(&pattern);

        Self {
            dump_file,
            output_dir,
            schema_pattern: pattern,
            cast_pattern,
            operator_pattern,
            schemas: HashMap::new(),
            objects: HashMap::new(),
            global_objects: Vec::new(),
            dry_run,
            logger,
        }
    }

    /// Parse the dump file and extract all objects.
    pub fn parse_dump(&mut self) -> Result<(), TopCatError> {
        let content = fs::read_to_string(&self.dump_file)?;
        let lines: Vec<&str> = content.lines().collect();

        let mut current_object: Option<ObjInfo> = None;
        let mut current_content: Vec<&str> = Vec::new();

        for line in &lines {
            if let Some(caps) = METADATA_PATTERN.captures(line) {
                // Skip COMMENT type entries in metadata
                if caps.name("type").map(|m| m.as_str().trim()) == Some("COMMENT") {
                    continue;
                }

                // Save previous object if exists
                if let Some(obj) = current_object.take() {
                    self.save_object(obj, current_content.join("\n"));
                }

                // Start new object
                let target_type = caps.name("tgt_type").map(|m| {
                    let t = m.as_str().trim();
                    if t == "COLUMN" { "TABLE" } else { t }.to_string()
                });

                current_object = Some(ObjInfo {
                    name: caps
                        .name("name")
                        .map(|m| m.as_str().trim())
                        .unwrap_or("")
                        .to_string(),
                    identity: caps
                        .name("identity")
                        .map(|m| m.as_str().trim())
                        .unwrap_or("")
                        .to_string(),
                    obj_type: caps
                        .name("type")
                        .map(|m| m.as_str().trim())
                        .unwrap_or("")
                        .to_string(),
                    target_type,
                    schema: caps
                        .name("schema")
                        .map(|m| m.as_str().trim())
                        .unwrap_or("")
                        .to_string(),
                });
                current_content.clear();
            } else if current_object.is_some() && !line.starts_with("--") && !line.trim().is_empty()
            {
                current_content.push(line);
            }
        }

        // Save last object
        if let Some(obj) = current_object {
            self.save_object(obj, current_content.join("\n"));
        }

        Ok(())
    }

    /// Save an object to the appropriate data structure.
    fn save_object(&mut self, mut obj_info: ObjInfo, content: String) {
        // Handle schema-less objects
        if obj_info.schema == "-" {
            if obj_info.obj_type == "SCHEMA" {
                obj_info.schema = obj_info.name.clone();
            } else if obj_info.obj_type == "CAST" {
                self.handle_cast(&content);
                return;
            } else if obj_info.obj_type == "EXTENSION" {
                if let Some(caps) = EXTENSION_PATTERN.captures(&content) {
                    obj_info.schema = caps
                        .name("ext_schema")
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_else(|| "public".to_string());
                    obj_info.name = caps
                        .name("ext_name")
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default();
                }
            } else {
                obj_info.schema = "public".to_string();
            }
        }

        // Handle OPERATOR type
        if obj_info.obj_type == "OPERATOR" {
            self.handle_operator(&content);
            return;
        }

        let category = self.categorize_object(&obj_info.obj_type, &obj_info.name, &content);
        let hydrated =
            HydratedObjInfo::from_info(obj_info.clone(), category.clone(), content.clone());

        // Handle TRIGGER - associate with function
        if obj_info.obj_type == "TRIGGER" {
            if let Some(caps) = TRIGGER_PATTERN.captures(&content) {
                let fn_schema = caps
                    .name("fn_schema")
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                let fn_name = caps
                    .name("fn_name")
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                self.get_or_create_related("FUNCTION", &fn_schema, &fn_name)
                    .trigger
                    .push(hydrated);
                return;
            }
        }

        // Handle ALTER TABLE, constraints, indexes, and related statements
        if let Some(caps) = ALTER_TABLE_PATTERN.captures(&content) {
            let tbl_schema = caps
                .name("tbl_schema")
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let tbl_name = caps
                .name("tbl_name")
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let related = self.get_or_create_related("TABLE", &tbl_schema, &tbl_name);

            // Store in appropriate bucket based on object type
            match obj_info.obj_type.as_str() {
                "CONSTRAINT" => related.constraint.push(hydrated),
                "INDEX" => related.index.push(hydrated),
                "POLICY" => related.policy.push(hydrated),
                "ROW SECURITY" => related.row_security.push(hydrated),
                "DEFAULT" => related.primary_obj.push(hydrated), // Column defaults go with table
                "FK CONSTRAINT" => related.fk_constraint.push(hydrated),
                _ => related.trigger.push(hydrated), // Fallback for triggers and unknowns
            }
            return;
        }

        // Handle FUNCTION/PROCEDURE - try to add to existing object
        if matches!(obj_info.obj_type.as_str(), "FUNCTION" | "PROCEDURE") {
            self.try_add_to_existing(&hydrated, &["TABLE", "TYPE"]);
        }

        // Primary object types
        if matches!(
            obj_info.obj_type.as_str(),
            "TABLE" | "TYPE" | "DOMAIN" | "SCHEMA" | "VIEW" | "FUNCTION" | "PROCEDURE" | "OPERATOR"
        ) {
            let related =
                self.get_or_create_related(&obj_info.obj_type, &obj_info.schema, &obj_info.name);
            let is_first = related.primary_obj.is_empty();
            related.primary_obj.push(hydrated.clone());

            if is_first {
                // Store key reference instead of clone
                let key = (
                    obj_info.obj_type.clone(),
                    obj_info.schema.clone(),
                    obj_info.name.clone(),
                );
                self.schemas
                    .entry(obj_info.schema.clone())
                    .or_default()
                    .entry(category)
                    .or_default()
                    .entry(obj_info.name.clone())
                    .or_default()
                    .push(key);
            }
        } else if let Some(target_type) = &obj_info.target_type {
            // Secondary objects - attach to parent
            let related = self.get_or_create_related(target_type, &obj_info.schema, &obj_info.name);
            match category.as_str() {
                "constraint" => related.constraint.push(hydrated),
                "fk_constraint" => related.fk_constraint.push(hydrated),
                "index" => related.index.push(hydrated),
                "sequence" => related.sequence.push(hydrated),
                "policy" => related.policy.push(hydrated),
                "row_security" => related.row_security.push(hydrated),
                "comment" => related.comment.push(hydrated),
                _ => {} // Ignore unknown categories
            }
        }
    }

    /// Handle CAST objects.
    fn handle_cast(&mut self, content: &str) {
        if let Some(caps) = self.cast_pattern.captures(content) {
            let from_name = caps
                .name("from_name")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let to_name = caps
                .name("to_name")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let to_schema = caps.name("to_schema").map(|m| m.as_str().to_string());

            // Try to add to existing TYPE object
            let schema = to_schema.clone().unwrap_or_else(|| "public".to_string());
            let hydrated = HydratedObjInfo {
                schema: schema.clone(),
                name: to_name.clone(),
                identity: format!("{from_name}::{to_name}"),
                obj_type: "CAST".to_string(),
                target_type: Some("CAST".to_string()),
                category: "CAST".to_string(),
                content: content.to_string(),
            };

            if !self.try_add_to_existing(&hydrated, &["TABLE", "TYPE"]) {
                // Add to global objects
                self.global_objects.push(GlobalObject {
                    category: "cast".to_string(),
                    name: format!("{from_name}_to_{to_name}"),
                    content: content.to_string(),
                });
            }
        }
    }

    /// Handle OPERATOR objects.
    fn handle_operator(&mut self, content: &str) {
        if let Some(caps) = self.operator_pattern.captures(content) {
            let operator_name = caps
                .name("operator_name")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let procedure_name = caps
                .name("procedure_name")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let left_arg_name = caps
                .name("left_arg_name")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let right_arg_name = caps
                .name("right_arg_name")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            let hydrated = HydratedObjInfo {
                schema: "public".to_string(),
                name: procedure_name.clone(),
                identity: format!("{left_arg_name}({operator_name}){right_arg_name}"),
                obj_type: "OPERATOR".to_string(),
                target_type: Some("OPERATOR".to_string()),
                category: "OPERATOR".to_string(),
                content: content.to_string(),
            };

            if !self.try_add_to_existing(&hydrated, &["TABLE", "TYPE", "FUNCTION", "PROCEDURE"]) {
                // Add to global objects
                self.global_objects.push(GlobalObject {
                    category: "operator".to_string(),
                    name: format!("{left_arg_name}_{operator_name}_{right_arg_name}"),
                    content: content.to_string(),
                });
            }
        }
    }

    /// Try to add an object to an existing parent object.
    fn try_add_to_existing(&mut self, obj: &HydratedObjInfo, parent_types: &[&str]) -> bool {
        for parent_type in parent_types {
            if let Some(schema_map) = self.objects.get_mut(*parent_type) {
                if let Some(name_map) = schema_map.get_mut(&obj.schema) {
                    if let Some(related) = name_map.get_mut(&obj.name) {
                        if !related.primary_obj.is_empty() {
                            related.primary_obj.push(obj.clone());
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Get or create a RelatedObjects for the given type, schema, and name.
    fn get_or_create_related(
        &mut self,
        obj_type: &str,
        schema: &str,
        name: &str,
    ) -> &mut RelatedObjects {
        self.objects
            .entry(obj_type.to_string())
            .or_default()
            .entry(schema.to_string())
            .or_default()
            .entry(name.to_string())
            .or_default()
    }

    /// Categorize an object based on its type and content.
    fn categorize_object(&self, obj_type: &str, name: &str, content: &str) -> String {
        match obj_type {
            "TYPE" => self.categorize_type(content),
            "DOMAIN" => "type/domain".to_string(),
            "FUNCTION" => self.categorize_function(name),
            _ => obj_type.to_lowercase().replace(' ', "_"),
        }
    }

    /// Categorize a TYPE object into type/enum, type/composite, or type/domain.
    fn categorize_type(&self, content: &str) -> String {
        if content.contains("AS ENUM") {
            "type/enum".to_string()
        } else if content.contains("CREATE DOMAIN") {
            "type/domain".to_string()
        } else {
            // Composite types (AS (...)) and other types
            "type/composite".to_string()
        }
    }

    /// Categorize a FUNCTION object.
    fn categorize_function(&self, name: &str) -> String {
        // Check for API function pattern
        let api_re = Regex::new(
            r"^api_(?P<name>[\w_]+)_(?P<method>create|delete|list|update|get)_(?P<version>v\d+)",
        )
        .expect("Invalid API function regex");

        if let Some(caps) = api_re.captures(name) {
            let api_name = caps.name("name").map(|m| m.as_str()).unwrap_or("");
            let version = caps.name("version").map(|m| m.as_str()).unwrap_or("");
            return format!("functions/api/{api_name}/{version}");
        }

        // Check for other function prefixes
        if name.starts_with("cast_") {
            "functions/casting".to_string()
        } else if name.starts_with("new_") {
            "functions/builder".to_string()
        } else if name.starts_with("util_") {
            "functions/utility".to_string()
        } else {
            "functions".to_string()
        }
    }

    /// Create a topcat-compatible file header.
    fn create_file_header(&self, name: &str, schema: &str) -> String {
        format!("-- name: {schema}.{name}\n\n")
    }

    /// Create a file header for global objects (no schema prefix).
    fn create_global_file_header(&self, name: &str) -> String {
        format!("-- name: {name}\n\n")
    }

    /// Write all parsed objects to the output directory.
    pub fn write_files(&self) -> Result<(), TopCatError> {
        if self.dry_run {
            self.logger
                .info("Dry run mode - showing what would be created:");
            self.logger.info("");
        } else {
            // Clean up existing output directory
            if self.output_dir.exists() {
                fs::remove_dir_all(&self.output_dir)?;
            }
            fs::create_dir_all(&self.output_dir)?;
        }

        let mut file_count = 0;

        // Write schema-based objects
        for (schema_name, categories) in &self.schemas {
            let schema_dir = self.output_dir.join(schema_name);

            for (category, items) in categories {
                let category_dir = if category == "schema" {
                    schema_dir.clone()
                } else {
                    schema_dir.join(category)
                };

                for (name, key_list) in items {
                    let file_path = category_dir.join(format!("{name}.sql"));

                    if self.dry_run {
                        self.logger
                            .info(&format!("  Would create: {}", file_path.display()));
                        file_count += 1;
                    } else {
                        fs::create_dir_all(&category_dir)?;

                        let header = self.create_file_header(name, schema_name);
                        // Look up actual objects using the stored keys
                        let content: Vec<String> = key_list
                            .iter()
                            .filter_map(|(obj_type, schema, obj_name)| {
                                self.objects
                                    .get(obj_type)
                                    .and_then(|s| s.get(schema))
                                    .and_then(|n| n.get(obj_name))
                                    .map(|r| r.render())
                            })
                            .collect();
                        let full_content = format!("{header}{}", content.join("\n\n"));

                        let final_content = if full_content.ends_with('\n') {
                            full_content
                        } else {
                            format!("{full_content}\n")
                        };

                        fs::write(&file_path, final_content)?;
                        file_count += 1;
                    }
                }
            }
        }

        // Write global objects to _global directory
        if !self.global_objects.is_empty() {
            // Group global objects by category
            let mut global_by_category: HashMap<&str, Vec<&GlobalObject>> = HashMap::new();
            for obj in &self.global_objects {
                global_by_category
                    .entry(&obj.category)
                    .or_default()
                    .push(obj);
            }

            for (category, objects) in global_by_category {
                let category_dir = self.output_dir.join("_global").join(category);

                for obj in objects {
                    let file_path = category_dir.join(format!("{}.sql", obj.name));

                    if self.dry_run {
                        self.logger
                            .info(&format!("  Would create: {}", file_path.display()));
                        file_count += 1;
                    } else {
                        fs::create_dir_all(&category_dir)?;

                        let header = self.create_global_file_header(&obj.name);
                        let full_content = format!("{header}{}", obj.content);

                        let final_content = if full_content.ends_with('\n') {
                            full_content
                        } else {
                            format!("{full_content}\n")
                        };

                        fs::write(&file_path, final_content)?;
                        file_count += 1;
                    }
                }
            }
        }

        if self.dry_run {
            self.logger.info("");
            self.logger
                .info(&format!("Would create {file_count} files"));
        } else {
            self.logger.success(&format!(
                "Created {file_count} files in {}",
                self.output_dir.display()
            ));
        }

        Ok(())
    }
}

impl PgDumpArgs {
    /// Execute the pg-dump import command.
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Load settings
        let config_path = self.global.config_path();
        let mut settings = Settings::load(config_path)
            .map_err(|e| TopCatError::ConfigError(format!("Failed to load configuration: {e}")))?;

        // Apply CLI overrides
        self.global.apply_to_settings(&mut settings);

        // Initialize logging
        let quiet = settings.behavior.quiet;
        let verbose = settings.behavior.verbose;
        init_logging(verbose, quiet);
        let logger = Logger::new(quiet, verbose);

        // Validate input file
        if !self.dump_file.exists() {
            return Err(TopCatError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Dump file not found: {}", self.dump_file.display()),
            )));
        }

        logger.info(&format!("Parsing: {}", self.dump_file.display()));
        logger.info(&format!("Output:  {}", self.output_dir.display()));

        // Get schema pattern from CLI or config
        let schema_pattern = self
            .schema_pattern
            .clone()
            .or_else(|| settings.sql_discovery.schema_pattern.clone());

        if let Some(ref pattern) = schema_pattern {
            logger.debug(&format!("Using schema pattern: {pattern}"));
        }

        // Parse and write
        let mut parser = PgDumpParser::new(
            self.dump_file.clone(),
            self.output_dir.clone(),
            schema_pattern,
            self.dry_run,
            logger.clone(),
        );

        parser.parse_dump()?;
        parser.write_files()?;

        Ok(())
    }
}

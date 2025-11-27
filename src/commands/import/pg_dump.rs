//! PostgreSQL dump file parser and splitter.
//!
//! This module provides functionality to parse pg_dump output files and split them
//! into organized per-object SQL files with topcat-compatible headers.
//!
//! # Supported Object Types
//!
//! The parser supports all PostgreSQL object types including:
//! - Schema-level: TABLE, VIEW, MATERIALIZED VIEW, FOREIGN TABLE, SEQUENCE
//! - Types: TYPE (enum, composite, domain, range), DOMAIN, COLLATION
//! - Routines: FUNCTION, PROCEDURE, AGGREGATE
//! - Full Text Search: TEXT SEARCH CONFIGURATION, DICTIONARY, PARSER, TEMPLATE
//! - Foreign Data: FOREIGN DATA WRAPPER, SERVER, USER MAPPING
//! - Operators: OPERATOR, OPERATOR CLASS, OPERATOR FAMILY
//! - Replication: PUBLICATION, SUBSCRIPTION
//! - Security: ACL, DEFAULT ACL, SECURITY LABEL
//! - And more...
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

use super::dependencies::{DependencyAnalyzer, build_global_header, build_header};
use super::object_types::{Layer, ObjectType, TypeSubcategory};
use super::patterns::{
    ALTER_TABLE_PATTERN, DEFAULT_ACL_PATTERN, DEFAULT_SCHEMA_PATTERN, EXTENSION_PATTERN,
    GRANT_PATTERN, METADATA_PATTERN, OWNER_PATTERN, REVOKE_PATTERN, STATISTICS_PATTERN,
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

    /// Include ACL (GRANT/REVOKE) statements with objects
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub include_acl: bool,

    /// Include OWNER statements with objects
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub include_owner: bool,

    /// Generate layer headers based on object type
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub generate_layers: bool,

    /// Auto-generate dependency headers from SQL content analysis
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub generate_deps: bool,
}

/// Basic object information extracted from pg_dump metadata comments.
#[derive(Debug, Clone)]
struct ObjInfo {
    schema: String,
    name: String,
    identity: String,
    obj_type: ObjectType,
    pg_type_str: String,
    target_type: Option<String>,
    owner: Option<String>,
}

/// Hydrated object with content and categorization.
#[derive(Debug, Clone)]
struct HydratedObjInfo {
    schema: String,
    name: String,
    #[allow(dead_code)]
    identity: String,
    obj_type: ObjectType,
    #[allow(dead_code)]
    target_type: Option<String>,
    #[allow(dead_code)]
    category: String,
    content: String,
    #[allow(dead_code)]
    owner: Option<String>,
    #[allow(dead_code)]
    acl: Vec<String>,
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
            owner: info.owner,
            acl: Vec::new(),
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
    statistics: Vec<HydratedObjInfo>,
    rule: Vec<HydratedObjInfo>,
    acl: Vec<String>,
    owner: Option<String>,
}

impl RelatedObjects {
    /// Render all related objects into a single SQL string.
    fn render(&self, include_acl: bool, include_owner: bool) -> String {
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
        for obj in &self.statistics {
            parts.push(&obj.content);
        }
        for obj in &self.rule {
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

        let mut result = parts.join("\n\n");

        // Add owner statement if present
        if include_owner {
            if let Some(ref owner_stmt) = self.owner {
                if !result.is_empty() {
                    result.push_str("\n\n");
                }
                result.push_str(owner_stmt);
            }
        }

        // Add ACL statements
        if include_acl && !self.acl.is_empty() {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(&self.acl.join("\n"));
        }

        result
    }
}

/// Key for looking up objects: (obj_type, schema, name)
type ObjectKey = (ObjectType, String, String);

/// Global object entry for database-level objects (casts, operators, etc.)
#[derive(Debug)]
struct GlobalObject {
    category: String,
    subcategory: Option<String>,
    name: String,
    content: String,
    layer: Layer,
    acl: Vec<String>,
}

/// Security statement to be attached to an object.
#[derive(Debug, Clone)]
struct SecurityStatement {
    obj_type: Option<String>,
    obj_schema: Option<String>,
    obj_name: String,
    statement: String,
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
    objects: HashMap<ObjectType, HashMap<String, HashMap<String, RelatedObjects>>>,
    /// Database-level objects that don't belong to a schema
    global_objects: Vec<GlobalObject>,
    /// Pending ACL statements to be attached
    pending_acl: Vec<SecurityStatement>,
    /// Pending owner statements to be attached
    pending_owner: Vec<SecurityStatement>,
    /// Default ACL statements
    default_acl: Vec<String>,
    /// Dependency analyzer for auto-generating requires headers
    dep_analyzer: Option<DependencyAnalyzer>,
    dry_run: bool,
    include_acl: bool,
    include_owner: bool,
    generate_layers: bool,
    generate_deps: bool,
    logger: Logger,
}

impl PgDumpParser {
    /// Create a new parser instance.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dump_file: PathBuf,
        output_dir: PathBuf,
        schema_pattern: Option<String>,
        dry_run: bool,
        include_acl: bool,
        include_owner: bool,
        generate_layers: bool,
        generate_deps: bool,
        logger: Logger,
    ) -> Self {
        let pattern = schema_pattern
            .clone()
            .unwrap_or_else(|| DEFAULT_SCHEMA_PATTERN.to_string());
        let cast_pattern = build_cast_pattern(&pattern);
        let operator_pattern = build_operator_pattern(&pattern);

        // Create dependency analyzer if enabled
        let dep_analyzer = if generate_deps {
            use topcat::sql_config::SqlDiscoveryConfig;
            let config = SqlDiscoveryConfig {
                schema_pattern: schema_pattern.clone(),
                ..Default::default()
            };
            DependencyAnalyzer::new(config).ok()
        } else {
            None
        };

        Self {
            dump_file,
            output_dir,
            schema_pattern: pattern,
            cast_pattern,
            operator_pattern,
            schemas: HashMap::new(),
            objects: HashMap::new(),
            global_objects: Vec::new(),
            pending_acl: Vec::new(),
            pending_owner: Vec::new(),
            default_acl: Vec::new(),
            dep_analyzer,
            dry_run,
            include_acl,
            include_owner,
            generate_layers,
            generate_deps,
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
            // Check for metadata comments first
            if let Some(caps) = METADATA_PATTERN.captures(line) {
                let type_str = caps.name("type").map(|m| m.as_str().trim()).unwrap_or("");
                let obj_type = ObjectType::from_pg_dump_type(type_str);

                // Skip COMMENT type entries in metadata - they're handled inline
                if obj_type == ObjectType::Comment {
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

                let owner = caps.name("owner").map(|m| m.as_str().trim().to_string());

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
                    obj_type,
                    pg_type_str: type_str.to_string(),
                    target_type,
                    schema: caps
                        .name("schema")
                        .map(|m| m.as_str().trim())
                        .unwrap_or("")
                        .to_string(),
                    owner,
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

        // Second pass: collect security statements
        self.collect_security_statements(&content);

        // Attach security statements to objects
        self.attach_security_statements();

        Ok(())
    }

    /// Collect GRANT/REVOKE/OWNER statements from content.
    fn collect_security_statements(&mut self, content: &str) {
        for line in content.lines() {
            // Check for GRANT statements
            if let Some(caps) = GRANT_PATTERN.captures(line) {
                self.pending_acl.push(SecurityStatement {
                    obj_type: caps.name("obj_type").map(|m| m.as_str().to_uppercase()),
                    obj_schema: caps.name("obj_schema").map(|m| m.as_str().to_string()),
                    obj_name: caps
                        .name("obj_name")
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default(),
                    statement: line.to_string(),
                });
            }

            // Check for REVOKE statements
            if let Some(caps) = REVOKE_PATTERN.captures(line) {
                self.pending_acl.push(SecurityStatement {
                    obj_type: caps.name("obj_type").map(|m| m.as_str().to_uppercase()),
                    obj_schema: caps.name("obj_schema").map(|m| m.as_str().to_string()),
                    obj_name: caps
                        .name("obj_name")
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default(),
                    statement: line.to_string(),
                });
            }

            // Check for OWNER statements
            if let Some(caps) = OWNER_PATTERN.captures(line) {
                self.pending_owner.push(SecurityStatement {
                    obj_type: caps.name("obj_type").map(|m| m.as_str().to_uppercase()),
                    obj_schema: caps.name("obj_schema").map(|m| m.as_str().to_string()),
                    obj_name: caps
                        .name("obj_name")
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default(),
                    statement: line.to_string(),
                });
            }

            // Check for DEFAULT ACL statements
            if DEFAULT_ACL_PATTERN.is_match(line) {
                self.default_acl.push(line.to_string());
            }
        }
    }

    /// Attach collected security statements to their parent objects.
    fn attach_security_statements(&mut self) {
        // Attach ACL statements
        for acl in &self.pending_acl {
            let schema = acl
                .obj_schema
                .clone()
                .unwrap_or_else(|| "public".to_string());

            // Try to find matching object
            let obj_type = acl
                .obj_type
                .as_ref()
                .map(|t| ObjectType::from_pg_dump_type(t))
                .unwrap_or(ObjectType::Unknown);

            // Try primary object types
            let types_to_try = match obj_type {
                ObjectType::Table => vec![ObjectType::Table],
                ObjectType::Sequence => vec![ObjectType::Sequence],
                ObjectType::Function => vec![ObjectType::Function],
                ObjectType::Procedure => vec![ObjectType::Procedure],
                ObjectType::Schema => vec![ObjectType::Schema],
                ObjectType::Type | ObjectType::Domain => vec![ObjectType::Type, ObjectType::Domain],
                _ => vec![
                    ObjectType::Table,
                    ObjectType::View,
                    ObjectType::MaterializedView,
                    ObjectType::Function,
                    ObjectType::Sequence,
                ],
            };

            for try_type in types_to_try {
                if let Some(schema_map) = self.objects.get_mut(&try_type) {
                    if let Some(name_map) = schema_map.get_mut(&schema) {
                        if let Some(related) = name_map.get_mut(&acl.obj_name) {
                            related.acl.push(acl.statement.clone());
                            break;
                        }
                    }
                }
            }
        }

        // Attach owner statements
        for owner_stmt in &self.pending_owner {
            let schema = owner_stmt
                .obj_schema
                .clone()
                .unwrap_or_else(|| "public".to_string());

            let obj_type = owner_stmt
                .obj_type
                .as_ref()
                .map(|t| ObjectType::from_pg_dump_type(t))
                .unwrap_or(ObjectType::Unknown);

            let types_to_try = match obj_type {
                ObjectType::Table => vec![ObjectType::Table],
                ObjectType::View => vec![ObjectType::View],
                ObjectType::MaterializedView => vec![ObjectType::MaterializedView],
                ObjectType::Function => vec![ObjectType::Function],
                ObjectType::Procedure => vec![ObjectType::Procedure],
                _ => vec![ObjectType::Table, ObjectType::View, ObjectType::Function],
            };

            for try_type in types_to_try {
                if let Some(schema_map) = self.objects.get_mut(&try_type) {
                    if let Some(name_map) = schema_map.get_mut(&schema) {
                        if let Some(related) = name_map.get_mut(&owner_stmt.obj_name) {
                            related.owner = Some(owner_stmt.statement.clone());
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Save an object to the appropriate data structure.
    fn save_object(&mut self, mut obj_info: ObjInfo, content: String) {
        // Handle schema-less objects
        if obj_info.schema == "-" {
            match obj_info.obj_type {
                ObjectType::Schema => {
                    obj_info.schema = obj_info.name.clone();
                }
                ObjectType::Cast => {
                    self.handle_cast(&content);
                    return;
                }
                ObjectType::Extension => {
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
                }
                // Global objects
                ObjectType::ForeignDataWrapper
                | ObjectType::Server
                | ObjectType::Language
                | ObjectType::EventTrigger
                | ObjectType::Publication
                | ObjectType::Subscription
                | ObjectType::AccessMethod => {
                    self.handle_global_object(&obj_info, &content);
                    return;
                }
                _ => {
                    obj_info.schema = "public".to_string();
                }
            }
        }

        // Handle OPERATOR type
        if obj_info.obj_type == ObjectType::Operator {
            self.handle_operator(&content);
            return;
        }

        // Handle operator class/family
        if matches!(
            obj_info.obj_type,
            ObjectType::OperatorClass | ObjectType::OperatorFamily
        ) {
            self.handle_global_object(&obj_info, &content);
            return;
        }

        let category = self.categorize_object(&obj_info, &content);
        let hydrated =
            HydratedObjInfo::from_info(obj_info.clone(), category.clone(), content.clone());

        // Handle TRIGGER - associate with function
        if obj_info.obj_type == ObjectType::Trigger {
            if let Some(caps) = TRIGGER_PATTERN.captures(&content) {
                let fn_schema = caps
                    .name("fn_schema")
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                let fn_name = caps
                    .name("fn_name")
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
                self.get_or_create_related(ObjectType::Function, &fn_schema, &fn_name)
                    .trigger
                    .push(hydrated);
                return;
            }
        }

        // Handle STATISTICS - attach to table
        if obj_info.obj_type == ObjectType::Statistics {
            if let Some(caps) = STATISTICS_PATTERN.captures(&content) {
                let tbl_schema = caps
                    .name("table_schema")
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| obj_info.schema.clone());
                let tbl_name = caps
                    .name("table_name")
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                self.get_or_create_related(ObjectType::Table, &tbl_schema, &tbl_name)
                    .statistics
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
            let related = self.get_or_create_related(ObjectType::Table, &tbl_schema, &tbl_name);

            // Store in appropriate bucket based on object type
            match obj_info.obj_type {
                ObjectType::Constraint => related.constraint.push(hydrated),
                ObjectType::Index => related.index.push(hydrated),
                ObjectType::Policy => related.policy.push(hydrated),
                ObjectType::RowSecurity => related.row_security.push(hydrated),
                ObjectType::Default => related.primary_obj.push(hydrated),
                ObjectType::FkConstraint => related.fk_constraint.push(hydrated),
                ObjectType::Rule => related.rule.push(hydrated),
                _ => related.trigger.push(hydrated),
            }
            return;
        }

        // Handle FUNCTION/PROCEDURE - try to add to existing object (for overloads)
        if matches!(
            obj_info.obj_type,
            ObjectType::Function | ObjectType::Procedure | ObjectType::Aggregate
        ) {
            self.try_add_to_existing(&hydrated, &[ObjectType::Table, ObjectType::Type]);
        }

        // Primary object types
        if obj_info.obj_type.is_primary() {
            let related =
                self.get_or_create_related(obj_info.obj_type, &obj_info.schema, &obj_info.name);
            let is_first = related.primary_obj.is_empty();
            related.primary_obj.push(hydrated.clone());

            if is_first {
                // Store key reference
                let key = (
                    obj_info.obj_type,
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

                // Register with dependency analyzer
                if let Some(ref mut analyzer) = self.dep_analyzer {
                    analyzer.register_object(&obj_info.schema, &obj_info.name);
                }
            }
        } else if let Some(ref target_type_str) = obj_info.target_type {
            // Secondary objects - attach to parent
            let target_type = ObjectType::from_pg_dump_type(target_type_str);
            let related = self.get_or_create_related(target_type, &obj_info.schema, &obj_info.name);
            match obj_info.obj_type {
                ObjectType::Constraint => related.constraint.push(hydrated),
                ObjectType::FkConstraint => related.fk_constraint.push(hydrated),
                ObjectType::Index => related.index.push(hydrated),
                ObjectType::Sequence => related.sequence.push(hydrated),
                ObjectType::Policy => related.policy.push(hydrated),
                ObjectType::RowSecurity => related.row_security.push(hydrated),
                ObjectType::Comment => related.comment.push(hydrated),
                ObjectType::Statistics => related.statistics.push(hydrated),
                ObjectType::Rule => related.rule.push(hydrated),
                _ => {} // Ignore unknown categories
            }
        }
    }

    /// Handle global objects that don't belong to a schema.
    fn handle_global_object(&mut self, obj_info: &ObjInfo, content: &str) {
        let (category, subcategory) = match obj_info.obj_type {
            ObjectType::ForeignDataWrapper => ("fdw".to_string(), Some("wrapper".to_string())),
            ObjectType::Server => ("fdw".to_string(), Some("server".to_string())),
            ObjectType::UserMapping => ("fdw".to_string(), Some("user_mapping".to_string())),
            ObjectType::Language => ("language".to_string(), None),
            ObjectType::EventTrigger => ("event_trigger".to_string(), None),
            ObjectType::Publication => ("replication".to_string(), Some("publication".to_string())),
            ObjectType::Subscription => {
                ("replication".to_string(), Some("subscription".to_string()))
            }
            ObjectType::AccessMethod => ("access_method".to_string(), None),
            ObjectType::OperatorClass => ("operator".to_string(), Some("class".to_string())),
            ObjectType::OperatorFamily => ("operator".to_string(), Some("family".to_string())),
            ObjectType::Transform => ("transform".to_string(), None),
            _ => ("other".to_string(), None),
        };

        self.global_objects.push(GlobalObject {
            category,
            subcategory,
            name: obj_info.name.clone(),
            content: content.to_string(),
            layer: obj_info.obj_type.layer(),
            acl: Vec::new(),
        });
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
                obj_type: ObjectType::Cast,
                target_type: Some("CAST".to_string()),
                category: "CAST".to_string(),
                content: content.to_string(),
                owner: None,
                acl: Vec::new(),
            };

            if !self.try_add_to_existing(&hydrated, &[ObjectType::Table, ObjectType::Type]) {
                // Add to global objects
                self.global_objects.push(GlobalObject {
                    category: "cast".to_string(),
                    subcategory: None,
                    name: format!("{from_name}_to_{to_name}"),
                    content: content.to_string(),
                    layer: Layer::Prepend,
                    acl: Vec::new(),
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
                obj_type: ObjectType::Operator,
                target_type: Some("OPERATOR".to_string()),
                category: "OPERATOR".to_string(),
                content: content.to_string(),
                owner: None,
                acl: Vec::new(),
            };

            if !self.try_add_to_existing(
                &hydrated,
                &[
                    ObjectType::Table,
                    ObjectType::Type,
                    ObjectType::Function,
                    ObjectType::Procedure,
                ],
            ) {
                // Add to global objects
                self.global_objects.push(GlobalObject {
                    category: "operator".to_string(),
                    subcategory: None,
                    name: format!("{left_arg_name}_{operator_name}_{right_arg_name}"),
                    content: content.to_string(),
                    layer: Layer::Prepend,
                    acl: Vec::new(),
                });
            }
        }
    }

    /// Try to add an object to an existing parent object.
    fn try_add_to_existing(&mut self, obj: &HydratedObjInfo, parent_types: &[ObjectType]) -> bool {
        for parent_type in parent_types {
            if let Some(schema_map) = self.objects.get_mut(parent_type) {
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
        obj_type: ObjectType,
        schema: &str,
        name: &str,
    ) -> &mut RelatedObjects {
        self.objects
            .entry(obj_type)
            .or_default()
            .entry(schema.to_string())
            .or_default()
            .entry(name.to_string())
            .or_default()
    }

    /// Categorize an object based on its type and content.
    fn categorize_object(&self, obj_info: &ObjInfo, content: &str) -> String {
        match obj_info.obj_type {
            ObjectType::Type => {
                let subcat = TypeSubcategory::from_content(content);
                format!("type/{}", subcat.subdirectory())
            }
            ObjectType::Domain => "type/domain".to_string(),
            ObjectType::Function | ObjectType::Procedure => {
                self.categorize_function(&obj_info.name)
            }
            ObjectType::Aggregate => "functions/aggregate".to_string(),
            ObjectType::MaterializedView => "materialized_view".to_string(),
            ObjectType::ForeignTable => "foreign_table".to_string(),
            ObjectType::Collation => "collation".to_string(),
            ObjectType::Conversion => "conversion".to_string(),
            ObjectType::TextSearchConfiguration => "fts/configuration".to_string(),
            ObjectType::TextSearchDictionary => "fts/dictionary".to_string(),
            ObjectType::TextSearchParser => "fts/parser".to_string(),
            ObjectType::TextSearchTemplate => "fts/template".to_string(),
            ObjectType::Sequence => "sequence".to_string(),
            _ => obj_info.pg_type_str.to_lowercase().replace(' ', "_"),
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
        } else if name.starts_with("trigger_") || name.ends_with("_trigger") {
            "functions/trigger".to_string()
        } else {
            "functions".to_string()
        }
    }

    /// Get the layer for a set of related objects.
    fn get_layer_for_objects(&self, objects: &RelatedObjects) -> Option<Layer> {
        objects.primary_obj.first().map(|obj| obj.obj_type.layer())
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

                        // Look up actual objects using the stored keys
                        let mut layer: Option<Layer> = None;
                        let content: Vec<String> = key_list
                            .iter()
                            .filter_map(|(obj_type, schema, obj_name)| {
                                self.objects
                                    .get(obj_type)
                                    .and_then(|s| s.get(schema))
                                    .and_then(|n| n.get(obj_name))
                                    .map(|r| {
                                        if layer.is_none() {
                                            layer = self.get_layer_for_objects(r);
                                        }
                                        r.render(self.include_acl, self.include_owner)
                                    })
                            })
                            .collect();

                        let rendered_content = content.join("\n\n");

                        // Analyze dependencies if enabled
                        let requires = if self.generate_deps {
                            self.dep_analyzer
                                .as_ref()
                                .map(|analyzer| analyzer.analyze_dependencies(&rendered_content))
                        } else {
                            None
                        };

                        let header = build_header(
                            schema_name,
                            name,
                            layer,
                            requires.as_ref(),
                            self.generate_layers,
                        );
                        let full_content = format!("{header}{rendered_content}");

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
            // Group global objects by category and subcategory
            let mut global_by_category: HashMap<(&str, Option<&str>), Vec<&GlobalObject>> =
                HashMap::new();
            for obj in &self.global_objects {
                global_by_category
                    .entry((&obj.category, obj.subcategory.as_deref()))
                    .or_default()
                    .push(obj);
            }

            for ((category, subcategory), objects) in global_by_category {
                let category_dir = if let Some(subcat) = subcategory {
                    self.output_dir.join("_global").join(category).join(subcat)
                } else {
                    self.output_dir.join("_global").join(category)
                };

                for obj in objects {
                    let file_path = category_dir.join(format!("{}.sql", obj.name));

                    if self.dry_run {
                        self.logger
                            .info(&format!("  Would create: {}", file_path.display()));
                        file_count += 1;
                    } else {
                        fs::create_dir_all(&category_dir)?;

                        // Analyze dependencies for global objects
                        let requires = if self.generate_deps {
                            self.dep_analyzer
                                .as_ref()
                                .map(|analyzer| analyzer.analyze_dependencies(&obj.content))
                        } else {
                            None
                        };

                        let header = build_global_header(
                            &obj.name,
                            Some(obj.layer),
                            requires.as_ref(),
                            self.generate_layers,
                        );
                        let mut full_content = format!("{header}{}", obj.content);

                        // Add ACL if present
                        if self.include_acl && !obj.acl.is_empty() {
                            full_content.push_str("\n\n");
                            full_content.push_str(&obj.acl.join("\n"));
                        }

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

        // Write default ACL statements
        if !self.default_acl.is_empty() && self.include_acl {
            let acl_dir = self.output_dir.join("_global").join("acl");
            let file_path = acl_dir.join("default_acl.sql");

            if self.dry_run {
                self.logger
                    .info(&format!("  Would create: {}", file_path.display()));
                file_count += 1;
            } else {
                fs::create_dir_all(&acl_dir)?;
                let header = build_global_header(
                    "default_acl",
                    Some(Layer::Append),
                    None,
                    self.generate_layers,
                );
                let content = format!("{header}{}\n", self.default_acl.join("\n"));
                fs::write(&file_path, content)?;
                file_count += 1;
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
            self.include_acl,
            self.include_owner,
            self.generate_layers,
            self.generate_deps,
            logger.clone(),
        );

        parser.parse_dump()?;
        parser.write_files()?;

        Ok(())
    }
}

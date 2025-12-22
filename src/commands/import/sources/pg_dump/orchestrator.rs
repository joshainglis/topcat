//! Orchestrator for processing pg_dump objects and writing files.
//!
//! This module bridges the gap between parsed objects (`Vec<RawObject>`) and
//! file output, handling:
//! - Object categorization and grouping
//! - Attachment resolution using extracted_deps
//! - Security statement attachment
//! - File writing with proper headers

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::commands::import::dependencies::DependencyAnalyzer;
use crate::commands::import::handlers::HandlerRegistry;
use crate::commands::import::object_types::{Layer, ObjectType, TypeSubcategory};
use crate::commands::import::output::header_builder::{build_global_header, build_header};
use crate::commands::import::sources::{RawObject, SecurityKind, SecurityStatement};
use topcat::exceptions::TopCatError;
use topcat::logging::Logger;

/// Configuration for the import orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Output directory for generated files.
    pub output_dir: PathBuf,

    /// Whether to run in dry-run mode (preview without writing).
    pub dry_run: bool,

    /// Whether to include ACL statements.
    pub include_acl: bool,

    /// Whether to include OWNER statements.
    pub include_owner: bool,

    /// Whether to generate layer headers.
    pub generate_layers: bool,

    /// Whether to auto-generate dependency headers.
    pub generate_deps: bool,
}

impl OrchestratorConfig {
    /// Create a new configuration with defaults.
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            dry_run: false,
            include_acl: true,
            include_owner: true,
            generate_layers: true,
            generate_deps: true,
        }
    }

    /// Set dry-run mode.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Set ACL inclusion.
    pub fn with_include_acl(mut self, include_acl: bool) -> Self {
        self.include_acl = include_acl;
        self
    }

    /// Set owner inclusion.
    pub fn with_include_owner(mut self, include_owner: bool) -> Self {
        self.include_owner = include_owner;
        self
    }

    /// Set layer generation.
    pub fn with_generate_layers(mut self, generate_layers: bool) -> Self {
        self.generate_layers = generate_layers;
        self
    }

    /// Set dependency generation.
    pub fn with_generate_deps(mut self, generate_deps: bool) -> Self {
        self.generate_deps = generate_deps;
        self
    }
}

/// Key for looking up objects: (obj_type, schema, name)
type ObjectKey = (ObjectType, String, String);

/// Collected content for a primary object and its attachments.
#[derive(Debug, Default)]
struct CollectedObject {
    /// Primary object content (can have multiple for overloaded functions).
    primary: Vec<RawObject>,
    /// Attached objects (indexes, constraints, triggers, etc.).
    attachments: Vec<RawObject>,
    /// ACL statements.
    acl: Vec<String>,
    /// Owner statement.
    owner: Option<String>,
}

impl CollectedObject {
    fn is_empty(&self) -> bool {
        self.primary.is_empty()
    }

    fn get_layer(&self) -> Option<Layer> {
        self.primary.first().map(|obj| obj.obj_type.layer())
    }

    fn get_identities(&self) -> Vec<&str> {
        self.primary
            .iter()
            .filter(|obj| !obj.identity.is_empty())
            .map(|obj| obj.identity.as_str())
            .collect()
    }

    fn render(&self, include_acl: bool, include_owner: bool) -> String {
        let mut parts: Vec<&str> = Vec::new();

        // Primary objects first
        for obj in &self.primary {
            parts.push(&obj.content);
        }

        // Then attachments in order
        for obj in &self.attachments {
            parts.push(&obj.content);
        }

        let mut result = parts.join("\n\n");

        // Add owner if present
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

/// Global object entry for database-level objects.
#[derive(Debug)]
struct GlobalObject {
    obj_type: ObjectType,
    category: String,
    subcategory: Option<String>,
    name: String,
    content: String,
    layer: Layer,
    acl: Vec<String>,
}

/// Orchestrator for processing and writing imported objects.
pub struct ImportOrchestrator {
    config: OrchestratorConfig,
    logger: Logger,
    dep_analyzer: Option<DependencyAnalyzer>,
    handler_registry: HandlerRegistry,

    /// schema -> category -> name -> list of keys
    schemas: HashMap<String, HashMap<String, HashMap<String, Vec<ObjectKey>>>>,
    /// type -> schema -> name -> CollectedObject
    objects: HashMap<ObjectType, HashMap<String, HashMap<String, CollectedObject>>>,
    /// Global objects
    global_objects: Vec<GlobalObject>,
    /// Default ACL statements
    default_acl: Vec<String>,
}

impl ImportOrchestrator {
    /// Create a new orchestrator with the given configuration.
    pub fn new(config: OrchestratorConfig, logger: Logger) -> Self {
        // Create dependency analyzer if enabled
        let dep_analyzer = if config.generate_deps {
            use topcat::sql_config::SqlDiscoveryConfig;
            DependencyAnalyzer::new(SqlDiscoveryConfig::default()).ok()
        } else {
            None
        };

        Self {
            config,
            logger,
            dep_analyzer,
            handler_registry: HandlerRegistry::new(),
            schemas: HashMap::new(),
            objects: HashMap::new(),
            global_objects: Vec::new(),
            default_acl: Vec::new(),
        }
    }

    /// Process objects and write files.
    pub fn process(
        &mut self,
        objects: Vec<RawObject>,
        security: Vec<SecurityStatement>,
        default_acl: Vec<String>,
    ) -> Result<(), TopCatError> {
        self.default_acl = default_acl;

        // Phase 1: Categorize and group objects
        self.process_objects(objects);

        // Phase 2: Attach security statements
        self.attach_security(security);

        // Phase 3: Write files
        self.write_files()
    }

    /// Process and categorize all objects.
    fn process_objects(&mut self, objects: Vec<RawObject>) {
        for obj in objects {
            self.process_single_object(obj);
        }
    }

    /// Process a single object.
    fn process_single_object(&mut self, obj: RawObject) {
        // Handle global objects (no schema)
        if obj.is_global() {
            self.handle_global_object(obj);
            return;
        }

        let schema = obj.schema.clone().unwrap_or_else(|| "public".to_string());

        // Check if this is an attachment - use AttachmentRegistry to find parent
        if self
            .handler_registry
            .attachment_registry()
            .is_attachment(&obj.obj_type)
        {
            if let Some(parent) = self
                .handler_registry
                .attachment_registry()
                .find_parent(&obj)
            {
                let parent_schema = parent.schema.unwrap_or_else(|| schema.clone());
                self.attach_to_parent(&parent.obj_type, &parent_schema, &parent.name, obj);
                return;
            }
            // Fallback: try to attach using target_type if no parent found
            if obj.target_type.is_some() {
                let name = obj.name.clone();
                self.attach_to_parent(&ObjectType::Table, &schema, &name, obj);
                return;
            }
        }

        // Primary objects
        if obj.obj_type.is_primary() {
            let category = self.categorize_object(&obj);
            let name = obj.name.clone();

            // Register object
            self.register_object(&schema, &category, &name, obj);
        }
    }

    /// Attach an object to its parent.
    fn attach_to_parent(
        &mut self,
        parent_type: &ObjectType,
        schema: &str,
        name: &str,
        obj: RawObject,
    ) {
        let collected = self.get_or_create_collected(*parent_type, schema, name);
        collected.attachments.push(obj);
    }

    /// Register a primary object.
    fn register_object(&mut self, schema: &str, category: &str, name: &str, obj: RawObject) {
        // Store key reference
        let key = (obj.obj_type, schema.to_string(), name.to_string());
        self.schemas
            .entry(schema.to_string())
            .or_default()
            .entry(category.to_string())
            .or_default()
            .entry(name.to_string())
            .or_default()
            .push(key.clone());

        // Add to collected object
        let collected = self.get_or_create_collected(obj.obj_type, schema, name);
        collected.primary.push(obj);

        // Register with dependency analyzer
        if let Some(ref mut analyzer) = self.dep_analyzer {
            analyzer.register_object_with_type(schema, name, key.0);
        }
    }

    /// Handle global objects.
    ///
    /// Uses ObjectType::global_category() to determine output organization.
    fn handle_global_object(&mut self, obj: RawObject) {
        let (category, subcategory) = obj.obj_type.global_category();

        self.global_objects.push(GlobalObject {
            obj_type: obj.obj_type,
            category,
            subcategory,
            name: obj.name.clone(),
            content: obj.content,
            layer: obj.obj_type.layer(),
            acl: Vec::new(),
        });
    }

    /// Get or create a CollectedObject.
    fn get_or_create_collected(
        &mut self,
        obj_type: ObjectType,
        schema: &str,
        name: &str,
    ) -> &mut CollectedObject {
        self.objects
            .entry(obj_type)
            .or_default()
            .entry(schema.to_string())
            .or_default()
            .entry(name.to_string())
            .or_default()
    }

    /// Categorize an object for file organization.
    ///
    /// Uses ObjectType::category_dir() for most types. Special handling:
    /// - Type: Uses TypeSubcategory for enum/composite/range subcategories
    /// - Function/Procedure: Uses FunctionHandler::subcategory() for naming-based organization
    fn categorize_object(&self, obj: &RawObject) -> String {
        use crate::commands::import::handlers::Categorizer;
        use crate::commands::import::handlers::routines::FunctionHandler;

        match obj.obj_type {
            // Types need content analysis for subcategory (enum, composite, range)
            ObjectType::Type => {
                let subcat = TypeSubcategory::from_content(&obj.content);
                format!("type/{}", subcat.subdirectory())
            }

            // Functions and procedures use handler-based subcategorization
            ObjectType::Function | ObjectType::Procedure => {
                let base = obj.obj_type.category_dir(); // "functions"
                match FunctionHandler::subcategory(&obj.name) {
                    Some(subcat) => format!("{base}/{subcat}"),
                    None => base,
                }
            }

            // All other types use the category_dir from ObjectType
            _ => obj.obj_type.category_dir(),
        }
    }

    /// Attach security statements to objects.
    fn attach_security(&mut self, security: Vec<SecurityStatement>) {
        for stmt in security {
            let schema = stmt
                .target_schema
                .clone()
                .unwrap_or_else(|| "public".to_string());

            match stmt.kind {
                SecurityKind::Grant | SecurityKind::Revoke => {
                    self.attach_acl(&stmt.target_type, &schema, &stmt.target_name, &stmt.content);
                }
                SecurityKind::Owner => {
                    self.attach_owner(&stmt.target_type, &schema, &stmt.target_name, &stmt.content);
                }
            }
        }
    }

    /// Attach an ACL statement to an object.
    fn attach_acl(
        &mut self,
        target_type: &Option<ObjectType>,
        schema: &str,
        name: &str,
        content: &str,
    ) {
        let types_to_try = match target_type {
            Some(ObjectType::Table) => vec![ObjectType::Table],
            Some(ObjectType::Sequence) => vec![ObjectType::Sequence],
            Some(ObjectType::Function) => vec![ObjectType::Function],
            Some(ObjectType::Procedure) => vec![ObjectType::Procedure],
            Some(ObjectType::Schema) => vec![ObjectType::Schema],
            Some(ObjectType::Type | ObjectType::Domain) => {
                vec![ObjectType::Type, ObjectType::Domain]
            }
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
                if let Some(name_map) = schema_map.get_mut(schema) {
                    if let Some(collected) = name_map.get_mut(name) {
                        collected.acl.push(content.to_string());
                        return;
                    }
                }
            }
        }
    }

    /// Attach an owner statement to an object.
    fn attach_owner(
        &mut self,
        target_type: &Option<ObjectType>,
        schema: &str,
        name: &str,
        content: &str,
    ) {
        let types_to_try = match target_type {
            Some(ObjectType::Table) => vec![ObjectType::Table],
            Some(ObjectType::View) => vec![ObjectType::View],
            Some(ObjectType::MaterializedView) => vec![ObjectType::MaterializedView],
            Some(ObjectType::Function) => vec![ObjectType::Function],
            Some(ObjectType::Procedure) => vec![ObjectType::Procedure],
            _ => vec![ObjectType::Table, ObjectType::View, ObjectType::Function],
        };

        for try_type in types_to_try {
            if let Some(schema_map) = self.objects.get_mut(&try_type) {
                if let Some(name_map) = schema_map.get_mut(schema) {
                    if let Some(collected) = name_map.get_mut(name) {
                        collected.owner = Some(content.to_string());
                        return;
                    }
                }
            }
        }
    }

    /// Write all files.
    fn write_files(&self) -> Result<(), TopCatError> {
        if self.config.dry_run {
            self.logger
                .info("Dry run mode - showing what would be created:");
            self.logger.info("");
        } else {
            // Clean up existing output directory
            if self.config.output_dir.exists() {
                fs::remove_dir_all(&self.config.output_dir)?;
            }
            fs::create_dir_all(&self.config.output_dir)?;
        }

        let mut file_count = 0;

        // Write schema-based objects
        for (schema_name, categories) in &self.schemas {
            let schema_dir = self.config.output_dir.join(schema_name);

            for (category, items) in categories {
                let category_dir = if category == "schema" {
                    schema_dir.clone()
                } else {
                    schema_dir.join(category)
                };

                for (name, key_list) in items {
                    let file_path = category_dir.join(format!("{name}.sql"));

                    if self.config.dry_run {
                        self.logger
                            .info(&format!("  Would create: {}", file_path.display()));
                        file_count += 1;
                    } else {
                        fs::create_dir_all(&category_dir)?;

                        // Collect content from objects
                        let mut layer: Option<Layer> = None;
                        let mut identities: Vec<String> = Vec::new();
                        let content: Vec<String> = key_list
                            .iter()
                            .filter_map(|(obj_type, schema, obj_name)| {
                                self.objects
                                    .get(obj_type)
                                    .and_then(|s| s.get(schema))
                                    .and_then(|n| n.get(obj_name))
                                    .map(|collected| {
                                        if layer.is_none() {
                                            layer = collected.get_layer();
                                        }
                                        identities.extend(
                                            collected
                                                .get_identities()
                                                .into_iter()
                                                .map(|s| s.to_string()),
                                        );
                                        collected.render(
                                            self.config.include_acl,
                                            self.config.include_owner,
                                        )
                                    })
                            })
                            .collect();

                        let rendered_content = content.join("\n\n");

                        // Analyze dependencies
                        let primary_type = key_list.first().map(|(t, _, _)| *t);
                        let requires = if self.config.generate_deps {
                            let mut deps = self
                                .dep_analyzer
                                .as_ref()
                                .map(|analyzer| {
                                    if let Some(obj_type) = primary_type {
                                        analyzer.analyze_dependencies_for_type(
                                            &rendered_content,
                                            obj_type,
                                        )
                                    } else {
                                        analyzer.analyze_dependencies(&rendered_content)
                                    }
                                })
                                .unwrap_or_default();

                            // Add handler pattern-based dependencies
                            if let Some(obj_type) = primary_type {
                                if let Some(handler) = self.handler_registry.get(&obj_type) {
                                    let pattern_deps =
                                        handler.extract_pattern_deps(&rendered_content);
                                    for (dep_name, _dep_type) in pattern_deps {
                                        deps.insert(dep_name);
                                    }
                                }
                            }

                            if deps.is_empty() { None } else { Some(deps) }
                        } else {
                            None
                        };

                        let mut header = build_header(
                            schema_name,
                            name,
                            layer,
                            requires.as_ref(),
                            self.config.generate_layers,
                        );

                        // Add identity comments for overloaded functions
                        if identities.len() > 1 {
                            let identities_comment = identities
                                .iter()
                                .map(|id| format!("--   {id}"))
                                .collect::<Vec<_>>()
                                .join("\n");
                            header.push_str(&format!("-- overloads:\n{identities_comment}\n\n"));
                        }

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

        // Write global objects
        if !self.global_objects.is_empty() {
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
                    self.config
                        .output_dir
                        .join("_global")
                        .join(category)
                        .join(subcat)
                } else {
                    self.config.output_dir.join("_global").join(category)
                };

                for obj in objects {
                    let file_path = category_dir.join(format!("{}.sql", obj.name));

                    if self.config.dry_run {
                        self.logger
                            .info(&format!("  Would create: {}", file_path.display()));
                        file_count += 1;
                    } else {
                        fs::create_dir_all(&category_dir)?;

                        // Analyze dependencies
                        let requires = if self.config.generate_deps {
                            let mut deps = self
                                .dep_analyzer
                                .as_ref()
                                .map(|analyzer| {
                                    analyzer
                                        .analyze_dependencies_for_type(&obj.content, obj.obj_type)
                                })
                                .unwrap_or_default();

                            // Add handler pattern-based dependencies
                            if let Some(handler) = self.handler_registry.get(&obj.obj_type) {
                                let pattern_deps = handler.extract_pattern_deps(&obj.content);
                                for (dep_name, _dep_type) in pattern_deps {
                                    deps.insert(dep_name);
                                }
                            }

                            if deps.is_empty() { None } else { Some(deps) }
                        } else {
                            None
                        };

                        let header = build_global_header(
                            &obj.name,
                            Some(obj.layer),
                            requires.as_ref(),
                            self.config.generate_layers,
                        );
                        let mut full_content = format!("{header}{}", obj.content);

                        // Add ACL if present
                        if self.config.include_acl && !obj.acl.is_empty() {
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
        if !self.default_acl.is_empty() && self.config.include_acl {
            let acl_dir = self.config.output_dir.join("_global").join("acl");
            let file_path = acl_dir.join("default_acl.sql");

            if self.config.dry_run {
                self.logger
                    .info(&format!("  Would create: {}", file_path.display()));
                file_count += 1;
            } else {
                fs::create_dir_all(&acl_dir)?;
                let header = build_global_header(
                    "default_acl",
                    Some(Layer::Append),
                    None,
                    self.config.generate_layers,
                );
                let content = format!("{header}{}\n", self.default_acl.join("\n"));
                fs::write(&file_path, content)?;
                file_count += 1;
            }
        }

        if self.config.dry_run {
            self.logger.info("");
            self.logger
                .info(&format!("Would create {file_count} files"));
        } else {
            self.logger.success(&format!(
                "Created {file_count} files in {}",
                self.config.output_dir.display()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_config_builder() {
        let config = OrchestratorConfig::new(PathBuf::from("/tmp/test"))
            .with_dry_run(true)
            .with_include_acl(false)
            .with_generate_layers(false);

        assert!(config.dry_run);
        assert!(!config.include_acl);
        assert!(!config.generate_layers);
        assert!(config.include_owner); // default true
    }

    #[test]
    fn test_function_subcategory() {
        use crate::commands::import::handlers::Categorizer;
        use crate::commands::import::handlers::routines::FunctionHandler;

        // Regular functions have no subcategory
        assert_eq!(FunctionHandler::subcategory("my_function"), None);

        // Casting functions
        assert_eq!(
            FunctionHandler::subcategory("cast_to_text"),
            Some("casting".to_string())
        );

        // Builder functions
        assert_eq!(
            FunctionHandler::subcategory("new_user"),
            Some("builder".to_string())
        );

        // Trigger functions
        assert_eq!(
            FunctionHandler::subcategory("trigger_audit"),
            Some("trigger".to_string())
        );

        // API functions
        assert_eq!(
            FunctionHandler::subcategory("api_users_get_v1"),
            Some("api/users/v1".to_string())
        );
    }
}

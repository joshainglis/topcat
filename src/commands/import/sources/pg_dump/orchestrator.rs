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
use std::path::{Component, Path, PathBuf};

use crate::commands::import::dependencies::DependencyAnalyzer;
use crate::commands::import::handlers::{
    HandlerRegistry, OutputConfig, RelatedObjects, categorize_primary_object, render_primary_object,
};
use crate::commands::import::object_types::{Layer, ObjectType};
use crate::commands::import::output::header_builder::HeaderBuilder;
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

    /// Convert to OutputConfig for use with Renderer trait.
    pub fn to_output_config(&self) -> OutputConfig {
        OutputConfig::from(self)
    }
}

impl From<&OrchestratorConfig> for OutputConfig {
    fn from(config: &OrchestratorConfig) -> Self {
        Self {
            include_acl: config.include_acl,
            include_owner: config.include_owner,
            generate_layers: config.generate_layers,
            generate_deps: config.generate_deps,
            dry_run: config.dry_run,
        }
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

    fn statement_in_primary_or_attachments(&self, statement: &str) -> bool {
        let statement = statement.trim();
        self.primary
            .iter()
            .chain(self.attachments.iter())
            .any(|obj| obj.content.trim() == statement)
    }

    fn has_acl_statement(&self, statement: &str) -> bool {
        let statement = statement.trim();
        self.acl.iter().any(|acl| acl.trim() == statement)
    }

    fn owner_matches(&self, statement: &str) -> bool {
        let statement = statement.trim();
        self.owner
            .as_deref()
            .is_some_and(|owner| owner.trim() == statement)
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
        if include_owner && let Some(ref owner_stmt) = self.owner {
            if !result.is_empty() {
                result.push_str("\n\n");
            }
            result.push_str(owner_stmt);
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
    owner: Option<String>,
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
        // Get handler for this object type
        let handler = self.handler_registry.get(&obj.obj_type);

        // Check if this object type should be skipped based on handler config
        if let Some(h) = handler {
            let config = h.default_config();
            if config.skip {
                log::debug!("Skipping {} (config.skip=true)", obj.qualified_name());
                return;
            }
        }

        // Handle global objects (no schema)
        // Use handler.is_global() when available, then RawObject helper, then ObjectType
        let is_global = obj.is_global()
            || handler
                .map(|h| h.is_global())
                .unwrap_or_else(|| obj.obj_type.is_global());

        if is_global {
            self.handle_global_object(obj);
            return;
        }

        let schema = obj.schema.clone().unwrap_or_else(|| "public".to_string());

        // Check if this is an attachment - use handler config, RawObject helper, or AttachmentRegistry
        // handler.default_config().attach_to_parent indicates if type attaches to parent
        let is_attachment = handler
            .map(|h| {
                let config = h.default_config();
                config.attach_to_parent || h.is_table_attachment()
            })
            .unwrap_or_else(|| obj.is_table_attachment());

        // Log extracted dependencies for debugging
        if !obj.extracted_deps().is_empty() {
            log::debug!(
                "Object {} has {} extracted dependencies",
                obj.qualified_name(),
                obj.extracted_deps().len()
            );
        }

        // Log source metadata if present (e.g., pg_dump_oid, line_number)
        if let Some(oid) = obj.get_metadata("pg_dump_oid") {
            log::trace!("Object {} has OID: {}", obj.qualified_name(), oid);
        }

        if is_attachment
            || self
                .handler_registry
                .attachment_registry()
                .is_attachment(&obj.obj_type)
        {
            if let Some(parent) = self
                .handler_registry
                .attachment_registry()
                .find_parent(&obj)
            {
                log::debug!(
                    "Attaching {} to parent {}",
                    obj.qualified_name(),
                    parent.qualified_name()
                );
                let parent_schema = parent.schema.clone().unwrap_or_else(|| schema.clone());
                self.attach_to_parent(&parent.obj_type, &parent_schema, &parent.name, obj);
                return;
            }
            // Fallback: try to attach using parent_types from config
            if let Some(h) = handler {
                let config = h.default_config();
                // Log parent types for debugging
                if !config.parent_types.is_empty() {
                    log::debug!(
                        "Object {} has configured parent types: {:?}",
                        obj.qualified_name(),
                        config.parent_types
                    );
                }
                // Try first parent type from config
                if let Some(first_parent) = config.parent_types.first() {
                    let name = obj.name.clone();
                    self.attach_to_parent(first_parent, &schema, &name, obj);
                    return;
                }
            }
            // Last resort: try Table
            if obj.target_type.is_some() {
                let name = obj.name.clone();
                self.attach_to_parent(&ObjectType::Table, &schema, &name, obj);
                return;
            }
        }

        // Primary objects - use handler.is_primary() when available, then RawObject helper
        let is_primary = handler
            .map(|h| h.is_primary())
            .unwrap_or_else(|| obj.is_primary());

        if is_primary {
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
    /// Uses handler or ObjectType::global_category() to determine output organization.
    fn handle_global_object(&mut self, obj: RawObject) {
        let (category, subcategory) = obj.obj_type.global_category();

        // Get layer from handler if available, otherwise from ObjectType
        let layer = self
            .handler_registry
            .get(&obj.obj_type)
            .map(|h| h.layer())
            .unwrap_or_else(|| obj.obj_type.layer());

        self.global_objects.push(GlobalObject {
            obj_type: obj.obj_type,
            category,
            subcategory,
            name: obj.name.clone(),
            content: obj.content,
            layer,
            owner: None,
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
    fn categorize_object(&self, obj: &RawObject) -> String {
        categorize_primary_object(obj)
    }

    /// Attach security statements to objects.
    fn attach_security(&mut self, security: Vec<SecurityStatement>) {
        for stmt in security {
            log::debug!(
                "Processing {:?} statement for {}",
                stmt.kind,
                stmt.qualified_target()
            );

            match stmt.kind {
                SecurityKind::Grant | SecurityKind::Revoke => {
                    self.attach_acl(
                        &stmt.target_type,
                        stmt.target_schema.as_deref(),
                        &stmt.target_name,
                        &stmt.content,
                    );
                }
                SecurityKind::Owner => {
                    self.attach_owner(
                        &stmt.target_type,
                        stmt.target_schema.as_deref(),
                        &stmt.target_name,
                        &stmt.content,
                    );
                }
            }
        }
    }

    /// Attach an ACL statement to an object.
    fn attach_acl(
        &mut self,
        target_type: &Option<ObjectType>,
        schema: Option<&str>,
        name: &str,
        content: &str,
    ) {
        if self.attach_acl_global(target_type, name, content) {
            return;
        }

        let candidate_schemas = match schema {
            Some(s) => vec![s],
            None => vec!["public"],
        };

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

        for candidate_schema in candidate_schemas {
            for try_type in &types_to_try {
                if let Some(schema_map) = self.objects.get_mut(try_type)
                    && let Some(name_map) = schema_map.get_mut(candidate_schema)
                    && let Some(collected) = name_map.get_mut(name)
                {
                    // Skip if the object is empty (has no primary content)
                    if !collected.is_empty() {
                        if !collected.statement_in_primary_or_attachments(content)
                            && !collected.has_acl_statement(content)
                        {
                            collected.acl.push(content.to_string());
                        }
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
        schema: Option<&str>,
        name: &str,
        content: &str,
    ) {
        if self.attach_owner_global(target_type, name, content) {
            return;
        }

        let candidate_schemas = match schema {
            Some(s) => vec![s],
            None => vec!["public"],
        };

        let types_to_try = match target_type {
            Some(ObjectType::Table) => vec![ObjectType::Table],
            Some(ObjectType::View) => vec![ObjectType::View],
            Some(ObjectType::MaterializedView) => vec![ObjectType::MaterializedView],
            Some(ObjectType::Function) => vec![ObjectType::Function],
            Some(ObjectType::Procedure) => vec![ObjectType::Procedure],
            _ => vec![ObjectType::Table, ObjectType::View, ObjectType::Function],
        };

        for candidate_schema in candidate_schemas {
            for try_type in &types_to_try {
                if let Some(schema_map) = self.objects.get_mut(try_type)
                    && let Some(name_map) = schema_map.get_mut(candidate_schema)
                    && let Some(collected) = name_map.get_mut(name)
                {
                    if collected.statement_in_primary_or_attachments(content)
                        || collected.owner_matches(content)
                    {
                        return;
                    }
                    collected.owner = Some(content.to_string());
                    return;
                }
            }
        }
    }

    /// Try attaching ACL content to a global object.
    fn attach_acl_global(
        &mut self,
        target_type: &Option<ObjectType>,
        name: &str,
        content: &str,
    ) -> bool {
        let candidate_types = match target_type {
            Some(t) => vec![*t],
            None => vec![],
        };

        for obj in &mut self.global_objects {
            let type_matches = if candidate_types.is_empty() {
                true
            } else {
                candidate_types.contains(&obj.obj_type)
            };

            if type_matches && obj.name == name {
                let statement = content.trim();
                if obj.content.trim() != statement
                    && !obj.acl.iter().any(|acl| acl.trim() == statement)
                {
                    obj.acl.push(content.to_string());
                }
                return true;
            }
        }

        false
    }

    /// Try attaching OWNER content to a global object.
    fn attach_owner_global(
        &mut self,
        target_type: &Option<ObjectType>,
        name: &str,
        content: &str,
    ) -> bool {
        let candidate_types = match target_type {
            Some(t) => vec![*t],
            None => vec![],
        };

        for obj in &mut self.global_objects {
            let type_matches = if candidate_types.is_empty() {
                true
            } else {
                candidate_types.contains(&obj.obj_type)
            };

            if type_matches && obj.name == name {
                let statement = content.trim();
                if obj.content.trim() == statement {
                    return true;
                }
                if obj
                    .owner
                    .as_deref()
                    .is_some_and(|owner| owner.trim() == statement)
                {
                    return true;
                }
                obj.owner = Some(content.to_string());
                return true;
            }
        }

        false
    }

    /// Write all files.
    fn write_files(&self) -> Result<(), TopCatError> {
        self.validate_output_paths()?;

        // Convert to OutputConfig for rendering decisions
        let output_config = self.config.to_output_config();

        // Log registered handler count for debugging
        let registered_count = self.handler_registry.registered_types().len();
        log::debug!("Writing files with {registered_count} registered handlers");

        if output_config.dry_run {
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

                    if output_config.dry_run {
                        self.logger
                            .info(&format!("  Would create: {}", file_path.display()));
                        file_count += 1;
                    } else {
                        fs::create_dir_all(&category_dir)?;

                        // Build RelatedObjects to track attachments (for future rendering use)
                        let mut related = RelatedObjects::new();

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
                                        // Populate RelatedObjects from collected attachments
                                        for attachment in &collected.attachments {
                                            match attachment.obj_type {
                                                ObjectType::Index => {
                                                    related.indexes.push(attachment.clone())
                                                }
                                                ObjectType::Constraint => {
                                                    related.constraints.push(attachment.clone())
                                                }
                                                ObjectType::FkConstraint => {
                                                    related.fk_constraints.push(attachment.clone())
                                                }
                                                ObjectType::Trigger => {
                                                    related.triggers.push(attachment.clone())
                                                }
                                                ObjectType::Policy => {
                                                    related.policies.push(attachment.clone())
                                                }
                                                ObjectType::RowSecurity => {
                                                    related.row_security.push(attachment.clone())
                                                }
                                                ObjectType::Default => {
                                                    related.defaults.push(attachment.clone())
                                                }
                                                ObjectType::Statistics => {
                                                    related.statistics.push(attachment.clone())
                                                }
                                                ObjectType::Rule => {
                                                    related.rules.push(attachment.clone())
                                                }
                                                ObjectType::Comment => {
                                                    related.comments.push(attachment.clone())
                                                }
                                                ObjectType::Acl
                                                | ObjectType::DefaultAcl
                                                | ObjectType::SecurityLabel => {
                                                    related.acl.push(attachment.content.clone())
                                                }
                                                _ => {} // Other attachment types
                                            }
                                        }
                                        // Copy ACL and owner to RelatedObjects
                                        related.acl.extend(collected.acl.iter().cloned());
                                        if collected.owner.is_some() {
                                            related.owner = collected.owner.clone();
                                        }

                                        if collected.primary.len() == 1 {
                                            render_primary_object(
                                                &collected.primary[0],
                                                &related,
                                                &output_config,
                                            )
                                        } else {
                                            collected.render(
                                                output_config.include_acl,
                                                output_config.include_owner,
                                            )
                                        }
                                    })
                            })
                            .collect();

                        // Log attachment count for debugging
                        if !related.is_empty() {
                            log::debug!(
                                "Object {}.{} has {} related objects",
                                schema_name,
                                name,
                                related.count()
                            );
                        }

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
                            if let Some(obj_type) = primary_type
                                && let Some(handler) = self.handler_registry.get(&obj_type)
                            {
                                // Log handler object type for debugging
                                let _ = handler.object_type();

                                // Get implicit dependency types for this handler
                                let implicit_types = handler.implicit_dep_types();
                                let _ = implicit_types; // Available for future filtering

                                let pattern_deps = handler.extract_pattern_deps(&rendered_content);
                                for (dep_name, _dep_type) in pattern_deps {
                                    deps.insert(dep_name);
                                }
                            }

                            if deps.is_empty() { None } else { Some(deps) }
                        } else {
                            None
                        };

                        // Build header using HeaderBuilder with chainable methods
                        let mut builder = HeaderBuilder::new(schema_name, name)
                            .with_layer_generation(output_config.generate_layers);

                        // Add layer if available using chainable method
                        if let Some(l) = layer {
                            builder = builder.layer(l);
                        }

                        // Add dependencies using chainable method
                        if let Some(ref deps) = requires {
                            builder = builder.requires(deps.iter().cloned());
                        }

                        let mut header = builder.build();

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

                    if output_config.dry_run {
                        self.logger
                            .info(&format!("  Would create: {}", file_path.display()));
                        file_count += 1;
                    } else {
                        fs::create_dir_all(&category_dir)?;

                        // Check if handler exists for this type (use has_handler)
                        let has_handler = self.handler_registry.has_handler(&obj.obj_type);

                        // Analyze dependencies
                        let requires = if output_config.generate_deps {
                            let mut deps = self
                                .dep_analyzer
                                .as_ref()
                                .map(|analyzer| {
                                    analyzer
                                        .analyze_dependencies_for_type(&obj.content, obj.obj_type)
                                })
                                .unwrap_or_default();

                            // Add handler pattern-based dependencies if handler exists
                            if has_handler
                                && let Some(handler) = self.handler_registry.get(&obj.obj_type)
                            {
                                let pattern_deps = handler.extract_pattern_deps(&obj.content);
                                for (dep_name, _dep_type) in pattern_deps {
                                    deps.insert(dep_name);
                                }
                            }

                            if deps.is_empty() { None } else { Some(deps) }
                        } else {
                            None
                        };

                        // Build header using HeaderBuilder with chainable methods
                        let mut builder = HeaderBuilder::global(&obj.name)
                            .layer(obj.layer)
                            .with_layer_generation(output_config.generate_layers);

                        // Add each dependency using require() chainable method
                        if let Some(ref deps) = requires {
                            for dep in deps.iter() {
                                builder = builder.require(dep.clone());
                            }
                        }

                        let header = builder.build();
                        let mut full_content = format!("{header}{}", obj.content);

                        // Add owner if present
                        if output_config.include_owner
                            && let Some(owner_stmt) = &obj.owner
                        {
                            full_content.push_str("\n\n");
                            full_content.push_str(owner_stmt);
                        }

                        // Add ACL if present
                        if output_config.include_acl && !obj.acl.is_empty() {
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
        if !self.default_acl.is_empty() && output_config.include_acl {
            let acl_dir = self.config.output_dir.join("_global").join("acl");
            let file_path = acl_dir.join("default_acl.sql");

            if output_config.dry_run {
                self.logger
                    .info(&format!("  Would create: {}", file_path.display()));
                file_count += 1;
            } else {
                fs::create_dir_all(&acl_dir)?;
                // Use HeaderBuilder with layer() method for default ACL
                let header = HeaderBuilder::global("default_acl")
                    .layer(Layer::Append)
                    .with_layer_generation(output_config.generate_layers)
                    .build();
                let content = format!("{header}{}\n", self.default_acl.join("\n"));
                fs::write(&file_path, content)?;
                file_count += 1;
            }
        }

        if output_config.dry_run {
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

    fn validate_output_paths(&self) -> Result<(), TopCatError> {
        for (schema_name, categories) in &self.schemas {
            Self::validate_file_stem(schema_name, "schema name")?;
            for (category, items) in categories {
                Self::validate_relative_path(category, "category path")?;
                for name in items.keys() {
                    Self::validate_file_stem(name, "object name")?;
                }
            }
        }

        for obj in &self.global_objects {
            Self::validate_relative_path(&obj.category, "global category path")?;
            if let Some(subcategory) = &obj.subcategory {
                Self::validate_relative_path(subcategory, "global subcategory path")?;
            }
            Self::validate_file_stem(&obj.name, "global object name")?;
        }

        Ok(())
    }

    fn validate_file_stem(value: &str, field: &str) -> Result<(), TopCatError> {
        if value.is_empty() || value.contains('\0') || value.contains('/') || value.contains('\\') {
            return Err(TopCatError::config_error(format!(
                "Unsafe {field} '{value}' in import metadata"
            )));
        }

        let mut components = Path::new(value).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(part)), None) => {
                let part = part.to_string_lossy();
                if part == "." || part == ".." {
                    Err(TopCatError::config_error(format!(
                        "Unsafe {field} '{value}' in import metadata"
                    )))
                } else {
                    Ok(())
                }
            }
            _ => Err(TopCatError::config_error(format!(
                "Unsafe {field} '{value}' in import metadata"
            ))),
        }
    }

    fn validate_relative_path(value: &str, field: &str) -> Result<(), TopCatError> {
        if value.is_empty() || value.contains('\0') || value.contains('\\') {
            return Err(TopCatError::config_error(format!(
                "Unsafe {field} '{value}' in import metadata"
            )));
        }

        let mut has_component = false;
        for component in Path::new(value).components() {
            match component {
                Component::Normal(part) => {
                    has_component = true;
                    let part = part.to_string_lossy();
                    if part == "." || part == ".." {
                        return Err(TopCatError::config_error(format!(
                            "Unsafe {field} '{value}' in import metadata"
                        )));
                    }
                }
                _ => {
                    return Err(TopCatError::config_error(format!(
                        "Unsafe {field} '{value}' in import metadata"
                    )));
                }
            }
        }

        if !has_component {
            return Err(TopCatError::config_error(format!(
                "Unsafe {field} '{value}' in import metadata"
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn quiet_logger() -> Logger {
        Logger::new(true, false)
    }

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

    #[test]
    fn test_process_attaches_index_to_table_via_extracted_deps() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("output");

        let table = RawObject::new(
            ObjectType::Table,
            Some("public".to_string()),
            "users".to_string(),
            r#"CREATE TABLE "public"."users" (id integer);"#.to_string(),
        );

        let mut index = RawObject::new(
            ObjectType::Index,
            Some("public".to_string()),
            "users_id_idx".to_string(),
            r#"CREATE INDEX "users_id_idx" ON "public"."users" (id);"#.to_string(),
        );
        index.add_extracted_dep("public.users", ObjectType::Table);

        let mut orchestrator =
            ImportOrchestrator::new(OrchestratorConfig::new(output_dir.clone()), quiet_logger());

        orchestrator
            .process(vec![table, index], vec![], vec![])
            .unwrap();

        let table_file = output_dir.join("public/table/users.sql");
        assert!(table_file.exists(), "expected table file at {table_file:?}");

        let content = fs::read_to_string(table_file).unwrap();
        assert!(content.contains(r#"CREATE TABLE "public"."users""#));
        assert!(content.contains(r#"CREATE INDEX "users_id_idx""#));
    }

    #[test]
    fn test_process_routes_schema_and_global_acl_owner_statements() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("output");

        let table = RawObject::new(
            ObjectType::Table,
            Some("public".to_string()),
            "users".to_string(),
            r#"CREATE TABLE "public"."users" (id integer);"#.to_string(),
        );
        let server = RawObject::new(
            ObjectType::Server,
            None,
            "remote_server".to_string(),
            r#"CREATE SERVER "remote_server" FOREIGN DATA WRAPPER "postgres_fdw";"#.to_string(),
        );

        let table_grant = r#"GRANT SELECT ON TABLE "public"."users" TO "reader";"#.to_string();
        let table_owner = r#"ALTER TABLE "public"."users" OWNER TO "admin";"#.to_string();
        let server_grant =
            r#"GRANT USAGE ON FOREIGN SERVER "remote_server" TO "reader";"#.to_string();
        let server_owner = r#"ALTER SERVER "remote_server" OWNER TO "admin";"#.to_string();

        let security = vec![
            SecurityStatement::grant(
                Some("public".to_string()),
                "users".to_string(),
                Some(ObjectType::Table),
                table_grant.clone(),
            ),
            SecurityStatement::owner(
                Some("public".to_string()),
                "users".to_string(),
                Some(ObjectType::Table),
                table_owner.clone(),
            ),
            SecurityStatement::grant(
                None,
                "remote_server".to_string(),
                Some(ObjectType::Server),
                server_grant.clone(),
            ),
            SecurityStatement::owner(
                None,
                "remote_server".to_string(),
                Some(ObjectType::Server),
                server_owner.clone(),
            ),
        ];

        let mut orchestrator =
            ImportOrchestrator::new(OrchestratorConfig::new(output_dir.clone()), quiet_logger());

        orchestrator
            .process(vec![table, server], security, vec![])
            .unwrap();

        let table_file = output_dir.join("public/table/users.sql");
        let table_content = fs::read_to_string(table_file).unwrap();
        assert!(table_content.contains(&table_grant));
        assert!(table_content.contains(&table_owner));

        let server_file = output_dir.join("_global/fdw/server/remote_server.sql");
        let server_content = fs::read_to_string(server_file).unwrap();
        assert!(server_content.contains(&server_grant));
        assert!(server_content.contains(&server_owner));
    }

    #[test]
    fn test_process_generates_requires_header_from_handler_pattern_deps() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("output");

        let foreign_table = RawObject::new(
            ObjectType::ForeignTable,
            Some("public".to_string()),
            "remote_users".to_string(),
            r#"CREATE FOREIGN TABLE "public"."remote_users" (id integer) SERVER "remote_server";"#
                .to_string(),
        );

        let mut orchestrator =
            ImportOrchestrator::new(OrchestratorConfig::new(output_dir.clone()), quiet_logger());

        orchestrator
            .process(vec![foreign_table], vec![], vec![])
            .unwrap();

        let file = output_dir.join("public/foreign_table/remote_users.sql");
        let content = fs::read_to_string(file).unwrap();
        assert!(content.contains("-- requires:"));
        assert!(content.contains("remote_server"));
    }

    #[test]
    fn test_process_rejects_unsafe_schema_name() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("output");

        let table = RawObject::new(
            ObjectType::Table,
            Some("../escape".to_string()),
            "users".to_string(),
            r#"CREATE TABLE "users" (id integer);"#.to_string(),
        );

        let mut orchestrator =
            ImportOrchestrator::new(OrchestratorConfig::new(output_dir.clone()), quiet_logger());

        let result = orchestrator.process(vec![table], vec![], vec![]);
        match result {
            Err(TopCatError::ConfigError(msg)) => {
                assert!(msg.contains("Unsafe schema name"), "got message: {msg}");
            }
            other => panic!("expected ConfigError for unsafe schema, got {other:?}"),
        }
        assert!(
            !output_dir.exists(),
            "output dir should not be created on validation failure"
        );
    }

    #[test]
    fn test_process_rejects_unsafe_global_object_name() {
        let temp_dir = TempDir::new().unwrap();
        let output_dir = temp_dir.path().join("output");

        let server = RawObject::new(
            ObjectType::Server,
            None,
            "../remote_server".to_string(),
            r#"CREATE SERVER "remote_server" FOREIGN DATA WRAPPER "postgres_fdw";"#.to_string(),
        );

        let mut orchestrator =
            ImportOrchestrator::new(OrchestratorConfig::new(output_dir.clone()), quiet_logger());

        let result = orchestrator.process(vec![server], vec![], vec![]);
        match result {
            Err(TopCatError::ConfigError(msg)) => {
                assert!(
                    msg.contains("Unsafe global object name"),
                    "got message: {msg}"
                );
            }
            other => panic!("expected ConfigError for unsafe global object, got {other:?}"),
        }
        assert!(
            !output_dir.exists(),
            "output dir should not be created on validation failure"
        );
    }
}

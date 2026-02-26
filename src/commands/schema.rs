//! Schema management commands.
//!
//! This module provides commands for inspecting and analyzing schemas
//! in the dependency graph.
//!
//! # Available Commands
//!
//! - **list**: List all schemas with statistics
//! - **analyze**: Analyze a specific schema in detail
//! - **dependencies**: Show cross-schema dependencies
//!
//! # Examples
//!
//! ```bash
//! # List all schemas
//! topcat schema -i sql/ -e sql list
//!
//! # Analyze a specific schema
//! topcat schema -i sql/ -e sql analyze my_schema
//!
//! # Show cross-schema dependencies
//! topcat schema -i sql/ -e sql dependencies
//! ```

use clap::{Args, Subcommand};
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};
use std::collections::HashMap;

use topcat::cli::{GlobalArgs, GraphInputArgs};
use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;
use topcat::logging::Logger;
use topcat::settings::Settings;

use super::common as cmd_common;

#[derive(Debug, Subcommand)]
pub enum SchemaCommand {
    /// List all schemas with statistics
    List,
    /// Analyze a specific schema in detail
    Analyze {
        /// Schema name to analyze
        schema: String,
    },
    /// Show cross-schema dependencies
    Dependencies,
}

/// Command-line arguments for the schema subcommand.
///
/// The schema command uses GlobalArgs + GraphInputArgs since it needs to
/// build the dependency graph but doesn't need analysis or filtering options.
#[derive(Debug, Args)]
pub struct SchemaArgs {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(flatten)]
    pub input: GraphInputArgs,

    #[command(subcommand)]
    command: SchemaCommand,
}

impl SchemaArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // 1-2. Load settings and apply CLI overrides
        let settings =
            cmd_common::load_settings_with_overrides(self.global.config_path(), |settings| {
                self.global.apply_to_settings(settings);
                self.input.apply_to_settings(settings);
            })?;

        // 3-4. Validate settings
        cmd_common::validate_settings(&settings, true)?;

        // 5. Initialize logging
        let logger = cmd_common::init_logger_from_settings(&settings);

        // 6. Build the graph
        let graph = self.build_graph(&settings)?;

        // 7. Execute the requested schema operation
        match &self.command {
            SchemaCommand::List => self.list_schemas(&graph, &logger),
            SchemaCommand::Analyze { schema } => self.analyze_schema(&graph, schema, &logger),
            SchemaCommand::Dependencies => self.show_dependencies(&graph, &logger),
        }
    }

    fn build_graph(&self, settings: &Settings) -> Result<TCGraph, TopCatError> {
        cmd_common::build_graph_from_settings(&None, settings)
    }

    fn list_schemas(&self, graph: &TCGraph, logger: &Logger) -> Result<(), TopCatError> {
        let schemas = graph.get_schemas();

        if schemas.is_empty() {
            logger.info("No schemas found in the project.");
            return Ok(());
        }

        // Calculate statistics
        let mut stats: Vec<(String, usize, usize)> = schemas
            .iter()
            .map(|(schema_name, nodes)| {
                let file_count = nodes.len();
                let dep_count: usize = nodes.iter().map(|n| n.deps.len()).sum();
                (schema_name.clone(), file_count, dep_count)
            })
            .collect();

        // Sort by schema name
        stats.sort_by(|a, b| a.0.cmp(&b.0));

        // Create table
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Schema").fg(Color::Cyan),
                Cell::new("Files").fg(Color::Cyan),
                Cell::new("Dependencies").fg(Color::Cyan),
                Cell::new("Distribution").fg(Color::Cyan),
            ]);

        let max_files = stats.iter().map(|(_, f, _)| *f).max().unwrap_or(1);

        for (schema, file_count, dep_count) in stats {
            // Create a simple bar chart
            let bar_width = 40;
            let filled = (file_count * bar_width) / max_files.max(1);
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(bar_width - filled));

            table.add_row(vec![
                Cell::new(&schema),
                Cell::new(file_count),
                Cell::new(dep_count),
                Cell::new(&bar).fg(Color::Green),
            ]);
        }

        logger.table(&table);
        Ok(())
    }

    fn analyze_schema(
        &self,
        graph: &TCGraph,
        schema: &str,
        logger: &Logger,
    ) -> Result<(), TopCatError> {
        let schemas = graph.get_schemas();
        let nodes = schemas
            .get(schema)
            .ok_or_else(|| TopCatError::UnknownError(format!("Schema '{schema}' not found")))?;

        logger.newline();
        logger.section(&format!("Schema: {schema}"));

        // Files in schema
        logger.info(&format!("Files ({}):", nodes.len()));
        let mut node_names: Vec<_> = nodes.iter().map(|n| &n.name).collect();
        node_names.sort();
        for name in node_names {
            logger.info(&format!("  - {name}"));
        }

        // Internal dependencies
        let internal_deps = graph.get_internal_dependencies(schema);
        logger.newline();
        logger.info(&format!("Internal Dependencies ({}):", internal_deps.len()));
        if internal_deps.is_empty() {
            logger.info("  (none)");
        } else {
            let mut deps: Vec<_> = internal_deps.iter().collect();
            deps.sort();
            for (source, target) in deps {
                logger.info(&format!("  {source} → {target}"));
            }
        }

        // External dependencies
        let external_deps = graph.get_external_dependencies(schema);
        logger.newline();
        logger.info(&format!("External Dependencies ({}):", external_deps.len()));
        if external_deps.is_empty() {
            logger.info("  (none)");
        } else {
            // Group by target schema
            let mut by_schema: HashMap<String, Vec<(String, String)>> = HashMap::new();
            for (source, target, target_schema) in external_deps {
                by_schema
                    .entry(target_schema)
                    .or_default()
                    .push((source, target));
            }

            let mut schemas: Vec<_> = by_schema.keys().collect();
            schemas.sort();

            for target_schema in schemas {
                let deps = by_schema
                    .get(target_schema)
                    .expect("schema key must exist in map we just collected from");
                logger.newline();
                logger.info(&format!("  To schema '{target_schema}':"));
                for (source, target) in deps {
                    logger.info(&format!("    {source} → {target}"));
                }
            }
        }

        // Dependent schemas (who depends on this schema)
        let dependent_schemas = graph.get_dependent_schemas(schema);
        logger.newline();
        logger.info(&format!("Dependent Schemas ({}):", dependent_schemas.len()));
        if dependent_schemas.is_empty() {
            logger.info("  (none)");
        } else {
            let mut deps: Vec<_> = dependent_schemas.iter().collect();
            deps.sort();
            for dep_schema in deps {
                logger.info(&format!("  {dep_schema}"));
            }
        }

        Ok(())
    }

    fn show_dependencies(&self, graph: &TCGraph, logger: &Logger) -> Result<(), TopCatError> {
        let cross_deps = graph.get_cross_schema_dependencies();

        if cross_deps.is_empty() {
            logger.info("No cross-schema dependencies found.");
            return Ok(());
        }

        logger.newline();
        logger.section("Cross-Schema Dependencies");

        // Create table
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Source Schema").fg(Color::Cyan),
                Cell::new("→").fg(Color::Yellow),
                Cell::new("Target Schema").fg(Color::Cyan),
            ]);

        for (source, target) in cross_deps {
            table.add_row(vec![
                Cell::new(&source).fg(Color::Green),
                Cell::new("→").fg(Color::Yellow),
                Cell::new(&target).fg(Color::Blue),
            ]);
        }

        logger.table(&table);

        Ok(())
    }
}

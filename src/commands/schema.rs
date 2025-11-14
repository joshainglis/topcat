use clap::{Args, Subcommand};
use comfy_table::{Cell, Color, Table, modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL};
use std::collections::HashMap;
use std::path::PathBuf;

use topcat::exceptions::TopCatError;
use topcat::file_dag::TCGraph;

use super::common;

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

#[derive(Debug, Args)]
pub struct SchemaArgs {
    /// Input directories containing files to analyze
    #[arg(short = 'i', long = "input-dirs", required = true)]
    input_dirs: Vec<PathBuf>,

    /// File extensions to include (e.g., "sql")
    #[arg(short = 'e', long = "include-exts")]
    include_extensions: Vec<String>,

    /// Comment string used in file headers
    #[arg(short = 'c', long = "comment-str", default_value = "--")]
    comment_str: String,

    /// Custom layer ordering (comma-separated)
    #[arg(short = 'l', long = "layers")]
    layers: Option<String>,

    /// Fallback layer for files without explicit layer declaration
    #[arg(short = 'f', long = "fallback-layer", default_value = "normal")]
    fallback_layer: String,

    #[command(subcommand)]
    command: SchemaCommand,
}

impl SchemaArgs {
    pub fn execute(&self) -> Result<(), TopCatError> {
        // Build the graph
        let graph = self.build_graph()?;

        match &self.command {
            SchemaCommand::List => self.list_schemas(&graph),
            SchemaCommand::Analyze { schema } => self.analyze_schema(&graph, schema),
            SchemaCommand::Dependencies => self.show_dependencies(&graph),
        }
    }

    fn build_graph(&self) -> Result<TCGraph, TopCatError> {
        let (layers, fallback_layer) =
            common::parse_and_validate_layers(&self.layers, &Some(self.fallback_layer.clone()))?;

        common::build_graph(
            self.input_dirs.clone(),
            if self.include_extensions.is_empty() {
                None
            } else {
                Some(&self.include_extensions)
            },
            None,  // exclude_extensions
            None,  // include_globs
            None,  // exclude_globs
            false, // include_hidden
            false, // verbose
            self.comment_str.clone(),
            layers,
            fallback_layer,
            Default::default(), // sql_discovery
            None,               // schema_filter_prefixes
        )
    }

    fn list_schemas(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let schemas = graph.get_schemas();

        if schemas.is_empty() {
            println!("No schemas found in the project.");
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

        println!("{table}");
        Ok(())
    }

    fn analyze_schema(&self, graph: &TCGraph, schema: &str) -> Result<(), TopCatError> {
        let schemas = graph.get_schemas();
        let nodes = schemas
            .get(schema)
            .ok_or_else(|| TopCatError::UnknownError(format!("Schema '{schema}' not found")))?;

        println!("\nSchema: {schema}\n");

        // Files in schema
        println!("Files ({}):", nodes.len());
        let mut node_names: Vec<_> = nodes.iter().map(|n| &n.name).collect();
        node_names.sort();
        for name in node_names {
            println!("  - {name}");
        }

        // Internal dependencies
        let internal_deps = graph.get_internal_dependencies(schema);
        println!("\nInternal Dependencies ({}):", internal_deps.len());
        if internal_deps.is_empty() {
            println!("  (none)");
        } else {
            let mut deps: Vec<_> = internal_deps.iter().collect();
            deps.sort();
            for (source, target) in deps {
                println!("  {source} → {target}");
            }
        }

        // External dependencies
        let external_deps = graph.get_external_dependencies(schema);
        println!("\nExternal Dependencies ({}):", external_deps.len());
        if external_deps.is_empty() {
            println!("  (none)");
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
                println!("\n  To schema '{target_schema}':");
                for (source, target) in deps {
                    println!("    {source} → {target}");
                }
            }
        }

        // Dependent schemas (who depends on this schema)
        let dependent_schemas = graph.get_dependent_schemas(schema);
        println!("\nDependent Schemas ({}):", dependent_schemas.len());
        if dependent_schemas.is_empty() {
            println!("  (none)");
        } else {
            let mut deps: Vec<_> = dependent_schemas.iter().collect();
            deps.sort();
            for dep_schema in deps {
                println!("  {dep_schema}");
            }
        }

        Ok(())
    }

    fn show_dependencies(&self, graph: &TCGraph) -> Result<(), TopCatError> {
        let cross_deps = graph.get_cross_schema_dependencies();

        if cross_deps.is_empty() {
            println!("No cross-schema dependencies found.");
            return Ok(());
        }

        println!("\nCross-Schema Dependencies:\n");

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

        println!("{table}");

        Ok(())
    }
}

use clap::{Parser, Subcommand};

use topcat::exceptions::TopCatError;

mod commands;

/// Topcat - Topological file concatenation and dependency analysis tool
#[derive(Debug, Parser)]
#[command(name = "topcat")]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Concatenate files in topological order based on dependencies
    Concat(commands::concat::ConcatArgs),
    /// Analyze dependency structure and find cleanup candidates
    Analyze(commands::analyze::AnalyzeArgs),
    /// Remove unused files based on dependency analysis
    Clean(commands::clean::CleanArgs),
    /// Analyze and manage schemas in the project
    Schema(commands::schema::SchemaArgs),
}

fn main() -> Result<(), TopCatError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Concat(args) => args.execute(),
        Commands::Analyze(args) => args.execute(),
        Commands::Clean(args) => args.execute(),
        Commands::Schema(args) => args.execute(),
    }
}

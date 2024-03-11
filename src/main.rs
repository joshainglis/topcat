use std::collections::HashMap;
use std::path::PathBuf;
use std::string::ToString;

use env_logger::Builder;
use log::{error, info, LevelFilter};
use structopt::StructOpt;

use file_dag::TCGraph;

use crate::exceptions::TopCatError;
use crate::file_dag::TCGraphType;

mod config;
mod exceptions;
mod file_dag;
mod file_node;
mod fs;
mod io_utils;
mod output;
mod stable_topo;

#[derive(Debug, StructOpt)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(
        short = "i",
        long = "input_dir",
        help = "Path to directory containing files to be concatenated",
        value_name = "DIR"
    )]
    input_dirs: Vec<PathBuf>,

    #[structopt(
        short = "n",
        long = "include",
        help = "Only include files matching glob pattern",
        value_name = "PATTERN"
    )]
    include_globs: Option<Vec<String>>,

    #[structopt(
        short = "x",
        long = "exclude",
        help = "Exclude files matching given glob pattern",
        value_name = "PATTERN"
    )]
    exclude_globs: Option<Vec<String>>,

    #[structopt(
        short = "o",
        long = "output",
        help = "Path to generate combined output file",
        value_name = "FILE"
    )]
    output: PathBuf,

    #[structopt(
        long = "comment-str",
        help = "The string used to denote a comment. eg '--'",
        default_value = "--"
    )]
    comment_str: String,

    #[structopt(
        long = "file-separator-str",
        help = "Add this between each concatenated file in the output. eg '---'",
        default_value = "------------------------------------------------------------------------------------------------------------------------"
    )]
    file_separator_str: String,

    #[structopt(
        long = "ensure-each-file-ends-with",
        help = "Add this string to the end of files if it does not exist. eg ';'",
        default_value = ";"
    )]
    ensure_each_file_ends_with_str: String,

    #[structopt(short = "v", long = "verbose", help = "Print debug information")]
    verbose: bool,

    #[structopt(long = "dry", help = "Only print the output, do not write to file.")]
    dry_run: bool,
}

fn main() -> Result<(), TopCatError> {
    let opt = Opt::from_args();
    if opt.verbose {
        Builder::new().filter(None, LevelFilter::Debug).init();
    } else {
        Builder::new().filter(None, LevelFilter::Info).init();
    }

    let config = config::Config {
        input_dirs: opt.input_dirs,
        include_globs: opt.include_globs.as_deref(),
        exclude_globs: opt.exclude_globs.as_deref(),
        output: opt.output,
        comment_str: opt.comment_str,
        file_separator_str: opt.file_separator_str,
        file_end_str: opt.ensure_each_file_ends_with_str,
        verbose: opt.verbose,
        dry_run: opt.dry_run,
    };

    let mut filedag = TCGraph::new(
        config.comment_str.clone(),
        config.input_dirs.clone(),
        config.exclude_globs.clone(),
        config.include_globs.clone(),
    );
    let res = filedag.build_graph();
    match res {
        Ok(_) => {
            info!("Graph built successfully!");
        }
        Err(e) => {
            let mut map = HashMap::new();
            map.insert(1, e);
            error!("Initialization Failure:\n{:#?}\n\nExiting.", map);
            std::process::exit(1);
        }
    }

    if config.verbose {
        println!(
            "Prepend Graph: {:#?}",
            filedag.graph_as_dot(TCGraphType::Prepend)?
        );
        println!("Graph: {:#?}", filedag.graph_as_dot(TCGraphType::Normal)?);
        println!(
            "Append Graph: {:#?}",
            filedag.graph_as_dot(TCGraphType::Append)?
        );
    }

    let result = output::generate(filedag, config, &mut fs::RealFileSystem);

    match result {
        Ok(()) => {
            info!("Generation Successful!");
        }
        Err(e) => {
            let mut map = HashMap::new();
            map.insert(1, e);
            error!("Initialization Failure:\n{:#?}\n\nExiting.", map);
            std::process::exit(1);
        }
    }

    Ok(())
}

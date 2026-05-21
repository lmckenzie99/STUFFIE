//5 - 19 - 2026
//STUFFIE: Sequential Task Utility For Flow Integration and Execution
//First Draft: CLAP Parser + Execution Mode Logic


// -----------------------------------------------------------------------------
// Imports:
// - clap for CLI parsing
// - std::path::PathBuf for filesystem paths
// -----------------------------------------------------------------------------
use clap::{ArgGroup, Parser};
use std::path::PathBuf;


// -----------------------------------------------------------------------------
// CLI Definition
// -----------------------------------------------------------------------------


// using parser and debug derives for easy CLI parsing and debugging
#[derive(Parser, Debug)]
// clap command attributes for name, version, about, and long_about
#[command(name = "stuffie")]
#[command(version)]
#[command(about = "Sequential Task Utility For Flow Integration and Execution")]
#[command(long_about = "
STUFFIE is a minimal pipeline orchestrator for sequential multi-stage workflows.
Each stage invokes a standalone program. Pipelines are defined in TOML config files.

Execution modes are mutually exclusive — exactly one must be provided.
")]


// defining an argument group named "mode" that requires exactly one of the three execution mode flags
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .args(["full_auto", "semi_auto", "single_break"])
))]

// the CLI struct with fields for config path and execution mode flags
// the config field is a required PathBuf with short and long flags
struct Cli {
    /// Path to the pipeline config file
    #[arg(short = 'c', long = "config", required = true, value_name = "FILE")]
    config: PathBuf,

    /// Full-auto: run all stages start to finish, no pauses
    #[arg(short = 'a', conflicts_with_all = ["semi_auto", "single_break"])]
    full_auto: bool,

    /// Semi-auto: pause at stages marked qa = true in config
    #[arg(short = 's', conflicts_with_all = ["full_auto", "single_break"])]
    semi_auto: bool,

    /// Single break: pause and prompt after every stage
    #[arg(short = 'b', conflicts_with_all = ["full_auto", "semi_auto"])]
    single_break: bool,
}

// -----------------------------------------------------------------------------
// Execution Mode
// -----------------------------------------------------------------------------

/// Represents the three mutually exclusive execution modes STUFFIE supports.
/// Derived from the raw CLI flags so downstream logic never touches bools.



#[derive(Debug)]
enum ExecutionMode {
    /// Run all stages unattended, no QA pauses
    FullAuto,
    /// Pause only at stages marked `qa = true` in config
    SemiAuto,
    /// Pause and prompt after every stage regardless of qa flag
    SingleBreak,
}

/*
from_cli is a function that belongs to ExecutionMode, 
takes a reference to a Cli struct, 
reads the three bool flags, and returns the right variant. 
*/

impl ExecutionMode {
    fn from_cli(cli: &Cli) -> Self {
        if cli.full_auto {
            ExecutionMode::FullAuto
        } else if cli.semi_auto {
            ExecutionMode::SemiAuto
        } else {
            ExecutionMode::SingleBreak
        }
    }
}

// -----------------------------------------------------------------------------
// Entry Point
// -----------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();
    
    // Call the from_cli function that belongs to ExecutionMode, passing in a reference to cli, 
    // and store the result in a variable named mode
    let mode = ExecutionMode::from_cli(&cli);

    // Validate that the config file actually exists before doing anything else
    if !cli.config.exists() {
        eprintln!(
            "[STUFFIE] Error: config file not found: {}",
            cli.config.display()
        );
        std::process::exit(1);
    }

    println!("[STUFFIE] Config:  {}", cli.config.display());
    println!("[STUFFIE] Mode:    {:?}", mode);

    // TODO: deserialize PipelineConfig from config file (serde + toml)
    // TODO: check output_dir for .stuffie_checkpoint, prompt resume or restart
    // TODO: iterate stages, spawn subprocesses via tokio::process::Command
}

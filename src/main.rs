//6 - 2 - 2026
//STUFFIE: Sequential Task Utility For Flow Integration and Execution
//Newest edit: Program Execution Module


// -----------------------------------------------------------------------------
// Imports:
// - clap for CLI parsing
// - std::path::PathBuf for filesystem paths
// - std::process::Command for spawning subprocesses
// -----------------------------------------------------------------------------
use clap::{ArgGroup, Parser};
use std::path::PathBuf;
use std::process::Command;

// Local module for config structs and loader function
mod config;


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

// A demo function to show how we might execute a stage based on the execution mode
// Using Google Chrome as a demo program to launch
// run_stages() below is the real implementation to be used.
fn run_demo_program(mode: &ExecutionMode) {
    match mode {
        ExecutionMode::FullAuto => {
            println!("[STUFFIE] Launching Google Chrome...");

            Command::new("open")
                .arg("-a")
                .arg("Google Chrome")
                .spawn()
                .expect("Failed to launch Google Chrome");
        }

        ExecutionMode::SemiAuto => {
            println!("[STUFFIE] SemiAuto selected (demo not implemented yet)");
        }

        ExecutionMode::SingleBreak => {
            println!("[STUFFIE] SingleBreak selected (demo not implemented yet)");
        }
    }
}

// A function to run the stages defined in the config file, demonstrating how we might summon subprocesses
// For now, it just prints the stage name and simulates running the command.
fn run_stages(stages: &Vec<config::Stage>) {
    for stage in stages {
        println!("[STUFFIE] Running stage: {}", stage.name);
        
        let mut cmd = Command::new(&stage.cmd);
        
        // If the stage has args, add them to the command
        if let Some(args) = &stage.args {
            cmd.args(args);
        }
        
        // Spawn the command and wait for it to finish, checking the exit status
        let status = cmd.status()
            .expect(&format!("Failed to launch stage: {}", stage.name));
        
        // If the command failed (non-zero exit code), print an error and exit the whole program
        if !status.success() {
            eprintln!("[STUFFIE] Stage '{}' failed with exit code {:?}", stage.name, status.code());
            std::process::exit(1);
        }
        
        // Or else, print that the stage completed successfully
        println!("[STUFFIE] Stage '{}' completed ", stage.name);
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

    // Load and deserialize the config file
    let pipeline_config = match config::load_config(&cli.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    println!("[STUFFIE] Pipeline: {}", pipeline_config.pipeline.name);
    println!("[STUFFIE] Stages:   {}", pipeline_config.stages.len());

    //Running stages is the core of the program execution module, 
    //So we'll call a function named run_stages and pass in a reference to the stages vector from the pipeline config.
    run_stages(&pipeline_config.stages);

    // TODO: check output_dir for .stuffie_checkpoint, prompt resume or restart
}

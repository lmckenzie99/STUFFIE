// 6 - 20 - 2026
// STUFFIE: Sequential Task Utility For Flow Integration and Execution
// Newest edit: Finalizing v1 of STUFFIE, cleaning up code, adding comments, and preparing for release.

// The main entry point for the STUFFIE pipeline orchestrator.
// This file defines the CLI, loads the config, and runs the stages in sequence based on the selected execution mode.

// -----------------------------------------------------------------------------
// Imports:
// - clap for CLI parsing
// - std::path::PathBuf for filesystem paths
// - std::path::Path for path manipulation
// - tokio::process::Command for asynchronous subprocess management
// - tracing and tracing_subscriber for logging
// - config module for config structs and loader function
// -----------------------------------------------------------------------------

use clap::{ArgGroup, Parser};
use std::path::PathBuf;
use std::path::Path;
use tokio::process::Command; 
use tracing::{info, error};
use tracing_subscriber;

// Local module for config structs and loader function
mod config;

// -----------------------------------------------------------------------------
// CLI Definition
// -----------------------------------------------------------------------------

// Using parser and debug derives for easy CLI parsing and debugging
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


// Defining an argument group named "mode" that requires exactly one of the three execution mode flags
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .args(["full_auto", "semi_auto", "single_break"])
))]

// The CLI struct with fields for config path and execution mode flags
// The config field is a required PathBuf with short and long flags
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

//run_one is a helper function to run a single command with arguments, and print the output or error
async fn run_one(cmd: &str, args: &[&str]) {
    println!("[STUFFIE] Running command: {} {}", cmd, args.join(" "));

    let output = Command::new(cmd)
        .args(args)
        .output()
        .await
        .expect(&format!("Failed to execute command: {} {}", cmd, args.join(" ")));

        if output.status.success(){
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("[STUFFIE] Command output: {}", stdout);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("[STUFFIE] Command error: {}", stderr);

        }
}

// A demo function to show how we might execute a stage based on the execution mode
// Using Google Chrome as a demo program to launch
// run_stages() below is the real implementation to be used.
async fn run_demo_program(mode: &ExecutionMode) {
    match mode {
        ExecutionMode::FullAuto => {
            println!("[STUFFIE] FullAuto selected, running demo program...");
            println!("[STUFFIE] Launching Google Chrome and Date...");

            tokio::join!(
                run_one("open", &["-a", "Google Chrome"]),
                run_one("date", &[]),
            );

            
        }

        ExecutionMode::SemiAuto => {
            println!("[STUFFIE] SemiAuto selected, running demo program...");
            println!("[STUFFIE] Launching Safari...");
            
            run_one("open", &["-a", "Safari"]).await;
            

        }

        ExecutionMode::SingleBreak => {
            println!("[STUFFIE] SingleBreak selected, running demo program...");
            println!("[STUFFIE] Launching Calendar...");

           
            run_one("open", &["-a", "Calendar"]).await;
            
        }
    }
}

// A function to run the stages defined in the config file, demonstrating how we might summon subprocesses
// Async because we'll want to await the completion of each stage before moving on to the next one, and also to handle any potential errors that might arise from running the commands.
async fn run_stages(stages: &Vec<config::Stage>, output_dir: &Path) {
    
    let log_dir = output_dir.join("logs");

    // Create the log directory if it doesn't exist, using tokio's async fs functions. If it fails, print an error and exit.
    let create_dirs = tokio::fs::create_dir_all(&log_dir).await;

    if let Err(e) = create_dirs {
        error!("[STUFFIE] Failed to create log directory: {}", e);
        std::process::exit(1);
    }

    for stage in stages {
        info!("[STUFFIE] Running stage: {}", stage.name);
        
        let mut cmd = Command::new(&stage.cmd);

        let log_file = log_dir.join(format!("{}.log", stage.name.replace(" ", "_")));
        
        // If the stage has args, add them to the command
        if let Some(args) = &stage.args {
            cmd.args(args);
        }
        
        // Spawn the command and wait for it to finish, checking the exit status
        // await the status of the command, and if it fails, print an error and exit the whole program
        let output = cmd.output().await
            .expect(&format!("Failed to launch stage: {}", stage.name));
        
        // Write the combined stdout and stderr to a log file for this stage
        let mut log_contents = output.stdout.clone();
            log_contents.extend_from_slice(&output.stderr); 
        
        // Using tokio's async fs functions to write the log file, and if it fails, print an error and exit
        tokio::fs::write(&log_file, log_contents).await
            .expect(&format!("Failed to write log file: {}", log_file.display()));
        
            // If the command failed (non-zero exit code), print an error and exit the whole program
        if !output.status.success() {
            error!("[STUFFIE] Stage '{}' failed with exit code {:?}", stage.name, output.status.code());
            std::process::exit(1);
        }
        
        // Or else, print that the stage completed successfully
        info!("[STUFFIE] Stage '{}' completed ", stage.name);
    }
}

// -----------------------------------------------------------------------------
// Entry Point
// -----------------------------------------------------------------------------

// Using tokio main macro to allow for async main function, which is necessary for awaiting the completion of subprocesses in run_stages
#[tokio::main]
async fn main() {
    // Initialize the tracing subscriber for logging. This sets up the global default subscriber that will handle all log events and write everything to the console.
    // Changed it to remove the need to run RUST_LOG=debug when running the program, and to include timestamps in the logs for better visibility.

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
            .init();

    let cli = Cli::parse();
    
    // Call the from_cli function that belongs to ExecutionMode, passing in a reference to cli, 
    // and store the result in a variable named mode
    let mode = ExecutionMode::from_cli(&cli);

    // Validate that the config file actually exists before doing anything else
    if !cli.config.exists() {
        error!("[STUFFIE] Error: config file not found: {}", cli.config.display());
        std::process::exit(1);
    }

    info!("[STUFFIE] Config:  {}", cli.config.display());
    info!("[STUFFIE] Mode:    {:?}", mode);

    // Load and deserialize the config file
    let pipeline_config = match config::load_config(&cli.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("[STUFFIE] Error loading config: {}", e);
            std::process::exit(1);
        }
    };

    info!("[STUFFIE] Pipeline: {}", pipeline_config.pipeline.name);
    info!("[STUFFIE] Stages:   {}", pipeline_config.stages.len());

    // Running stages is the core of the program execution module, 
    // So we'll call a function named run_stages and pass in a reference to the stages vector from the pipeline config.
    // run_stages will be an async function that iterates through the stages, runs each command, and awaits its completion before moving on to the next one.
    run_stages(&pipeline_config.stages, &pipeline_config.pipeline.output_dir).await;
    run_demo_program(&mode).await;
}

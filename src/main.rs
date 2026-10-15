/* 10 - 15 - 2026
STUFFIE: Sequential Task Utility For Flow Integration and Execution
main.rs, the main entry point for the STUFFIE program, responsible for parsing command line arguments, loading the pipeline configuration, and executing the stages of the pipeline.

-----------------------------------------------
Crates used
    clap
        Used for parsing command line arguments and options
    std
        path: Used for handling file paths and directories
        io: Used for reading input from stdin and handling I/O operations
        process: Used for spawning and managing child processes
    tokio
        Used for asynchronous programming, allowing for concurrent execution of tasks
    time
        Used for measuring the duration of each stage's execution
    serde
        Used for serialization and deserialization of configuration files and checkpoint data
    tracing
        Used for logging and tracing the execution of the pipeline and its stages
-----------------------------------------------
*/

use clap::{ArgGroup, Parser};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use tokio::process::Command;
use tracing::{error, info};
use tracing_subscriber::fmt::writer::MakeWriterExt;

mod config;

// clap command line argument parsing
#[derive(Parser, Debug)]
#[command(name = "stuffie")]
#[command(version)]
#[command(about = "Sequential Task Utility For Flow Integration and Execution")]
#[command(long_about = "
STUFFIE is a minimal pipeline orchestrator for sequential multi-stage workflows.
Each stage invokes a standalone program. Pipelines are defined in TOML config files.
")]
/*
Command Groups
    mode: Execution modes are mutually exclusive — exactly one must be provided.
        full_auto: Executes all stages without pausing for user input.
        semi_auto: Pauses for user input only on stages marked with qa = true in the TOML file.
        single_break: Pauses for user input after every stage, regardless of the qa setting.

    config_source: Configuration source is mutually exclusive — exactly one must be provided.
        config: Specifies the path to the TOML configuration file.
        resolved_config: Specifies the path to a resolved configuration file.
*/
#[command(group(
    ArgGroup::new("mode")
        .required(false)
        .args(["full_auto", "semi_auto", "single_break"])
))]
#[command(group(
    ArgGroup::new("config_source")
        .required(true)
        .args(["config", "resolved_config"])
))]

/*
Cli struct
    Represents the command line arguments and options for the STUFFIE program.
    Uses clap for parsing command line arguments and options.

    Fields:

    config: Optional path to the TOML configuration file.
    resolved_config: Optional path to the resolved configuration file.

    full_auto: Flag indicating whether to execute all stages without pausing for user input.
    semi_auto: Flag indicating whether to pause for user input only on stages marked with qa = true in the TOML file.
    single_break: Flag indicating whether to pause for user input after every stage, regardless of the qa setting.
*/

struct Cli {
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    config: Option<PathBuf>,

    //Bring in ResolvedPipelineConfig from config.rs to main.rs
    #[arg(long = "resolved-config")]
    resolved_config: Option<PathBuf>,

    #[arg(short = 'a', conflicts_with_all = ["semi_auto", "single_break"])]
    full_auto: bool,

    #[arg(short = 's', conflicts_with_all = ["full_auto", "single_break"])]
    semi_auto: bool,

    #[arg(short = 'b', conflicts_with_all = ["full_auto", "semi_auto"])]
    single_break: bool,

    /// Flag indicating whether to resume execution from the last completed stage, if a checkpoint exists.
    #[arg(long = "resume")]
    resume: bool,
}

enum ConfigSource {
    File(PathBuf),
    Resolved(PathBuf),
}

// Implementation of ExecutionMode
impl config::ExecutionMode {
    // Returns the mode requested on the command line, or None if no mode flag was given.
    fn from_cli(cli: &Cli) -> Option<Self> {
        if cli.full_auto {
            Some(config::ExecutionMode::FullAuto)
        } else if cli.semi_auto {
            Some(config::ExecutionMode::SemiAuto)
        } else if cli.single_break {
            Some(config::ExecutionMode::SingleBreak)
        } else {
            None
        }
    }

    // The CLI flag that selects this mode, used to forward it to child pipelines.
    fn flag(self) -> &'static str {
        match self {
            config::ExecutionMode::FullAuto => "-a",
            config::ExecutionMode::SemiAuto => "-s",
            config::ExecutionMode::SingleBreak => "-b",
        }
    }
}

// Prompts the user with a yes/no question and returns true for yes and false for no.
// Used in wait_for_confirmation and run_stages to handle user input for pausing and resuming execution.
fn prompt_yes_no(msg: &str) -> bool {
    loop {
        println!("{}", msg);

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                error!("[STUFFIE] stdin closed; cannot prompt. Aborting.");
                std::process::exit(2);
            }
            Ok(_) => {}
            Err(e) => {
                error!("[STUFFIE] Failed to read input: {}", e);
                std::process::exit(1);
            }
        }

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => println!("[STUFFIE] Please enter 'y' or 'n'."),
        }
    }
}

// Waits for the user to press Enter before continuing execution.
fn wait_for_confirmation(stage_name: &str, output_dir: &Path) -> bool {
    println!("[STUFFIE] Stage '{}' complete.", stage_name);
    println!("[STUFFIE] Review outputs at {}.", output_dir.display());
    prompt_yes_no("[STUFFIE] Continue? (y)es / (n)o abort:")
}

fn should_pause(stage: &config::ResolvedStage, mode: &config::ExecutionMode) -> bool {
    match mode {
        config::ExecutionMode::FullAuto => false,
        config::ExecutionMode::SemiAuto => stage.qa.unwrap_or(false),
        config::ExecutionMode::SingleBreak => true,
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Checkpoint {
    last_completed_stage: usize,
    last_completed_name: String,
}

async fn write_checkpoint(path: &Path, checkpoint: &Checkpoint) {
    let json = serde_json::to_string_pretty(checkpoint).unwrap_or_else(|e| {
        error!("Failed to serialize checkpoint: {}", e);
        std::process::exit(1);
    });
    tokio::fs::write(path, json).await.unwrap_or_else(|e| {
        error!("Failed to write checkpoint: {}", e);
        std::process::exit(1);
    });
}
/*
Runs the stages of the pipeline sequentially, handling both command and pipeline stages.

Creates the loggging directory and log files for each stage, and writes the output of each stage to its corresponding log file.
Checks if the configuration is being run in a child process (stdin mode) or as the main process (file mode), and handles the execution accordingly.
*/
async fn run_stages(
    stages: &[config::ResolvedStage],
    output_dir: &Path,
    mode: &config::ExecutionMode,
    cli_mode: Option<config::ExecutionMode>,
    resume: bool,
) {
    let internal_dir = output_dir.join(".stuffie");
    let checkpoint_path = internal_dir.join("checkpoint.json");

    let mut resume_children = resume;

    let mut checkpoint: Checkpoint = if resume
        && tokio::fs::try_exists(&checkpoint_path)
            .await
            .unwrap_or(false)
    {
        let checkpoint_contents = tokio::fs::read_to_string(&checkpoint_path)
            .await
            .unwrap_or_else(|e| {
                error!("Failed to read checkpoint: {}", e);
                std::process::exit(1);
            });

        let checkpoint: Checkpoint =
            serde_json::from_str(&checkpoint_contents).unwrap_or_else(|e| {
                error!("Failed to parse checkpoint: {}", e);
                std::process::exit(1);
            });

        let done = checkpoint.last_completed_stage;
        let stale = done > stages.len()
            || (done > 0 && stages[done - 1].name != checkpoint.last_completed_name);

        let restart = if stale {
            println!(
                "[STUFFIE] Checkpoint doesn't match this pipeline (last completed: '{}').",
                checkpoint.last_completed_name
            );
            if !prompt_yes_no("[STUFFIE] Restart from the beginning? (y)es / (n)o abort:") {
                std::process::exit(2);
            }
            true
        } else {
            println!(
                "[STUFFIE] Checkpoint found. Last completed stage: '{}'",
                checkpoint.last_completed_name
            );
            let next = stages
                .get(done)
                .map(|s| s.name.as_str())
                .unwrap_or("(end of pipeline)");
            if matches!(mode, config::ExecutionMode::FullAuto) {
                println!("[STUFFIE] Full-auto: resuming from '{}'.", next);
                false
            } else {
                !prompt_yes_no(&format!(
                    "[STUFFIE] Resume from '{}'? (y)es / (n)o restart:",
                    next
                ))
            }
        };

        if restart {
            resume_children = false;
            let _ = tokio::fs::remove_file(&checkpoint_path).await;
            println!("[STUFFIE] Restarting from the beginning.");
            Checkpoint::default()
        } else {
            checkpoint
        }
    } else {
        Checkpoint::default()
    };

    tokio::fs::create_dir_all(output_dir)
        .await
        .unwrap_or_else(|e| panic!("[STUFFIE] Failed to create output directory: {}", e));

    let log_dir = output_dir.join("logs");

    tokio::fs::create_dir_all(&log_dir)
        .await
        .unwrap_or_else(|e| panic!("[STUFFIE] Failed to create log directory: {}", e));

    tokio::fs::create_dir_all(&internal_dir)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to create .stuffie directory: {}", e);
            std::process::exit(1);
        });

    for (index, stage) in stages.iter().enumerate() {
        let stage_number = index + 1;

        if stage_number <= checkpoint.last_completed_stage {
            continue;
        }

        info!(
            "[STUFFIE] Running stage {}/{}: {}",
            stage_number,
            stages.len(),
            stage.name
        );

        let log_file = log_dir.join(format!("stage_{:03}.log", stage_number));

        let start_time = Instant::now();

        match &stage.kind {
            config::ResolvedStageKind::Command { cmd, args } => {
                let mut command = Command::new(cmd);

                if let Some(args) = args {
                    command.args(args);
                }

                let output = command
                    .output()
                    .await
                    //.expect(&format!("Failed to launch stage: {}", stage.name));
                    .unwrap_or_else(|_| panic!("Failed to launch stage: {}", stage.name));

                let mut log_contents = output.stdout.clone();
                log_contents.extend_from_slice(&output.stderr);

                tokio::fs::write(&log_file, log_contents)
                    .await
                    .unwrap_or_else(|_| panic!("Failed to write log file: {}", log_file.display()));

                if !output.status.success() {
                    error!(
                        "[STUFFIE] Stage '{}' failed with exit code {:?}",
                        stage.name,
                        output.status.code()
                    );
                    std::process::exit(1);
                }
            }

            config::ResolvedStageKind::Pipeline(boxed_config) => {
                let serialized_config = serde_json::to_string(&boxed_config).unwrap();

                let internal_output_dir = boxed_config.output_dir.join(".stuffie");

                tokio::fs::create_dir_all(&internal_output_dir)
                    .await
                    .unwrap_or_else(|_| {
                        panic!(
                            "Failed to create internal output directory: {}",
                            internal_output_dir.display()
                        )
                    });

                tokio::fs::write(
                    internal_output_dir.join("resolved_config.json"),
                    serialized_config.as_bytes(),
                )
                .await
                .unwrap_or_else(|_| {
                    panic!(
                        "Failed to write resolved config file: {}",
                        internal_output_dir.join("resolved_config.json").display()
                    )
                });

                let mut command = Command::new(std::env::current_exe().unwrap());

                command
                    .arg("--resolved-config")
                    .arg(internal_output_dir.join("resolved_config.json"));

                if let Some(m) = cli_mode {
                    command.arg(m.flag());
                }

                if resume_children {
                    command.arg("--resume");
                }

                let mut child = command
                    .spawn()
                    .unwrap_or_else(|_| panic!("Failed to spawn child pipeline: {}", stage.name));

                let status: std::process::ExitStatus = child.wait().await.unwrap_or_else(|_| {
                    panic!("Failed to wait for child pipeline: {}", stage.name)
                });

                if status.code() == Some(2) {
                    info!("[STUFFIE] Child pipeline '{}' aborted by user.", stage.name);
                    std::process::exit(2);
                }

                if !status.success() {
                    error!(
                        "[STUFFIE] Child pipeline '{}' failed with exit code {:?}",
                        stage.name,
                        status.code()
                    );
                    std::process::exit(1);
                }
            }
        }

        info!(
            "[STUFFIE] Stage '{}' completed in {:.2} seconds",
            stage.name,
            start_time.elapsed().as_secs_f64()
        );

        checkpoint.last_completed_stage += 1;
        checkpoint.last_completed_name = stage.name.clone();
        write_checkpoint(&checkpoint_path, &checkpoint).await;

        if should_pause(stage, mode) {
            if !wait_for_confirmation(&stage.name, output_dir) {
                info!("[STUFFIE] Pipeline stopped by user.");
                std::process::exit(2);
            }
        }
    }
    if tokio::fs::try_exists(&checkpoint_path)
        .await
        .unwrap_or(false)
    {
        tokio::fs::remove_file(&checkpoint_path)
            .await
            .unwrap_or_else(|e| panic!("[STUFFIE] Failed to remove checkpoint: {}", e));
    }
}
/*
main function
    Initializes logging and tracing, parses command line arguments, loads the pipeline configuration, and runs the stages of the pipeline.
    Handles errors in loading the configuration and running the stages, and exits with an appropriate error code if any stage fails.
*/

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config_source = if let Some(path) = &cli.config {
        ConfigSource::File(path.clone())
    } else if let Some(path) = &cli.resolved_config {
        ConfigSource::Resolved(path.clone())
    } else {
        unreachable!("[STUFFIE] A configuration source is required by clap")
    };

    let pipeline_config = match &config_source {
        ConfigSource::File(path) => match config::resolve_config(path, vec![]) {
            Ok(cfg) => cfg,

            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        },
        ConfigSource::Resolved(path) => {
            match std::fs::read_to_string(path)
                .map_err(|e| format!("[STUFFIE] Failed to read resolved config: {}", e))
                .and_then(|input| {
                    serde_json::from_str::<config::ResolvedPipelineConfig>(&input)
                        .map_err(|e| format!("[STUFFIE] Failed to parse resolved config: {}", e))
                }) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    let cli_mode = config::ExecutionMode::from_cli(&cli);
    let mode = cli_mode.unwrap_or(pipeline_config.mode);

    tokio::fs::create_dir_all(&pipeline_config.output_dir)
        .await
        .unwrap_or_else(|e| {
            eprintln!("[STUFFIE] Failed to create output directory: {}", e);
            std::process::exit(1);
        });

    let log_dir = pipeline_config.output_dir.join("logs");

    tokio::fs::create_dir_all(&log_dir)
        .await
        .unwrap_or_else(|e| {
            eprintln!("[STUFFIE] Failed to create log directory: {}", e);
            std::process::exit(1);
        });

    let log_file = log_dir.join("stuffie.log");

    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .expect("[STUFFIE] Failed to open log file");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stdout.and(file))
        .init();

    info!("[STUFFIE] Starting pipeline: {}", &pipeline_config.name);

    run_stages(
        &pipeline_config.stages,
        &pipeline_config.output_dir,
        &mode,
        cli_mode,
        cli.resume,
    )
    .await;
}

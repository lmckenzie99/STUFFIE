/* 9 - 30 - 2026
    STUFFIE: Sequential Task Utility For Flow Integration and Execution
    Configuration File, for loading and resolving the pipeline configuration from a TOML file.

--------------------------------------------------------------------------------
Crates used:
    serde
        Used for Deserialization and Serialization of TOML files inputted by users
        Moving from TOML to Rust structs for easier manipulation and access
        Moving from Rust structs to TOML for saving resolved configurations
    std::fs
        Used for reading in the configuration file from the filesystem (fs)
    std::io
        Used for handling input/output operations, such as reading from and writing to files
    std::path::PathBuf
        Used for representing file paths in a platform-independent way
--------------------------------------------------------------------------------
*/

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/*
------------------------------------------------------------------------------
Configuring the serialization and deserialization of the configuration file
(Going from the TOML file to Rust structs and back)

ExecutionMode: An enum representing the execution mode of the pipeline, either FullAuto, SemiAuto, or SingleBreak.
PipelineConfig: The top-level configuration struct, containing the pipeline and its stages.
Pipeline: Represents the pipeline itself, including its name, output directory, and source directory.
Stage: Represents a single stage in the pipeline, which can be either a command or a child pipeline.
StageKind: An enum representing the kind of stage, either a command or a child pipeline.

------------------------------------------------------------------------------
*/

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    FullAuto,
    SemiAuto,
    SingleBreak,
}

#[derive(Deserialize, Debug)]
pub struct PipelineConfig {
    pub pipeline: Pipeline,
    pub stages: Vec<Stage>,
}

#[derive(Deserialize, Debug)]
pub struct Pipeline {
    pub name: String,
    pub output_dir: PathBuf,
    pub source_dir: PathBuf,
    pub mode: ExecutionMode,
}

#[derive(Deserialize, Debug)]
pub struct Stage {
    pub name: String,
    pub qa: Option<bool>,
    #[serde(flatten)]
    pub kind: StageKind,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StageKind {
    Command {
        cmd: String,
        args: Option<Vec<String>>,
    },
    Pipeline {
        path: PathBuf,
    },
}

/*
------------------------------------------------------------------------------
Configuring child pipelines and resolving the configuration into a fully resolved structure
New feature to handle child pipelines, allowing for nested pipeline execution and configuration resolution.

The main difference between the original configuration and the resolved configuration is that the resolved configuration includes the fully resolved stages,
including any child pipelines that have been resolved into their own ResolvedPipelineConfig structs.

If a single STUFFIE instance is running a pipeline without child pipelines, the resolved configuration will be the same as the original configuration.

ResolvedStageKind: An enum representing the resolved kind of stage, either a command or a resolved child pipeline.
ResolvedStage: Represents a single resolved stage in the pipeline, including its name, QA flag, and resolved kind.
ResolvedPipelineConfig: Represents the fully resolved pipeline configuration, including its name, output directory, source directory, and resolved stages.
------------------------------------------------------------------------------
*/

#[derive(Serialize, Deserialize)]
pub enum ResolvedStageKind {
    Command {
        cmd: String,
        args: Option<Vec<String>>,
    },
    Pipeline(Box<ResolvedPipelineConfig>),
}

#[derive(Serialize, Deserialize)]
pub struct ResolvedStage {
    pub name: String,
    pub qa: Option<bool>,
    pub kind: ResolvedStageKind,
}

#[derive(Serialize, Deserialize)]
pub struct ResolvedPipelineConfig {
    pub name: String,
    pub output_dir: PathBuf,
    pub source_dir: PathBuf,
    pub stages: Vec<ResolvedStage>,
    pub mode: ExecutionMode,
}

/*
------------------------------------------------------------------------------
New features for loading and resolving the configuration, including child pipelines.

load_config:
    Loads the configuration from a TOML file and parses it into a PipelineConfig struct,
    utilizing the serde crate for deserialization.
    Returns a Result containing the PipelineConfig or an error message.
resolve_config:
    Resolves the configuration, including any child pipelines, into a fully resolved structure.
    Returns a Result containing the ResolvedPipelineConfig or an error message.
------------------------------------------------------------------------------
*/

pub fn load_config(path: &PathBuf) -> Result<PipelineConfig, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("[STUFFIE] Failed to read config file: {}", e))?;

    // Parse the string into PipelineConfig struct
    let config: PipelineConfig = toml::from_str(&contents)
        .map_err(|e| format!("[STUFFIE] Failed to parse config file: {}", e))?;

    Ok(config)
}

pub fn resolve_config(
    path: &PathBuf,
    ancestors: Vec<PathBuf>,
) -> Result<ResolvedPipelineConfig, String> {
    if ancestors.contains(path) {
        return Err(format!(
            "[STUFFIE] Circular dependency detected in pipeline configuration: {:?}",
            path
        ));
    }
    let config = load_config(path)?;

    let mut child_ancestors = ancestors.clone();
    child_ancestors.push(path.clone());

    let parent_dir = path.parent().unwrap_or(std::path::Path::new(""));

    let mut resolved_stages = Vec::new();

    for stage in &config.stages {
        let resolved_kind = match &stage.kind {
            StageKind::Command { cmd, args } => ResolvedStageKind::Command {
                cmd: cmd.clone(),
                args: args.clone(),
            },
            StageKind::Pipeline { path } => {
                let child_path = parent_dir.join(path);

                ResolvedStageKind::Pipeline(Box::new(resolve_config(
                    &child_path,
                    child_ancestors.clone(),
                )?))
            }
        };
        resolved_stages.push(ResolvedStage {
            name: stage.name.clone(),
            qa: stage.qa,
            kind: resolved_kind,
        });
    }

    /*
    ------------------------------------------------------------------------------
    Resolved configuration is returned, containing the fully resolved stages,
    including any child pipelines that have been resolved into their own ResolvedPipelineConfig structs.
    ------------------------------------------------------------------------------
    */

    Ok(ResolvedPipelineConfig {
        name: config.pipeline.name,
        output_dir: config.pipeline.output_dir,
        source_dir: config.pipeline.source_dir,
        stages: resolved_stages,
        mode: config.pipeline.mode,
    })
}

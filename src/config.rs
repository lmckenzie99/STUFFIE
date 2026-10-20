//! STUFFIE: Sequential Task Utility For Flow Integration and Execution
//! Configuration File, for loading and resolving the pipeline configuration from a TOML file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// # Configuration Serialization and Deserialization
///
/// Defines the types used to serialize and deserialize the TOML configuration
/// file.
///
/// - [`ExecutionMode`] represents the pipeline's execution mode.
/// - [`PipelineConfig`] is the top-level configuration structure.
/// - [`Pipeline`] represents the pipeline metadata.
/// - [`Stage`] represents an individual pipeline stage.
/// - [`StageKind`] identifies a command or child-pipeline stage.

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

/// Configures child pipelines and resolves the configuration into a fully
/// resolved structure.
///
/// The resolved configuration includes all stages, including child pipelines
/// resolved into their own [`ResolvedPipelineConfig`] values. Without child
/// pipelines, it is equivalent to the original configuration.
///
/// [`ResolvedStageKind`] represents a command or resolved child pipeline.
/// [`ResolvedStage`] represents a resolved stage with its name, QA flag, and resolved kind.

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

/// Loads and resolves configuration, including child pipelines.
///
/// - [`load_config`] loads a TOML file into a [`PipelineConfig`].
/// - [`resolve_config`] resolves child pipelines into a [`ResolvedPipelineConfig`].

pub fn load_config(path: &PathBuf) -> Result<PipelineConfig, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("[STUFFIE] Failed to read config file: {}", e))?;

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

    Ok(ResolvedPipelineConfig {
        name: config.pipeline.name,
        output_dir: config.pipeline.output_dir,
        source_dir: config.pipeline.source_dir,
        stages: resolved_stages,
        mode: config.pipeline.mode,
    })
}

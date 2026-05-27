// 5 - 24 - 2026
// STUFFIE: Sequential Task Utility For Flow Integration and Execution
// Config Structs + Loader Function

// Using the serde crate to deserialize the config file, and std::fs and std::path for file handling
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

// -----------------------------------------------------------------------------
// Config Structs
// -----------------------------------------------------------------------------

// The top-level config struct that matches the structure of the TOML file
// It has two fields: pipeline (the [pipeline] section) and stages (the [[stages]] array)
#[derive(Deserialize, Debug)]
pub struct PipelineConfig {
    pub pipeline: Pipeline,
    pub stages: Vec<Stage>,
}

// The [pipeline] section
// Three fields: name (string), output_dir (path), and source_dir (path)
#[derive(Deserialize, Debug)]
pub struct Pipeline {
    pub name: String,
    pub output_dir: PathBuf,
    pub source_dir: PathBuf,
}

// Each [[stages]] entry
#[derive(Deserialize, Debug)]
pub struct Stage {
    pub name: String,
    pub cmd: String,
    pub args: Option<Vec<String>>,  // optional, not every stage needs args
    pub qa: Option<bool>,           // optional, defaults to false if missing
}

// -----------------------------------------------------------------------------
// Loader
// -----------------------------------------------------------------------------


// A function to load and parse the config file given its path
// It returns a Result with PipelineConfig on success, or a String error message on failure
pub fn load_config(path: &PathBuf) -> Result<PipelineConfig, String> {
    // Read the file contents into a string
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("[STUFFIE] Failed to read config file: {}", e))?;

    // Parse the string into PipelineConfig struct
    let config: PipelineConfig = toml::from_str(&contents)
        .map_err(|e| format!("[STUFFIE] Failed to parse config file: {}", e))?;

    Ok(config)
}
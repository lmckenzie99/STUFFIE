# Sequential Task Utility For Flow Integration and Execution (STUFFIE)
---

## Use Case

- Orchestrates sequential multi-stage pipelines where multiple programs combine to produce one final product
- Supports interactive pause points for manual QA review of intermediate outputs
- Designed for pipelines where stages are computationally expensive and must not be unnecessarily rerun
- Supports nested pipelines, allowing a stage to execute another pipeline as part of a larger workflow



## Key Features

- **Command line invocation** with explicit config path
- **Full-auto** (`-a`): no QA stages, pipeline runs start to finish unattended
- **Semi-auto** (`-s`): pause points read from config via `qa = true` stage flag
- **Single break** (`-b`): pauses after every stage, regardless of its QA setting
- **Checkpoint/resume**: last completed stage written to a checkpoint file; on restart, user is prompted to resume or start fresh
- **Structured logging**: per-stage stdout/stderr capture, timestamped log file written to output directory
- **Recursive pipelines**: resolves and executes child pipelines, including nested child pipelines
- **Child-specific configuration**: each child pipeline can define its own execution mode unless a command-line mode override is forwarded from the parent
- **Failure handling**: distinguishes execution failures from user-initiated aborts



## Tech Spec

- **Language**: Rust (stable, 1.78+)
- **Crates**:
  - `tokio` — async runtime for subprocess management (`tokio::process`)
  - `clap` — command-line argument parsing
  - `serde` + `toml` — configuration deserialization
  - `serde_json` — serialization of resolved configurations and checkpoint data
  - `tracing` + `tracing-subscriber` — logging and tracing
  - `std::time::Instant` — measuring stage execution duration

---

## CLI Usage

```
stuffie --config <path/to/config.toml> [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `-a` | Full-auto: run all stages without pause |
| `-s` | Semi-auto: pause at stages marked `qa = true` in config |
| `-b` | Single break: pause and prompt after every stage |
| `-c, --config` | Path to pipeline config file (required) |
| `--resume` | Resume from a saved checkpoint when one exists

The execution-mode flags are mutually exclusive. If no mode flag is supplied, STUFFIE uses the mode defined in the pipeline configuration.
The `--config` and `--resolved-config`  options are *mutually exclusive*. `--resolved-config` is used internally to launch child pipelines from their resolved JSON configurations.


## Config Schema

The pipeline is defined in a .toml file. Each pipeline specifies its name, source directory, output directory, execution mode, and stages.

Stages can be either commands or child pipelines. Command stages specify an executable and optional arguments. Pipeline stages reference another TOML configuration file.

```toml
[pipeline]
name = "nerfberg"
output_dir = "/data/runs/nerfberg_001"
source_dir = "/data/raw/footage"
mode = "semi_auto"

[[stages]]
name = "extract"
type = "command"
cmd = "./bin/extract"
args = ["--input", "/data/raw/footage", "--output", "/data/runs/nerfberg_001/frames"]
qa = false

[[stages]]
name = "mask"
type = "command"
cmd = "./bin/mask"
args = ["--input", "/data/runs/nerfberg_001/frames", "--output", "/data/runs/nerfberg_001/masks"]
qa = true

[[stages]]
name = "colmap"
type = "command"
cmd = "colmap"
args = ["automatic_reconstructor", "--image_path", "/data/runs/nerfberg_001/masks"]
qa = true

[[stages]]
name = "nerf"
type = "command"
cmd = "python"
args = ["train.py", "--data", "/data/runs/nerfberg_001/colmap"]
qa = false
```


### Config Fields

| Field | Type | Description |
|-------|------|-------------|
| `pipeline.name` | string | Human-readable pipeline identifier |
| `pipeline.output_dir` | path | Root output directory for all stage artifacts |
| `pipeline.source_dir` | path | Root source directory for pipeline inputs |
| `pipeline.mode` | string | Execution mode: `"full_auto"`, `"semi_auto"`, or `"single_break"` |
| `stages[].name` | string | Stage identifier, used in logging and checkpointing |
| `stages[].type` | string | Type of stage: `"command"` or `"pipeline"` |
| `stages[].cmd` | string | Executable or command to invoke |
| `stages[].args` | string[] | Arguments passed to the command |
| `stages[].path` | path | Path to child pipeline config (for nested pipelines) |
| `stages[].qa` | bool | Whether to pause for QA review after this stage (default: false) |

The type field determines which additional fields a stage requires. Command stages use cmd and optionally args; pipeline stages use path.

Child configuration paths are resolved relative to the directory containing the configuration file that references them. Child pipelines may themselves contain additional child pipelines.

## Recursive Pipelines

STUFFIE supports nested pipeline execution by resolving child configurations before execution.

- **Configuration resolution**: STUFFIE loads the parent TOML configuration and recursively resolves child pipeline configurations into a fully resolved structure.
- **Resolved configurations**: child pipeline configurations are serialized to JSON at '''<child_output_dir>/.stuffie/resolved_config.json.'''
- **Child execution**: STUFFIE launches a child process using the current executable and passes the resolved configuration through --resolved-config.
- **Execution modes**: a child uses its configured mode unless a command-line execution-mode override is forwarded from the parent.
- **Checkpoint/resume**: when resuming a parent pipeline, STUFFIE forwards --resume to child pipelines. If the parent is restarted from the beginning, child resume is disabled for that run.
- **Error propagation**: a child pipeline failure causes the parent to fail. A child aborted by the user propagates exit code 2 to the parent.
- **Circular dependencies**: configuration resolution detects circular pipeline references and returns an error.

 Each child pipeline maintains its own output directory, logs, and checkpoint. This allows individual pipelines to track their execution independently.


## Checkpoint / Resume

STUFFIE writes a checkpoint file to the pipeline output directory after each stage completes successfully.

**Checkpoint file location**: `<output_dir>/.stuffie/checkpoint.json`

**Checkpoint file format**:

```toml
  "last_completed_stage": 2,
  "last_completed_name": "mask"
```
- last_completed_stage is the one-based index of the last completed stage. A value of 0 indicates that no stages have completed.
- last_completed_name is the name of the last completed stage.


**Behavior on startup**:
- If --resume is supplied and a checkpoint exists, STUFFIE validates the checkpoint against the current pipeline.
- In full-auto mode, STUFFIE resumes automatically from the next stage.
- In semi-auto and single-break modes, STUFFIE prompts the user to confirm whether to resume or restart.
- If the checkpoint does not match the current pipeline, STUFFIE warns the user and prompts for a restart or abort.
- If --resume is not supplied, execution starts from the beginning.
- On successful pipeline completion, the checkpoint file is deleted.

**Edge cases**:
- A checkpoint referencing a stage index or stage name that does not match the current pipeline is treated as stale.
- A checkpoint is written only after a stage completes successfully.
- If the user aborts at a QA pause, the checkpoint for the completed stage is preserved.
- Checkpoints are maintained independently for parent and child pipelines.


## QA Pause Behavior

When a stage completes in semi-auto mode with qa = true, or any stage completes in single-break mode, STUFFIE pauses and prompts the user.

```
[STUFFIE] Stage 'mask' complete.
[STUFFIE] Review outputs at /data/runs/nerfberg_001.
[STUFFIE] Continue? (y)es / (n)o abort:
```

y — continues to the next stage
n — aborts execution with exit code 2; the completed stage remains checkpointed

Full-auto mode does not pause for QA, regardless of the stage's qa setting.


## Stage Lifecycle

For each stage, STUFFIE:

1. Logs the stage start.
2. Starts the command or child pipeline.
3. Captures command stdout/stderr and writes the output to a numbered stage log.
4. Measures and logs the stage execution duration.
5. On successful completion, updates the checkpoint and evaluates whether a QA pause is required.
6. On failure, logs the error and exits with a nonzero error code.

```
[STUFFIE] Starting stage: mask
[STUFFIE] Running: ./bin/mask --input /data/frames --output /data/masks
[STUFFIE] Stage 'mask' complete.
[STUFFIE] Review outputs at /data/runs/nerfberg_001.
[STUFFIE] Continue? (y)es / (n)o abort:
[STUFFIE] Stage 'mask' failed with exit code 1. See /data/logs/mask.log
```
**Per-stage log naming:**
'''
<output_dir>/logs/stage_001.log
<output_dir>/logs/stage_002.log
<output_dir>/logs/stage_003.log
'''

Stage log files are numbered according to their position in the pipeline.

## Logging

**Log locations:**
- Per-stage logs: written to <output_dir>/logs/stage_<number>.log
- Pipeline-level log: written to <output_dir>/logs/stuffie.log
- Pipeline-level logging: uses tracing and tracing-subscriber to record execution events, errors, and stage durations
- Log persistence: the pipeline-level log is opened in append mode, preserving existing log entries across runs

**Exit Codes and Error Handling**

- 0 — successful completion
- 1 — execution or configuration error
- 2 — user-initiated abort, including an abort propagated from a child pipeline

A failed command or child pipeline prevents subsequent stages from running.


## Future Work (v2.0+)

- SLURM/HPC job submission per stage
- Parallel stage execution for independent branches
- Web UI for pipeline monitoring (Harmony use case)

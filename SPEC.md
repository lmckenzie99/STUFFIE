# Sequential Task Utility For Flow Integration and Execution (STUFFIE)
---

## Use Case

- Orchestrates sequential multi-stage pipelines where many programs combine to produce one final product
- Supports interactive pause points for manual QA review of intermediate outputs
- Designed for pipelines where stages are computationally expensive and must not be unnecessarily rerun

---

## Key Features

- **Command line invocation** with explicit config path
- **Full-auto** (`-a`): no QA stages, pipeline runs start to finish unattended
- **Semi-auto** (`-s`): pause points read from config via `qa = true` stage flag
- **Single break** (`-b`): pauses and prompts user after every stage output
- **Checkpoint/resume**: last completed stage written to a checkpoint file; on restart, user is prompted to resume or start fresh
- **Structured logging**: per-stage stdout/stderr capture, timestamped log file written to output directory

---

## Tech Spec

- **Language**: Rust (stable, 1.78+)
- **Crates**:
  - `tokio` — async runtime for subprocess management (`tokio::process`)
  - `clap` — CLI argument parsing
  - `serde` + `toml` — config file deserialization
  - `tracing` + `tracing-subscriber` — structured logging

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

---

## Config Schema

The pipeline is defined in a `.toml` file. Each stage defines a program to run, its arguments, and optional QA behavior.

```toml
[pipeline]
name = "nerfberg"
output_dir = "/data/runs/nerfberg_001"
source_dir = "/data/raw/footage"

[[stages]]
name = "extract"
cmd = "./bin/extract"
args = ["--input", "${source_dir}", "--output", "${output_dir}/frames"]
qa = false

[[stages]]
name = "mask"
cmd = "./bin/mask"
args = ["--input", "${output_dir}/frames", "--output", "${output_dir}/masks"]
qa = true

[[stages]]
name = "colmap"
cmd = "colmap"
args = ["automatic_reconstructor", "--image_path", "${output_dir}/masks"]
qa = true

[[stages]]
name = "nerf"
cmd = "python"
args = ["train.py", "--data", "${output_dir}/colmap"]
qa = false
```

### Config Fields

| Field | Type | Description |
|-------|------|-------------|
| `pipeline.name` | string | Human-readable pipeline identifier |
| `pipeline.output_dir` | path | Root output directory for all stage artifacts |
| `pipeline.source_dir` | path | Root source directory for pipeline inputs |
| `stages[].name` | string | Stage identifier, used in logging and checkpointing |
| `stages[].cmd` | string | Executable or command to invoke |
| `stages[].args` | string[] | Arguments passed to the command |
| `stages[].qa` | bool | Whether to pause for QA review after this stage (default: false) |

---

## Checkpoint / Resume

STUFFIE writes a checkpoint file to the pipeline output directory after each stage completes successfully.

**Checkpoint file location**: `${output_dir}/.stuffie_checkpoint`

**Checkpoint file format**:

```toml
run_id = "nerfberg_20260501_143022"
last_completed = "mask"
config = "/home/liam/pipelines/nerfberg.toml"
```

**Behavior on startup**:
- If a checkpoint file exists in the output directory, STUFFIE prompts:
  ```
  [STUFFIE] Checkpoint found. Last completed stage: 'mask'
  Resume from 'colmap'? (y/n):
  ```
- If user selects `y`: pipeline resumes from the next stage
- If user selects `n`: pipeline restarts from the beginning, checkpoint is overwritten
- On clean pipeline completion: checkpoint file is deleted

**Edge cases**:
- If checkpoint references a stage name not found in the current config, STUFFIE warns and prompts for full restart
- Checkpoint is only written after a stage exits with code 0

---

## QA Pause Behavior

When a stage with `qa = true` completes (in `-s` mode) or any stage completes (in `-b` mode), STUFFIE blocks and prints:

```
[STUFFIE] Stage 'mask' complete.
[STUFFIE] Review outputs at: /data/runs/nerfberg_001/masks/
[STUFFIE] Continue? (y)es / (n)o - abort:
```

- `y` — writes checkpoint, advances to next stage
- `n` — writes checkpoint of last completed stage, exits cleanly

---

## Stage Lifecycle

For each stage, STUFFIE:

1. Logs stage start with timestamp
2. Spawns subprocess via `tokio::process::Command`
3. Captures stdout/stderr, writes to `${output_dir}/logs/${stage_name}.log`
4. On exit code 0: writes checkpoint, evaluates QA flag
5. On non-zero exit: logs error, prints failure message, exits with error code

```
[STUFFIE] Starting stage: mask
[STUFFIE] Running: ./bin/mask --input /data/frames --output /data/masks
[STUFFIE] Stage 'mask' failed with exit code 1. See /data/logs/mask.log
```

---

## Logging

- Per-stage logs written to `${output_dir}/logs/<stage_name>.log`
- Pipeline-level log written to `${output_dir}/logs/stuffie.log`
- Log format: `[TIMESTAMP] [LEVEL] [STAGE] message`
- Uses `tracing` crate for structured output compatible with HPC environments

---

## Future Work (v2.0+)

- SLURM/HPC job submission per stage
- Parallel stage execution for independent branches
- Web UI for pipeline monitoring (Harmony use case)

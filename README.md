# 🦀 STUFFIE

**Sequential Task Utility For Flow Integration and Execution**

> A minimal, opinionated pipeline orchestrator for multi-stage automation workflows.

```
stuffie --config pipelines/my_pipeline.toml -s
```

```
[STUFFIE] Starting pipeline: my_pipeline
[STUFFIE] Running stage 1/3: preprocess
[STUFFIE] Stage 'preprocess' completed in 12.40 seconds
[STUFFIE] Running stage 2/3: transform
[STUFFIE] Stage 'transform' completed in 47.10 seconds
[STUFFIE] Stage 'transform' complete.
[STUFFIE] Review outputs at /data/runs/my_pipeline_001.
[STUFFIE] Continue? (y)es / (n)o abort:
```

---

## Why STUFFIE?

Most pipeline tools are built for services, DAGs, or distributed systems. STUFFIE is built for workflows where:

- Each stage calls a standalone program
- Stages are sequential and order-dependent
- Intermediate outputs need human review before continuing
- Restarting from scratch after a failure is expensive

STUFFIE stays out of your way. It runs your programs, pauses when you need it to, and picks up where it left off.

---

## Features

- **Three execution modes** — full-auto, semi-auto, or single-break
- **QA pause points** — block pipeline execution for manual review of intermediate outputs
- **Checkpoint/resume** — resume from the last successful stage after failure or abort
- **Structured logging** — per-stage logs and a pipeline-level log, HPC-friendly via `tracing`
- **Single static binary** — no runtime, no daemon, no dependencies
- **Configurable via TOML** — define your pipeline in a simple, human-readable format
- **Recursive pipeline execution** — run child pipelines from within parent pipelines for modularity and reusability.

---

## Installation

### From source

```bash
git clone https://github.com/lmckenzie99/stuffie
cd stuffie
cargo build --release
```

Binary will be at `./target/release/stuffie`. Copy it wherever you need it.

```bash
cp target/release/stuffie ~/.local/bin/
```

### Requirements

- Rust (stable)

---

## Usage

```
stuffie --config <path/to/config.toml> [OPTIONS]
```

| Flag | Mode | Description |
|------|------|-------------|
| `-a` | Full-auto | Run all stages start to finish, no pauses |
| `-s` | Semi-auto | Pause at stages marked `qa = true` in config |
| `-b` | Single break | Pause and prompt after every stage |
| `-c, --config` | — | Path to pipeline config file (required) |
| `--resume` | — | Resume from the last checkpoint if one exists. Passed on to child pipelines. In full-auto (`-a`), resumes without prompting. |

If no mode flag is given, each pipeline uses the `mode` set in its own TOML. If a mode flag is given, it overrides the `mode` of every pipeline in the run, including nested ones.

### Internal Option

| Flag | Description |
|---|---|
| `--resolved-config <FILE>` | Loads a resolved JSON pipeline configuration. Primarily used internally to execute nested pipelines. |

This option is intended for internal use. Most users should start pipelines with `--config <FILE>`.

### Examples

```bash
# Run fully automated, no interaction
stuffie --config pipelines/my_pipeline.toml -a

# Pause at designated QA stages
stuffie --config pipelines/my_pipeline.toml -s

# Pause after every stage
stuffie --config pipelines/my_pipeline.toml -b

# Resume from the last completed stage
stuffie --config pipelines/my_pipeline.toml --resume

```

---

# Configuration
Pipelines are defined in TOML. Each stage is either a command to execute or a nested pipeline to invoke.

## Example: Command Pipeline
```toml
[pipeline]
name = "my_pipeline"
output_dir = "/data/runs/my_pipeline_001"
source_dir = "/data/raw/input"
mode = "semi_auto"

[[stages]]
name = "preprocess"
type = "command"
cmd = "./bin/preprocess"
args = ["--input", "/data/raw/input", "--output", "/data/runs/my_pipeline_001/preprocessed"]
qa = false

[[stages]]
name = "transform"
type = "command"
cmd = "./bin/transform"
args = ["--input", "/data/runs/my_pipeline_001/preprocessed", "--output", "/data/runs/my_pipeline_001/transformed"]
qa = true

[[stages]]
name = "analyze"
type = "command"
cmd = "./bin/analyze"
args = ["--input", "/data/runs/my_pipeline_001/transformed"]
qa = true
```
Note: STUFFIE passes arguments directly to the executable. Placeholders such as ${source_dir} and ${output_dir} are not automatically expanded.

### Config reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pipeline.name` | `String` | yes | Human-readable pipeline identifier |
| `pipeline.output_dir` | `Path` | yes | Root output directory for all stage artifacts |
| `pipeline.source_dir` | `Path` | yes | Root source directory for pipeline inputs |
| `pipeline.mode` | `String` | yes | Execution mode: `"full_auto"`, `"semi_auto"`, or `"single_break"`. Overridden by a CLI mode flag. |
| `stages[].name` | `String` | yes | Stage identifier, used in logging and checkpointing |
| `stages[].type` | `String` | yes | Type of stage: `"command"` or `"pipeline"` |
| `stages[].cmd` | `String` | only for command stages | Executable or command to invoke |
| `stages[].args` | `[String]` | no | Arguments passed to the command |
| `stages[].path` | `Path` | only for pipeline stages | Path to child pipeline config (for nested pipelines) |
| `stages[].qa` | `bool` | no | Pause for QA review after this stage (default: `false`) |

---
## Nested Pipelines

STUFFIE supports running one pipeline from within another. This allows larger workflows to be organized into smaller, reusable pipeline configurations.

A nested pipeline stage uses `type = "pipeline"` and a path pointing to the child configuration. Child paths are resolved relative to the directory containing the parent configuration file.

## Example: Parent pipeline
```toml
[pipeline]
name = "parent_pipeline"
output_dir = "./output/parent"
source_dir = "./input"
mode = "semi_auto"

[[stages]]
name = "prepare_data"
type = "command"
cmd = "python"
args = ["prepare.py"]
qa = true

[[stages]]
name = "run_child"
type = "pipeline"
path = "child.toml"
qa = false
```

## Example: Child pipeline (child.toml)

```toml
[pipeline]
name = "child_pipeline"
output_dir = "./output/child"
source_dir = "./input"
mode = "full_auto"

[[stages]]
name = "process_data"
type = "command"
cmd = "python"
args = ["process.py"]
qa = false

[[stages]]
name = "analyze_results"
type = "command"
cmd = "python"
args = ["analyze.py"]
qa = false
```
### In this example:

- The parent pauses after prepare_data in semi-auto mode.
- The child executes in full-auto mode according to its own configuration.
- Parent and child pipelines maintain independent checkpoints.
- Child configurations can themselves contain nested pipelines.


STUFFIE recursively resolves child configurations and writes each child's resolved configuration to:

```text
<child_output_dir>/.stuffie/resolved_config.json
```

The child is launched using the current STUFFIE executable and the internal
`--resolved-config` option.

> **Path note:** The child configuration path is resolved relative to the
> parent configuration file. Output directories and command arguments are not
> automatically rewritten relative to the configuration file.


## Checkpoint / Resume

STUFFIE writes a checkpoint file after each successful stage. If a run is interrupted by an abort, crash, or other failure, STUFFIE can use the checkpoint on the next startup to offer resumption.

**Checkpoint location**: '${output_dir}/.stuffie/checkpoint.json'

```toml
# .stuffie_checkpoint
run_id         = "my_pipeline_20260501_143022"
last_completed = "transform"
config         = "/home/user/pipelines/my_pipeline.toml"
```

On startup with an existing checkpoint:

```
[STUFFIE] Checkpoint found. Last completed stage: 'transform'
[STUFFIE] Resume from 'analyze'? (y)es / (n)o restart:
```

- Selecting `y` skips all previously completed stages
- Selecting `n` overwrites the checkpoint and restarts from the beginning
- On clean pipeline completion the checkpoint file is deleted automatically
- If the checkpoint does not match the current pipeline's stage count or last completed stage name, STUFFIE prompts for a full restart or abort.
- In full-auto mode (`-a`), STUFFIE resumes without asking. A checkpoint that doesn't match the pipeline still prompts, and aborts with exit code 2 if stdin is closed.

*A checkpoint records the last completed stage and its name. It does not detect every possible change to a pipeline configuration.*

---

## QA Pause Behavior

When a QA pause is triggered (stage `qa = true` in `-s` mode, or any stage in `-b` mode):

```
[STUFFIE] Stage 'transform' complete.
[STUFFIE] Review outputs at: /data/runs/my_pipeline_001.
[STUFFIE] Continue? (y)es / (n)o abort:
```

- y — continues to the next stage.
- n — aborts the pipeline with exit code 2. The checkpoint of the last completed stage is retained for a possible later resume.

---

## Stage Lifecycle

For each stage STUFFIE:

1. Logs stage start.
2. Spawns the command using tokio::process::Command.
3. Captures stdout and stderr and writes them to a stage log.
4. On exit code 0, records the completed stage in the checkpoint and evaluates the QA flag.
5. On a non-zero exit code, logs an error and exits.

Nested pipeline stages launch another STUFFIE process, which manages its own stages, logs, and checkpoints.

```
[STUFFIE] Running stage 2/3: transform
[STUFFIE] Stage 'transform' completed in 47.10 seconds
```

---

## Logging

- Command-stage output is written to `${output_dir}/logs/stage_NNN.log`, where NNN is the zero-padded stage number. Output is captured and written when the stage finishes, so it isn't visible live while a long stage runs.
- Pipeline-level STUFFIE logs are written to `${output_dir}/logs/stuffie.log`. The file is appended to across runs, so a resumed run keeps the history of the interrupted one.
- Each nested pipeline maintains its own logs in its configured output directory.
- Command stdout and stderr are captured and written to the command-stage log. Nested pipeline output is not captured into a parent stage log.
- Built on tracing and tracing-subscriber for logging.

---

## Crate Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime, subprocess management via `tokio::process` |
| `clap` | CLI argument parsing |
| `serde` + `toml` | Config file deserialization |
| `serde_json` | Serialization of resolved configurations and checkpoint data |
| `tracing` + `tracing-subscriber` | Structured logging |
| `std::time::Instant` | Measuring stage execution duration |

---

## Roadmap

- SLURM/HPC job submission per stage
- Parallel stage execution for independent branches
- Pipeline registry for sharing reusable pipeline definitions

---

## License

MIT License

Copyright (c) 2026 Julianna Della Selva, Liam McKenzie

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.

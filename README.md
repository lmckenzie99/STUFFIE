# 🦀 STUFFIE

**Sequential Task Utility For Flow Integration and Execution**

> A minimal, opinionated pipeline orchestrator for multi-stage automation workflows.

```
stuffie --config pipelines/my_pipeline.toml -s
```

```
[STUFFIE] Starting pipeline: my_pipeline
[STUFFIE] Stage 1/4 — preprocess
[STUFFIE] Stage 'preprocess' completed in 12.4s ✓
[STUFFIE] Stage 2/4 — transform
[STUFFIE] Stage 'transform' completed in 47.1s ✓
[STUFFIE] QA pause. Review outputs at: /data/runs/my_pipeline_001/transform/
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

### Examples

```bash
# Run fully automated, no interaction
stuffie --config pipelines/my_pipeline.toml -a

# Pause at designated QA stages
stuffie --config pipelines/my_pipeline.toml -s

# Pause after every stage
stuffie --config pipelines/my_pipeline.toml -b
```

---

## Configuration

Pipelines are defined in TOML. Each stage is a program STUFFIE will invoke in sequence.

```toml
[pipeline]
name       = "my_pipeline"
output_dir = "/data/runs/my_pipeline_001"
source_dir = "/data/raw/input"

[[stages]]
name = "preprocess"
cmd  = "./bin/preprocess"
args = ["--input", "${source_dir}", "--output", "${output_dir}/preprocessed"]
qa   = false

[[stages]]
name = "transform"
cmd  = "./bin/transform"
args = ["--input", "${output_dir}/preprocessed", "--output", "${output_dir}/transformed"]
qa   = true

[[stages]]
name = "analyze"
cmd  = "./bin/analyze"
args = ["--input", "${output_dir}/transformed", "--output", "${output_dir}/results"]
qa   = true

[[stages]]
name = "export"
cmd  = "python"
args = ["export.py", "--data", "${output_dir}/results"]
qa   = false
```

### Config reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pipeline.name` | `String` | yes | Human-readable pipeline identifier |
| `pipeline.output_dir` | `Path` | yes | Root output directory for all stage artifacts |
| `pipeline.source_dir` | `Path` | yes | Root source directory for pipeline inputs |
| `stages[].name` | `String` | yes | Stage identifier, used in logging and checkpointing |
| `stages[].cmd` | `String` | yes | Executable or command to invoke |
| `stages[].args` | `[String]` | no | Arguments passed to the command |
| `stages[].qa` | `bool` | no | Pause for QA review after this stage (default: `false`) |

---

## Checkpoint / Resume

STUFFIE writes a checkpoint file after each successful stage. If a run is interrupted — by abort, crash, or power loss — STUFFIE detects the checkpoint on next startup and offers to resume.

**Checkpoint location**: `${output_dir}/.stuffie_checkpoint`

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
- If the checkpoint references a stage that no longer exists in the config, STUFFIE warns and prompts for a full restart

---

## QA Pause Behavior

When a QA pause is triggered (stage `qa = true` in `-s` mode, or any stage in `-b` mode):

```
[STUFFIE] Stage 'transform' complete.
[STUFFIE] Review outputs at: /data/runs/my_pipeline_001/transformed/
[STUFFIE] Continue? (y)es / (n)o abort:
```

- `y` — writes checkpoint, advances to next stage
- `n` — writes checkpoint of last completed stage, exits cleanly

---

## Stage Lifecycle

For each stage STUFFIE:

1. Logs stage start with timestamp
2. Spawns subprocess via `tokio::process::Command`
3. Captures stdout/stderr, writes to `${output_dir}/logs/${stage_name}.log`
4. On exit code `0` — writes checkpoint, evaluates QA flag
5. On non-zero exit — logs error, prints failure message, exits

```
[STUFFIE] Starting stage: transform
[STUFFIE] Running: ./bin/transform --input /data/preprocessed --output /data/transformed
[STUFFIE] Stage 'transform' failed with exit code 1. See /data/logs/transform.log
```

---

## Logging

- Per-stage logs written to `${output_dir}/logs/<stage_name>.log`
- Pipeline-level log written to `${output_dir}/logs/stuffie.log`
- Log format: `[TIMESTAMP] [LEVEL] [STAGE] message`
- Built on `tracing` for structured, HPC-compatible output

---

## Crate Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime, subprocess management via `tokio::process` |
| `clap` | CLI argument parsing |
| `serde` + `toml` | Config file deserialization |
| `tracing` + `tracing-subscriber` | Structured logging |

---

## Roadmap

- SLURM/HPC job submission per stage
- Parallel stage execution for independent branches
- Pipeline registry for sharing reusable pipeline definitions

---

## License

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

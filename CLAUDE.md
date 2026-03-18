# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run --bin c2pa-validate -- [OPTIONS] INPUT...

# Test (all)
cargo test -p c2pa-validate

# Test (single test, serial to avoid network races)
cargo test -p c2pa-validate -- test_name --test-threads=1

# Run a specific test file
cargo test -p c2pa-validate --test output_formats
```

## Architecture

Single Rust workspace with one application crate (`crates/c2pa-validate`) and three vendored git submodules:
- `vendor/c2pa-rs` — C2PA SDK (asset reading, manifest extraction)
- `vendor/profile-evaluator-rs` — Evaluates YAML profiles against asset indicators
- `vendor/json-formula-rs` — JSON formula evaluation (used by profile evaluator)

### Module Responsibilities

- **`cli.rs`** — Clap derive-based CLI: `OutputFormat` (Json/Yaml/Markdown/Html), `TrustMode` (Default/Itl/Custom)
- **`validator.rs`** — Core validation engine: input type detection, trust scenario orchestration, profile evaluation
- **`report.rs`** — Report data structures (`CrJsonReport`, `AssetReport`, etc.) and rendering (Markdown/HTML)
- **`lib.rs`** — Orchestration: calls `Validator`, routes output (file vs. stdout, per-file vs. aggregate), renders final report
- **`main.rs`** — Thin entry point calling `c2pa_validate::run()`

### Validation Flow

1. **Input classification**: file extension + JSON `schema` field → Asset, SidecarManifest (.c2pa), or CrJson
2. **Trust scenarios**: build ordered list based on `--trust-mode`; try each in order, return first Trusted result or last Valid
   - `default`: fetch official C2PA trust list from GitHub
   - `itl`: try official, then ITL
   - `custom`: use user-supplied PEM file/URL
3. **crJSON generation**: `Reader::to_crjson_value()` from c2pa-rs SDK
4. **Profile evaluation**: `profile_evaluator_rs::evaluate(profile, indicators)` against crJSON indicators
5. **Report aggregation**: all results → `CrJsonReport` with summary counters; exit code from `summary.errors + summary.invalid`

### Output Routing (lib.rs)

- Single asset + structured format → write to `-o FILE`
- Multiple assets + structured format → write per-file to `-o DIR` (collision: `_2`, `_3` suffixes)
- Profile evaluation → writes both crJSON file and `_report` file
- Markdown/HTML → single comprehensive report file

### Testing

Tests use `run_with_cli(cli)` for programmatic invocation. Fixtures in `testfiles/`:
- `assets/` — JPEG, PNG, MP4, PDF samples
- `manifests/` — `.c2pa` sidecar files
- `crjson/` — valid/invalid crJSON fixtures
- `profiles/` — YAML asset profiles

Known coverage gaps: offline trust modes, custom trust PEM, strict mode, settings overlay.

## Code Style (from Cursor rules)

- No `unwrap()`/`expect()` on fallible values in production paths
- Use `?` for error propagation with `anyhow` context
- No `unsafe` blocks without justification
- Logs → stderr via `tracing`; machine-readable output → stdout only
- Exit codes: 0 for success, non-zero for failure
- Validate all external input (files, env, network) at boundaries

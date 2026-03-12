# Testing

## Running tests

From the repository root:

```bash
cargo test -p c2pa-validate
```

Unit tests (no I/O) run in the library; integration tests use fixtures under `testfiles/`.

## Test layout

| Location | Purpose |
|----------|--------|
| `crates/c2pa-validate/src/*.rs` `#[cfg(test)]` | Unit tests (report, lib, validator helpers) |
| `crates/c2pa-validate/tests/common/` | Path helpers for testfiles |
| `crates/c2pa-validate/tests/asset_validation.rs` | C2PA asset validation (JPEG, PNG, MP4, PDF, .c2pa sidecars) |
| `crates/c2pa-validate/tests/crjson_validation.rs` | crJSON schema validation (valid/invalid fixtures) |
| `crates/c2pa-validate/tests/output_formats.rs` | Exit code, Markdown/HTML render, full run with `-o` |
| `crates/c2pa-validate/tests/schema_validation.rs` | Output JSON validated against crJSON schema (draft 2020-12) |
| `crates/c2pa-validate/tests/negative_tests.rs` | Invalid/missing paths, empty inputs, glob matching nothing, missing params |
| `testfiles/assets/` | Sample C2PA assets (JPEG, PNG, MP4, PDF) |
| `testfiles/manifests/` | Standalone `.c2pa` manifest files (manifest-only, no source asset) |
| `testfiles/crjson/` | Minimal crJSON fixtures (valid and invalid) |
| `crJSON-docs/crJSON-schema.json` | Official crJSON schema for formal validation of output JSON |

## What is tested

- **Asset validation**: Single/multiple assets from `testfiles/assets/` (JPEG, PNG, MP4, PDF); `.c2pa` sidecar manifests from `testfiles/manifests/`; report shape, `reader_json`, validation state, input path, `InputType::SidecarManifest`.
- **crJSON validation**: Valid minimal crJSON passes; missing `schema_version` or `results` array fails with expected messages; JSON without `schema: "crjson"` is treated as Asset and fails accordingly.
- **crJSON schema validation**: Output JSON (Reader crJSON) from assets and sidecars is validated against `crJSON-docs/crJSON-schema.json` (JSON Schema draft 2020-12) via the `jsonschema` crate.
- **Report**: `exit_code()` (success/failure from summary), `render_markdown()`, `render_html()` (structure and escaping).
- **Lib**: `normalize_output_path()`, `run_with_cli()` (writes to `-o`).
- **Validator**: Glob expansion, glob detection, trust classification mapping.
- **Multi-asset JSON output**: Two assets with `-o <directory>` and JSON format produce two `.json` files (one per asset) with correct stems; each file is valid crJSON.
- **Glob patterns**: Pattern `testfiles/assets/PXL*.jpg` expands to both PXL assets; report has two results and correct summary.
- **Negative / error paths**: Empty inputs → "no input files matched"; glob matching no files → "did not match any files"; non-existent file → "failed to validate" / "failed to resolve"; `--trust-mode custom` without `--trust-list` → `Validator::new` error; `run_with_cli` with empty or bad path returns `ExitCode::FAILURE`.

## What prevents testing specific features

1. **Trust modes `default` and `itl`**  
   Both fetch trust lists from the network (official C2PA and ITL URLs). Asset tests that use `TrustMode::Default` therefore **require network access**. Without it, validation fails with a fetch error.  
   - **Gap**: No offline tests for “trusted” vs “valid” vs “invalid” outcomes.  
   - **Mitigation**: Add a test PEM in `testfiles/` and use `--trust-mode custom --trust-list testfiles/...` for offline trust tests (blocked until a test trust list is added).

2. **`--trust-mode custom`**  
   Requires `--trust-list FILE_OR_URL`. There is **no test PEM or test URL** in the repo, so custom trust list behavior is not exercised by tests.  
   - **Gap**: Custom trust list loading (file and URL) and validation against it are untested.  
   - **Mitigation**: Add a small test PEM (e.g. from c2pa-org/conformance-public or a self-signed fixture) under `testfiles/` and add integration tests that use it.

3. **`--strict`**  
   Strict mode turns “valid but with warnings” into an error. Current assets may or may not produce warnings; there is no fixture designed to have warnings.  
   - **Gap**: No test that asserts strict mode changes exit code or summary when warnings are present.  
   - **Mitigation**: Add an asset known to produce warnings, or mock a report with warnings and assert strict behavior.

4. **`--settings`**  
   Overlay of c2pa-rs settings from a JSON/TOML file is not tested.  
   - **Gap**: No test that passes `--settings` and verifies the settings affect validation (e.g. different verification result).  
   - **Mitigation**: Add a settings file and an asset or scenario whose result changes when the setting is applied.

## Fixtures reference

**Assets** (`testfiles/assets/`):

- `PXL_20260208_202351558.jpg` — C2PA JPEG.  
- `PXL_20250818_155024632~4.jpg` — Second PXL JPEG (used with first for multi-asset and glob tests).  
- `ChatGPT_Image.png` — C2PA PNG.  
- `manifest_tcID_112.mp4` — C2PA MP4.  
- `adobe-20240110-single_manifest_store.pdf` — C2PA PDF.  
- `gettyimages-1500448395-612x612.jpg` — C2PA JPEG; used as **negative test** (fails with default trust list; we assert validation returns an error).

**Manifest-only** (`testfiles/manifests/`):

- `manifest_data.c2pa` — Standalone .c2pa sidecar.  
- `cloud_manifest.c2pa` — Standalone .c2pa sidecar.

**crJSON** (`testfiles/crjson/`):

- `valid_minimal.json` — `schema`, `schema_version`, `results: []` (valid crJSON).  
- `invalid_missing_schema.json` — No `schema`; treated as Asset, so validation fails.  
- `invalid_schema_version_missing.json` — `schema: "crjson"` but no `schema_version`; crJSON validation fails.  
- `invalid_no_results_array.json` — `schema` and `schema_version` but no `results`; crJSON validation fails.

**Schema** (`crJSON-docs/`):

- `crJSON-schema.json` — JSON Schema (draft 2020-12) for Content Credential JSON; used to validate tool output in `schema_validation` tests.

# c2pa-conformance-tool-cli

A Rust CLI for validating C2PA media assets, standalone `.c2pa` manifests, and crJSON reports. Output is structured crJSON by default, with optional Markdown or HTML summaries.

## What it validates

- **Media assets** — JPEG, PNG, PDF, MP4, and other formats that can contain or reference C2PA manifests
- **Standalone manifests** — `.c2pa` sidecar files (manifest structure and signatures only; no source asset)
- **crJSON reports** — JSON files with `schema: "crjson"` (structure and required fields)

## Trust list modes

- **`default`** — Validate against the [official C2PA Conformance Trust List](https://c2pa.org/conformance) only
- **`itl`** — Try official list first, then the Interim Trust List (ITL)
- **`custom`** — Use your own trust list (requires `--trust-list FILE_OR_URL`)

Custom trust lists can be a local path or URL to a PEM file. Use `--settings` to overlay c2pa-rs settings for advanced trust configuration.

## Output

- **Formats**: `--format json` (default), `markdown`, or `html`
- **Output location**: `-o/--output` file or directory. If omitted, a single result is written next to the source (e.g. `photo.jpg` → `photo.json`); with multiple inputs and JSON, use `-o <directory>` to write one file per input
- **crJSON**: JSON output follows the crJSON schema (Reader crJSON for assets; full report with summary for multi-asset)

## Usage examples

Validate one or more assets (default JSON to stdout or next to file):

```bash
cargo run --bin c2pa-validate -- image.jpg
cargo run --bin c2pa-validate -- --format json ./samples/*.jpg
```

Write to a specific file or directory:

```bash
cargo run --bin c2pa-validate -- -o report.json image.jpg
cargo run --bin c2pa-validate -- -o ./out --format json ./samples/*.jpg
```

Standalone `.c2pa` manifest (no source asset):

```bash
cargo run --bin c2pa-validate -- manifest.c2pa
```

Trust list options:

```bash
cargo run --bin c2pa-validate -- --trust-mode itl image.jpg
cargo run --bin c2pa-validate -- --trust-mode custom --trust-list ./my-trust.pem image.jpg
```

Human-readable output and strict mode (warnings as failures):

```bash
cargo run --bin c2pa-validate -- --format markdown -o report.md image.jpg
cargo run --bin c2pa-validate -- --strict image.jpg
```

Overlay c2pa-rs settings from a file:

```bash
cargo run --bin c2pa-validate -- --settings settings.json image.jpg
```

Help and version:

```bash
cargo run --bin c2pa-validate -- --help
cargo run --bin c2pa-validate -- --version
```

## Options summary

| Option | Description |
|--------|-------------|
| `INPUT...` | Files or glob patterns to validate |
| `-o, --output FILE_OR_DIR` | Output file or directory |
| `-f, --format json\|markdown\|html` | Output format (default: json) |
| `-t, --trust-mode default\|itl\|custom` | Trust list mode |
| `--trust-list FILE_OR_URL` | Trust list path/URL (required for custom) |
| `--settings FILE` | Overlay c2pa-rs settings (JSON/TOML) |
| `--strict` | Fail on warnings, not only invalid assets |
| `-v, --verbose` | Increase verbosity (repeat for debug) |

For build, workspace layout, and contributing, see [CONTRIBUTING.md](CONTRIBUTING.md).

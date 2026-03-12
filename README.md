# c2pa-conformance-tool-cli

A Rust CLI for validating C2PA media assets, standalone `.c2pa` manifests, and generated `crJSON` reports.

## Workspace layout

- `crates/c2pa-validate`: application crate and CLI binary
- `vendor/c2pa-rs`: local git subproject for the `c2pa` SDK

The CLI depends on `c2pa-rs` by local path instead of crates.io.

## Build

```bash
cargo build
```

## Usage

```bash
cargo run --bin c2pa-validate -- --format json ./samples/*.jpg
```

For a standalone sidecar manifest, pass the source asset explicitly:

```bash
cargo run --bin c2pa-validate -- manifest.c2pa --asset image.jpg
```

Use built-in or custom asset profiles:

```bash
cargo run --bin c2pa-validate -- --profile trusted image.jpg
cargo run --bin c2pa-validate -- --profile-file profile.json image.jpg
```

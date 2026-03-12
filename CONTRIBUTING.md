# Building and contributing

## Build

```bash
cargo build
```

Release build:

```bash
cargo build --release
```

Run the CLI:

```bash
cargo run --bin c2pa-validate -- [OPTIONS] INPUT...
```

## Testing

Run tests with `cargo test -p c2pa-validate`. See [TESTING.md](TESTING.md) for test layout, what is covered, and what currently prevents testing specific features (e.g. trust modes, sidecar manifests).

## Workspace layout

- **`crates/c2pa-validate`** — Application crate and CLI binary (`c2pa-validate`)
- **`vendor/c2pa-rs`** — Local git subproject for the `c2pa` SDK

The CLI depends on `c2pa-rs` by path (see `crates/c2pa-validate/Cargo.toml`), not from crates.io.

## Contributing

1. Ensure the project builds and tests pass: `cargo build` and `cargo test`.
2. Follow existing code style and the project’s Rust/CLI conventions.
3. For larger changes, open an issue or discussion first if helpful.
4. Submit changes via pull request with a clear description of what changed and why.

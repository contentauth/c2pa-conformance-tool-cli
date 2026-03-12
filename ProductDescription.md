# C2PA Conformance Testing Tool

## Overall Description

A Command Line Interface (CLI) tool built in Rust based on the c2pa-rs library for validating C2PA manifests in media files. Reports will use the new crJSON format for comprehensive, structured output but also provide human-readable summaries for ease of use.

## Core Features

- **Standard CLI Tooling**: Familiar command-line interface for developers and testers
  - single character (along with common long form) flags for common options (e.g., `-o` & `--output` for output file)
  - clear help documentation (`--help` flag & `--version` flag)
- **Support various formats**: Accepts a wide range of media file formats (e.g., JPEG, PNG, PDF, MP4), stand alone manifests (.c2pa) and stand-alone crJSON files for validation
- **Batch Processing**: Support multiple files in a single command
  - multiple files can be specified (e.g., `c2pa-validate file1.jpg file2.png`)
  - wildcard support (e.g., `c2pa-validate ./images/*.jpg`)
- **JSON Output**: Output results in structured JSON format (crJSON) for integration into automated workflows


## Trust List Support
- **Official C2PA Trust List**: Validates signatures against the official [C2PA Conformance Trust List](https://c2pa.org/conformance)
- **Interim Trust List (ITL)**: Validates signatures against the ITL
- **Test Certificate Upload**: Allows for the use of custom test certificates to be supplied by the user

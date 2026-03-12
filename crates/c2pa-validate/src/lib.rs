pub mod cli;
pub mod profiles;
pub mod report;
pub mod validator;

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::Value as JsonValue;
use tracing::Level;

use crate::{
    cli::{Cli, OutputFormat},
    report::{CrJsonReport, ReportItem},
    validator::Validator,
};

pub fn run() -> ExitCode {
    match try_run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn try_run() -> Result<ExitCode> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;

    let validator = Validator::new(cli.clone())?;
    let report = validator.run()?;

    let asset_count = report
        .results
        .iter()
        .filter(|r| matches!(r, ReportItem::Asset(_)))
        .count();
    let multiple_assets_json = cli.format == OutputFormat::Json && asset_count > 1;

    if multiple_assets_json {
        let output = cli.output.as_ref().ok_or_else(|| {
            anyhow::anyhow!("With multiple inputs and JSON format, specify -o <directory> to write one file per input.")
        })?;
        let out_dir = resolve_output_dir(output)?;
        write_crjson_per_asset(&report, &out_dir)?;
    } else {
        let rendered = render_report(&report, cli.format)?;
        if let Some(output) = cli.output.as_ref() {
            let path = resolve_output_path(output, cli.format, &report)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create output directory {}", parent.display())
                })?;
            }
            fs::write(&path, rendered)
                .with_context(|| format!("failed to write output to {}", path.display()))?;
        } else {
            write_output(cli.format, &rendered)?;
        }
    }

    Ok(report.exit_code())
}

fn init_tracing(verbose: u8) -> Result<()> {
    let level = match verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        _ => Level::DEBUG,
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(io::stderr)
        .without_time()
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize logging: {error}"))?;

    Ok(())
}

fn render_report(report: &CrJsonReport, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Json => {
            let crjson_value = report_to_crjson_only(report);
            serde_json::to_string_pretty(&crjson_value).context("failed to render crJSON")
        }
        OutputFormat::Markdown => Ok(report.render_markdown()),
        OutputFormat::Html => Ok(report.render_html()),
    }
}

/// Output is only Reader crJSON: one object for a single asset, or null for none.
fn report_to_crjson_only(report: &CrJsonReport) -> JsonValue {
    let crjsons: Vec<JsonValue> = report
        .results
        .iter()
        .filter_map(|r| {
            if let ReportItem::Asset(asset) = r {
                asset.reader_json.clone()
            } else {
                None
            }
        })
        .collect();
    match crjsons.len() {
        0 => JsonValue::Null,
        1 => crjsons.into_iter().next().unwrap_or(JsonValue::Null),
        _ => JsonValue::Array(crjsons), // unused when multiple (we write per-file)
    }
}

/// Resolve -o to a directory for multi-file JSON output. Errors if path looks like a single file.
fn resolve_output_dir(output: &Path) -> Result<PathBuf> {
    let looks_like_file = output
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".json") || n.ends_with(".md") || n.ends_with(".html"));
    if looks_like_file && !output.is_dir() {
        anyhow::bail!(
            "With multiple inputs and JSON format, use -o <directory> to write one file per input (e.g. -o ./out)"
        );
    }
    fs::create_dir_all(output)
        .with_context(|| format!("failed to create output directory {}", output.display()))?;
    Ok(output.to_path_buf())
}

/// Write one .json file per asset into out_dir, named from each input's stem (duplicate stems get _2, _3, ...).
fn write_crjson_per_asset(report: &CrJsonReport, out_dir: &Path) -> Result<()> {
    let mut stem_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for r in &report.results {
        if let ReportItem::Asset(asset) = r {
            let Some(ref crjson) = asset.reader_json else {
                continue;
            };
            let stem = Path::new(&asset.input.resolved_path)
                .file_stem()
                .and_then(|s| s.to_os_string().into_string().ok())
                .unwrap_or_else(|| "report".to_string());
            let count = stem_counts.entry(stem.clone()).or_insert(0);
            *count += 1;
            let filename = if *count == 1 {
                format!("{stem}.json")
            } else {
                format!("{stem}_{}.json", count)
            };
            let path = out_dir.join(&filename);
            let rendered =
                serde_json::to_string_pretty(crjson).context("failed to render crJSON")?;
            fs::write(&path, rendered)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }
    Ok(())
}

fn format_extension(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Json => "json",
        OutputFormat::Markdown => "md",
        OutputFormat::Html => "html",
    }
}

/// Treats -o as a directory when it is an existing directory or has no format extension.
/// When treated as a directory, creates it if needed and chooses a filename from the source file(s).
fn resolve_output_path(
    output: &Path,
    format: OutputFormat,
    report: &CrJsonReport,
) -> Result<PathBuf> {
    let ext = format_extension(format);
    let is_output_dir = output.is_dir()
        || !output
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".json") || n.ends_with(".md") || n.ends_with(".html"));

    if is_output_dir {
        fs::create_dir_all(output)
            .with_context(|| format!("failed to create output dir {}", output.display()))?;
        let base = if report.results.len() == 1 {
            report
                .results
                .first()
                .and_then(|r| {
                    Path::new(r.input_path())
                        .file_stem()
                        .and_then(|s| s.to_os_string().into_string().ok())
                })
                .unwrap_or_else(|| "report".to_string())
        } else {
            "report".to_string()
        };
        Ok(output.join(format!("{base}.{ext}")))
    } else {
        Ok(output.to_path_buf())
    }
}

fn write_output(format: OutputFormat, rendered: &str) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{rendered}");
        }
        OutputFormat::Markdown | OutputFormat::Html => {
            let mut stderr = io::stderr().lock();
            stderr
                .write_all(rendered.as_bytes())
                .context("failed to write report to stderr")?;
            if !rendered.ends_with('\n') {
                stderr.write_all(b"\n").context("failed to flush newline")?;
            }
        }
    }

    Ok(())
}

pub fn normalize_output_path(path: Option<PathBuf>) -> Option<PathBuf> {
    path.map(|candidate| {
        if candidate.is_absolute() {
            candidate
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(candidate)
        }
    })
}

/*
Copyright 2026 Adobe. All rights reserved.
This file is licensed to you under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License. You may obtain a copy
of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under
the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
OF ANY KIND, either express or implied. See the License for the specific language
governing permissions and limitations under the License.
*/

use std::process::ExitCode;

use c2pa::validation_results::ValidationState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrJsonReport {
    pub schema: &'static str,
    pub schema_version: &'static str,
    pub tool: ToolMetadata,
    pub generated_at: String,
    pub summary: Summary,
    pub results: Vec<ReportItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub c2pa_sdk: SdkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkMetadata {
    pub name: &'static str,
    pub version: &'static str,
    pub source: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Summary {
    pub total: usize,
    pub trusted: usize,
    pub valid: usize,
    pub invalid: usize,
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReportItem {
    Asset(AssetReport),
    CrJsonValidation(CrJsonValidationReport),
}

impl ReportItem {
    /// Resolved path of the input file for this result.
    pub fn input_path(&self) -> &str {
        match self {
            ReportItem::Asset(r) => &r.input.resolved_path,
            ReportItem::CrJsonValidation(r) => &r.input.resolved_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReport {
    pub input: InputDescriptor,
    pub validation_state: ValidationState,
    pub trust: TrustAssessment,
    pub active_manifest_label: Option<String>,
    pub manifest_count: usize,
    pub ingredient_count: usize,
    pub assertion_labels: Vec<String>,
    pub statuses: Vec<StatusRecord>,
    pub manifests: Vec<ManifestRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reader_json: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrJsonValidationReport {
    pub input: InputDescriptor,
    pub valid: bool,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDescriptor {
    pub original: String,
    pub resolved_path: String,
    pub input_type: InputType,
    pub detected_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputType {
    Asset,
    SidecarManifest,
    CrJson,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustAssessment {
    pub mode: String,
    pub classification: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRecord {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestRecord {
    pub label: Option<String>,
    pub title: Option<String>,
    pub format: Option<String>,
    pub claim_generator: Option<String>,
    pub signature: Option<SignatureRecord>,
    pub ingredients: Vec<IngredientRecord>,
    pub assertions: Vec<AssertionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureRecord {
    pub alg: Option<String>,
    pub issuer: Option<String>,
    pub common_name: Option<String>,
    pub serial_number: Option<String>,
    pub time: Option<String>,
    pub revoked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngredientRecord {
    pub title: Option<String>,
    pub format: Option<String>,
    pub relationship: Option<String>,
    pub active_manifest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionRecord {
    pub label: String,
    pub instance: usize,
    pub kind: String,
}

impl CrJsonReport {
    pub fn exit_code(&self) -> ExitCode {
        if self.summary.errors > 0 || self.summary.invalid > 0 {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }

    pub fn render_markdown(&self) -> String {
        let mut output = String::new();
        output.push_str("# C2PA Conformance Report\n\n");
        output.push_str(&format!(
            "- Total: {}\n- Trusted: {}\n- Valid: {}\n- Invalid: {}\n- Errors: {}\n\n",
            self.summary.total,
            self.summary.trusted,
            self.summary.valid,
            self.summary.invalid,
            self.summary.errors
        ));

        for result in &self.results {
            match result {
                ReportItem::Asset(report) => {
                    output.push_str(&format!("## `{}`\n\n", report.input.resolved_path));
                    output.push_str(&format!(
                        "- Validation state: `{}`\n- Trust: `{}`\n- Trust source: `{}`\n- Manifests: `{}`\n- Ingredients: `{}`\n\n",
                        render_state(report.validation_state),
                        report.trust.classification,
                        report.trust.source.as_deref().unwrap_or("n/a"),
                        report.manifest_count,
                        report.ingredient_count
                    ));
                }
                ReportItem::CrJsonValidation(report) => {
                    output.push_str(&format!("## `{}`\n\n", report.input.resolved_path));
                    output.push_str(&format!(
                        "- crJSON validation: `{}`\n\n",
                        if report.valid { "pass" } else { "fail" }
                    ));
                }
            }
        }

        output
    }

    pub fn render_html(&self) -> String {
        let body = self
            .results
            .iter()
            .map(|result| match result {
                ReportItem::Asset(report) => format!(
                    "<section><h2>{}</h2><p>state: {} | trust: {} | source: {}</p></section>",
                    html_escape(&report.input.resolved_path),
                    render_state(report.validation_state),
                    html_escape(&report.trust.classification),
                    html_escape(report.trust.source.as_deref().unwrap_or("n/a"))
                ),
                ReportItem::CrJsonValidation(report) => format!(
                    "<section><h2>{}</h2><p>crJSON validation: {}</p></section>",
                    html_escape(&report.input.resolved_path),
                    if report.valid { "pass" } else { "fail" }
                ),
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><title>C2PA Conformance Report</title><style>body{{font-family:ui-monospace,monospace;margin:2rem;background:#f7f4ec;color:#1e1b16}}section{{padding:1rem 1.25rem;margin:0 0 1rem;background:#fff;border:1px solid #d4c8b2;border-radius:12px}}</style></head><body><h1>C2PA Conformance Report</h1>{body}</body></html>"
        )
    }
}

fn render_state(state: ValidationState) -> &'static str {
    match state {
        ValidationState::Trusted => "trusted",
        ValidationState::Valid => "valid",
        ValidationState::Invalid => "invalid",
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

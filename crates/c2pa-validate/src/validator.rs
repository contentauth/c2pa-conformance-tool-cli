use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use c2pa::{
    format_from_path, settings::Settings, validation_results::ValidationState,
    Context as C2paContext, Manifest, Reader,
};
use glob::glob;
use serde_json::Value;
use tracing::debug;

use crate::{
    cli::{BuiltInProfile, Cli, TrustMode},
    profiles::{built_in_profile, load_profile_file, AssetProfile},
    report::{
        AssertionRecord, AssetReport, CrJsonReport, CrJsonValidationReport, IngredientRecord,
        InputDescriptor, InputType, ManifestRecord, ProfileReport, ReportItem, SdkMetadata,
        SignatureRecord, StatusRecord, Summary, ToolMetadata, TrustAssessment,
    },
};

const OFFICIAL_TRUST_LIST_URL: &str =
    "https://raw.githubusercontent.com/c2pa-org/conformance-public/main/trust-list/C2PA-TRUST-LIST.pem";
const DEFAULT_ITL_URL: &str =
    "https://raw.githubusercontent.com/c2pa-org/conformance-public/main/trust-list/ITL.pem";

#[derive(Debug, Clone)]
pub struct Validator {
    cli: Cli,
    profiles: Vec<AssetProfile>,
}

#[derive(Debug, Clone)]
struct TrustScenario {
    label: String,
    source: String,
    settings: Settings,
}

impl Validator {
    pub fn new(cli: Cli) -> Result<Self> {
        let profiles = load_profiles(&cli)?;
        Ok(Self { cli, profiles })
    }

    pub fn run(&self) -> Result<CrJsonReport> {
        let inputs = expand_inputs(&self.cli.inputs)?;
        if inputs.is_empty() {
            bail!("no input files matched");
        }

        let mut summary = Summary::default();
        let mut results = Vec::with_capacity(inputs.len());

        for input in inputs {
            let report = self
                .validate_input(&input)
                .with_context(|| format!("failed to validate {}", input.display()))?;

            match &report {
                ReportItem::Asset(asset) => {
                    summary.total += 1;
                    match asset.validation_state {
                        ValidationState::Trusted => summary.trusted += 1,
                        ValidationState::Valid => summary.valid += 1,
                        ValidationState::Invalid => summary.invalid += 1,
                    }
                    summary.warnings += asset.warnings.len();
                    summary.profile_failures += asset
                        .profile_results
                        .iter()
                        .filter(|profile| !profile.passed)
                        .count();
                }
                ReportItem::CrJsonValidation(report) => {
                    summary.total += 1;
                    if !report.valid {
                        summary.errors += 1;
                    }
                }
            }

            results.push(report);
        }

        if self.cli.strict {
            summary.errors += results
                .iter()
                .filter_map(|item| match item {
                    ReportItem::Asset(asset) => Some(
                        asset.validation_state == ValidationState::Valid
                            && !asset.profile_results.iter().all(|profile| profile.passed),
                    ),
                    ReportItem::CrJsonValidation(_) => None,
                })
                .filter(|needs_error| *needs_error)
                .count();
        }

        Ok(CrJsonReport {
            schema: "crjson",
            schema_version: "0.1.0",
            tool: ToolMetadata {
                name: env!("CARGO_PKG_NAME"),
                version: env!("CARGO_PKG_VERSION"),
                c2pa_sdk: SdkMetadata {
                    name: c2pa::NAME,
                    version: c2pa::VERSION,
                    source: "vendor/c2pa-rs/sdk",
                },
            },
            generated_at: iso_now(),
            summary,
            results,
        })
    }

    fn validate_input(&self, path: &Path) -> Result<ReportItem> {
        match detect_input_type(path)? {
            InputType::CrJson => self.validate_crjson(path),
            InputType::Asset | InputType::SidecarManifest => {
                Ok(ReportItem::Asset(self.validate_asset(path)?))
            }
        }
    }

    fn validate_crjson(&self, path: &Path) -> Result<ReportItem> {
        let data = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let value: Value = serde_json::from_str(&data)
            .with_context(|| format!("failed to parse {}", path.display()))?;

        let mut messages = Vec::new();
        let mut valid = true;

        if value.get("schema").and_then(Value::as_str) != Some("crjson") {
            valid = false;
            messages.push("schema must equal 'crjson'".to_string());
        }

        if value
            .get("schema_version")
            .and_then(Value::as_str)
            .is_none()
        {
            valid = false;
            messages.push("schema_version is required".to_string());
        }

        if !value.get("results").is_some_and(Value::is_array) {
            valid = false;
            messages.push("results must be an array".to_string());
        }

        if messages.is_empty() {
            messages.push("crJSON structure is valid".to_string());
        }

        Ok(ReportItem::CrJsonValidation(CrJsonValidationReport {
            input: input_descriptor(path, InputType::CrJson)?,
            valid,
            messages,
        }))
    }

    fn validate_asset(&self, path: &Path) -> Result<AssetReport> {
        let input_type = detect_input_type(path)?;
        let scenarios = self.build_trust_scenarios()?;

        let mut last_asset = None;
        let mut warnings = Vec::new();

        for scenario in scenarios {
            match self.read_asset(path, input_type.clone(), &scenario.settings) {
                Ok((reader, reader_json)) => {
                    let asset = self.build_asset_report(
                        path,
                        input_type.clone(),
                        scenario.label,
                        scenario.source,
                        reader,
                        reader_json,
                    )?;

                    if asset.validation_state == ValidationState::Trusted {
                        return Ok(asset);
                    }

                    last_asset = Some(asset);
                }
                Err(error) => {
                    debug!(
                        "validation with scenario {} failed: {error:#}",
                        scenario.label
                    );
                    warnings.push(format!(
                        "trust scenario '{}' could not complete: {error}",
                        scenario.label
                    ));
                }
            }
        }

        let mut asset = last_asset.ok_or_else(|| anyhow!("all trust scenarios failed"))?;
        asset.warnings.extend(warnings);
        Ok(asset)
    }

    fn read_asset(
        &self,
        path: &Path,
        input_type: InputType,
        settings: &Settings,
    ) -> Result<(Reader, Option<Value>)> {
        let context = C2paContext::new()
            .with_settings(settings.clone())
            .context("failed to build c2pa context")?;

        let reader = match input_type {
            InputType::Asset => Reader::from_context(context)
                .with_file(path)
                .with_context(|| format!("failed to read asset {}", path.display()))?,
            InputType::SidecarManifest => self.read_sidecar(path, context)?,
            InputType::CrJson => bail!("crjson is not handled by read_asset"),
        };

        let reader_json = Some(
            reader
                .to_crjson_value()
                .context("failed to render reader crJSON")?,
        );

        Ok((reader, reader_json))
    }

    fn read_sidecar(&self, path: &Path, context: C2paContext) -> Result<Reader> {
        let asset_path = self.cli.asset.clone().ok_or_else(|| {
            anyhow!("--asset is required when validating a standalone .c2pa manifest")
        })?;
        let asset_format = format_from_path(&asset_path)
            .ok_or_else(|| anyhow!("unsupported asset type for {}", asset_path.display()))?;
        let c2pa_data =
            fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        let stream = fs::File::open(&asset_path)
            .with_context(|| format!("failed to open {}", asset_path.display()))?;

        Reader::from_context(context)
            .with_manifest_data_and_stream(&c2pa_data, &asset_format, stream)
            .with_context(|| {
                format!(
                    "failed to validate {} against asset {}",
                    path.display(),
                    asset_path.display()
                )
            })
    }

    fn build_asset_report(
        &self,
        path: &Path,
        input_type: InputType,
        scenario_label: String,
        scenario_source: String,
        reader: Reader,
        reader_json: Option<Value>,
    ) -> Result<AssetReport> {
        let validation_state = reader.validation_state();
        let manifests = reader
            .iter_manifests()
            .map(manifest_record)
            .collect::<Result<Vec<_>>>()?;
        let active_manifest_label = reader.active_label().map(ToOwned::to_owned);
        let ingredient_count = reader
            .iter_manifests()
            .map(|manifest| manifest.ingredients().len())
            .sum();
        let assertion_labels = reader
            .iter_manifests()
            .flat_map(|manifest| {
                manifest
                    .assertions()
                    .iter()
                    .map(|assertion| assertion.label().to_string())
            })
            .collect::<Vec<_>>();
        let statuses = collect_statuses(&reader);

        let mut report = AssetReport {
            input: input_descriptor(path, input_type)?,
            validation_state,
            trust: TrustAssessment {
                mode: self.cli.trust_mode.to_string(),
                classification: trust_classification(validation_state).to_string(),
                source: Some(scenario_source),
                notes: trust_notes(&self.cli),
            },
            active_manifest_label,
            manifest_count: manifests.len(),
            ingredient_count,
            assertion_labels,
            statuses,
            manifests,
            profile_results: Vec::new(),
            reader_json,
            warnings: Vec::new(),
        };

        report.profile_results = self
            .profiles
            .iter()
            .map(|profile| profile.evaluate(&report))
            .collect::<Vec<ProfileReport>>();

        if validation_state != ValidationState::Trusted && scenario_label == "itl" {
            report
                .trust
                .notes
                .push("ITL was checked but the manifest did not chain to it".to_string());
        }

        Ok(report)
    }

    fn build_trust_scenarios(&self) -> Result<Vec<TrustScenario>> {
        let base = self.base_settings()?;
        let scenarios = match self.cli.trust_mode {
            TrustMode::None => vec![TrustScenario {
                label: "none".to_string(),
                source: "validation_only".to_string(),
                settings: base.clone().with_value("verify.verify_trust", false)?,
            }],
            TrustMode::Official => vec![self.with_trust_source(
                &base,
                "official",
                self.cli
                    .official_trust_list
                    .clone()
                    .unwrap_or_else(|| OFFICIAL_TRUST_LIST_URL.to_string()),
            )?],
            TrustMode::Itl => vec![self.with_trust_source(
                &base,
                "itl",
                self.cli
                    .itl_trust_list
                    .clone()
                    .unwrap_or_else(|| DEFAULT_ITL_URL.to_string()),
            )?],
            TrustMode::Custom => vec![self.with_custom_trust(&base)?],
            TrustMode::Auto => {
                let mut items = vec![self.with_trust_source(
                    &base,
                    "official",
                    self.cli
                        .official_trust_list
                        .clone()
                        .unwrap_or_else(|| OFFICIAL_TRUST_LIST_URL.to_string()),
                )?];

                if self.cli.itl_trust_list.is_some() {
                    items.push(self.with_trust_source(
                        &base,
                        "itl",
                        self.cli.itl_trust_list.clone().unwrap_or_default(),
                    )?);
                }

                if !self.cli.test_cert.is_empty() || self.cli.trust_anchors.is_some() {
                    items.push(self.with_custom_trust(&base)?);
                }

                items.push(TrustScenario {
                    label: "none".to_string(),
                    source: "validation_only".to_string(),
                    settings: base.with_value("verify.verify_trust", false)?,
                });

                items
            }
        };

        Ok(scenarios)
    }

    fn base_settings(&self) -> Result<Settings> {
        let mut settings = Settings::new();
        if let Some(settings_path) = &self.cli.settings {
            settings = settings
                .with_file(settings_path)
                .with_context(|| format!("failed to load settings {}", settings_path.display()))?;
        }
        Ok(settings)
    }

    fn with_custom_trust(&self, base: &Settings) -> Result<TrustScenario> {
        let mut settings = base.clone().with_value("verify.verify_trust", true)?;

        if let Some(trust_anchors) = &self.cli.trust_anchors {
            settings = settings.with_value("trust.trust_anchors", read_resource(trust_anchors)?)?;
        }

        if let Some(allowed_list) = &self.cli.allowed_list {
            settings = settings.with_value("trust.allowed_list", read_resource(allowed_list)?)?;
        }

        if let Some(trust_config) = &self.cli.trust_config {
            settings = settings.with_value("trust.trust_config", read_resource(trust_config)?)?;
        }

        if !self.cli.test_cert.is_empty() {
            let bundle = self
                .cli
                .test_cert
                .iter()
                .map(|resource| read_resource(resource))
                .collect::<Result<Vec<_>>>()?
                .join("\n");
            settings = settings.with_value("trust.user_anchors", bundle)?;
        }

        Ok(TrustScenario {
            label: "custom".to_string(),
            source: "custom_or_test".to_string(),
            settings,
        })
    }

    fn with_trust_source(
        &self,
        base: &Settings,
        label: &str,
        resource: String,
    ) -> Result<TrustScenario> {
        let settings = base
            .clone()
            .with_value("verify.verify_trust", true)?
            .with_value("trust.trust_anchors", read_resource(&resource)?)?;

        Ok(TrustScenario {
            label: label.to_string(),
            source: label.to_string(),
            settings,
        })
    }
}

fn load_profiles(cli: &Cli) -> Result<Vec<AssetProfile>> {
    let mut profiles = cli
        .profile
        .iter()
        .map(|profile| {
            profile
                .parse::<BuiltInProfile>()
                .map(built_in_profile)
                .map_err(anyhow::Error::msg)
        })
        .collect::<Result<Vec<_>>>()?;

    for path in &cli.profile_files {
        profiles.push(load_profile_file(path)?);
    }

    Ok(profiles)
}

fn expand_inputs(inputs: &[String]) -> Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();

    for input in inputs {
        if has_glob_pattern(input) {
            let mut matched = false;
            for path in glob(input).with_context(|| format!("invalid glob pattern '{input}'"))? {
                let path = path.with_context(|| format!("invalid match for pattern '{input}'"))?;
                expanded.push(path);
                matched = true;
            }
            if !matched {
                bail!("pattern '{input}' did not match any files");
            }
        } else {
            expanded.push(PathBuf::from(input));
        }
    }

    Ok(expanded)
}

fn has_glob_pattern(input: &str) -> bool {
    input.contains('*') || input.contains('?') || input.contains('[')
}

fn detect_input_type(path: &Path) -> Result<InputType> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("c2pa") => Ok(InputType::SidecarManifest),
        Some("json") => {
            let value = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let json: Value = serde_json::from_str(&value)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            if json.get("schema").and_then(Value::as_str) == Some("crjson") {
                Ok(InputType::CrJson)
            } else {
                Ok(InputType::Asset)
            }
        }
        _ => Ok(InputType::Asset),
    }
}

fn input_descriptor(path: &Path, input_type: InputType) -> Result<InputDescriptor> {
    let resolved = path
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", path.display()))?;

    Ok(InputDescriptor {
        original: path.display().to_string(),
        resolved_path: resolved.display().to_string(),
        detected_format: format_from_path(path)
            .unwrap_or_else(|| "application/octet-stream".to_string()),
        input_type,
    })
}

fn collect_statuses(reader: &Reader) -> Vec<StatusRecord> {
    reader
        .validation_status()
        .unwrap_or(&[])
        .iter()
        .map(|status| StatusRecord {
            code: status.code().to_string(),
            url: status.url().map(ToOwned::to_owned),
            explanation: status.explanation().map(ToOwned::to_owned),
            kind: format!("{:?}", status.kind()).to_lowercase(),
        })
        .collect()
}

fn manifest_record(manifest: &Manifest) -> Result<ManifestRecord> {
    Ok(ManifestRecord {
        label: manifest.label().map(ToOwned::to_owned),
        title: manifest.title().map(ToOwned::to_owned),
        format: manifest.format().map(ToOwned::to_owned),
        claim_generator: manifest.claim_generator().map(ToOwned::to_owned),
        signature: manifest.signature_info().map(|signature| SignatureRecord {
            alg: signature
                .alg
                .as_ref()
                .map(|alg| format!("{alg:?}").to_lowercase()),
            issuer: signature.issuer.clone(),
            common_name: signature.common_name.clone(),
            serial_number: signature.cert_serial_number.clone(),
            time: signature.time.clone(),
            revoked: signature.revocation_status,
        }),
        ingredients: manifest
            .ingredients()
            .iter()
            .map(|ingredient| IngredientRecord {
                title: ingredient.title().map(ToOwned::to_owned),
                format: ingredient.format().map(ToOwned::to_owned),
                relationship: Some(format!("{:?}", ingredient.relationship())),
                active_manifest: ingredient.active_manifest().map(ToOwned::to_owned),
            })
            .collect(),
        assertions: manifest
            .assertions()
            .iter()
            .map(|assertion| AssertionRecord {
                label: assertion.label().to_string(),
                instance: assertion.instance(),
                kind: format!("{:?}", assertion.kind()).to_lowercase(),
            })
            .collect(),
    })
}

fn trust_classification(state: ValidationState) -> &'static str {
    match state {
        ValidationState::Trusted => "trusted",
        ValidationState::Valid => "valid_untrusted",
        ValidationState::Invalid => "invalid",
    }
}

fn trust_notes(cli: &Cli) -> Vec<String> {
    let mut notes = Vec::new();
    if !cli.test_cert.is_empty() {
        notes.push("custom test certificates were provided for this process only".to_string());
    }
    notes
}

fn read_resource(resource: &str) -> Result<String> {
    if resource.starts_with("http://") || resource.starts_with("https://") {
        reqwest::blocking::get(resource)
            .with_context(|| format!("failed to fetch {resource}"))?
            .error_for_status()
            .with_context(|| format!("request failed for {resource}"))?
            .text()
            .with_context(|| format!("failed to read body from {resource}"))
    } else {
        fs::read_to_string(resource).with_context(|| format!("failed to read {resource}"))
    }
}

fn iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_globs() {
        let inputs = expand_inputs(&["Cargo.toml".to_string()]).unwrap();
        assert_eq!(inputs.len(), 1);
    }

    #[test]
    fn detects_glob_patterns() {
        assert!(has_glob_pattern("*.jpg"));
        assert!(!has_glob_pattern("image.jpg"));
    }

    #[test]
    fn trust_classification_maps_state() {
        assert_eq!(trust_classification(ValidationState::Trusted), "trusted");
        assert_eq!(
            trust_classification(ValidationState::Valid),
            "valid_untrusted"
        );
        assert_eq!(trust_classification(ValidationState::Invalid), "invalid");
    }
}

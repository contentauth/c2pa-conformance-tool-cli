use std::{fs, path::Path};

use anyhow::{Context, Result};
use c2pa::validation_results::ValidationState;
use serde::{Deserialize, Serialize};

use crate::{
    cli::BuiltInProfile,
    report::{AssetReport, ProfileReport},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetProfile {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub require_trusted: bool,
    #[serde(default)]
    pub require_ingredients: bool,
    #[serde(default)]
    pub allowed_formats: Vec<String>,
    #[serde(default)]
    pub required_assertions: Vec<String>,
    #[serde(default)]
    pub forbidden_assertions: Vec<String>,
}

impl AssetProfile {
    pub fn evaluate(&self, report: &AssetReport) -> ProfileReport {
        let mut messages = Vec::new();
        let mut passed = true;

        if self.require_trusted && report.validation_state != ValidationState::Trusted {
            passed = false;
            messages.push("asset is not trusted".to_string());
        }

        if self.require_ingredients && report.ingredient_count == 0 {
            passed = false;
            messages.push("asset does not contain ingredients".to_string());
        }

        if !self.allowed_formats.is_empty()
            && !self
                .allowed_formats
                .iter()
                .any(|format| format.eq_ignore_ascii_case(&report.input.detected_format))
        {
            passed = false;
            messages.push(format!(
                "format '{}' is not permitted by the profile",
                report.input.detected_format
            ));
        }

        for required in &self.required_assertions {
            if !report
                .assertion_labels
                .iter()
                .any(|label| label == required)
            {
                passed = false;
                messages.push(format!("required assertion '{required}' is missing"));
            }
        }

        for forbidden in &self.forbidden_assertions {
            if report
                .assertion_labels
                .iter()
                .any(|label| label == forbidden)
            {
                passed = false;
                messages.push(format!("forbidden assertion '{forbidden}' is present"));
            }
        }

        if passed && messages.is_empty() {
            messages.push("profile checks passed".to_string());
        }

        ProfileReport {
            name: self.name.clone(),
            description: self.description.clone(),
            passed,
            messages,
        }
    }
}

pub fn load_profile_file(path: &Path) -> Result<AssetProfile> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read profile {}", path.display()))?;
    serde_json::from_str(&data)
        .with_context(|| format!("failed to parse profile {}", path.display()))
}

pub fn built_in_profile(name: BuiltInProfile) -> AssetProfile {
    match name {
        BuiltInProfile::Basic => AssetProfile {
            name: name.to_string(),
            description: Some("Requires a parseable C2PA manifest".to_string()),
            require_trusted: false,
            require_ingredients: false,
            allowed_formats: Vec::new(),
            required_assertions: Vec::new(),
            forbidden_assertions: Vec::new(),
        },
        BuiltInProfile::Trusted => AssetProfile {
            name: name.to_string(),
            description: Some("Requires a trusted manifest chain".to_string()),
            require_trusted: true,
            require_ingredients: false,
            allowed_formats: Vec::new(),
            required_assertions: Vec::new(),
            forbidden_assertions: Vec::new(),
        },
        BuiltInProfile::IngredientAware => AssetProfile {
            name: name.to_string(),
            description: Some(
                "Requires at least one ingredient and an actions assertion".to_string(),
            ),
            require_trusted: false,
            require_ingredients: true,
            allowed_formats: Vec::new(),
            required_assertions: vec!["c2pa.actions".to_string()],
            forbidden_assertions: Vec::new(),
        },
    }
}

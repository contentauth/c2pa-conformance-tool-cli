use std::{fmt, path::PathBuf, str::FromStr};

use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, Parser)]
#[command(
    author,
    version,
    about = "Validate C2PA assets, sidecar manifests, and crJSON reports",
    arg_required_else_help = true
)]
pub struct Cli {
    #[arg(
        value_name = "INPUT",
        help = "Files or glob patterns to validate. Supports media assets, .c2pa sidecars, and crJSON reports."
    )]
    pub inputs: Vec<String>,

    #[arg(
        short,
        long,
        value_name = "FILE_OR_DIR",
        help = "Output file path, or directory (auto-named from source file, e.g. photo.jpg → photo.json)"
    )]
    pub output: Option<PathBuf>,

    #[arg(short = 'f', long = "format", value_enum, default_value_t = OutputFormat::Json)]
    pub format: OutputFormat,

    #[arg(long, value_enum, default_value_t = TrustMode::Auto)]
    pub trust_mode: TrustMode,

    #[arg(long, value_name = "FILE_OR_URL")]
    pub official_trust_list: Option<String>,

    #[arg(long, value_name = "FILE_OR_URL")]
    pub itl_trust_list: Option<String>,

    #[arg(long, value_name = "FILE_OR_URL")]
    pub trust_anchors: Option<String>,

    #[arg(long, value_name = "FILE_OR_URL")]
    pub allowed_list: Option<String>,

    #[arg(long, value_name = "FILE_OR_URL")]
    pub trust_config: Option<String>,

    #[arg(long, value_name = "FILE_OR_URL")]
    pub test_cert: Vec<String>,

    #[arg(
        long,
        value_name = "FILE",
        help = "Asset file to validate against when INPUT is a standalone .c2pa manifest"
    )]
    pub asset: Option<PathBuf>,

    #[arg(long, value_name = "PROFILE")]
    pub profile: Vec<String>,

    #[arg(long = "profile-file", value_name = "FILE")]
    pub profile_files: Vec<PathBuf>,

    #[arg(
        long,
        value_name = "FILE",
        help = "Overlay c2pa-rs settings from JSON or TOML"
    )]
    pub settings: Option<PathBuf>,

    #[arg(
        long,
        help = "Fail on warnings/profile failures, not only invalid assets"
    )]
    pub strict: bool,

    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Markdown,
    Html,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
pub enum TrustMode {
    Auto,
    Official,
    Itl,
    Custom,
    None,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltInProfile {
    Basic,
    Trusted,
    IngredientAware,
}

impl fmt::Display for BuiltInProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Basic => "basic",
            Self::Trusted => "trusted",
            Self::IngredientAware => "ingredient-aware",
        };
        f.write_str(value)
    }
}

impl fmt::Display for TrustMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Auto => "auto",
            Self::Official => "official",
            Self::Itl => "itl",
            Self::Custom => "custom",
            Self::None => "none",
        };
        f.write_str(value)
    }
}

impl FromStr for BuiltInProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "basic" => Ok(Self::Basic),
            "trusted" => Ok(Self::Trusted),
            "ingredient-aware" => Ok(Self::IngredientAware),
            other => Err(format!("unknown profile '{other}'")),
        }
    }
}

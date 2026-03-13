/*
Copyright 2026 Adobe. All rights reserved.
This file is licensed to you under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License. You may obtain a copy
of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under
the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
OF ANY KIND, either express or implied. See the License for the specific language
governing permissions and limitations in the License.
*/

#![allow(dead_code)] // test helpers used by different test binaries

use std::path::PathBuf;

/// Path to the testfiles directory (workspace root / testfiles).
/// Integration tests run from the crate dir, so testfiles is at ../../testfiles.
pub fn testfiles_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testfiles")
}

pub fn testfiles_assets_dir() -> PathBuf {
    testfiles_dir().join("assets")
}

pub fn testfiles_profiles_dir() -> PathBuf {
    testfiles_dir().join("profiles")
}

pub fn testfiles_manifests_dir() -> PathBuf {
    testfiles_dir().join("manifests")
}

pub fn testfile_asset_jpg() -> PathBuf {
    testfiles_dir().join("assets/PXL_20260208_202351558.jpg")
}

/// Second PXL asset (for multi-asset and glob tests).
pub fn testfile_asset_jpg_second() -> PathBuf {
    testfiles_dir().join("assets/PXL_20250818_155024632~4.jpg")
}

/// Glob pattern that matches both PXL*.jpg assets (relative to workspace root).
pub fn glob_pxl_jpg() -> String {
    testfiles_dir()
        .join("assets/PXL*.jpg")
        .display()
        .to_string()
}

pub fn testfile_asset_png() -> PathBuf {
    testfiles_dir().join("assets/ChatGPT_Image.png")
}

pub fn testfile_asset_mp4() -> PathBuf {
    testfiles_dir().join("assets/manifest_tcID_112.mp4")
}

pub fn testfile_asset_pdf() -> PathBuf {
    testfiles_dir().join("assets/adobe-20240110-single_manifest_store.pdf")
}

pub fn testfile_asset_getty_jpg() -> PathBuf {
    testfiles_dir().join("assets/gettyimages-1500448395-612x612.jpg")
}

/// Sidecar .c2pa manifest (manifest only, no source asset).
pub fn testfile_manifest_data_c2pa() -> PathBuf {
    testfiles_dir().join("manifests/manifest_data.c2pa")
}

pub fn testfile_cloud_manifest_c2pa() -> PathBuf {
    testfiles_dir().join("manifests/cloud_manifest.c2pa")
}

/// Path to the crJSON schema for validating output JSON (draft 2020-12).
pub fn crjson_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crJSON-docs/crJSON-schema.json")
}

pub fn testfile_crjson_valid() -> PathBuf {
    testfiles_dir().join("crjson/valid_minimal.json")
}

pub fn testfile_crjson_invalid_schema() -> PathBuf {
    testfiles_dir().join("crjson/invalid_missing_schema.json")
}

pub fn testfile_crjson_invalid_schema_version() -> PathBuf {
    testfiles_dir().join("crjson/invalid_schema_version_missing.json")
}

pub fn testfile_crjson_invalid_no_results() -> PathBuf {
    testfiles_dir().join("crjson/invalid_no_results_array.json")
}

pub fn testfile_profile_real_media() -> PathBuf {
    testfiles_profiles_dir().join("real-media_profile.yml")
}

pub fn testfile_profile_real_life_capture() -> PathBuf {
    testfiles_profiles_dir().join("real-life-capture_profile.yml")
}

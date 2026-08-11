use crate::utils::{
    exe_has_signature, get_exe_company_name, get_exe_description, get_exe_product_name,
    get_exe_product_version, get_native_arch, get_program_arch, has_icon_in_program,
    is_gui_program, name_similarity,
};
use serde::Serialize;
use std::path::Path;
use windows::Win32::System::SystemInformation::{
    PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64, PROCESSOR_ARCHITECTURE_INTEL,
};

pub const FEATURE_SCHEMA_VERSION: u32 = 1;
pub const FEATURE_COUNT: usize = 30;
pub const FEATURE_NAMES: [&str; FEATURE_COUNT] = [
    "name_root_similarity",
    "name_parent_similarity",
    "is_root_executable",
    "relative_depth",
    "is_launch_path",
    "launch_path_depth",
    "is_gui",
    "gui_known",
    "has_icon",
    "has_description",
    "description_known",
    "has_product_name",
    "has_company_name",
    "has_file_version",
    "is_signed",
    "signature_known",
    "arch_native",
    "arch_compatible",
    "arch_known",
    "file_size_log2",
    "size_rank_percentile",
    "candidate_count_log2",
    "name_has_launcher",
    "name_has_uninstaller",
    "name_has_installer",
    "name_has_updater",
    "name_has_service",
    "name_has_helper",
    "name_has_diagnostic",
    "name_has_plugin",
];

const LAUNCH_DIR_NAMES: &[&str] = &[
    "bin",
    "program",
    "executables",
    "x86",
    "x64",
    "amd64",
    "arm64",
    "win32",
    "win64",
    "32bit",
    "64bit",
];

#[derive(Clone, Debug, Serialize)]
pub struct ExecutableFeatures {
    pub name_root_similarity: f64,
    pub name_parent_similarity: f64,
    pub is_root_executable: bool,
    pub relative_depth: u32,
    pub is_launch_path: bool,
    pub launch_path_depth: u32,
    pub is_gui: bool,
    pub gui_known: bool,
    pub has_icon: bool,
    pub has_description: bool,
    pub description_known: bool,
    pub has_product_name: bool,
    pub has_company_name: bool,
    pub has_file_version: bool,
    pub is_signed: bool,
    pub signature_known: bool,
    pub arch_native: bool,
    pub arch_compatible: bool,
    pub arch_known: bool,
    pub file_size_log2: f64,
    pub size_rank_percentile: f64,
    pub candidate_count_log2: f64,
    pub name_has_launcher: bool,
    pub name_has_uninstaller: bool,
    pub name_has_installer: bool,
    pub name_has_updater: bool,
    pub name_has_service: bool,
    pub name_has_helper: bool,
    pub name_has_diagnostic: bool,
    pub name_has_plugin: bool,
}

impl ExecutableFeatures {
    pub fn extract(app_root: &Path, candidate: &Path, candidate_count: usize) -> Self {
        let stem = candidate
            .file_stem()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let parent_name = candidate
            .parent()
            .and_then(|parent| parent.file_name())
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let root_name = app_root
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let relative_depth = candidate
            .parent()
            .and_then(|parent| parent.strip_prefix(app_root).ok())
            .map(|relative| relative.components().count() as u32)
            .unwrap_or_default();
        let launch_path_depth = candidate
            .parent()
            .and_then(|parent| parent.strip_prefix(app_root).ok())
            .map(|relative| {
                relative
                    .components()
                    .filter(|component| {
                        let name = component.as_os_str().to_string_lossy().to_ascii_lowercase();
                        LAUNCH_DIR_NAMES.contains(&name.as_str())
                    })
                    .count() as u32
            })
            .unwrap_or_default();

        let (is_gui, gui_known) = is_gui_program(candidate)
            .map(|value| (value, true))
            .unwrap_or((false, false));
        let description = get_exe_description(candidate);
        let description_known = description.is_ok();
        let has_description = description
            .ok()
            .flatten()
            .is_some_and(|value| !value.trim().is_empty());
        let has_product_name = get_exe_product_name(candidate)
            .ok()
            .flatten()
            .is_some_and(|value| !value.trim().is_empty());
        let has_company_name = get_exe_company_name(candidate)
            .ok()
            .flatten()
            .is_some_and(|value| !value.trim().is_empty());
        let has_file_version = get_exe_product_version(candidate)
            .ok()
            .flatten()
            .is_some_and(|value| !value.trim().is_empty());
        let (is_signed, signature_known) = exe_has_signature(candidate)
            .map(|value| (value, true))
            .unwrap_or((false, false));
        let (arch_native, arch_compatible, arch_known) = match get_program_arch(candidate) {
            Ok(program_arch) => {
                let (native, compatible) =
                    architecture_compatibility(program_arch, get_native_arch());
                (native, compatible, true)
            }
            Err(_) => (false, false, false),
        };
        let file_size_log2 = candidate
            .metadata()
            .map(|metadata| (metadata.len() as f64 + 1.0).log2())
            .unwrap_or_default();
        let normalized_stem = normalize_token(&stem);

        Self {
            name_root_similarity: name_similarity(&stem, &root_name) as f64,
            name_parent_similarity: name_similarity(&stem, &parent_name) as f64,
            is_root_executable: relative_depth == 0,
            relative_depth,
            is_launch_path: launch_path_depth > 0,
            launch_path_depth,
            is_gui,
            gui_known,
            has_icon: has_icon_in_program(candidate),
            has_description,
            description_known,
            has_product_name,
            has_company_name,
            has_file_version,
            is_signed,
            signature_known,
            arch_native,
            arch_compatible,
            arch_known,
            file_size_log2,
            size_rank_percentile: 0.0,
            candidate_count_log2: (candidate_count as f64 + 1.0).log2(),
            name_has_launcher: contains_any(&normalized_stem, &["launcher", "start"]),
            name_has_uninstaller: contains_any(&normalized_stem, &["uninstall", "unins"]),
            name_has_installer: contains_any(&normalized_stem, &["setup", "installer"]),
            name_has_updater: contains_any(&normalized_stem, &["update", "updater", "upgrade"]),
            name_has_service: contains_any(&normalized_stem, &["service", "agent", "guard"]),
            name_has_helper: contains_any(
                &normalized_stem,
                &["helper", "host", "broker", "worker"],
            ),
            name_has_diagnostic: contains_any(
                &normalized_stem,
                &["diagnostic", "test", "repair", "devcon"],
            ),
            name_has_plugin: contains_any(&normalized_stem, &["plugin", "devtool"]),
        }
    }

    pub fn model_input(&self) -> [f64; FEATURE_COUNT] {
        [
            self.name_root_similarity,
            self.name_parent_similarity,
            self.is_root_executable as u8 as f64,
            self.relative_depth as f64,
            self.is_launch_path as u8 as f64,
            self.launch_path_depth as f64,
            self.is_gui as u8 as f64,
            self.gui_known as u8 as f64,
            self.has_icon as u8 as f64,
            self.has_description as u8 as f64,
            self.description_known as u8 as f64,
            self.has_product_name as u8 as f64,
            self.has_company_name as u8 as f64,
            self.has_file_version as u8 as f64,
            self.is_signed as u8 as f64,
            self.signature_known as u8 as f64,
            self.arch_native as u8 as f64,
            self.arch_compatible as u8 as f64,
            self.arch_known as u8 as f64,
            self.file_size_log2,
            self.size_rank_percentile,
            self.candidate_count_log2,
            self.name_has_launcher as u8 as f64,
            self.name_has_uninstaller as u8 as f64,
            self.name_has_installer as u8 as f64,
            self.name_has_updater as u8 as f64,
            self.name_has_service as u8 as f64,
            self.name_has_helper as u8 as f64,
            self.name_has_diagnostic as u8 as f64,
            self.name_has_plugin as u8 as f64,
        ]
    }
}

pub fn extract_candidate_features(
    app_root: &Path,
    candidates: &[std::path::PathBuf],
) -> Vec<(std::path::PathBuf, ExecutableFeatures)> {
    let mut sizes = candidates
        .iter()
        .map(|candidate| {
            candidate
                .metadata()
                .map(|value| value.len())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    sizes.sort_unstable();
    let denominator = sizes.len().saturating_sub(1).max(1) as f64;

    candidates
        .iter()
        .map(|candidate| {
            let size = candidate
                .metadata()
                .map(|value| value.len())
                .unwrap_or_default();
            let rank = sizes.partition_point(|value| *value < size) as f64 / denominator;
            let mut features = ExecutableFeatures::extract(app_root, candidate, candidates.len());
            features.size_rank_percentile = rank;
            (candidate.clone(), features)
        })
        .collect()
}

pub fn metadata_values(
    candidate: &Path,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    (
        get_exe_product_name(candidate).ok().flatten(),
        get_exe_description(candidate).ok().flatten(),
        get_exe_company_name(candidate).ok().flatten(),
        get_exe_product_version(candidate).ok().flatten(),
    )
}

fn normalize_token(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn contains_any(value: &str, tokens: &[&str]) -> bool {
    tokens.iter().any(|token| value.contains(token))
}

fn architecture_compatibility(program_arch: u16, system_arch: u16) -> (bool, bool) {
    let native = match program_arch {
        0x014c => system_arch == PROCESSOR_ARCHITECTURE_INTEL.0,
        0x8664 => system_arch == PROCESSOR_ARCHITECTURE_AMD64.0,
        0xAA64 => system_arch == PROCESSOR_ARCHITECTURE_ARM64.0,
        _ => false,
    };
    let compatible = native
        || (program_arch == 0x014c
            && (system_arch == PROCESSOR_ARCHITECTURE_AMD64.0
                || system_arch == PROCESSOR_ARCHITECTURE_ARM64.0));
    (native, compatible)
}

#[cfg(test)]
mod tests {
    use super::{
        ExecutableFeatures, FEATURE_COUNT, FEATURE_NAMES, architecture_compatibility,
        extract_candidate_features,
    };
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn feature_schema_and_model_vector_have_the_same_fixed_order() {
        assert_eq!(FEATURE_NAMES.len(), FEATURE_COUNT);
        let temp = TempDir::new().unwrap();
        let candidate = temp.path().join("App.exe");
        File::create(&candidate).unwrap();
        let features = ExecutableFeatures::extract(temp.path(), &candidate, 1);
        assert_eq!(features.model_input().len(), FEATURE_COUNT);
        assert_eq!(FEATURE_NAMES[0], "name_root_similarity");
        assert_eq!(FEATURE_NAMES[FEATURE_COUNT - 1], "name_has_plugin");
    }

    #[test]
    fn size_percentile_is_relative_to_the_app_candidates() {
        let temp = TempDir::new().unwrap();
        let small = temp.path().join("small.exe");
        let large = temp.path().join("large.exe");
        File::create(&small).unwrap();
        File::create(&large).unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&large)
            .unwrap()
            .set_len(1024)
            .unwrap();
        let values = extract_candidate_features(temp.path(), &[small, large]);
        assert!(values[0].1.size_rank_percentile <= values[1].1.size_rank_percentile);
        assert_eq!(values[1].1.candidate_count_log2, (3.0_f64).log2());
    }

    #[test]
    fn pe_machine_codes_are_compared_with_windows_architecture_codes() {
        assert_eq!(architecture_compatibility(0x8664, 9), (true, true));
        assert_eq!(architecture_compatibility(0xAA64, 9), (false, false));
        assert_eq!(architecture_compatibility(0x014c, 9), (false, true));
    }
}

use crate::DEBUG;
use crate::config::ConfigInfo;
use crate::console::{ConsoleType, write_console};
use crate::utils::{
    exe_has_signature, get_exe_description, get_native_arch, get_program_arch, has_icon_in_program,
    is_gui_program, name_similarity,
};
use rust_i18n::t;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use windows::Win32::System::SystemInformation::{
    PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64, PROCESSOR_ARCHITECTURE_INTEL,
};

pub(crate) fn select_main_executable(
    app_root_path: &Path,
    candidates: &[PathBuf],
    config_info: Option<&ConfigInfo>,
    score_ratio: f32,
) -> Option<(PathBuf, PathBuf)> {
    const CONFIG_MATCH_SCORE: i32 = 100;
    const NAME_APP_MAX_SCORE: i32 = 40;
    const GUI_SCORE: i32 = 50;
    const ICON_SCORE: i32 = 40;
    const DESCRIPTION_SCORE: i32 = 30;
    const NATIVE_ARCH_SCORE: i32 = 45;
    const X86_COMPATIBLE_ARCH_SCORE: i32 = 35;
    const SIGNATURE_SCORE: i32 = 60;
    const MAX_SIZE_SCORE: i32 = 30;
    const MIN_NAME_SIMILARITY: f32 = 0.70;
    const MAX_HEURISTIC_SCORE: i32 = NAME_APP_MAX_SCORE
        + GUI_SCORE
        + ICON_SCORE
        + DESCRIPTION_SCORE
        + NATIVE_ARCH_SCORE
        + SIGNATURE_SCORE
        + MAX_SIZE_SCORE;

    let mut best_candidate = None;
    let mut best_score = 0;
    let system_arch_code = get_native_arch();

    for file_path in candidates {
        if !file_path.is_file()
            || !file_path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        {
            continue;
        }

        if is_ignored_by_config(file_path, config_info) {
            if DEBUG.load(Ordering::Relaxed) {
                write_console(
                    ConsoleType::Debug,
                    &t!("scan.ignore_in_config", path = file_path.display()),
                );
            }
            continue;
        }

        let mut score = 0;
        let mut breakdown = Vec::new();
        let explicit_config_match = matches_config_shortcut(file_path, config_info);
        if explicit_config_match {
            score += CONFIG_MATCH_SCORE;
            breakdown.push(("config_match", CONFIG_MATCH_SCORE));
        }

        if let Some(file_stem) = file_path.file_stem().and_then(|stem| stem.to_str()) {
            let parent_similarity = file_path
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .map_or(0.0, |name| name_similarity(file_stem, name));
            let root_similarity = app_root_path
                .file_name()
                .and_then(|name| name.to_str())
                .map_or(0.0, |name| name_similarity(file_stem, name));
            let similarity = parent_similarity.max(root_similarity);
            if similarity >= MIN_NAME_SIMILARITY {
                let name_score = (similarity * NAME_APP_MAX_SCORE as f32).round() as i32;
                score += name_score;
                breakdown.push(("name_app_match", name_score));
            }
        }

        if !explicit_config_match {
            let penalty = automatic_executable_role_penalty(file_path);
            if penalty != 0 {
                score += penalty;
                breakdown.push(("non_entry_role", penalty));
            }
        }

        if let Ok(is_gui) = is_gui_program(file_path) {
            let gui_score = if is_gui { GUI_SCORE } else { -30 };
            score += gui_score;
            breakdown.push((if is_gui { "gui" } else { "gui_penalty" }, gui_score));
        }
        if has_icon_in_program(file_path) {
            score += ICON_SCORE;
            breakdown.push(("icon", ICON_SCORE));
        }
        if let Ok(Some(_)) = get_exe_description(file_path) {
            score += DESCRIPTION_SCORE;
            breakdown.push(("description", DESCRIPTION_SCORE));
        }
        if let Ok(program_arch_code) = get_program_arch(file_path) {
            let arch_score = match (program_arch_code, system_arch_code) {
                (0x014c, arch) if arch == PROCESSOR_ARCHITECTURE_INTEL.0 => NATIVE_ARCH_SCORE,
                (0x8664, arch) if arch == PROCESSOR_ARCHITECTURE_AMD64.0 => NATIVE_ARCH_SCORE,
                (0xAA64, arch) if arch == PROCESSOR_ARCHITECTURE_ARM64.0 => NATIVE_ARCH_SCORE,
                (0x014c, arch) if arch == PROCESSOR_ARCHITECTURE_AMD64.0 => {
                    X86_COMPATIBLE_ARCH_SCORE
                }
                _ => 0,
            };
            if arch_score > 0 {
                score += arch_score;
                breakdown.push(("arch", arch_score));
            }
        }
        if exe_has_signature(file_path).unwrap_or(false) {
            score += SIGNATURE_SCORE;
            breakdown.push(("signature", SIGNATURE_SCORE));
        }
        if let Ok(metadata) = file_path.metadata() {
            let size_score = ((metadata.len() / (1024 * 1024)) as i32).min(MAX_SIZE_SCORE);
            if size_score > 0 {
                score += size_score;
                breakdown.push(("size", size_score));
            }
        }

        if DEBUG.load(Ordering::Relaxed) {
            let details = breakdown
                .iter()
                .map(|(name, delta)| format!("{}{:+}", name, delta))
                .collect::<Vec<_>>()
                .join(", ");
            write_console(
                ConsoleType::Debug,
                &t!(
                    "scan.score_calculation",
                    file = file_path.file_name().unwrap().to_string_lossy(),
                    score = score,
                    details = details
                ),
            );
        }

        if score <= (MAX_HEURISTIC_SCORE as f32 * score_ratio).round() as i32 {
            continue;
        }
        if score > best_score {
            best_score = score;
            best_candidate = Some((app_root_path.to_path_buf(), file_path.clone()));
        }
    }

    best_candidate
}

fn is_ignored_by_config(file_path: &Path, config_info: Option<&ConfigInfo>) -> bool {
    let Some(config) = config_info else {
        return false;
    };
    let Some(file_name) = file_path
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
    else {
        return false;
    };
    config.ignore.iter().any(|keyword| {
        if let Ok(keyword_path) = PathBuf::from(keyword).canonicalize() {
            file_path
                .canonicalize()
                .is_ok_and(|path| path == keyword_path)
        } else {
            file_name.contains(keyword.to_lowercase().as_str())
        }
    })
}

fn matches_config_shortcut(file_path: &Path, config_info: Option<&ConfigInfo>) -> bool {
    config_info.is_some_and(|config| {
        config.shortcut.iter().any(|shortcut| {
            let configured = PathBuf::from(&shortcut.exec);
            let expected = if configured.is_absolute() {
                configured
            } else {
                file_path.parent().unwrap().join(configured)
            };
            expected == file_path
        })
    })
}

pub(crate) fn automatic_executable_role_penalty(file_path: &Path) -> i32 {
    let Some(stem) = file_path.file_stem().and_then(|stem| stem.to_str()) else {
        return 0;
    };
    let stem = stem.to_ascii_lowercase();
    const STRONG_NEGATIVE: &[&str] = &[
        "uninstall",
        "unins",
        "setup",
        "installer",
        "updater",
        "update",
        "upgrade",
        "maintenance",
        "crashpad",
        "cleanup",
    ];
    if STRONG_NEGATIVE.iter().any(|token| stem.contains(token)) {
        return -180;
    }
    const SUPPORT_PROCESS: &[&str] = &[
        "service",
        "helper",
        "diagnostic",
        "broker",
        "integrator",
        "monitor",
        "worker",
        "elevate",
        "devcon",
        "regdll",
        "repair",
        "agent",
        "guard",
        "host",
        "test",
        "grhlp",
        "plugin",
        "devtool",
    ];
    if SUPPORT_PROCESS.iter().any(|token| stem.contains(token)) {
        return -100;
    }
    0
}

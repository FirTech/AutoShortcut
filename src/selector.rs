use crate::DEBUG;
use crate::config::ConfigInfo;
use crate::console::{ConsoleType, write_console};
use crate::features::{ExecutableFeatures, extract_candidate_features};
use crate::model;
use rust_i18n::t;
use std::cmp::Ordering as CmpOrdering;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

const CONFIG_MATCH_SCORE: i32 = 100;
const NAME_APP_MAX_SCORE: i32 = 40;
const GUI_SCORE: i32 = 50;
const ICON_SCORE: i32 = 40;
const DESCRIPTION_SCORE: i32 = 30;
const NATIVE_ARCH_SCORE: i32 = 45;
const X86_COMPATIBLE_ARCH_SCORE: i32 = 35;
const SIGNATURE_SCORE: i32 = 60;
const MAX_SIZE_SCORE: i32 = 30;
const MIN_NAME_SIMILARITY: f64 = 0.70;
const MAX_HEURISTIC_SCORE: i32 = NAME_APP_MAX_SCORE
    + GUI_SCORE
    + ICON_SCORE
    + DESCRIPTION_SCORE
    + NATIVE_ARCH_SCORE
    + SIGNATURE_SCORE
    + MAX_SIZE_SCORE;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionSource {
    ExplicitConfig,
    SingleCandidate,
    XGBoost,
    RuleFallback,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SelectionResult {
    pub(crate) app_root: PathBuf,
    pub(crate) executable: PathBuf,
    pub(crate) source: SelectionSource,
    pub(crate) model_probability: Option<f64>,
    pub(crate) model_margin: Option<f64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuleCandidateDiagnostic {
    pub(crate) executable: PathBuf,
    pub(crate) score: i32,
    pub(crate) rank: usize,
    pub(crate) selected: bool,
}

pub(crate) fn select_main_executable(
    app_root_path: &Path,
    candidates: &[PathBuf],
    config_info: Option<&ConfigInfo>,
    score_ratio: f32,
) -> Option<SelectionResult> {
    select_main_executable_with_predictor(
        app_root_path,
        candidates,
        config_info,
        score_ratio,
        &model::predict_main_probability,
        model::MODEL_MIN_TOP_PROBABILITY,
        model::MODEL_MIN_PROBABILITY_MARGIN,
    )
}

fn select_main_executable_with_predictor(
    app_root_path: &Path,
    candidates: &[PathBuf],
    config_info: Option<&ConfigInfo>,
    score_ratio: f32,
    predictor: &dyn Fn(&ExecutableFeatures) -> Option<f64>,
    min_top_probability: f64,
    min_probability_margin: f64,
) -> Option<SelectionResult> {
    let eligible = eligible_candidates(candidates, config_info);
    if let Some(executable) = eligible
        .iter()
        .find(|candidate| matches_config_shortcut(candidate, app_root_path, config_info))
    {
        return Some(selection_result(
            app_root_path,
            executable,
            SelectionSource::ExplicitConfig,
            None,
            None,
        ));
    }

    let featured = extract_candidate_features(app_root_path, &eligible);
    let safe_candidates = featured
        .iter()
        .filter(|(candidate, _)| !is_hard_negative(candidate))
        .collect::<Vec<_>>();

    if let [single] = safe_candidates.as_slice() {
        return Some(selection_result(
            app_root_path,
            &single.0,
            SelectionSource::SingleCandidate,
            None,
            None,
        ));
    }

    let mut predictions = safe_candidates
        .iter()
        .filter_map(|(candidate, features)| {
            predictor(features).map(|probability| ((*candidate).clone(), probability))
        })
        .collect::<Vec<_>>();
    if predictions.len() == safe_candidates.len() && predictions.len() >= 2 {
        predictions.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(CmpOrdering::Equal)
                .then_with(|| left.0.cmp(&right.0))
        });
        let top_probability = predictions[0].1;
        let margin = top_probability - predictions[1].1;
        if top_probability >= min_top_probability && margin >= min_probability_margin {
            return Some(selection_result(
                app_root_path,
                &predictions[0].0,
                SelectionSource::XGBoost,
                Some(top_probability),
                Some(margin),
            ));
        }
    }

    rule_candidate_diagnostics_from_features(app_root_path, &featured, config_info, score_ratio)
        .into_iter()
        .find(|diagnostic| diagnostic.selected)
        .map(|diagnostic| {
            selection_result(
                app_root_path,
                &diagnostic.executable,
                SelectionSource::RuleFallback,
                predictions.first().map(|prediction| prediction.1),
                if predictions.len() >= 2 {
                    Some(predictions[0].1 - predictions[1].1)
                } else {
                    None
                },
            )
        })
}

pub(crate) fn select_main_executable_by_rules(
    app_root_path: &Path,
    candidates: &[PathBuf],
    config_info: Option<&ConfigInfo>,
    score_ratio: f32,
) -> Option<RuleCandidateDiagnostic> {
    rule_candidate_diagnostics(app_root_path, candidates, config_info, score_ratio)
        .into_iter()
        .find(|diagnostic| diagnostic.selected)
}

pub(crate) fn rule_candidate_diagnostics(
    app_root_path: &Path,
    candidates: &[PathBuf],
    config_info: Option<&ConfigInfo>,
    score_ratio: f32,
) -> Vec<RuleCandidateDiagnostic> {
    let eligible = eligible_candidates(candidates, config_info);
    let featured = extract_candidate_features(app_root_path, &eligible);
    rule_candidate_diagnostics_from_features(app_root_path, &featured, config_info, score_ratio)
}

pub(crate) fn rule_candidate_diagnostics_from_features(
    app_root_path: &Path,
    featured: &[(PathBuf, ExecutableFeatures)],
    config_info: Option<&ConfigInfo>,
    score_ratio: f32,
) -> Vec<RuleCandidateDiagnostic> {
    let threshold = (MAX_HEURISTIC_SCORE as f32 * score_ratio).round() as i32;
    let mut diagnostics = featured
        .iter()
        .map(|(candidate, features)| {
            let (score, breakdown) = score_candidate(
                candidate,
                features,
                matches_config_shortcut(candidate, app_root_path, config_info),
            );
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
                        file = candidate.file_name().unwrap().to_string_lossy(),
                        score = score,
                        details = details
                    ),
                );
            }
            RuleCandidateDiagnostic {
                executable: candidate.clone(),
                score,
                rank: 0,
                selected: false,
            }
        })
        .collect::<Vec<_>>();

    diagnostics.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.executable.cmp(&right.executable))
    });
    for (index, diagnostic) in diagnostics.iter_mut().enumerate() {
        diagnostic.rank = index + 1;
    }
    if diagnostics
        .first()
        .is_some_and(|diagnostic| diagnostic.score > threshold && diagnostic.score > 0)
    {
        diagnostics[0].selected = true;
    }
    diagnostics
}

fn score_candidate(
    file_path: &Path,
    features: &ExecutableFeatures,
    explicit_config_match: bool,
) -> (i32, Vec<(&'static str, i32)>) {
    let mut score = 0;
    let mut breakdown = Vec::new();
    if explicit_config_match {
        score += CONFIG_MATCH_SCORE;
        breakdown.push(("config_match", CONFIG_MATCH_SCORE));
    }

    let similarity = features
        .name_parent_similarity
        .max(features.name_root_similarity);
    if similarity >= MIN_NAME_SIMILARITY {
        let name_score = (similarity * NAME_APP_MAX_SCORE as f64).round() as i32;
        score += name_score;
        breakdown.push(("name_app_match", name_score));
    }

    if !explicit_config_match {
        let penalty = automatic_executable_role_penalty(file_path);
        if penalty != 0 {
            score += penalty;
            breakdown.push(("non_entry_role", penalty));
        }
    }
    if features.gui_known {
        let gui_score = if features.is_gui { GUI_SCORE } else { -30 };
        score += gui_score;
        breakdown.push((
            if features.is_gui {
                "gui"
            } else {
                "gui_penalty"
            },
            gui_score,
        ));
    }
    if features.has_icon {
        score += ICON_SCORE;
        breakdown.push(("icon", ICON_SCORE));
    }
    if features.has_description {
        score += DESCRIPTION_SCORE;
        breakdown.push(("description", DESCRIPTION_SCORE));
    }
    let arch_score = if features.arch_native {
        NATIVE_ARCH_SCORE
    } else if features.arch_compatible {
        X86_COMPATIBLE_ARCH_SCORE
    } else {
        0
    };
    if arch_score > 0 {
        score += arch_score;
        breakdown.push(("arch", arch_score));
    }
    if features.is_signed {
        score += SIGNATURE_SCORE;
        breakdown.push(("signature", SIGNATURE_SCORE));
    }
    let size_score = file_path
        .metadata()
        .map(|metadata| ((metadata.len() / (1024 * 1024)) as i32).min(MAX_SIZE_SCORE))
        .unwrap_or_default();
    if size_score > 0 {
        score += size_score;
        breakdown.push(("size", size_score));
    }
    (score, breakdown)
}

fn selection_result(
    app_root: &Path,
    executable: &Path,
    source: SelectionSource,
    model_probability: Option<f64>,
    model_margin: Option<f64>,
) -> SelectionResult {
    SelectionResult {
        app_root: app_root.to_path_buf(),
        executable: executable.to_path_buf(),
        source,
        model_probability,
        model_margin,
    }
}

fn eligible_candidates(candidates: &[PathBuf], config_info: Option<&ConfigInfo>) -> Vec<PathBuf> {
    candidates
        .iter()
        .filter(|file_path| {
            file_path.is_file()
                && file_path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                && !is_ignored_by_config(file_path, config_info)
        })
        .cloned()
        .collect()
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
    let ignored = config.ignore.iter().any(|keyword| {
        if let Ok(keyword_path) = PathBuf::from(keyword).canonicalize() {
            file_path
                .canonicalize()
                .is_ok_and(|path| path == keyword_path)
        } else {
            file_name.contains(keyword.to_lowercase().as_str())
        }
    });
    if ignored && DEBUG.load(Ordering::Relaxed) {
        write_console(
            ConsoleType::Debug,
            &t!("scan.ignore_in_config", path = file_path.display()),
        );
    }
    ignored
}

fn matches_config_shortcut(
    file_path: &Path,
    app_root_path: &Path,
    config_info: Option<&ConfigInfo>,
) -> bool {
    config_info.is_some_and(|config| {
        config.shortcut.iter().any(|shortcut| {
            let configured = PathBuf::from(&shortcut.exec);
            let expected = if configured.is_absolute() {
                configured
            } else {
                app_root_path.join(configured)
            };
            paths_equal(&expected, file_path)
        })
    })
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn is_hard_negative(file_path: &Path) -> bool {
    automatic_executable_role_penalty(file_path) < 0
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

#[cfg(test)]
mod tests {
    use super::{SelectionSource, select_main_executable_with_predictor};
    use crate::config::{ConfigInfo, Lnk};
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn accepts_a_high_confidence_model_result() {
        use std::cell::Cell;

        let temp = TempDir::new().unwrap();
        let first = temp.path().join("First.exe");
        let second = temp.path().join("Second.exe");
        File::create(&first).unwrap();
        File::create(&second).unwrap();
        let calls = Cell::new(0);
        let result = select_main_executable_with_predictor(
            temp.path(),
            &[first.clone(), second],
            None,
            0.0,
            &|_| {
                let call = calls.get();
                calls.set(call + 1);
                Some(if call == 0 { 0.99 } else { 0.10 })
            },
            0.90,
            0.20,
        )
        .unwrap();
        assert_eq!(result.source, SelectionSource::XGBoost);
        assert!(result.model_probability.unwrap() >= 0.99);
    }

    #[test]
    fn insufficient_probability_margin_falls_back_to_rules() {
        use std::cell::Cell;

        let temp = TempDir::new().unwrap();
        let root = temp.path().join("Product");
        std::fs::create_dir(&root).unwrap();
        let main = root.join("Product.exe");
        let other = root.join("Other.exe");
        File::create(&main).unwrap();
        File::create(&other).unwrap();
        let calls = Cell::new(0);
        let result = select_main_executable_with_predictor(
            &root,
            &[main.clone(), other],
            None,
            0.0,
            &|_| {
                let call = calls.get();
                calls.set(call + 1);
                Some(if call == 0 { 0.95 } else { 0.90 })
            },
            0.80,
            0.10,
        )
        .unwrap();
        assert_eq!(result.source, SelectionSource::RuleFallback);
        assert_eq!(result.executable, main);
    }

    #[test]
    fn unavailable_model_falls_back_to_rules() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("Product");
        std::fs::create_dir(&root).unwrap();
        let main = root.join("Product.exe");
        let other = root.join("Other.exe");
        File::create(&main).unwrap();
        File::create(&other).unwrap();
        let result = select_main_executable_with_predictor(
            &root,
            &[main, other],
            None,
            0.0,
            &|_| None,
            0.90,
            0.20,
        )
        .unwrap();
        assert_eq!(result.source, SelectionSource::RuleFallback);
    }

    #[test]
    fn low_confidence_model_falls_back_to_rules() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("Product");
        std::fs::create_dir(&root).unwrap();
        let main = root.join("Product.exe");
        let other = root.join("Other.exe");
        File::create(&main).unwrap();
        File::create(&other).unwrap();
        let result = select_main_executable_with_predictor(
            &root,
            &[main.clone(), other],
            None,
            0.0,
            &|_| Some(0.60),
            0.90,
            0.20,
        )
        .unwrap();
        assert_eq!(result.source, SelectionSource::RuleFallback);
        assert_eq!(result.executable, main);
    }

    #[test]
    fn explicit_config_has_priority_over_model() {
        let temp = TempDir::new().unwrap();
        let first = temp.path().join("First.exe");
        let second = temp.path().join("Second.exe");
        File::create(&first).unwrap();
        File::create(&second).unwrap();
        let mut config = ConfigInfo::default();
        config.shortcut.push(Lnk::new("Second.exe".to_string()));
        let result = select_main_executable_with_predictor(
            temp.path(),
            &[first, second.clone()],
            Some(&config),
            0.0,
            &|_| Some(0.99),
            0.90,
            0.20,
        )
        .unwrap();
        assert_eq!(result.source, SelectionSource::ExplicitConfig);
        assert_eq!(result.executable, second);
    }

    #[test]
    fn hard_negative_is_never_offered_to_model() {
        let temp = TempDir::new().unwrap();
        let main = temp.path().join("Main.exe");
        let updater = temp.path().join("Updater.exe");
        File::create(&main).unwrap();
        File::create(&updater).unwrap();
        let result = select_main_executable_with_predictor(
            temp.path(),
            &[main.clone(), updater],
            None,
            0.0,
            &|_| Some(0.01),
            0.90,
            0.20,
        )
        .unwrap();
        assert_eq!(result.source, SelectionSource::SingleCandidate);
        assert_eq!(result.executable, main);
    }
}

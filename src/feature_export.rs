use crate::config::ConfigInfo;
use crate::directory::{DirectoryAnalysis, DirectoryRole, analyze_directory_tree};
use crate::features::{
    FEATURE_NAMES, FEATURE_SCHEMA_VERSION, extract_candidate_features, metadata_values,
};
use crate::selector::rule_candidate_diagnostics_from_features;
use anyhow::{Context, Result, anyhow};
use csv::Writer;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

const METADATA_COLUMNS: &[&str] = &[
    "schema_version",
    "sample_id",
    "software_id",
    "app_root_relative",
    "candidate_relative",
    "candidate_name",
    "directory_role",
    "directory_confidence",
    "candidate_count",
    "product_name",
    "file_description",
    "company_name",
    "file_version",
    "rule_score",
    "rule_rank",
    "rule_selected",
    "label_state",
    "label_is_main",
    "label_source",
    "label_note",
];

pub(crate) fn export_features(
    root: &Path,
    output: &Path,
    excluded: &[String],
    config_info: Option<&ConfigInfo>,
    score_ratio: f32,
) -> Result<()> {
    if output.exists() {
        return Err(anyhow!(
            "feature CSV already exists; refusing to overwrite: {}",
            output.display()
        ));
    }
    let analysis = analyze_directory_tree(root, excluded);
    let mut app_roots = Vec::new();
    collect_app_roots(&analysis, &mut app_roots);

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .with_context(|| format!("failed to create feature CSV: {}", output.display()))?;
    let mut writer = Writer::from_writer(file);
    let mut header = METADATA_COLUMNS.to_vec();
    header.extend(FEATURE_NAMES);
    writer.write_record(header)?;

    for app in app_roots {
        let app_relative = relative_path(root, &app.path)?;
        let app_relative_text = normalized_relative(&app_relative);
        let software_id = if app_relative_text.is_empty() {
            app.path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        } else {
            app_relative_text.clone()
        };
        let candidates = extract_candidate_features(&app.path, &app.owned_exes);
        let diagnostics = rule_candidate_diagnostics_from_features(
            &app.path,
            &candidates,
            config_info,
            score_ratio,
        );
        let diagnostic_by_path = diagnostics
            .into_iter()
            .map(|diagnostic| (diagnostic.executable.clone(), diagnostic))
            .collect::<HashMap<_, _>>();
        let candidate_count = candidates.len().to_string();

        for (candidate, features) in candidates {
            let candidate_relative = relative_path(root, &candidate)?;
            let candidate_relative_text = normalized_relative(&candidate_relative);
            let sample_id = format!("{}::{}", software_id, candidate_relative_text);
            let metadata = metadata_values(&candidate);
            let diagnostic = diagnostic_by_path.get(&candidate);
            let mut record = vec![
                FEATURE_SCHEMA_VERSION.to_string(),
                sample_id,
                software_id.clone(),
                app_relative_text.clone(),
                candidate_relative_text,
                candidate
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_default(),
                format!("{:?}", app.role),
                format!("{:?}", app.confidence),
                candidate_count.clone(),
                metadata.0.unwrap_or_default(),
                metadata.1.unwrap_or_default(),
                metadata.2.unwrap_or_default(),
                metadata.3.unwrap_or_default(),
                diagnostic
                    .map(|value| value.score.to_string())
                    .unwrap_or_default(),
                diagnostic
                    .map(|value| value.rank.to_string())
                    .unwrap_or_default(),
                diagnostic
                    .map(|value| value.selected.to_string())
                    .unwrap_or_default(),
                "unreviewed".to_string(),
                String::new(),
                String::new(),
                String::new(),
            ];
            record.extend(features.model_input().iter().map(ToString::to_string));
            writer.write_record(record)?;
        }
    }
    writer.flush()?;
    Ok(())
}

fn collect_app_roots(analysis: &DirectoryAnalysis, output: &mut Vec<DirectoryAnalysis>) {
    if analysis.role == DirectoryRole::AppRoot {
        output.push(analysis.clone());
        return;
    }
    for child in &analysis.children {
        collect_app_roots(child, output);
    }
}

fn relative_path(base: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(base).map(Path::to_path_buf).map_err(|_| {
        anyhow!(
            "refusing to export a path outside the scan root: {}",
            path.display()
        )
    })
}

fn normalized_relative(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::export_features;
    use crate::config::ConfigInfo;
    use std::fs::{self, File};
    use std::path::Path;
    use tempfile::TempDir;

    fn touch(path: &Path) {
        File::create(path).unwrap();
    }

    #[test]
    fn csv_contains_only_relative_paths_and_app_roots() {
        let temp = TempDir::new().unwrap();
        let app = temp.path().join("中文,App");
        fs::create_dir(&app).unwrap();
        touch(&app.join("中文,App.exe"));
        touch(&app.join("中文,App.dll"));
        let collection = temp.path().join("collection");
        fs::create_dir(&collection).unwrap();
        touch(&collection.join("one.exe"));
        touch(&collection.join("two.exe"));
        let output = temp.path().join("features.csv");

        export_features(temp.path(), &output, &[], Some(&ConfigInfo::default()), 0.0).unwrap();
        let text = fs::read_to_string(&output).unwrap();
        assert!(text.contains("app_root_relative"));
        assert!(text.contains("中文,App/中文,App.exe"));
        assert!(!text.contains(temp.path().to_string_lossy().as_ref()));
        assert!(!text.contains("one.exe"));
        assert!(!text.contains("two.exe"));
        assert!(text.contains("rule_score"));
    }

    #[test]
    fn csv_refuses_to_overwrite_existing_file() {
        let temp = TempDir::new().unwrap();
        let output = temp.path().join("existing.csv");
        touch(&output);
        let result = export_features(temp.path(), &output, &[], None, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn ignored_candidates_are_not_exported_or_counted() {
        let temp = TempDir::new().unwrap();
        touch(&temp.path().join("Main.exe"));
        touch(&temp.path().join("Ignored.exe"));
        touch(&temp.path().join("Main.dll"));
        let output = temp.path().join("features.csv");
        export_features(temp.path(), &output, &["Ignored".to_string()], None, 0.0).unwrap();
        let mut reader = csv::Reader::from_path(output).unwrap();
        let headers = reader.headers().unwrap().clone();
        let count_index = headers
            .iter()
            .position(|header| header == "candidate_count")
            .unwrap();
        let name_index = headers
            .iter()
            .position(|header| header == "candidate_name")
            .unwrap();
        let rows = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(&rows[0][count_index], "1");
        assert_eq!(&rows[0][name_index], "Main.exe");
    }
}

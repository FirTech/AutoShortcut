use std::fs;
use std::path::{Path, PathBuf};

const COMPONENT_DIR_NAMES: &[&str] = &[
    "bin",
    "program",
    "executables",
    "x64",
    "win64",
    "lib",
    "data",
    "assets",
    "content",
    "resources",
    "plugins",
    "modules",
    "drivers",
    "x86",
    "amd64",
    "arm64",
    "win32",
    "32bit",
    "64bit",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectoryRole {
    AppRoot,
    ExeCollection,
    Container,
    Mixed,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectoryEvidence {
    DirectExe,
    MultipleDirectExes,
    SupportFiles,
    DocumentationOnly,
    CommonComponentDirectory,
    IndependentChildApplication,
    ComponentChildApplication,
    UnknownFiles,
    Inaccessible,
}

#[derive(Clone, Debug)]
pub(crate) struct DirectoryAnalysis {
    pub(crate) path: PathBuf,
    pub(crate) role: DirectoryRole,
    pub(crate) confidence: Confidence,
    pub(crate) direct_exes: Vec<PathBuf>,
    pub(crate) owned_exes: Vec<PathBuf>,
    pub(crate) children: Vec<DirectoryAnalysis>,
    pub(crate) evidence: Vec<DirectoryEvidence>,
}

impl DirectoryAnalysis {
    pub(crate) fn has_self_app(&self) -> bool {
        self.role == DirectoryRole::AppRoot
    }

    fn contains_applications(&self) -> bool {
        self.role != DirectoryRole::Unknown
            || self
                .children
                .iter()
                .any(DirectoryAnalysis::contains_applications)
    }
}

pub(crate) fn analyze_directory_tree(root: &Path, excluded: &[String]) -> DirectoryAnalysis {
    let matcher = ExclusionMatcher::new(excluded);
    analyze_directory(root, &matcher)
}

fn analyze_directory(path: &Path, excluded: &ExclusionMatcher) -> DirectoryAnalysis {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return inaccessible_analysis(path),
    };

    let mut direct_exes = Vec::new();
    let mut children = Vec::new();
    let mut strong_support_files = 0usize;
    let mut weak_support_files = 0usize;
    let mut documentation_files = 0usize;
    let mut unknown_files = 0usize;

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let entry_path = entry.path();
        if excluded.matches(&entry_path) {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            children.push(analyze_directory(&entry_path, excluded));
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let extension = entry_path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        match extension.as_deref() {
            Some("exe") => direct_exes.push(entry_path),
            Some("dll" | "pak" | "lng") => strong_support_files += 1,
            Some(
                "ini" | "json" | "xml" | "yaml" | "yml" | "dat" | "cfg" | "conf" | "reg" | "key"
                | "cupf",
            ) => weak_support_files += 1,
            Some("ico") => {}
            Some("txt" | "md" | "pdf") if is_documentation_file(&entry_path) => {
                documentation_files += 1
            }
            _ => unknown_files += 1,
        }
    }

    direct_exes.sort();
    children.sort_by(|left, right| left.path.cmp(&right.path));

    let mut evidence = Vec::new();
    if !direct_exes.is_empty() {
        evidence.push(DirectoryEvidence::DirectExe);
    }
    if direct_exes.len() > 1 {
        evidence.push(DirectoryEvidence::MultipleDirectExes);
    }

    let has_support_files = strong_support_files > 0 || weak_support_files >= 2;
    if has_support_files {
        evidence.push(DirectoryEvidence::SupportFiles);
    }
    if documentation_files > 0 && strong_support_files == 0 && weak_support_files == 0 {
        evidence.push(DirectoryEvidence::DocumentationOnly);
    }
    if unknown_files > 0 {
        evidence.push(DirectoryEvidence::UnknownFiles);
    }

    let mut launch_exes = Vec::new();
    let mut independent_children = 0usize;
    let mut has_component_directory = false;
    for child in &children {
        if is_component_directory(&child.path) {
            has_component_directory = true;
            let before = launch_exes.len();
            if is_launch_directory(&child.path) {
                collect_launch_exes(child, &mut launch_exes, 0);
            }
            if launch_exes.len() > before || child.contains_applications() {
                push_unique(&mut evidence, DirectoryEvidence::ComponentChildApplication);
            }
        } else if child.contains_applications() && !is_non_application_child(&child.path) {
            independent_children += 1;
            push_unique(
                &mut evidence,
                DirectoryEvidence::IndependentChildApplication,
            );
        }
    }
    launch_exes.sort();
    launch_exes.dedup();

    if has_component_directory {
        evidence.push(DirectoryEvidence::CommonComponentDirectory);
    }

    let has_matching_direct_exe = direct_exes
        .iter()
        .any(|executable| executable_matches_directory(executable, path));
    let has_self_app = ((!direct_exes.is_empty()
        && (has_support_files || has_component_directory || has_matching_direct_exe))
        || !launch_exes.is_empty())
        && !is_non_launchable_package(path);
    let is_collection = !direct_exes.is_empty()
        && !has_self_app
        && !has_component_directory
        && unknown_files == 0
        && strong_support_files == 0
        && weak_support_files == 0;

    // A strong local application owns its descendant tree by default. Child executables are not
    // independent applications unless the current directory has no application of its own.
    let (role, confidence) = if has_self_app {
        (DirectoryRole::AppRoot, Confidence::High)
    } else if is_collection && independent_children > 0 {
        (DirectoryRole::Mixed, Confidence::High)
    } else if is_collection {
        (DirectoryRole::ExeCollection, Confidence::High)
    } else if independent_children > 0 && direct_exes.is_empty() {
        let confidence = if independent_children > 1 {
            Confidence::High
        } else {
            Confidence::Medium
        };
        (DirectoryRole::Container, confidence)
    } else {
        (DirectoryRole::Unknown, Confidence::Low)
    };

    let owned_exes = match role {
        DirectoryRole::AppRoot => merge_exes(&direct_exes, &launch_exes),
        DirectoryRole::ExeCollection => direct_exes.clone(),
        DirectoryRole::Mixed => direct_exes.clone(),
        DirectoryRole::Container | DirectoryRole::Unknown => Vec::new(),
    };

    DirectoryAnalysis {
        path: path.to_path_buf(),
        role,
        confidence,
        direct_exes,
        owned_exes,
        children,
        evidence,
    }
}

fn inaccessible_analysis(path: &Path) -> DirectoryAnalysis {
    DirectoryAnalysis {
        path: path.to_path_buf(),
        role: DirectoryRole::Unknown,
        confidence: Confidence::Low,
        direct_exes: Vec::new(),
        owned_exes: Vec::new(),
        children: Vec::new(),
        evidence: vec![DirectoryEvidence::Inaccessible],
    }
}

fn collect_launch_exes(
    analysis: &DirectoryAnalysis,
    output: &mut Vec<PathBuf>,
    relative_depth: usize,
) {
    output.extend(analysis.direct_exes.iter().cloned());
    if relative_depth >= 2 {
        return;
    }
    for child in &analysis.children {
        if is_launch_directory(&child.path) {
            collect_launch_exes(child, output, relative_depth + 1);
        }
    }
}

fn merge_exes(direct: &[PathBuf], component: &[PathBuf]) -> Vec<PathBuf> {
    let mut result = direct.to_vec();
    result.extend(component.iter().cloned());
    result.sort();
    result.dedup();
    result
}

fn push_unique(values: &mut Vec<DirectoryEvidence>, value: DirectoryEvidence) {
    if !values.contains(&value) {
        values.push(value);
    }
}

pub(crate) fn is_component_directory(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_string_lossy().to_ascii_lowercase();
        COMPONENT_DIR_NAMES.contains(&name.as_str())
    })
}

fn is_launch_directory(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_string_lossy().to_ascii_lowercase();
        LAUNCH_DIR_NAMES.contains(&name.as_str())
    })
}

fn executable_matches_directory(executable: &Path, directory: &Path) -> bool {
    let Some(executable_name) = executable.file_stem() else {
        return false;
    };
    let Some(directory_name) = directory.file_name() else {
        return false;
    };

    let executable_name = normalize_name(&executable_name.to_string_lossy());
    let directory_name = normalize_name(&directory_name.to_string_lossy());
    if executable_name.is_empty() || directory_name.is_empty() {
        return false;
    }

    executable_name == directory_name
        || (directory_name.len() >= 4 && executable_name.starts_with(&directory_name))
        || (executable_name.len() >= 4 && directory_name.ends_with(&executable_name))
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_non_application_child(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_string_lossy().to_ascii_lowercase();
        [
            "docs",
            "documentation",
            "examples",
            "samples",
            "src",
            "source",
            "test",
            "tests",
            "tools",
        ]
        .contains(&name.as_str())
    })
}

fn is_non_launchable_package(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        let name = name.to_string_lossy().to_ascii_lowercase();
        ["sdk", "runtime", "redistributable", "development"]
            .iter()
            .any(|token| name == *token || name.ends_with(&format!("-{token}")))
    })
}

fn is_documentation_file(path: &Path) -> bool {
    path.file_stem().is_some_and(|stem| {
        let stem = stem.to_string_lossy().to_ascii_lowercase();
        ["readme", "license", "licence", "eula", "changelog"]
            .iter()
            .any(|prefix| stem.starts_with(prefix))
    })
}

struct ExclusionMatcher {
    exact_paths: Vec<PathBuf>,
    exact_names: Vec<String>,
    name_fragments: Vec<String>,
}

impl ExclusionMatcher {
    fn new(excluded: &[String]) -> Self {
        let mut exact_paths = Vec::new();
        let mut exact_names = Vec::new();
        let mut name_fragments = Vec::new();
        for value in excluded {
            if let Some(name) = value.strip_prefix('=') {
                exact_names.push(name.to_ascii_lowercase());
                continue;
            }
            let candidate = PathBuf::from(value);
            if candidate.is_absolute() {
                exact_paths.push(candidate.canonicalize().unwrap_or(candidate));
            } else {
                name_fragments.push(value.to_ascii_lowercase());
            }
        }
        Self {
            exact_paths,
            exact_names,
            name_fragments,
        }
    }

    fn matches(&self, path: &Path) -> bool {
        let exact_match = if self.exact_paths.is_empty() {
            false
        } else {
            let normalized = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            self.exact_paths
                .iter()
                .any(|excluded| paths_equal(excluded, &normalized))
        };
        if exact_match {
            return true;
        }

        path.file_name().is_some_and(|name| {
            let name = name.to_string_lossy().to_ascii_lowercase();
            self.exact_names.contains(&name)
                || self
                    .name_fragments
                    .iter()
                    .any(|fragment| name.contains(fragment))
        })
    }
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::{Confidence, DirectoryEvidence, DirectoryRole, analyze_directory_tree};
    use std::fs::{self, File};
    use std::path::Path;
    use tempfile::TempDir;

    fn touch(path: impl AsRef<Path>) {
        File::create(path).unwrap();
    }

    #[test]
    fn recognizes_exe_collection_with_benign_files() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("One.exe"));
        touch(temp.path().join("Two.exe"));
        touch(temp.path().join("README.txt"));
        touch(temp.path().join("One.ico"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::ExeCollection);
        assert_eq!(analysis.confidence, Confidence::High);
        assert_eq!(analysis.direct_exes.len(), 2);
        assert!(
            analysis
                .evidence
                .contains(&DirectoryEvidence::DocumentationOnly)
        );
    }

    #[test]
    fn recognizes_app_root_from_support_files() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("App.exe"));
        touch(temp.path().join("App.dll"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::AppRoot);
        assert!(analysis.has_self_app());
        assert_eq!(analysis.owned_exes.len(), 1);
    }

    #[test]
    fn one_weak_support_file_is_not_enough_for_app_root() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("App.exe"));
        touch(temp.path().join("settings.json"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::Unknown);
    }

    #[test]
    fn multiple_weak_support_files_form_app_evidence() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("App.exe"));
        touch(temp.path().join("settings.json"));
        touch(temp.path().join("defaults.ini"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::AppRoot);
    }

    #[test]
    fn component_executable_belongs_to_parent_app() {
        let temp = TempDir::new().unwrap();
        let bin = temp.path().join("bin");
        fs::create_dir(&bin).unwrap();
        touch(bin.join("App.exe"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::AppRoot);
        assert_eq!(analysis.owned_exes, vec![bin.join("App.exe")]);
    }

    #[test]
    fn recognizes_nested_containers() {
        let temp = TempDir::new().unwrap();
        for app in ["Gimp", "Krita"] {
            let app_dir = temp.path().join("Graphics").join(app);
            fs::create_dir_all(&app_dir).unwrap();
            touch(app_dir.join(format!("{app}.exe")));
            touch(app_dir.join(format!("{app}.dll")));
        }

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::Container);
        assert_eq!(analysis.children[0].role, DirectoryRole::Container);
    }

    #[test]
    fn recognizes_mixed_directory() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("Rufus.exe"));
        let child = temp.path().join("Everything");
        fs::create_dir(&child).unwrap();
        touch(child.join("Everything.exe"));
        touch(child.join("Everything.dll"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::Mixed);
        assert_eq!(analysis.owned_exes, vec![temp.path().join("Rufus.exe")]);
    }

    #[test]
    fn app_root_owns_non_component_descendants() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("Root.exe"));
        touch(temp.path().join("Root.dll"));

        let wrapper = temp.path().join("Downloads");
        fs::create_dir(&wrapper).unwrap();
        touch(wrapper.join("ambiguous.exe"));
        touch(wrapper.join("note.zip"));
        let child = wrapper.join("NestedApp");
        fs::create_dir(&child).unwrap();
        touch(child.join("NestedApp.exe"));
        touch(child.join("NestedApp.dll"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::AppRoot);
        assert_eq!(analysis.owned_exes, vec![temp.path().join("Root.exe")]);
        assert_eq!(analysis.children[0].role, DirectoryRole::Unknown);
    }

    #[test]
    fn unknown_wrapper_still_exposes_nested_application() {
        let temp = TempDir::new().unwrap();
        let wrapper = temp.path().join("Downloads");
        fs::create_dir(&wrapper).unwrap();
        touch(wrapper.join("ambiguous.exe"));
        touch(wrapper.join("note.zip"));
        let child = wrapper.join("NestedApp");
        fs::create_dir(&child).unwrap();
        touch(child.join("NestedApp.exe"));
        touch(child.join("NestedApp.dll"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::Container);
        assert_eq!(analysis.children[0].role, DirectoryRole::Unknown);
        assert_eq!(
            analysis.children[0].children[0].role,
            DirectoryRole::AppRoot
        );
    }

    #[test]
    fn matching_root_executable_claims_config_tools() {
        let temp = TempDir::new().unwrap();
        let app = temp.path().join("Dism++");
        let config = app.join("Config").join("amd64");
        fs::create_dir_all(&config).unwrap();
        for executable in ["Dism++ARM64.exe", "Dism++x64.exe", "Dism++x86.exe"] {
            touch(app.join(executable));
        }
        touch(app.join("payload.zip"));
        touch(config.join("bcdboot.exe"));
        touch(config.join("bcdboot.dll"));

        let analysis = analyze_directory_tree(&app, &[]);

        assert_eq!(analysis.role, DirectoryRole::AppRoot);
        assert_eq!(analysis.owned_exes.len(), 3);
        assert!(
            analysis
                .owned_exes
                .iter()
                .all(|path| path.parent() == Some(app.as_path()))
        );
    }

    #[test]
    fn app_root_does_not_offer_setup_directory_executables() {
        let temp = TempDir::new().unwrap();
        let app = temp.path().join("GameViewer");
        let setup = app.join("setup");
        fs::create_dir_all(&setup).unwrap();
        touch(app.join("GameViewer.exe"));
        touch(app.join("GameViewer.dll"));
        touch(setup.join("GameViewer_Setup_1.exe"));
        touch(setup.join("GameViewer_Setup_2.exe"));

        let analysis = analyze_directory_tree(&app, &[]);

        assert_eq!(analysis.role, DirectoryRole::AppRoot);
        assert_eq!(analysis.owned_exes, vec![app.join("GameViewer.exe")]);
    }

    #[test]
    fn nested_standard_launch_directory_belongs_to_app_root() {
        let temp = TempDir::new().unwrap();
        let app = temp.path().join("obs-studio");
        let bin = app.join("bin").join("64bit");
        fs::create_dir_all(&bin).unwrap();
        touch(app.join("uninstall.exe"));
        touch(bin.join("obs64.exe"));

        let analysis = analyze_directory_tree(&app, &[]);

        assert_eq!(analysis.role, DirectoryRole::AppRoot);
        assert_eq!(analysis.owned_exes.len(), 2);
        assert!(analysis.owned_exes.contains(&app.join("uninstall.exe")));
        assert!(analysis.owned_exes.contains(&bin.join("obs64.exe")));
    }

    #[test]
    fn leaves_ambiguous_directory_unknown() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("compiler.exe"));
        touch(temp.path().join("archive.zip"));

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::Unknown);
        assert!(analysis.owned_exes.is_empty());
    }

    #[test]
    fn sdk_layout_remains_unknown() {
        let temp = TempDir::new().unwrap();
        let sdk = temp.path().join("SDK");
        fs::create_dir_all(&sdk).unwrap();
        touch(sdk.join("sdk.exe"));
        for (directory, executable) in [
            ("bin", "compiler.exe"),
            ("tools", "helper.exe"),
            ("samples", "demo.exe"),
        ] {
            let child = sdk.join(directory);
            fs::create_dir_all(&child).unwrap();
            touch(child.join(executable));
        }

        let analysis = analyze_directory_tree(&sdk, &[]);

        assert_eq!(analysis.role, DirectoryRole::Unknown);
        assert!(analysis.owned_exes.is_empty());
    }

    #[test]
    fn excluded_entries_do_not_affect_analysis() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("App.exe"));
        touch(temp.path().join("ignored.dll"));

        let analysis = analyze_directory_tree(temp.path(), &["ignored".to_string()]);

        assert_eq!(analysis.role, DirectoryRole::ExeCollection);
    }

    #[test]
    fn exact_name_exclusion_does_not_hide_similar_file_names() {
        let temp = TempDir::new().unwrap();
        touch(temp.path().join("RecoveryTool.exe"));

        let analysis = analyze_directory_tree(temp.path(), &["=Recovery".to_string()]);

        assert_eq!(analysis.role, DirectoryRole::ExeCollection);
    }

    #[test]
    fn inaccessible_path_is_unknown() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("missing");

        let analysis = analyze_directory_tree(&missing, &[]);

        assert_eq!(analysis.role, DirectoryRole::Unknown);
        assert!(analysis.evidence.contains(&DirectoryEvidence::Inaccessible));
    }

    #[test]
    fn empty_directory_is_unknown() {
        let temp = TempDir::new().unwrap();

        let analysis = analyze_directory_tree(temp.path(), &[]);

        assert_eq!(analysis.role, DirectoryRole::Unknown);
        assert!(analysis.children.is_empty());
    }

    #[test]
    fn component_directory_name_is_recognized() {
        let temp = TempDir::new().unwrap();
        let resources = temp.path().join("resources");
        fs::create_dir(&resources).unwrap();
        assert!(super::is_component_directory(&resources));
    }
}

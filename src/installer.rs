use crate::console::{ConsoleType, write_console};
use crate::utils::matches_glob;
use rust_i18n::t;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

pub(crate) fn run_install_scripts(dir: &Path, scripts: Option<&[String]>, parallel: bool) {
    for entry in WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_dir() {
                return true;
            }
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy().to_lowercase();
            if file_name.ends_with("setup.cmd")
                || file_name.ends_with("setup.bat")
                || file_name.ends_with("install.cmd")
                || file_name.ends_with("install.bat")
                || file_name.contains("绿化.cmd")
                || file_name.contains("绿化.bat")
            {
                return true;
            }
            scripts.is_some_and(|patterns| {
                patterns.iter().any(|pattern| {
                    if pattern.contains(std::path::MAIN_SEPARATOR) {
                        path.to_string_lossy().eq_ignore_ascii_case(pattern)
                    } else {
                        matches_glob(pattern, &file_name)
                    }
                })
            })
        })
    {
        let Ok(entry) = entry else {
            continue;
        };
        if entry.file_type().is_dir() {
            continue;
        }
        let script = entry.path();
        write_console(
            ConsoleType::Info,
            &t!("shortcut.run_install", path = script.display()),
        );
        let mut command = Command::new(script);
        command.creation_flags(0x08000000).current_dir(dir);
        if parallel {
            command.spawn().ok();
        } else {
            command.output().ok();
        }
    }
}

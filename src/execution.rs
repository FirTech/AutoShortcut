use crate::DEBUG;
use crate::config::{ConfigInfo, Lnk};
use crate::console::{ConsoleType, write_console};
use crate::directory::{DirectoryAnalysis, DirectoryRole, is_component_directory};
use crate::installer::run_install_scripts;
use crate::selector::select_main_executable;
use crate::shortcut::create_program_shortcut;
use rust_i18n::t;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::Ordering;

/// Runtime options shared by recursive directory execution.
pub(crate) struct AutoExecutionContext<'a> {
    pub(crate) lnk_path: Option<&'a Path>,
    pub(crate) config_info: Option<&'a ConfigInfo>,
    pub(crate) only_match: bool,
    pub(crate) create_dir: bool,
    pub(crate) install_script: bool,
    pub(crate) install_parallel: bool,
    pub(crate) start: bool,
    pub(crate) list_mode: bool,
    pub(crate) use_filename: bool,
    pub(crate) score_ratio: f32,
}

pub(crate) fn execute_directory(analysis: &DirectoryAnalysis, context: &AutoExecutionContext<'_>) {
    if DEBUG.load(Ordering::Relaxed) {
        write_console(
            ConsoleType::Debug,
            &format!(
                "[Directory] {} => {:?}/{:?} ({:?})",
                analysis.path.display(),
                analysis.role,
                analysis.confidence,
                analysis.evidence
            ),
        );
    }

    if context.only_match {
        for executable in &analysis.direct_exes {
            if context
                .config_info
                .and_then(|config| Lnk::get_lnk_info(executable, &config.shortcut))
                .is_some()
            {
                process_program(executable, context);
            }
        }
        for child in &analysis.children {
            execute_directory(child, context);
        }
        return;
    }

    match analysis.role {
        DirectoryRole::AppRoot => {
            log_directory_role("directory.green", analysis, context);
            process_app_root(analysis, context);
        }
        DirectoryRole::ExeCollection => {
            log_directory_role("directory.single_file", analysis, context);
            for executable in &analysis.direct_exes {
                process_program(executable, context);
            }
        }
        DirectoryRole::Container => {
            log_directory_role("directory.category", analysis, context);
            for child in &analysis.children {
                execute_directory(child, context);
            }
        }
        DirectoryRole::Mixed => {
            log_directory_role("directory.hybrid", analysis, context);
            if analysis.has_self_app() {
                process_app_root(analysis, context);
            } else {
                for executable in &analysis.direct_exes {
                    process_program(executable, context);
                }
            }
            for child in &analysis.children {
                if !is_component_directory(&child.path) {
                    execute_directory(child, context);
                }
            }
        }
        DirectoryRole::Unknown => {
            for child in &analysis.children {
                execute_directory(child, context);
            }
        }
    }
}

fn log_directory_role(key: &str, analysis: &DirectoryAnalysis, context: &AutoExecutionContext<'_>) {
    if !context.list_mode {
        write_console(ConsoleType::Info, &t!(key, path = analysis.path.display()));
    }
}

fn process_app_root(analysis: &DirectoryAnalysis, context: &AutoExecutionContext<'_>) {
    let selected = select_main_executable(
        &analysis.path,
        &analysis.owned_exes,
        context.config_info,
        context.score_ratio,
    );
    let Some((app_root, executable)) = selected else {
        if !context.list_mode {
            write_console(
                ConsoleType::Warning,
                &t!("scan.main_not_recognized", path = analysis.path.display()),
            );
        }
        return;
    };

    if context.install_script {
        run_install_scripts(
            &app_root,
            context.config_info.map(|config| config.scripts.as_slice()),
            context.install_parallel,
        );
    }
    process_program(&executable, context);
}

fn process_program(program_path: &Path, context: &AutoExecutionContext<'_>) {
    if context.list_mode {
        println!("{}", program_path.display());
    }

    if context.start {
        write_console(
            ConsoleType::Info,
            &t!("shortcut.start", path = program_path.display()),
        );
        if let Some(parent) = program_path.parent() {
            Command::new(program_path)
                .creation_flags(0x08000000)
                .current_dir(parent)
                .spawn()
                .ok();
        }

        let has_config_destination = context
            .config_info
            .and_then(|config| Lnk::get_lnk_info(program_path, &config.shortcut))
            .as_ref()
            .and_then(|shortcut| shortcut.dest.as_ref())
            .is_some();
        if context.lnk_path.is_none() && !has_config_destination {
            return;
        }
    }

    if context.list_mode {
        return;
    }

    let shortcut = context
        .config_info
        .and_then(|config| Lnk::get_lnk_info(program_path, &config.shortcut));
    let template = context
        .config_info
        .and_then(|config| config.template.clone());
    match create_program_shortcut(
        program_path,
        context.lnk_path,
        shortcut,
        template,
        context.use_filename,
        context.create_dir,
    ) {
        Ok((name, _)) => write_console(
            ConsoleType::Success,
            &t!(
                "shortcut.create_success",
                name = name,
                path = program_path.display()
            ),
        ),
        Err(_) => write_console(
            ConsoleType::Error,
            &t!("shortcut.create_failed", path = program_path.display()),
        ),
    }
}

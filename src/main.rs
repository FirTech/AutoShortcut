// 禁用变量命名警告
#![allow(non_snake_case)]
// 禁用未使用代码警告
#![allow(dead_code)]

mod cli;
mod config;
mod console;
mod directory;
mod template;
mod utils;

#[cfg(test)]
mod test;

use crate::config::{ConfigInfo, DEFAULT_NAME_TEMPLATE, Lnk, Template};
use crate::console::{ConsoleType, write_console};
use crate::directory::{
    DirectoryAnalysis, DirectoryRole, analyze_directory_tree, is_component_directory,
};
use crate::template::process_template;
use crate::utils::{
    create_shortcut, exe_has_signature, get_exe_description, get_native_arch, get_program_arch,
    get_shortcut_target, has_icon_in_program, is_gui_program, is_running_under_wow64,
    launched_from_explorer, matches_glob, name_similarity, parse_hotkey, parse_icon_spec,
    replace_ignore_case, resolve_relative_path, validate_shortcut_name_for_config,
};
use anyhow::{Result, anyhow};
use clap::Parser;
use rust_i18n::{set_locale, t};
use std::env;
use std::fs::create_dir_all;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;
use sys_locale::get_locale;
use walkdir::WalkDir;
use windows::Win32::System::SystemInformation::{
    PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64, PROCESSOR_ARCHITECTURE_INTEL,
};

/// 调试模式
static DEBUG: AtomicBool = AtomicBool::new(false);

// 国际化
rust_i18n::i18n!("locales");

fn main() -> Result<()> {
    // 设置国际化
    let system_locale = get_locale().unwrap_or("en".into());
    match system_locale.as_str() {
        "zh-CN" => set_locale("zh-CN"),
        "zh-TW" => set_locale("zh-TW"),
        _ => set_locale("en"),
    }

    // 判断是否从资源管理器启动
    if launched_from_explorer() && env::args().len() == 1 {
        println!("{}", t!("cmdline_tool_tips"));
        sleep(Duration::from_secs(5));
        return Ok(());
    }

    // 处理命令行
    let cli = crate::cli::Cli::parse();
    if cli.debug {
        DEBUG.store(true, Ordering::Relaxed);
    }

    // 配置文件模式
    if cli.config.is_some() && cli.targetPath.is_none() && cli.lnkPath.is_none() {
        let cfg = cli.config.unwrap();
        config_shortcut(
            cfg,
            cli.install,
            cli.install_parallel,
            cli.start,
            cli.use_filename,
        )?;
        return Ok(());
    }

    // 自动搜索模式
    auto_shortcut(
        &cli.targetPath.unwrap(),
        cli.lnkPath.as_deref(),
        cli.config.as_deref(),
        cli.only_match,
        cli.create_dir,
        cli.install,
        cli.install_parallel,
        cli.start,
        cli.list,
        cli.use_filename,
        cli.score_ratio,
    )?;
    Ok(())
}

/// 自动创建快捷方式
///
/// # 参数
/// - `target_path`: 程序路径
/// - `lnk_path`: 快捷方式路径
/// - `config_path`: 配置文件路径
/// - `only_match`: 是否为仅配置文件模式
/// - `create_dir`: 是否创建目录
/// - `install_script`: 是否运行脚本
/// - `install_parallel`: 是否并行运行脚本
/// - `use_filename`: 使用原始文件名
/// - `list_mode`: 是否仅列出快捷方式路径
///
/// # 返回值
/// - `Ok(())`: 创建成功
/// - `Err(...)`：失败则返回错误
pub fn auto_shortcut(
    target_path: &Path,
    lnk_path: Option<&Path>,
    config_path: Option<&Path>,
    only_match: bool,
    create_dir: bool,
    install_script: bool,
    install_parallel: bool,
    start: bool,
    list_mode: bool,
    use_filename: bool,
    score_ratio: f32,
) -> Result<()> {
    let mut config_info = None;
    let mut score_ratio = score_ratio;
    let mut only_match = only_match;
    let mut use_filename = use_filename;
    let mut install_script = install_script;
    let mut install_parallel = install_parallel;

    if let Some(config) = config_path {
        match ConfigInfo::parse_config_file(config) {
            Ok(mut config) => {
                only_match |= config.only_match;
                use_filename |= config.use_filename;
                install_script |= config.install;
                install_parallel |= config.install_parallel;

                if config.score_ratio.is_some_and(|ratio| ratio > 1.0) {
                    write_console(
                        ConsoleType::Warning,
                        &t!(
                            "config.invalid_ratio",
                            ratio = config.score_ratio.unwrap_or_default()
                        ),
                    );
                    config.score_ratio = None;
                }
                if let Some(ratio) = config.score_ratio {
                    score_ratio = ratio;
                }
                for shortcut in &config.shortcut {
                    if shortcut
                        .name
                        .as_ref()
                        .is_some_and(|name| !validate_shortcut_name_for_config(name))
                    {
                        write_console(
                            ConsoleType::Warning,
                            &t!(
                                "config.invalid_name",
                                name = shortcut.name.as_deref().unwrap_or_default()
                            ),
                        );
                    }
                }
                config_info = Some(config);
            }
            Err(error) => {
                write_console(
                    ConsoleType::Error,
                    &format!("{}: {}", &t!("config.parse_failed"), error),
                );
                return Err(anyhow!("Configuration file parsing failed"));
            }
        }
    }

    const SYSTEM_EXCLUDED_DIRS: &[&str] = &[
        "$RECYCLE.BIN",
        "System Volume Information",
        "Recovery",
        "Config.Msi",
        "MSOCache",
    ];
    let mut excluded = SYSTEM_EXCLUDED_DIRS
        .iter()
        .map(|value| format!("={value}"))
        .collect::<Vec<_>>();
    if let Some(config_path) = config_path {
        excluded.push(config_path.to_string_lossy().to_string());
    }
    if let Some(config) = &config_info {
        excluded.extend(config.ignore.iter().cloned());
    }

    let analysis = analyze_directory_tree(target_path, &excluded);
    let context = AutoExecutionContext {
        lnk_path,
        config_info: config_info.as_ref(),
        only_match,
        create_dir,
        install_script,
        install_parallel,
        start,
        list_mode,
        use_filename,
        score_ratio,
    };
    execute_directory(&analysis, &context);
    Ok(())
}

/// Runtime options shared by recursive directory execution.
struct AutoExecutionContext<'a> {
    lnk_path: Option<&'a Path>,
    config_info: Option<&'a ConfigInfo>,
    only_match: bool,
    create_dir: bool,
    install_script: bool,
    install_parallel: bool,
    start: bool,
    list_mode: bool,
    use_filename: bool,
    score_ratio: f32,
}

fn execute_directory(analysis: &DirectoryAnalysis, context: &AutoExecutionContext<'_>) {
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
    let selected = find_software_best_exe_from_candidates(
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

/// Creates shortcuts exclusively from explicit configuration entries.
fn config_shortcut(
    config_path: PathBuf,
    install: bool,
    install_parallel: bool,
    start: bool,
    use_name: bool,
) -> Result<()> {
    // 读取配置文件信息
    let config_info = match ConfigInfo::parse_config_file(&config_path) {
        Ok(config) => config,
        Err(e) => {
            write_console(
                ConsoleType::Error,
                &format!("{}: {}", &t!("config.parse_failed"), e),
            );
            return Err(anyhow!("Configuration file parsing failed"));
        }
    };

    // 执行安装脚本
    if install {
        for pat in config_info.scripts.iter() {
            let file_path = PathBuf::from(pat);
            if !file_path.exists() {
                write_console(ConsoleType::Warning, &t!("file.not_found", path = pat));
                continue;
            }

            write_console(
                ConsoleType::Info,
                &t!("shortcut.run_install", path = file_path.display()),
            );
            if install_parallel {
                Command::new(&file_path)
                    .creation_flags(0x08000000)
                    .current_dir(file_path.parent().unwrap())
                    .spawn()
                    .ok();
            } else {
                Command::new(&file_path)
                    .creation_flags(0x08000000)
                    .current_dir(file_path.parent().unwrap())
                    .output()
                    .ok();
            }
        }
    }

    // 遍历配置文件快捷方式信息
    for lnk in config_info.shortcut {
        // 运行程序
        if start {
            Command::new(&lnk.exec)
                .creation_flags(0x08000000)
                .current_dir(Path::new(&lnk.exec).parent().unwrap())
                .spawn()
                .ok();
        }

        // 创建快捷方式
        match create_program_shortcut(
            Path::new(&lnk.exec.clone()),
            None,
            Some(lnk.clone()),
            config_info.template.clone(),
            use_name,
            false,
        ) {
            Ok((name, _path)) => write_console(
                ConsoleType::Success,
                &t!("shortcut.create_success", name = name, path = lnk.exec),
            ),
            Err(_) => write_console(
                ConsoleType::Error,
                &t!("shortcut.create_failed", path = lnk.exec),
            ),
        };
    }

    Ok(())
}

fn find_software_best_exe_from_candidates(
    app_root_path: &Path,
    candidates: &[PathBuf],
    config_info: Option<&ConfigInfo>,
    score_ratio: f32,
) -> Option<(PathBuf, PathBuf)> {
    const CONFIG_MATCH_SCORE: i32 = 100;
    const NAME_PARENT_MAX_SCORE: i32 = 40;
    const GUI_SCORE: i32 = 50;
    const ICON_SCORE: i32 = 40;
    const DESCRIPTION_SCORE: i32 = 30;
    const NATIVE_ARCH_SCORE: i32 = 45;
    const X86_COMPATIBLE_ARCH_SCORE: i32 = 35;
    const SIGNATURE_SCORE: i32 = 60;
    const MAX_SIZE_SCORE: i32 = 30;
    const MIN_NAME_SIMILARITY: f32 = 0.70;

    // The config bonus expresses an explicit user preference, not evidence that an EXE is a
    // runnable main program. It must not make the automatic-recognition threshold harder to meet.
    const MAX_HEURISTIC_SCORE: i32 = NAME_PARENT_MAX_SCORE
        + GUI_SCORE
        + ICON_SCORE
        + DESCRIPTION_SCORE
        + NATIVE_ARCH_SCORE
        + SIGNATURE_SCORE
        + MAX_SIZE_SCORE;

    let mut best_candidate: Option<(PathBuf, PathBuf)> = None;
    let mut best_score = 0;
    let system_arch_code = get_native_arch();

    // Score only candidates owned by this analyzed application root.
    for file_path in candidates {
        if file_path.is_file()
            && file_path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            // 当前分数
            let mut score = 0;
            // 分数明细记录 (metric_name, delta)
            let mut breakdown: Vec<(&str, i32)> = Vec::new();

            // 应用配置中的忽略列表
            if let Some(cfg) = &config_info {
                let file_name = file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase());

                // 检查是否需要忽略此文件或目录
                if file_name.is_some_and(|name| {
                    cfg.ignore.iter().any(|kw| {
                        // 尝试将关键词解析为绝对路径
                        if let Ok(keyword_path) = PathBuf::from(kw).canonicalize() {
                            // 对当前完整路径进行规范化并比较
                            if let Ok(current_path) = file_path.canonicalize() {
                                current_path == keyword_path
                            } else {
                                // 如果无法规范化当前路径，回退到文件名匹配
                                name.contains(kw.to_lowercase().as_str())
                            }
                        } else {
                            // 不是绝对路径，使用文件名包含匹配
                            name.contains(kw.to_lowercase().as_str())
                        }
                    })
                }) {
                    if DEBUG.load(Ordering::Relaxed) {
                        write_console(
                            ConsoleType::Debug,
                            &t!("scan.ignore_in_config", path = file_path.display()),
                        );
                    }
                    continue;
                }
            }

            // 配置文件名指定程序文件
            let explicit_config_match = config_info.is_some_and(|config_info| {
                config_info.shortcut.iter().any(|kw| {
                    let exec_cfg = PathBuf::from(&kw.exec);
                    let full_path = if exec_cfg.is_absolute() {
                        exec_cfg.clone()
                    } else {
                        file_path.parent().unwrap().join(&exec_cfg)
                    };
                    full_path == *file_path
                })
            });
            if explicit_config_match {
                score += CONFIG_MATCH_SCORE;
                breakdown.push(("config_match", CONFIG_MATCH_SCORE));
            }

            // 文件名与所在目录或应用根目录匹配
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
                    let name_score = (similarity * NAME_PARENT_MAX_SCORE as f32).round() as i32;
                    score += name_score;
                    breakdown.push(("name_app_match", name_score));
                }
            }

            // Explicit configuration wins over automatic role heuristics.
            if !explicit_config_match {
                let penalty = automatic_executable_role_penalty(file_path);
                if penalty != 0 {
                    score += penalty;
                    breakdown.push(("non_entry_role", penalty));
                }
            }

            // 判断是否为界面程序
            if let Ok(is_gui) = is_gui_program(file_path) {
                if is_gui {
                    score += GUI_SCORE;
                    breakdown.push(("gui", GUI_SCORE));
                } else {
                    score -= 30;
                    breakdown.push(("gui_penalty", -30));
                }
            }

            // 判断是否有图标
            if has_icon_in_program(file_path) {
                score += ICON_SCORE;
                breakdown.push(("icon", ICON_SCORE));
            }

            // 判断是否有程序描述信息
            if let Ok(Some(_description)) = get_exe_description(file_path) {
                score += DESCRIPTION_SCORE;
                breakdown.push(("description", DESCRIPTION_SCORE));
            }

            // 判断程序位数是否与系统相匹配
            if let Ok(program_arch_code) = get_program_arch(file_path) {
                let arch_score = match (program_arch_code, system_arch_code) {
                    // Native programs are preferred when otherwise comparable.
                    (0x014c, arch) if arch == PROCESSOR_ARCHITECTURE_INTEL.0 => NATIVE_ARCH_SCORE,
                    (0x8664, arch) if arch == PROCESSOR_ARCHITECTURE_AMD64.0 => NATIVE_ARCH_SCORE,
                    (0xAA64, arch) if arch == PROCESSOR_ARCHITECTURE_ARM64.0 => NATIVE_ARCH_SCORE,
                    // WoW64 allows x86 Windows programs to run on x64 Windows.
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

            // 判断是否有数字签名
            if exe_has_signature(file_path).unwrap_or(false) {
                score += SIGNATURE_SCORE;
                breakdown.push(("signature", SIGNATURE_SCORE));
            }

            // 获取程序大小
            if let Ok(metadata) = file_path.metadata() {
                // 转换为 MB
                let file_size_mb = metadata.len() / (1024 * 1024);

                // 只对大于等于 1MB 的文件进行评分
                if file_size_mb >= 1 {
                    // 每 MB 增加 1 分，并四舍五入、并设置最高分上限，防止分数过高
                    let size_score = (file_size_mb as i32).min(MAX_SIZE_SCORE);
                    score += size_score;
                    if size_score != 0 {
                        breakdown.push(("size", size_score));
                    }
                }
            }

            // 判断文件名包含辅助功能关键词
            // let include_keyword = ["launcher", "start"];
            // for include in include_keyword {
            //     if file_path.file_name().unwrap().to_ascii_lowercase().to_str().unwrap().contains(include) {
            //         score += 25;
            //         break;
            //     }
            // }

            if DEBUG.load(Ordering::Relaxed) {
                let details = breakdown
                    .iter()
                    .map(|(k, v)| format!("{}{:+}", k, v))
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

            // 只有得分超过阈值才继续
            if score <= (MAX_HEURISTIC_SCORE as f32 * score_ratio).round() as i32 {
                // println!("[局部扫描][非主程序] {} 得分过低 ({})，跳过", file_path.display(), score);
                continue;
            }

            let candidate = (app_root_path.to_path_buf(), PathBuf::from(file_path));

            // 比较并更新最佳候选
            if score > best_score {
                best_score = score;
                best_candidate = Some(candidate);
            }
        }
    }
    best_candidate
}

fn automatic_executable_role_penalty(file_path: &Path) -> i32 {
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

/// Runs matching installation scripts from an analyzed application root.
fn run_install_scripts(dir: &Path, scripts: Option<&[String]>, install_parallel: bool) {
    // max_depth(1): 只检查当前目录下的文件，不深入子目录
    for entry in WalkDir::new(dir).max_depth(1).into_iter().filter_entry({
        move |entry| {
            // 排除目录
            if entry.file_type().is_dir() {
                return true;
            }

            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy().to_lowercase();

            // 默认规则
            if file_name.ends_with("setup.cmd")
                || file_name.ends_with("setup.bat")
                || file_name.ends_with("install.cmd")
                || file_name.ends_with("install.bat")
                || file_name.contains("绿化.cmd")
                || file_name.contains("绿化.bat")
            {
                return true;
            }

            // 自定义规则（支持通配符）
            if let Some(patterns) = scripts {
                for pat in patterns {
                    if pat.contains(std::path::MAIN_SEPARATOR) {
                        // 路径里带有分隔符，就当作路径
                        if path.to_string_lossy().to_ascii_lowercase() == pat.to_ascii_lowercase() {
                            return true;
                        }
                    } else {
                        // 只匹配文件名
                        if matches_glob(pat, &file_name.to_lowercase()) {
                            return true;
                        }
                    };
                }
            }

            false
        }
    }) {
        // 跳过访问失败的路径
        let entry = match entry {
            Ok(e) => e,
            Err(_e) => {
                continue;
            }
        };

        // 跳过目录
        if entry.file_type().is_dir() {
            continue;
        }

        let file_path = entry.path();
        write_console(
            ConsoleType::Info,
            &t!("shortcut.run_install", path = file_path.display()),
        );

        if install_parallel {
            Command::new(file_path)
                .creation_flags(0x08000000)
                .current_dir(dir)
                .spawn()
                .ok();
        } else {
            Command::new(file_path)
                .creation_flags(0x08000000)
                .current_dir(dir)
                .output()
                .ok();
        }
    }
}

/// 创建程序快捷方式
///
/// # 参数
/// - `program_path`: 程序路径
/// - `link_path`: 快捷方式保存路径
/// - `use_filename`: 是否使用原始文件名
/// - `link_info`: 快捷方式信息
///
/// # 返回值
/// - `Ok(())`: 创建成功
/// - `Err(...)`：失败则返回错误
fn create_program_shortcut(
    program_path: &Path,
    link_path: Option<&Path>,
    lnk_info: Option<Lnk>,
    template: Option<Template>,
    use_filename: bool,
    create_dir: bool,
) -> Result<(String, PathBuf)> {
    // 判断程序是否存在
    if let Ok(true) = is_running_under_wow64() {
        let alt = replace_ignore_case(&program_path.to_string_lossy(), "\\System32", "\\SysNative");

        if !Path::new(&alt).exists() {
            write_console(
                ConsoleType::Warning,
                &t!("file.not_found", path = program_path.display()),
            );
            return Err(anyhow!(t!("file.not_found", path = program_path.display())));
        }
    } else if !program_path.exists() {
        write_console(
            ConsoleType::Warning,
            &t!("file.not_found", path = program_path.display()),
        );
        return Err(anyhow!(t!("file.not_found", path = program_path.display())));
    }

    // 位置
    let dest = &lnk_info
        .clone()
        // 首先使用配置项中的信息
        .and_then(|li| li.dest.as_ref().map(PathBuf::from))
        // 然后尝试命令行传入的 link_path
        .or_else(|| link_path.map(|p| p.to_path_buf()))
        // 最后尝试全局模板配置 template.dest（Option<String>）
        .or_else(|| {
            template
                .clone()
                .clone()
                .and_then(|t| t.dest.as_ref().map(PathBuf::from))
        });
    let mut dest = if let Some(dest) = dest {
        dest.to_path_buf()
    } else {
        write_console(
            ConsoleType::Warning,
            &t!("config.miss_dest", path = program_path.display()),
        );
        return Err(anyhow!("configuration missing `dest`"));
    };

    if create_dir {
        if let Some(parent) = program_path.parent() {
            if let Some(file_name) = parent.file_name() {
                dest = dest.join(file_name);
            }
        }
    }

    // 创建快捷方式目录
    if !dest.exists() {
        create_dir_all(&dest)?;
    }

    // 快捷方式名称
    let mut name = {
        let stem = program_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let name_template = template
            .as_ref()
            .and_then(|t| t.name.as_deref())
            .unwrap_or(DEFAULT_NAME_TEMPLATE);

        // 指定使用原始文件名
        if use_filename {
            stem.to_string()
        } else if let Some(link_info) = &lnk_info {
            // 优先使用配置文件指定名称（未指定则使用模板）
            if let Some(name) = &link_info.name {
                if validate_shortcut_name_for_config(name) {
                    name.to_string()
                } else {
                    write_console(
                        ConsoleType::Warning,
                        &t!("config.invalid_name", name = name),
                    );
                    return Err(anyhow!(t!("config.invalid_name", name = name)));
                }
            } else {
                process_template(program_path, name_template)
            }
        } else {
            // 没有配置文件，使用全局模板
            process_template(program_path, name_template)
        }
    };

    // 命令行
    let args = lnk_info.as_ref().and_then(|li| li.args.clone());

    // 图标
    let icon: Option<(String, i32)> = lnk_info
        .as_ref()
        .and_then(|li| li.icon.as_ref())
        .and_then(|raw| {
            let (file_part, idx) = parse_icon_spec(raw);
            let file_part = Path::new(&file_part).to_string_lossy().to_string();

            if PathBuf::from(&file_part).is_absolute() {
                // 绝对路径
                if let Ok(true) = is_running_under_wow64() {
                    let alt = replace_ignore_case(&file_part, "\\System32", "\\SysNative");
                    if Path::new(&alt).exists() {
                        return Some((file_part, idx));
                    }
                } else if Path::new(&file_part).exists() {
                    return Some((file_part, idx));
                }
            } else {
                // 相对路径
                let full_path = program_path.parent().unwrap().join(&file_part);
                if full_path.exists() {
                    return Some((full_path.to_string_lossy().to_string(), idx));
                } else if let Some(full_path) = resolve_relative_path(&PathBuf::from(&file_part)) {
                    return Some((full_path.to_string_lossy().to_string(), idx));
                }
            }

            // 如果找不到就警告
            write_console(
                ConsoleType::Warning,
                &t!("file.icon_not_found", path = file_part),
            );
            None
        })
        // 如果 link_info.icon 没有，则尝试使用全局模板配置
        .or_else(|| {
            template.as_ref().and_then(|t| {
                t.icon.as_ref().and_then(|s| {
                    let rendered = process_template(program_path, s);
                    if Path::new(&rendered).exists() {
                        Some((rendered, 0))
                    } else {
                        write_console(
                            ConsoleType::Warning,
                            &t!("file.icon_not_found", path = rendered),
                        );
                        None
                    }
                })
            })
        })
        // 如果 template 也没有，则尝试同目录下同名 ico 文件
        .or_else(|| {
            program_path.parent().and_then(|p| {
                let sibling = p.join(format!(
                    "{}.ico",
                    program_path.file_stem().unwrap().to_string_lossy()
                ));
                sibling
                    .exists()
                    .then(|| (sibling.to_string_lossy().to_string(), 0))
            })
        });

    // 工作路径
    let work_dir = lnk_info
        .clone()
        // 首先使用配置项中的信息
        .and_then(|li| li.work_dir.as_ref().map(|s| s.to_string()))
        // 尝试全局模板配置
        .or_else(|| {
            template
                .clone()
                .and_then(|t| t.work_dir.as_ref().map(|s| s.to_string()))
        });

    // 显示模式
    let window_state = lnk_info.as_ref().and_then(|li| li.window_state.clone());

    // 备注：优先配置项，如有模板则使用模板
    let comment: Option<String> =
        lnk_info
            .as_ref()
            .and_then(|li| li.comment.clone())
            .or_else(|| {
                template.and_then(|t| {
                    t.comment
                        .as_ref()
                        .map(|tmpl| process_template(program_path, tmpl))
                })
            });

    // 快捷键解析
    let hotkey: Option<u16> = lnk_info
        .as_ref()
        .and_then(|li| li.hotkey.as_ref())
        .and_then(|hotkey_str| match parse_hotkey(hotkey_str) {
            Ok(hk) => Some(hk),
            Err(_e) => {
                write_console(
                    ConsoleType::Warning,
                    &t!("config.invalid_hotkey", hotkey = hotkey_str),
                );
                None
            }
        });

    // 检测是否存在同名快捷方式
    let current_shortcut = dest.join(format!("{}.lnk", name));
    if current_shortcut.exists() {
        // 判断源快捷方式与当前快捷方式指向路径是否一致
        if let Ok(original_path) = get_shortcut_target(&current_shortcut) {
            if original_path.to_string_lossy().to_ascii_lowercase()
                != program_path.to_string_lossy().to_ascii_lowercase()
            {
                // 获取程序架构
                // let original_arch = get_program_arch(&original_path)?;
                // let current_arch = get_program_arch(program_path)?;

                // 获取程序版本
                // let original_version = get_exe_file_version(&original_path)?.unwrap_or_default();
                // let current_version = get_exe_file_version(program_path)?.unwrap_or_default();

                // if original_arch != current_arch && original_version == current_version {
                //     // 程序位数不一致，在快捷方式名称后追加程序架构
                //     let get_arch = |arch: u16| match arch {
                //         0x014c => "x86",
                //         0x8664 => "x64",
                //         0xAA64 => "ARM64",
                //         _ => { "Unknown" }
                //     };
                //
                //     rename(current_shortcut, dest.join(format!("{} {}.lnk", name, get_arch(original_arch))))?;
                //     name = format!("{} {}", name, get_arch(current_arch));
                // } else if original_version != current_version && original_arch == current_arch {
                //     // 程序版本不一样，在快捷方式名称后追加程序版本
                //     rename(current_shortcut, dest.join(format!("{} {}.lnk", name, original_version)))?;
                //     name = format!("{} {}", name, current_version);
                // } else {
                // 未知情况，在快捷方式名称后追加数字
                for n in 2..1000 {
                    let cand = format!("{} ({})", name, n);
                    let path = dest.join(format!("{}.lnk", cand));
                    if path.exists() {
                        // 再次检查现有 .lnk 的目标是否相同（同则可以复用）
                        if let Ok(existing_target) = get_shortcut_target(&path) {
                            if existing_target.to_string_lossy().to_ascii_lowercase()
                                == program_path.to_string_lossy().to_ascii_lowercase()
                            {
                                name = cand.clone();
                                break;
                            }
                        }
                        continue;
                    }
                    name = cand.clone();
                    break;
                }
            }
            // }
        }
    }

    create_shortcut(
        program_path,
        &dest.join(format!("{}.lnk", name)),
        args,
        icon,
        work_dir,
        window_state,
        comment,
        hotkey,
    )?;
    Ok((name.clone(), dest.join(format!("{}.lnk", name))))
}

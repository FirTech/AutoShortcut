// 禁用变量命名警告
#![allow(non_snake_case)]
// 禁用未使用代码警告
#![allow(dead_code)]

mod cli;
mod config;
mod console;
mod template;
mod utils;

use crate::config::{ConfigInfo, Lnk, Template, DEFAULT_NAME_TEMPLATE};
use crate::console::{write_console, ConsoleType};
use crate::template::process_template;
use crate::utils::{
    create_shortcut, exe_has_signature, get_exe_description, get_native_arch, get_program_arch,
    get_shortcut_target, has_icon_in_program, is_gui_program, is_running_under_wow64,
    launched_from_explorer, matches_glob, normalize_app_name, parse_hotkey, parse_icon_spec,
    replace_ignore_case, resolve_relative_path, validate_shortcut_name_for_config,
};
use anyhow::{anyhow, Result};
use clap::Parser;
use rust_i18n::{set_locale, t};
use std::collections::HashSet;
use std::fs;
use std::fs::create_dir_all;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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
    if launched_from_explorer() {
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
    score_ratio: Option<f32>,
) -> Result<()> {
    // 评分阈值百分比
    let mut score_ratio = score_ratio.unwrap_or(0.3);

    // 读取配置文件信息
    let mut config_info = None;
    let mut only_match = only_match;
    let mut use_filename = use_filename;
    let mut install_script = install_script;
    let mut install_parallel = install_parallel;

    if let Some(config) = config_path {
        if let Ok(mut config) = ConfigInfo::parse_config_file(config) {
            if config.only_match {
                only_match = true;
            }

            if config.use_filename {
                use_filename = true;
            }

            if config.install {
                install_script = true;
            }

            if config.install_parallel {
                install_parallel = true;
            }

            // 判断评分阈值是否合法
            if let Some(ratio) = config.score_ratio {
                if ratio > 1.0 {
                    write_console(
                        ConsoleType::Warning,
                        &t!("config.invalid_ratio", ratio = ratio),
                    );
                    config.score_ratio = None;
                }
            }

            // 指定评分阈值百分比
            if let Some(ratio) = config.score_ratio {
                score_ratio = ratio;
            }

            // 验证配置文件中快捷方式名称是否合法
            for ln in &config.shortcut {
                if let Some(ref provided_name) = ln.name {
                    if !validate_shortcut_name_for_config(provided_name) {
                        write_console(
                            ConsoleType::Warning,
                            &t!("config.invalid_name", name = provided_name),
                        );
                    }
                }
            }

            config_info = Some(config.clone());
        } else {
            write_console(ConsoleType::Error, &t!("config.parse_failed"));
            return Err(anyhow!("Configuration file parsing failed"));
        }
    }

    let identified_app_roots: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));

    // 内置排除目录
    let EXCLUDED_DIRS: &[&str] = &[
        // 系统目录
        "$RECYCLE.BIN",
        "System Volume Information",
        "Recovery",
        "Config.Msi",
        "MSOCache",
        // 配置文件路径
        if let Some(config) = config_path {
            &*config.to_string_lossy().to_string()
        } else {
            ""
        },
    ];

    let mut all_excluded = EXCLUDED_DIRS.to_vec();
    if let Some(config) = &config_info {
        all_excluded.extend(config.ignore.iter().map(|s| s.as_str()));
    }

    // 主循环: 遍历所有文件（包括子目录）
    for entry_result in WalkDir::new(target_path).into_iter().filter_entry({
        let roots_for_filter = Arc::clone(&identified_app_roots);
        let config_info = config_info.clone();

        move |entry| {
            let path = entry.path();

            // 排除：特殊目录
            if entry.file_type().is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if EXCLUDED_DIRS
                        .iter()
                        .any(|&ex| ex.eq_ignore_ascii_case(name))
                    {
                        if DEBUG.load(Ordering::Relaxed) {
                            write_console(
                                ConsoleType::Debug,
                                &t!("scan.ignore", path = path.display()),
                            );
                        }
                        return false;
                    }
                }
            }

            // 排除: 自定义排除
            if let Some(cfg) = &config_info {
                let file_name = path.file_name().map(|n| n.to_string_lossy().to_lowercase());

                // 检查是否需要忽略此文件或目录
                if file_name.is_some_and(|name| {
                    cfg.ignore.iter().any(|kw| {
                        // 尝试将关键词解析为绝对路径
                        if let Ok(keyword_path) = PathBuf::from(kw).canonicalize() {
                            // 对当前完整路径进行规范化并比较
                            if let Ok(current_path) = path.canonicalize() {
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
                            &t!("scan.ignore", path = path.display()),
                        );
                    }
                    return false;
                }
            }

            // 剪枝算法
            let roots = roots_for_filter.lock().unwrap();
            if roots.iter().any(|root| path.starts_with(root)) {
                if DEBUG.load(Ordering::Relaxed) {
                    write_console(ConsoleType::Debug, &t!("scan.prune", path = path.display()));
                }
                return false;
            }

            true
        }
    }) {
        // 判断是否正常访问路径
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                if !list_mode {
                    write_console(ConsoleType::Warning, &t!("file.access_failed", error = e));
                }
                continue;
            }
        };
        let file_path = entry.path();

        // 仅配置文件匹配模式
        if only_match && entry.file_type().is_dir() {
            continue;
        }

        // 自动识别主程序逻辑
        if entry.file_type().is_dir() {
            if is_category_dir(file_path) {
                if !list_mode {
                    write_console(
                        ConsoleType::Info,
                        &t!("directory.category", path = file_path.display()),
                    );
                }
                continue;
            }

            //判断是否为单文件目录
            if is_single_file_dir(file_path) {
                // 单文件程序目录
                if !list_mode {
                    write_console(
                        ConsoleType::Info,
                        &t!("directory.single_file", path = file_path.display()),
                    );
                }

                let mut roots = identified_app_roots.lock().unwrap();
                if roots.insert(file_path.to_path_buf()) {
                    // println!("剪枝: {}", file_path.display());
                    for entry in WalkDir::new(file_path)
                        .max_depth(1)
                        .into_iter()
                        .filter_map(Result::ok)
                        .filter(|file| {
                            file.path().is_file()
                                && file
                                    .path()
                                    .extension()
                                    .unwrap_or_default()
                                    .eq_ignore_ascii_case("exe")
                        })
                    {
                        let path = entry.path();

                        // 单文件排除: 自定义排除
                        if let Some(cfg) = &config_info {
                            // 如果文件名包含任一 ignore 关键字，就跳过（返回 false）
                            if path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_lowercase())
                                .is_some_and(|name| {
                                    cfg.ignore
                                        .iter()
                                        .any(|kw| name.contains(&kw.to_lowercase()))
                                })
                            {
                                if DEBUG.load(Ordering::Relaxed) {
                                    write_console(
                                        ConsoleType::Debug,
                                        &t!("scan.ignore", path = path.display()),
                                    );
                                }
                                continue;
                            }
                        }

                        if list_mode {
                            println!("{}", path.display());
                        }

                        // 运行程序
                        if start {
                            write_console(
                                ConsoleType::Info,
                                &t!("shortcut.start", path = path.display()),
                            );
                            Command::new(path)
                                .creation_flags(0x08000000)
                                .current_dir(path.parent().unwrap())
                                .spawn()
                                .ok();

                            // 如果命令行没有指定 lnk_path，且配置里也没有对应的 dest，则跳过后续处理
                            if lnk_path.is_none()
                                && config_info
                                    .as_ref()
                                    .and_then(|cfg| Lnk::get_lnk_info(path, &cfg.shortcut))
                                    .as_ref()
                                    .and_then(|li| li.dest.as_ref())
                                    .is_none()
                            {
                                continue;
                            }
                        }

                        if list_mode {
                            continue;
                        }

                        // 创建快捷方式
                        let lnk_info = config_info
                            .as_ref()
                            .and_then(|cfg| Lnk::get_lnk_info(path, &cfg.shortcut));
                        let template = config_info.as_ref().and_then(|cfg| cfg.template.clone());

                        match create_program_shortcut(
                            path,
                            lnk_path,
                            lnk_info,
                            template,
                            use_filename,
                            create_dir,
                        ) {
                            Ok((name, _path)) => write_console(
                                ConsoleType::Success,
                                &t!(
                                    "shortcut.create_success",
                                    name = name,
                                    path = path.display()
                                ),
                            ),
                            Err(_) => write_console(
                                ConsoleType::Error,
                                &t!("shortcut.create_failed", path = path.display()),
                            ),
                        };
                    }
                } else {
                    if DEBUG.load(Ordering::Relaxed) {
                        write_console(
                            ConsoleType::Debug,
                            &t!("scan.prune", path = file_path.display()),
                        );
                    }
                };
            } else if is_hybrid_software_dir(file_path, &all_excluded) {
                // 单文件软件、绿色软件混合目录
                if !list_mode {
                    write_console(
                        ConsoleType::Info,
                        &t!("directory.hybrid", path = file_path.display()),
                    );
                }
                continue;
            } else {
                // 绿色软件目录
                if !list_mode {
                    write_console(
                        ConsoleType::Info,
                        &t!("directory.green", path = file_path.display()),
                    );
                }

                let mut roots_guard = identified_app_roots.lock().unwrap();

                // 遍历全部exe进行打分
                if let Some((suggested_app_root, exe_path)) = find_software_best_exe(
                    file_path,
                    config_info.clone().as_ref(),
                    target_path,
                    score_ratio,
                    list_mode,
                ) {
                    // 找到了最佳EXE，并且它的根目录是新的（没有被处理过）
                    if roots_guard.insert(suggested_app_root.clone()) {
                        // 运行安装脚本
                        if install_script {
                            run_install_scripts(
                                &suggested_app_root,
                                config_info
                                    .as_ref()
                                    .map(|config| config.scripts.clone())
                                    .as_deref(),
                                install_parallel,
                            );
                        }

                        if list_mode {
                            println!("{}", exe_path.display());
                            continue;
                        }

                        // 运行程序
                        if start {
                            write_console(
                                ConsoleType::Info,
                                &t!("shortcut.start", path = exe_path.display()),
                            );
                            Command::new(&exe_path)
                                .creation_flags(0x08000000)
                                .current_dir(exe_path.parent().unwrap())
                                .spawn()
                                .ok();

                            // 如果命令行没有指定 lnk_path，且配置里也没有对应的 dest，则跳过后续处理
                            if lnk_path.is_none()
                                && config_info
                                    .as_ref()
                                    .and_then(|cfg| Lnk::get_lnk_info(&exe_path, &cfg.shortcut))
                                    .as_ref()
                                    .and_then(|li| li.dest.as_ref())
                                    .is_none()
                            {
                                continue;
                            }
                        }

                        if list_mode {
                            continue;
                        }

                        // 创建快捷方式
                        let lnk_info = config_info
                            .as_ref()
                            .and_then(|cfg| Lnk::get_lnk_info(&exe_path, &cfg.shortcut));
                        let template = config_info.as_ref().and_then(|cfg| cfg.template.clone());

                        match create_program_shortcut(
                            &exe_path,
                            lnk_path,
                            lnk_info,
                            template,
                            use_filename,
                            create_dir,
                        ) {
                            Ok((name, _path)) => write_console(
                                ConsoleType::Success,
                                &t!(
                                    "shortcut.create_success",
                                    name = name,
                                    path = exe_path.display()
                                ),
                            ),
                            Err(_) => write_console(
                                ConsoleType::Error,
                                &t!("shortcut.create_failed", path = exe_path.display()),
                            ),
                        };
                    } else {
                        // 已处理软件根目录
                        if DEBUG.load(Ordering::Relaxed) {
                            write_console(
                                ConsoleType::Debug,
                                &t!("scan.prune", path = suggested_app_root.display()),
                            );
                        }
                    }
                } else {
                    // 绿色软件目录中，根据评分规则没有识别到主程序
                    if !list_mode {
                        write_console(
                            ConsoleType::Warning,
                            &t!("scan.main_not_recognized", path = file_path.display()),
                        );
                    }
                    roots_guard.insert(file_path.to_path_buf());
                }
            }
        } else if entry.file_type().is_file()
            && file_path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            // 匹配配置文件模式
            if let Some(cfg) = &config_info {
                if only_match && Lnk::get_lnk_info(file_path, &cfg.shortcut).is_none() {
                    continue;
                }
            }

            // 情况1: “绿色软件”打分失败，识别为可能的绿色根目录，却又在 collect_and_score_best_exe_in_root() 里因为所有 EXE 分数都低于阈值而拿不出一个“最佳主程序”
            // 情况2: 所有其他未被剪枝、又没被当作应用根的 exe,在深层子目录里有临时 exe、测试文件、脚本等，

            if list_mode {
                println!("{}", file_path.display());
            }

            // 运行程序
            if start {
                write_console(
                    ConsoleType::Info,
                    &t!("shortcut.start", path = file_path.display()),
                );
                Command::new(file_path)
                    .creation_flags(0x08000000)
                    .current_dir(file_path.parent().unwrap())
                    .spawn()
                    .ok();

                // 如果命令行没有指定 lnk_path，且配置里也没有对应的 dest，则跳过后续处理
                if lnk_path.is_none()
                    && config_info
                        .as_ref()
                        .and_then(|cfg| Lnk::get_lnk_info(file_path, &cfg.shortcut))
                        .as_ref()
                        .and_then(|li| li.dest.as_ref())
                        .is_none()
                {
                    continue;
                }
            }

            if list_mode {
                continue;
            }

            // 创建快捷方式
            let lnk_info = config_info
                .as_ref()
                .and_then(|cfg| Lnk::get_lnk_info(file_path, &cfg.shortcut));
            let template = config_info.as_ref().and_then(|cfg| cfg.template.clone());

            match create_program_shortcut(
                file_path,
                lnk_path,
                lnk_info,
                template,
                use_filename,
                create_dir,
            ) {
                Ok((name, _path)) => write_console(
                    ConsoleType::Success,
                    &t!(
                        "shortcut.create_success",
                        name = name,
                        path = file_path.display()
                    ),
                ),
                Err(_) => write_console(
                    ConsoleType::Error,
                    &t!("shortcut.create_failed", path = file_path.display()),
                ),
            };
        }
    }

    Ok(())
}

/// 仅通过配置文件创建快捷方式
///
/// # 参数
///
/// - `config_path` - 配置文件路径
/// - `install` - 是否执行安装脚本
/// - `install_parallel` - 是否并行执行安装脚本
/// - `start` - 是否运行程序
/// - `use_name` - 是否使用程序名称作为快捷方式名称
///
/// # 返回值
///
/// 如果创建快捷方式成功，返回 `Ok(())`；否则返回 `Err`。
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
        Err(_e) => {
            write_console(ConsoleType::Error, &t!("config.parse_failed"));
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

/// 检查目录是否包含常见的应用程序支持文件或子目录
///
/// 不进行PE解析，只看文件/目录名和类型
///
/// # 参数
///
/// - `dir_path` - 要检查的目录路径
///
/// # 返回值
///
/// 如果目录符合应用程序结构条件，返回 `true`；否则返回 `false`。
fn contains_app_structure_lightweight(dir_path: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(dir_path) {
        let mut has_exe = false;
        let mut has_support_files = false; //.dll,.ini,.json,.xml,.dat,.cfg,.conf
        let mut has_common_subdirs = false; // bin, lib, data, program, assets, resources, content, modules, plugins, drivers
        let mut has_doc_files = false; // README.txt, LICENSE.txt, EULA.txt, CHANGELOG.txt
        let mut exe_count = 0;

        for entry_result in entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue, // 忽略无法读取的条目
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };

            if file_type.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let lower_ext = ext.to_ascii_lowercase();
                    if lower_ext == "exe" {
                        has_exe = true;
                        exe_count += 1;
                    } else if [
                        "dll", "pak", "ini", "json", "xml", "yaml", "dat", "cfg", "conf", "log",
                        "reg", "key", "cupf",
                    ]
                    .contains(&lower_ext.as_str())
                    {
                        has_support_files = true;
                    } else if ["txt", "md", "pdf"].contains(&lower_ext.as_str()) {
                        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let lower_stem = file_stem.to_ascii_lowercase();
                            if ["readme", "license", "eula", "changelog"]
                                .contains(&lower_stem.as_str())
                            {
                                has_doc_files = true;
                            }
                        }
                    }
                }
            } else if file_type.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                    let lower_dir_name = dir_name.to_ascii_lowercase();
                    // 增加更多常见的应用程序子目录 [1, 2, 3]
                    if [
                        "bin",
                        "lib",
                        "data",
                        "program",
                        "assets",
                        "resources",
                        "content",
                        "modules",
                        "plugins",
                        "drivers",
                    ]
                    .contains(&lower_dir_name.as_str())
                    {
                        has_common_subdirs = true;
                    }
                }
            }
        }

        // 判断当前目录是否是常见的组件文件夹名
        let is_component_folder_name = dir_path.file_name().is_some_and(|n| {
            let lower_name = n.to_string_lossy().to_ascii_lowercase();
            // 常见的组件目录名，这些通常不是应用程序的最高层根目录
            [
                "bin",
                "program",
                "executables",
                "x64",
                "win64",
                "modules",
                "plugins",
                "drivers",
            ]
            .contains(&lower_name.as_str())
        });

        // 规则组合：
        // 1. 包含EXE，且有支持文件或常见子目录 (最常见的多文件应用)
        // 2. 包含EXE，且有常见文档文件 (一些简单的便携式应用)
        // 3. 包含多个EXE，且目录名不是组件文件夹 (例如“硬件检测”目录)
        return
            // Rule 1
            (has_exe && (has_support_files || has_common_subdirs)) ||
                // Rule 2
                (has_exe && has_doc_files) ||
                // Rule 3 (for multi-single-file apps)
                (exe_count > 1 && !is_component_folder_name);
    }
    false
}

/// 在绿色软件目录中收集所有EXE并评分，选出最佳的EXE文件
///
/// # 参数
///
/// - `app_root_path` - 绿色软件目录的根路径
/// - `config_info` - 可选的配置信息，用于忽略某些文件
/// - `initial_scan_root` - 初始扫描的根路径，用于确定扫描范围
/// - `score_ratio` - 评分比例，用于调整评分权重
/// - `list_mode` - 是否以列表模式运行，用于控制输出
///
/// # 返回值
///
/// 如果找到最佳的EXE文件，返回 `Some((best_exe_path, best_exe_name))`；否则返回 `None`。
fn find_software_best_exe(
    app_root_path: &Path,
    config_info: Option<&ConfigInfo>,
    initial_scan_root: &Path,
    score_ratio: f32,
    list_mode: bool,
) -> Option<(PathBuf, PathBuf)> {
    let mut best_candidate: Option<(PathBuf, PathBuf)> = None;
    let mut best_score = 0;

    // 局部扫描：扫描当前目录及子目录（最大两层）
    for entry_result in WalkDir::new(app_root_path).max_depth(2).into_iter() {
        //
        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                if !list_mode {
                    write_console(ConsoleType::Warning, &t!("file.access_failed", error = e));
                }
                continue;
            }
        };

        let file_path = entry.path();

        if file_path.is_file()
            && file_path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            /// 最高分
            const MAX_SCORE: i32 = 100  // 配置匹配
                + 40   //文件名与父目录名匹配
                + 50   // GUI
                + 40   // 图标
                + 30   // 描述
                + 45   // 架构
                + 60   // 数字签名
                + 30; // 文件体积分

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
            if let Some(config_info) = &config_info {
                if config_info.shortcut.iter().any(|kw| {
                    let exec_cfg = PathBuf::from(&kw.exec);
                    let full_path = if exec_cfg.is_absolute() {
                        exec_cfg.clone()
                    } else {
                        file_path.parent().unwrap().join(&exec_cfg)
                    };
                    full_path == file_path
                }) {
                    score += 100;
                    breakdown.push(("config_match", 100));
                }
            }

            // 文件名与父目录名匹配
            if let Some(parent_dir_name) = file_path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
            {
                if let Some(file_stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                    let normalized_file_stem = normalize_app_name(file_stem);
                    let normalized_parent_name = normalize_app_name(parent_dir_name);
                    if !normalized_parent_name.is_empty()
                        && (normalized_file_stem.contains(&normalized_parent_name)
                            || normalized_parent_name.contains(&normalized_file_stem))
                    {
                        score += 40;
                        breakdown.push(("name_parent_match", 40));
                    }
                }
            }

            // 判断是否为界面程序
            if let Ok(is_gui) = is_gui_program(file_path) {
                if is_gui {
                    score += 50;
                    breakdown.push(("gui", 50));
                } else {
                    score -= 30;
                    breakdown.push(("gui_penalty", -30));
                }
            }

            // 判断是否有图标
            if has_icon_in_program(file_path) {
                score += 40;
                breakdown.push(("icon", 40));
            }

            // 判断是否有程序描述信息
            if let Ok(Some(_description)) = get_exe_description(file_path) {
                score += 30;
                breakdown.push(("description", 30));
            }

            // 判断程序位数是否与系统相匹配
            if let Ok(program_arch_code) = get_program_arch(file_path) {
                let system_arch_code = unsafe { get_native_arch() };
                if match program_arch_code {
                    0x014c => {
                        // IMAGE_FILE_MACHINE_I386 (x86 程序)
                        system_arch_code == PROCESSOR_ARCHITECTURE_INTEL.0 // 匹配 x86 系统
                    }
                    0x8664 => {
                        // IMAGE_FILE_MACHINE_AMD64 (x64 程序)
                        system_arch_code == PROCESSOR_ARCHITECTURE_AMD64.0 // 匹配 x64 系统
                    }
                    0xAA64 => {
                        // IMAGE_FILE_MACHINE_ARM64 (ARM64 程序)
                        system_arch_code == PROCESSOR_ARCHITECTURE_ARM64.0 // 匹配 ARM64 系统
                    }
                    _ => false, // 遇到未知或不常见的程序架构，默认不匹配
                } {
                    score += 45;
                    breakdown.push(("arch", 45));
                }
            }

            // 判断是否有数字签名
            if exe_has_signature(file_path).unwrap_or(false) {
                score += 60;
                breakdown.push(("signature", 60));
            }

            // 获取程序大小
            if let Ok(metadata) = file_path.metadata() {
                // 转换为 MB
                let file_size_mb = metadata.len() / (1024 * 1024);

                // 只对大于等于 1MB 的文件进行评分
                if file_size_mb >= 1 {
                    // 每 MB 增加 1 分，并四舍五入、并设置最高分上限，防止分数过高
                    let size_score = (file_size_mb as i32).min(30);
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
            if score <= (MAX_SCORE as f32 * score_ratio).round() as i32 {
                // println!("[局部扫描][非主程序] {} 得分过低 ({})，跳过", file_path.display(), score);
                continue;
            }

            // 识别应用程序根目录 (向上回溯)
            let mut current_root_candidate = file_path
                .parent()
                .map_or_else(|| file_path.to_path_buf(), |p| p.to_path_buf());
            let mut final_app_root = current_root_candidate.clone();
            let mut depth_checked = 0;
            let max_upward_depth = 3; // 向上回溯的最大层数

            while let Some(parent) = current_root_candidate.parent() {
                if depth_checked >= max_upward_depth
                    || parent == initial_scan_root
                    || parent.parent().is_none()
                {
                    break;
                }

                // 检查父目录是否具有应用结构特征
                let mut parent_has_app_structure = false;
                if let Ok(entries) = fs::read_dir(parent) {
                    for entry_in_parent in entries.filter_map(|e| e.ok()) {
                        let entry_path = entry_in_parent.path();
                        if entry_path.is_file() {
                            if let Some(ext) = entry_path.extension().and_then(|s| s.to_str()) {
                                let lower_ext = ext.to_ascii_lowercase();
                                if lower_ext == "dll"
                                    || ["ini", "json", "xml", "dat", "cfg", "conf"]
                                        .contains(&lower_ext.as_str())
                                {
                                    parent_has_app_structure = true;
                                    break;
                                }
                            }
                        } else if entry_path.is_dir() {
                            if let Some(dir_name) = entry_path.file_name().and_then(|s| s.to_str())
                            {
                                let lower_dir_name = dir_name.to_ascii_lowercase();
                                if [
                                    "bin",
                                    "lib",
                                    "data",
                                    "program",
                                    "assets",
                                    "resources",
                                    "content",
                                ]
                                .contains(&lower_dir_name.as_str())
                                {
                                    parent_has_app_structure = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                let current_dir_name_lower = current_root_candidate
                    .file_name()
                    .map_or("".to_string(), |n| n.to_string_lossy().to_ascii_lowercase());
                let is_component_folder = ["bin", "program", "executables", "x64", "win64"]
                    .contains(&current_dir_name_lower.as_str());

                if parent_has_app_structure || is_component_folder {
                    final_app_root = parent.to_path_buf();
                    current_root_candidate = parent.to_path_buf();
                    depth_checked += 1;
                } else {
                    break;
                }
            }

            let candidate = (final_app_root, PathBuf::from(file_path));

            // 比较并更新最佳候选
            if score > best_score {
                best_score = score;
                best_candidate = Some(candidate);
            }
        }
    }
    best_candidate
}

/// 判断程序目录是否为单文件程序目录
///
/// # 参数
///
/// - `app_root` - 要检查的程序目录路径
///
/// # 返回值
///
/// 如果目录符合单文件程序目录条件，返回 `true`；否则返回 `false`。
fn is_single_file_dir(app_root: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(app_root) {
        let mut exe_count = 0;
        let mut other_file_count = 0;
        let mut dir_count = 0;

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let lower_ext = ext.to_ascii_lowercase();
                    if lower_ext == "exe" {
                        exe_count += 1;
                    } else if lower_ext != "ico" {
                        // 允许.ico 文件存在
                        other_file_count += 1;
                    }
                } else {
                    other_file_count += 1; // 没有扩展名的文件也算作其他文件
                }
            } else if path.is_dir() {
                dir_count += 1;
            }
        }
        // 单文件程序：只有exe，没有其他文件（排除ico），没有子目录
        return exe_count > 0 && other_file_count == 0 && dir_count == 0;
    }
    false
}

/// 判断目录是否为单文件程序、绿色软件的混合目录
/// 条件：
///  1. 根目录下至少有一个 exe，且除了 exe 之外没有其它文件（视情况也可允许 .ico/.ini/.xml）
///  2. 根目录的子目录是单文件目录或绿色软件目录
///
/// # 参数
///
/// - `dir` - 要检查的目录路径
/// - `exclude_keyword` - 排除的关键词列表，用于过滤目录
///
/// # 返回值
///
/// 如果目录符合混合目录条件，返回 `true`；否则返回 `false`。
fn is_hybrid_software_dir(dir: &Path, exclude_keyword: &[&str]) -> bool {
    /// 是否把某些文件扩展名视为“允许的辅助文件”，不会导致拒绝混合目录判定
    const ALLOWED_ROOT_FILE_EXT: &[&str] = &["ico"];

    /// 根目录中允许的“其它不认识文件”最大数量比例（例如 0.3 表示最多 30% 的根文件为未知类型）
    const ROOT_UNKNOWN_FILE_RATIO_ALLOWED: f32 = 0.3;

    /// 子目录中被识别为“应用子包”的占比阈值（例如 0.6 表示 >=60% 子目录是应用子包就认定为混合）
    const SUBDIR_APP_RATIO_THRESHOLD: f32 = 0.8;

    // 收集根目录一级 entries
    let mut root_exe_count = 0usize;
    let mut root_unknown_file_count = 0usize;
    let mut root_allowed_file_count = 0usize;
    let mut subdirs: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let p = entry.path();
            if exclude_keyword.iter().any(|k| {
                if let Ok(keyword_path) = PathBuf::from(k).canonicalize() {
                    // 判断绝对路径
                    if let Ok(current_path) = p.canonicalize() {
                        current_path == keyword_path
                    } else {
                        p.display().to_string().to_lowercase().contains(k)
                    }
                } else {
                    // 不是绝对路径，使用现有的包含匹配
                    p.display().to_string().to_lowercase().contains(k)
                }
            }) {
                continue;
            }
            if p.is_file() {
                // extension 的处理要小心无扩展名的文件
                if let Some(ext) = p
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_ascii_lowercase())
                {
                    if ext == "exe" {
                        root_exe_count += 1;
                    } else if ALLOWED_ROOT_FILE_EXT.contains(&ext.as_str()) {
                        root_allowed_file_count += 1;
                    } else {
                        root_unknown_file_count += 1;
                    }
                } else {
                    // 无扩展名的文件视为未知
                    root_unknown_file_count += 1;
                }
            } else if p.is_dir() {
                subdirs.push(p);
            }
        }
    }

    // 前置条件：顶层需要至少有一个 exe（表明根会存放单文件程序）
    if root_exe_count == 0 {
        return false;
    }

    // 需要有至少一个子目录，才考虑是混合目录
    if subdirs.is_empty() {
        return false;
    }

    // 统计子目录被识别为绿色软件或单文件程序的数量
    let mut app_subdirs = 0usize;
    for sd in &subdirs {
        // 对每个子目录使用已有的轻量检测函数（它们本身要足够稳健）
        if contains_app_structure_lightweight(sd) || is_single_file_dir(sd) {
            app_subdirs += 1;
        }
    }

    let appdir_ratio = app_subdirs as f32 / subdirs.len() as f32;

    // 根目录里未知文件占比过高时，应判定为非混合（例如存大量数据文件）
    let total_root_files =
        (root_exe_count + root_allowed_file_count + root_unknown_file_count) as f32;
    let unknown_ratio = if total_root_files > 0.0 {
        root_unknown_file_count as f32 / total_root_files
    } else {
        0.0
    };
    if unknown_ratio > ROOT_UNKNOWN_FILE_RATIO_ALLOWED {
        // 根目录里有过多未知文件，保守认为不是混合软件目录
        return false;
    }

    // 最终判定：子目录中大多数为应用包，或至少有足够数量的 app 子目录
    if appdir_ratio >= SUBDIR_APP_RATIO_THRESHOLD || app_subdirs >= 1 {
        return true;
    }

    false
}

/// 判断一个目录是否为分类目录
///
/// # 参数
///
/// - `dir` - 要检查的目录路径
///
/// # 返回值
///
/// 如果目录符合分类目录条件，返回 `true`；否则返回 `false`。
fn is_category_dir(dir: &Path) -> bool {
    // 根目录下有没有顶层 exe
    let mut has_exe = false;

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_ascii_lowercase();
                    if ext == "exe" {
                        has_exe = true;
                        break;
                    }
                }
            }
        }
    }

    // 分类目录不能有顶层 exe
    if has_exe {
        return false;
    }

    // 列出一级子目录
    let sub_dirs: Vec<_> = match fs::read_dir(dir) {
        Ok(rd) => rd
            // 跳过读取出错的条目
            .filter_map(Result::ok)
            .filter_map(|e| {
                // 跳过 file_type 出错的条目
                match e.file_type() {
                    Ok(ft) if ft.is_dir() => Some(e.path()),
                    _ => None,
                }
            })
            .collect(),
        Err(_) => return false,
    };

    // 子目录中，至少有一个是真正的“应用子包”（单文件 或 绿色软件）
    let mut has_app_subdir = false;
    for sd in &sub_dirs {
        if is_single_file_dir(sd) || contains_app_structure_lightweight(sd) {
            has_app_subdir = true;
            break;
        } else {
            // 子目录只要有一个不符合，就整目录不算分类容器
            // return false;
        }
    }

    has_app_subdir
}

/// 运行安装脚本
///
/// # 参数
/// - `dir`: 路径
/// - `scripts`: 自定义脚本规则
/// - `install_parallel`: 是否并行运行
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

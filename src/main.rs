// 禁用变量命名警告
#![allow(non_snake_case)]
// 禁用未使用代码警告
#![allow(dead_code)]

mod console;
mod utils;
mod cli;
mod config;

use crate::config::{ConfigInfo, LnkInfo};
use crate::console::{write_console, ConsoleType};
use crate::utils::{create_shortcut, getProgramArch, get_exe_description, get_native_arch, has_icon_in_program, is_gui_program, matches_glob, process_env};
use clap::Parser;
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use windows::Win32::System::SystemInformation::{PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM64, PROCESSOR_ARCHITECTURE_INTEL};

fn main() -> Result<(), Box<dyn Error>> {
    let cli = crate::cli::Cli::parse();
    auto_shortcut(&cli.targetPath, &process_env(&cli.lnkPath), cli.configPath, cli.createdir, cli.install, cli.list)?;
    Ok(())
}

/// 自动创建快捷方式
///
/// 参数
/// - `targetPath`: 程序路径
/// - `lnkPath`: 快捷方式路径
/// - `configPath`: 配置文件路径
/// - `createdir`: 是否创建目录
///
/// 返回
/// - `Ok(())`: 创建成功
/// - `Err(...)`：失败则返回错误
pub fn auto_shortcut(target_path: &Path, lnk_path: &Path, config_path: Option<PathBuf>, createdir: bool, install: bool, list_mode: bool) -> Result<(), Box<dyn Error>> {
    // 初始化统计计数器
    let mut folder_count = 0;
    let mut lnk_count = 0;
    let start_time = std::time::Instant::now();

    // 已处理目录集合
    let mut processed_dirs = HashSet::new();

    // 读取配置文件信息
    let mut config_info = None;
    if let Some(config) = config_path {
        if config.exists() {
            if let Ok(config) = ConfigInfo::parse_config_file(&config) {
                config_info = Some(config);
            } else {
                write_console(ConsoleType::Error, "Configuration file parsing failed");
                return Err("Configuration file parsing failed".into());
            }
        } else {
            write_console(ConsoleType::Error, "Config file does not exist");
            return Err("Config file does not exist".into());
        }
    }

    // 场景: 遍历根目录中的程序（不包括子目录）
    for root_path in WalkDir::new(target_path).max_depth(1).into_iter().skip(1).filter_map(|e| e.ok())
        .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().eq_ignore_ascii_case("exe")) {
        let program = root_path.path();

        // 列出快捷方式
        if list_mode {
            println!("{}", program.to_str().unwrap());
            continue;
        }

        lnk_count += 1;
        write_console(ConsoleType::Info, &format!("Create Shortcut: {}", program.to_str().unwrap()));

        // 创建快捷方式
        let lnk_info = if let Some(config_info) = &config_info {
            LnkInfo::get_lnk_info(&config_info.lnkInfo, program)
        } else {
            None
        };
        let parent_path = lnk_path.join(program.parent().unwrap().file_stem().unwrap());
        create_program_shortcut(program, if createdir { &parent_path } else { lnk_path }, &lnk_info)?;
    }

    // 获取搜索深度
    let search_depth = config_info.as_ref().map(|c| { if c.searchDepth == 0 { 2 } else { c.searchDepth } }).unwrap_or(2);

    // 场景: 遍历子目录
    for root_path in WalkDir::new(target_path).max_depth(search_depth).into_iter().skip(1).filter_map(|e| e.ok()).filter(|file| file.path().is_dir()) {
        // 检查是否已处理该目录
        if processed_dirs.contains(root_path.path().parent().unwrap()) {
            continue;
        }

        // 排除目录: 有文件夹但无文件
        if WalkDir::new(root_path.path()).max_depth(1).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file()).count() == 0 {
            continue;
        }

        // 排除目录: 自定义排除
        if let Some(config_info) = &config_info {
            if config_info.ignore.iter().filter(|&item| root_path.path().to_str().unwrap().contains(item)).count() > 0 {
                continue;
            }
        }

        // 枚举子目录的所有exe文件
        let total_exe_files: Vec<PathBuf> = WalkDir::new(root_path.path()).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().eq_ignore_ascii_case("exe"))
            .filter(|file| if config_info.is_none() { true } else { config_info.as_ref().unwrap().ignore.iter().filter(|&item| file.file_name().to_str().unwrap().contains(item)).count() == 0 })
            .map(|file| file.into_path()).collect();

        // 排除目录: 无exe文件的子目录
        if total_exe_files.is_empty() {
            continue;
        }

        // 统计遍历的文件夹
        folder_count += 1;

        // 运行安装脚本
        if install {
            for install_script in WalkDir::new(root_path.path()).max_depth(1).into_iter().filter_map(|e| e.ok())
                .filter(|file| file.path().is_file()) {
                let mut matched = false;
                let filename = install_script.file_name().to_str().unwrap().to_lowercase();

                // 通配符匹配逻辑
                if let Some(config_info) = &config_info {
                    if !config_info.scripts.is_empty() {
                        matched = config_info.scripts.iter().any(|rule| matches_glob(rule, &filename));
                    }
                }

                // 默认脚本规则
                if filename.to_ascii_lowercase().contains("setup.cmd") || filename.to_ascii_lowercase().contains("setup.bat") || filename.to_ascii_lowercase().contains("install.cmd") || filename.to_ascii_lowercase().contains("install.bat") {
                    matched = true;
                }
                if matched {
                    write_console(ConsoleType::Info, &format!("Install Script: {}", install_script.path().to_str().unwrap()));
                    Command::new(install_script.path()).creation_flags(0x08000000)
                        .current_dir(install_script.path().parent().unwrap_or_else(|| Path::new(".")))
                        .output().ok();
                }
            }
        }

        // 判断根目录是否存在除exe文件以外的文件（判断绿色软件/单文件程序）
        let other_files = WalkDir::new(root_path.path()).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file() && !file.path().extension().unwrap_or_default().eq_ignore_ascii_case("exe"))
            .filter(|file| file.path().is_file() && !file.path().extension().unwrap_or_default().eq_ignore_ascii_case("ico"));
        if other_files.count() == 0 {
            // 单文件程序
            for program in total_exe_files {
                // 列出快捷方式
                if list_mode {
                    println!("{}", program.to_str().unwrap());
                    continue;
                }

                lnk_count += 1;
                write_console(ConsoleType::Info, &format!("Create Shortcut: {}", program.to_str().unwrap()));

                // 创建快捷方式（单文件）
                let lnk_info = if let Some(config_info) = &config_info {
                    LnkInfo::get_lnk_info(&config_info.lnkInfo, &program)
                } else {
                    None
                };
                let parent_path = lnk_path.join(program.parent().unwrap().file_stem().unwrap());
                create_program_shortcut(&program, if createdir { &parent_path } else { lnk_path }, &lnk_info)?;
            }
            continue;
        }

        // 绿色软件

        // 加权评分机制，以最高分数为判定
        let mut candidates = total_exe_files.iter().map(|exe| unsafe {
            let mut score = 0;
            // 指定主程序文件名
            if let Some(config_info) = &config_info {
                for item in config_info.lnkInfo.iter() {
                    if exe.file_name().unwrap().to_ascii_lowercase() == *item.name {
                        score += 100;
                        break;
                    }
                }
            }

            // 判断程序名是否包含目录名(转小写+移除空格+去符号)
            let root_path_name = root_path.path().file_name().unwrap().to_str().unwrap()
                .to_lowercase().replace(" ", "").replace("-", "").replace("_", "");
            let exe_stem = exe.file_stem().unwrap().to_str().unwrap().to_lowercase()
                .replace(" ", "").replace("-", "").replace("_", "");
            // 双向包含检测：目录名包含程序名 或 程序名包含目录名
            if root_path_name.contains(&exe_stem) || exe_stem.contains(&root_path_name) {
                score += 40;
            }

            // 判断程序大小是否为最大
            if exe == &total_exe_files.iter().max_by(|x, y| x.metadata().unwrap().len().cmp(&y.metadata().unwrap().len())).unwrap().to_path_buf() {
                score += 10;
            }

            // 判断是否为界面程序
            if let Ok(is_gui) = is_gui_program(exe) {
                if is_gui {
                    score += 50;
                } else {
                    score -= 30;
                }
            }

            // 判断是否有图标
            if has_icon_in_program(exe) {
                score += 40;
            }

            // 判断是否有程序描述信息
            if let Ok(Some(_description)) = get_exe_description(exe) {
                score += 30;
            }

            // 判断程序位数是否与系统相匹配
            if let Ok(program_arch_code) = getProgramArch(exe) {
                let system_arch_code = get_native_arch();
                if match program_arch_code {
                    0x014c => { // IMAGE_FILE_MACHINE_I386 (x86 程序)
                        system_arch_code == PROCESSOR_ARCHITECTURE_INTEL.0 // 匹配 x86 系统
                    }
                    0x8664 => { // IMAGE_FILE_MACHINE_AMD64 (x64 程序)
                        system_arch_code == PROCESSOR_ARCHITECTURE_AMD64.0 // 匹配 x64 系统
                    }
                    0xAA64 => { // IMAGE_FILE_MACHINE_ARM64 (ARM64 程序)
                        system_arch_code == PROCESSOR_ARCHITECTURE_ARM64.0 // 匹配 ARM64 系统
                    }
                    _ => false, // 遇到未知或不常见的程序架构，默认不匹配
                } {
                    score += 45;
                }
            }

            // 判断文件名包含辅助功能关键词
            let exclude_keyword = ["setup", "install", "uninst", "uninstall", "updater", "config", "editor", "server", "diagnostics", "service", "crashhandler", "helper", "卸载"];
            for exclude in exclude_keyword {
                if exe.file_name().unwrap().to_ascii_lowercase().to_str().unwrap().contains(exclude) {
                    score -= 50;
                    break;
                }
            }

            (exe, score)
        }).collect::<Vec<_>>();

        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        let (program, _score) = candidates.first().unwrap();

        // 列出快捷方式
        if list_mode {
            println!("{}", program.to_str().unwrap());
            continue;
        }

        // 标记父目录已处理
        processed_dirs.insert(program.parent().unwrap().to_path_buf());

        lnk_count += 1;

        // 创建快捷方式
        let lnk_info = if let Some(config_info) = &config_info {
            LnkInfo::get_lnk_info(&config_info.lnkInfo, program)
        } else {
            None
        };
        let parent_path = lnk_path.join(program.parent().unwrap().file_stem().unwrap());
        match create_program_shortcut(program, if createdir { &parent_path } else { lnk_path }, &lnk_info) {
            Ok(_) => write_console(ConsoleType::Info, &format!("Create Shortcut: {}", program.to_str().unwrap())),
            Err(_) => write_console(ConsoleType::Error, &format!("Create Shortcut: {}", program.to_str().unwrap()))
        };
    }

    if list_mode {
        return Ok(());
    }

    let duration = start_time.elapsed().as_secs_f32();
    if lnk_count == 0 {
        write_console(ConsoleType::Error, &format!("No shortcuts created, Time: {:.2}s, Subfolders scanned: {}, Shortcuts created: {}.", duration, folder_count, lnk_count));
    } else {
        write_console(ConsoleType::Success, &format!("Operation completed, Time: {:.2}s, Subfolders scanned: {}, Shortcuts created: {}.", duration, folder_count, lnk_count));
    }

    Ok(())
}

/// 创建程序快捷方式
///
/// 参数
/// - `program_path`: 程序路径
/// - `link_path`: 快捷方式保存路径
/// - `link_info`: 快捷方式信息
///
/// 返回
/// - `Ok(())`: 创建成功
/// - `Err(...)`：失败则返回错误
fn create_program_shortcut(program_path: &Path, link_path: &Path, link_info: &Option<LnkInfo>) -> Result<(), Box<dyn Error>> {
    // 位置
    let location = {
        if let Some(link_info) = link_info {
            match &link_info.location {
                None => link_path,
                Some(location) => location.as_ref()
            }
        } else {
            link_path
        }
    };

    if !link_path.exists() {
        fs::create_dir_all(link_path)?;
    }

    // 别名
    let name = {
        if let Some(link_info) = link_info {
            match &link_info.alia {
                None => program_path.file_stem().unwrap().to_str().unwrap().to_string(),
                Some(alia) => alia.clone()
            }
        } else {
            // 获取程序描述
            if let Ok(Some(description)) = get_exe_description(program_path) {
                if !description.trim().to_string().trim_matches('_').is_empty() {
                    description.trim().to_string()
                } else {
                    // 默认程序名称（描述为空）
                    program_path.file_stem().unwrap().to_str().unwrap().to_string()
                }
            } else {
                // 默认程序名称
                program_path.file_stem().unwrap().to_str().unwrap().to_string()
            }
        }
    };

    // 命令行
    let cmdline = {
        if let Some(link_info) = link_info {
            link_info.cmdline.clone()
        } else {
            None
        }
    };

    // 图标
    let icon_path = {
        if let Some(link_info) = link_info {
            match &link_info.icon {
                None => None,
                Some(icon) => {
                    let icon_path = process_env(Path::new(icon));
                    if icon_path.exists() {
                        Some(icon_path.to_str().unwrap().to_string())
                    } else {
                        write_console(ConsoleType::Warning, &format!("Icon not found: {}", icon_path.to_str().unwrap()));
                        None
                    }
                }
            }
        } else {
            None
        }
    };

    // 检查程序目录下同名ico文件
    let icon_path = {
        if icon_path.is_some() {
            icon_path
        } else {
            let default_icon = program_path.parent().unwrap().join(format!("{}.ico", program_path.file_stem().unwrap().to_str().unwrap()));
            if default_icon.exists() {
                Some(default_icon.to_str().unwrap().to_string())
            } else {
                None
            }
        }
    };
    create_shortcut(program_path, &location.join(format!("{}.lnk", name)), cmdline, icon_path)?;
    Ok(())
}

// 禁用变量命名警告
#![allow(non_snake_case)]
// 禁用未使用代码警告
#![allow(dead_code)]

mod console;
mod utils;
mod cli;
mod config;

use std::collections::HashSet;
use std::{env, fs};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use clap::Parser;
use walkdir::WalkDir;
use crate::console::{ConsoleType, write_console};
use crate::utils::{create_shortcut, matches_glob};
use crate::config::ConfigInfo;

fn main() {
    let cli = crate::cli::Cli::parse();
    auto_shortcut(&cli.targetPath, &process_env(&cli.lnkPath), cli.configPath, cli.createdir, cli.install, cli.list);
}

/// 自动创建快捷方式
///
/// 参数
/// - `targetPath`: 程序路径
/// - `lnkPath`: 快捷方式路径
/// - `configPath`: 配置文件路径
pub fn auto_shortcut(target_path: &Path, lnk_path: &Path, config_path: Option<PathBuf>, createdir: bool, install: bool, list_mode: bool) {
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
                return;
            }
        } else {
            write_console(ConsoleType::Error, "config file does not exist");
            return;
        }
    }


    // 遍历指定目录的程序
    for root_path in WalkDir::new(target_path).max_depth(1).into_iter().skip(1).filter_map(|e| e.ok())
        .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().to_ascii_lowercase() == "exe") {
        let program = root_path.path();

        // 处理快捷方式信息
        let lnk_name = get_lnk_alia(&config_info, &program);
        let cmdline = get_lnk_cmdline(&config_info, &program);
        let icon = get_lnk_icon(&config_info, &program);

        lnk_count += 1;
        if list_mode {
            println!("{}", program.to_str().unwrap());
        } else {
            write_console(ConsoleType::Info, &format!("Create Shortcut: {}", program.to_str().unwrap()));
            let shortcut_file = lnk_path.join(format!("{}.lnk", lnk_name));
            create_shortcut(&program, &shortcut_file, cmdline, icon).ok();
        }
    }

    // 获取搜索深度
    let search_depth = config_info.as_ref().map(|c| { if c.searchDepth == 0 {2}  else {c.searchDepth} }).unwrap_or(2);

    // 遍历指定目录的子目录
    for root_path in WalkDir::new(target_path).max_depth(search_depth).into_iter().skip(1).filter_map(|e| e.ok()).filter(|file| file.path().is_dir()) {

        // 检查是否已处理该目录
        if processed_dirs.contains(root_path.path().parent().unwrap()) {
            continue;
        }

        folder_count += 1; // 统计遍历的文件夹

        // 排除 有文件夹但无文件 的目录
        if WalkDir::new(root_path.path()).max_depth(1).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file()).count() == 0 {
            continue;
        }

        // 判断是否为自定义排除目录
        if let Some(config_info) = &config_info {
            if config_info.ignore.iter().filter(|&item| root_path.path().to_str().unwrap().contains(item)).count() > 0 {
                continue;
            }
        }

        // 枚举根目录的所有exe文件
        let total_exe_files: Vec<PathBuf> = WalkDir::new(&root_path.path()).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().to_ascii_lowercase() == "exe")
            .filter(|file| if config_info.is_none() { true } else { &config_info.as_ref().unwrap().ignore.iter().filter(|&item| file.file_name().to_str().unwrap().contains(item)).count() == &0 })
            .map(|file| file.into_path()).collect();
        if total_exe_files.is_empty() {
            continue;
        }

        // 尝试安装软件
        if install  {
            for install_script in WalkDir::new(&root_path.path()).max_depth(1).into_iter().filter_map(|e| e.ok())
                .filter(|file|file.path().is_file()){
                let mut matched = false;
                let filename = install_script.file_name().to_str().unwrap().to_lowercase();

                // 通配符匹配逻辑
                if let Some(config_info) = &config_info {
                    if !config_info.scripts.is_empty() {
                        matched = config_info.scripts.iter().any(|rule| matches_glob(rule, &filename));
                    }
                }

                // 默认脚本规则
                if filename.to_ascii_lowercase().contains("setup.cmd")  || filename.to_ascii_lowercase().contains("setup.bat") || filename.to_ascii_lowercase().contains("install.cmd") || filename.to_ascii_lowercase().contains("install.bat") {
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
        let other_files = WalkDir::new(&root_path.path()).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().to_ascii_lowercase() != "exe")
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().to_ascii_lowercase() != "ico");
        if other_files.count() == 0 {
            // 单文件程序
            for program in total_exe_files {
                // 处理快捷方式信息
                let lnk_name = get_lnk_alia(&config_info, &program);
                let cmdline = get_lnk_cmdline(&config_info, &program);
                let icon = get_lnk_icon(&config_info, &program);

                lnk_count += 1;
                if list_mode {
                    println!("{}", program.to_str().unwrap());
                } else {
                    write_console(ConsoleType::Info, &format!("Create Shortcut: {}", program.to_str().unwrap()));
                    if createdir {
                        let parent_path = lnk_path.join(program.parent().unwrap().file_stem().unwrap());
                        fs::create_dir_all(&parent_path).ok();
                        let shortcut_file = parent_path.join(format!("{}.lnk", lnk_name));
                        create_shortcut(&program, &shortcut_file, cmdline, icon).ok();
                    } else {
                        let shortcut_file = lnk_path.join(format!("{}.lnk", lnk_name));
                        create_shortcut(&program, &shortcut_file, cmdline, icon).ok();
                    }
                }
            }
            continue;
        }
        // 绿色软件
        let get_main_program = || {
            // 1.判断特定规则
            if let Some(config_info) = &config_info {
                for item in config_info.lnkInfo.iter() {
                    let file_path = root_path.path().join(&item.name);
                    // 判断是否指定路径
                    if file_path.exists() {
                        return file_path;
                    }
                }
            }

            // 2.判断程序名是否包含目录名(转小写+移除空格+去符号)
            let root_path_name = root_path.path().file_name().unwrap().to_str().unwrap()
                .to_lowercase().replace(" ", "").replace("-", "").replace("_", "");
            for exe_file in total_exe_files.iter() {
                let exe_stem = exe_file.file_stem().unwrap().to_str().unwrap().to_lowercase()
                    .replace(" ", "").replace("-", "").replace("_", "");
                // 双向包含检测：目录名包含程序名 或 程序名包含目录名
                if root_path_name.contains(&exe_stem) || exe_stem.contains(&root_path_name) {
                    return exe_file.to_path_buf();
                }
            }

            // 3.程序大小最大的即为主程序
            total_exe_files.iter().max_by(|x, y| x.metadata().unwrap().len().cmp(&y.metadata().unwrap().len())).unwrap().to_path_buf()
        };
        let main_program = get_main_program();

        // 处理快捷方式信息
        let lnk_name = get_lnk_alia(&config_info, &main_program);
        let cmdline = get_lnk_cmdline(&config_info, &main_program);
        let icon = get_lnk_icon(&config_info, &main_program);

        lnk_count += 1;

        // 标记父目录已处理
        processed_dirs.insert(main_program.parent().unwrap().to_path_buf());

        // 创建快捷方式
        if list_mode {
            println!("{}", main_program.to_str().unwrap());
        } else {
            write_console(ConsoleType::Info, &format!("Create Shortcut: {}", main_program.to_str().unwrap()));
            if createdir {
                let parent_path = lnk_path.join(main_program.parent().unwrap().file_stem().unwrap());
                fs::create_dir_all(&parent_path).ok();
                let shortcut_file = parent_path.join(format!("{}.lnk", lnk_name));
                create_shortcut(&main_program, &shortcut_file, cmdline, icon).ok();
            } else {
                let shortcut_file = lnk_path.join(format!("{}.lnk", lnk_name));
                create_shortcut(&main_program, &shortcut_file, cmdline, icon).ok();
            }
        }
    }

    if list_mode {
        return;
    }

    let duration = start_time.elapsed().as_secs_f32();
    if lnk_count == 0 {
        write_console(
            ConsoleType::Error,
            &format!(
                "No shortcuts created, Time: {:.2}s, Subfolders scanned: {}, Shortcuts created: {}.",
                duration, folder_count, lnk_count
            )
        );
    } else {
        write_console(
            ConsoleType::Success,
            &format!(
                "Operation completed, Time: {:.2}s, Subfolders scanned: {}, Shortcuts created: {}.",
                duration, folder_count, lnk_count
            )
        );
    }
}

/// 处理特殊环境变量
pub fn process_env(path: &Path) -> PathBuf {
    let mut output = path.display().to_string().to_lowercase();
    // 处理 %Desktop% 和 %Programs%
    if let Ok(user_profile) = env::var("USERPROFILE") {
        let desktop = PathBuf::from(&user_profile).join("Desktop");
        output = output.replace("%desktop%", &desktop.to_string_lossy());

        let programs = PathBuf::from(&user_profile).join("AppData/Roaming/Microsoft/Windows/Start Menu/Programs");
        output = output.replace("%programs%", &programs.to_string_lossy());
    }

    // 处理系统环境变量（如 %APPDATA%）
    for (key, value) in env::vars() {
        let placeholder = format!("%{}%", key.to_lowercase());
        output = output.replace(&placeholder, &value);
    }

    PathBuf::from(output)
}

/// 获取快捷方式别名
fn get_lnk_alia(config_info: &Option<ConfigInfo>, program: &Path) -> String {
    if let Some(config_info) = &config_info {
        for item in config_info.lnkInfo.iter() {
            if item.name == program.file_name().unwrap().to_str().unwrap() {
                if item.alia.is_empty() {
                    return program.file_stem().unwrap().to_str().unwrap().to_string();
                }
                return item.alia.clone();
            }
        }
    }
    program.file_stem().unwrap().to_str().unwrap().to_string()
}

/// 获取命令行参数
fn get_lnk_cmdline(config_info: &Option<ConfigInfo>, program: &Path) -> Option<String> {
    if let Some(config_info) = &config_info {
        for item in config_info.lnkInfo.iter() {
            if item.name == program.file_name()?.to_str()? {
                if item.cmdline.is_empty() {
                    return None;
                }
                return Some((*item.cmdline).to_string());
            }
        }
    }
    None
}

/// 获取图标路径
fn get_lnk_icon(config_info: &Option<ConfigInfo>, program: &Path) -> Option<String> {
    if let Some(config_info) = config_info {
        // 优先从配置文件获取
        for item in config_info.lnkInfo.iter() {
            if item.name == program.file_name()?.to_str()? && !item.icon.is_empty() {
                let icon_path = process_env(Path::new(&item.icon));
                if icon_path.exists() {
                    return Some(icon_path.to_str()?.to_string());
                } else {
                    write_console(ConsoleType::Warning, &format!("Icon not found: {}", icon_path.to_str()?));
                }
            }
        }
    }
    // 检查程序目录下同名ico文件
    let default_icon = program.parent()?.join(
        format!("{}.ico", program.file_stem()?.to_str()?)
    );
    if default_icon.exists() {
        return Some(default_icon.to_str()?.to_string());
    }
    None
}


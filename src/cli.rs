use std::{env, fs};
use std::fs::File;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashSet;
use clap::{Parser};
use walkdir::WalkDir;
use anyhow::Result;

use crate::{ConfigInfo, ConsoleType, createShortcut, writeConsole};
use crate::utils::matches_glob;

#[derive(Parser, Debug)]
#[clap(version)]
#[clap(propagate_version = true)]
struct Cli {
    /// 程序目录
    #[clap(help="Program Path")]
    targetPath: PathBuf,
    /// 快捷方式路径
    #[clap(help="Shortcut Path")]
    lnkPath: PathBuf,
    /// 配置文件路径
    #[clap(help="Config Path")]
    configPath: Option<PathBuf>,
    /// 是否建立目录
    #[clap(help="Create a directory")]
    #[clap(short, long)]
    createdir: bool,
    /// 安装程序脚本
    #[clap(help="Install program script")]
    #[clap(short, long)]
    install: bool,
    /// 仅列出程序路径（不创建快捷方式）
    #[clap(help="Only list program path")]
    #[clap(short, long)]
    list: bool,
}

pub fn cli() {
    let cli: Cli = Cli::parse();
    AutoShortcut(&*cli.targetPath, &*processEnv(&cli.lnkPath), cli.configPath, cli.createdir, cli.install, cli.list);
}

/// 处理特殊环境变量
pub fn processEnv(path: &Path) -> PathBuf {
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

/// 自动创建快捷方式
///
/// 参数
/// - `targetPath`: 程序路径
/// - `lnkPath`: 快捷方式路径
/// - `configPath`: 配置文件路径
pub fn AutoShortcut(targetPath: &Path, lnkPath: &Path, configPath: Option<PathBuf>, createdir: bool, install: bool, list_mode: bool) {
    // 初始化统计计数器
    let mut folder_count = 0;
    let mut lnk_count = 0;
    let start_time = std::time::Instant::now();

    // 已处理目录集合
    let mut processed_dirs = HashSet::new();

    // 读取配置文件信息
    let mut configInfo = None;
    if let Some(config) = configPath {
        if config.exists() {
            if let Ok(config) = parse_config_file(&config) {
                configInfo = Some(config);
            } else {
                writeConsole(ConsoleType::Error, "Configuration file parsing failed");
                return;
            }
        } else {
            writeConsole(ConsoleType::Error, "config file does not exist");
            return;
        }
    }


    // 遍历指定目录的程序
    for rootPath in WalkDir::new(targetPath).max_depth(1).into_iter().skip(1).filter_map(|e| e.ok())
        .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().to_ascii_lowercase() == "exe") {
        let program = rootPath.path();

        // 处理快捷方式信息
        let lnkName = getLnkAlia(&configInfo, &program);
        let cmdline = getLnkCmdline(&configInfo, &program);
        let icon = getLnkIcon(&configInfo, &program);

        lnk_count += 1;
        if list_mode {
            println!("{}", program.to_str().unwrap());
        } else {
            writeConsole(ConsoleType::Info, &format!("Create Shortcut: {}", program.to_str().unwrap()));
            let shortcutFile = lnkPath.join(format!("{}.lnk", lnkName));
            createShortcut(&program, &*shortcutFile, cmdline, icon).ok();
        }
    }

    // 获取搜索深度
    let search_depth = configInfo.as_ref().map(|c| { if c.searchDepth == 0 {2}  else {c.searchDepth} }).unwrap_or(2);

    // 遍历指定目录的子目录
    for rootPath in WalkDir::new(targetPath).max_depth(search_depth).into_iter().skip(1).filter_map(|e| e.ok()).filter(|file| file.path().is_dir()) {

        // 检查是否已处理该目录
        if processed_dirs.contains(rootPath.path().parent().unwrap()) {
            continue;
        }

        folder_count += 1; // 统计遍历的文件夹

        // 排除 有文件夹但无文件 的目录
        if WalkDir::new(rootPath.path()).max_depth(1).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file()).count() == 0 {
            continue;
        }

        // 判断是否为自定义排除目录
        if let Some(configInfo) = &configInfo {
            if configInfo.ignore.iter().filter(|&item| rootPath.path().to_str().unwrap().contains(item)).count() > 0 {
                continue;
            }
        }

        // 枚举根目录的所有exe文件
        let totalExeFiles: Vec<PathBuf> = WalkDir::new(&rootPath.path()).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().to_ascii_lowercase() == "exe")
            .filter(|file| if configInfo.is_none() { true } else { &configInfo.as_ref().unwrap().ignore.iter().filter(|&item| file.file_name().to_str().unwrap().contains(item)).count() == &0 })
            .map(|file| file.into_path()).collect();
        if totalExeFiles.len() == 0 {
            continue;
        }

        // 尝试安装软件
        if install  {
            for installScript in WalkDir::new(&rootPath.path()).max_depth(1).into_iter().filter_map(|e| e.ok())
                .filter(|file|file.path().is_file()){
                let mut matched = false;
                let filename = installScript.file_name().to_str().unwrap().to_lowercase();

                // 通配符匹配逻辑
                if let Some(configInfo) = &configInfo {
                    if !configInfo.scripts.is_empty() {
                        matched = configInfo.scripts.iter().any(|rule| matches_glob(rule, &*filename));
                    }
                }

                // 默认脚本规则
                if filename.to_ascii_lowercase().contains("setup.cmd")  || filename.to_ascii_lowercase().contains("setup.bat") || filename.to_ascii_lowercase().contains("install.cmd") || filename.to_ascii_lowercase().contains("install.bat") {
                    matched = true;
                }
                if matched {
                    writeConsole(ConsoleType::Info, &format!("Install Script: {}", installScript.path().to_str().unwrap()));
                    Command::new(installScript.path()).creation_flags(0x08000000)
                        .current_dir(installScript.path().parent().unwrap_or_else(|| Path::new(".")))
                        .output().ok();
                }
            }
        }

        // 判断根目录是否存在除exe文件以外的文件（判断绿色软件/单文件程序）
        let otherFiles = WalkDir::new(&rootPath.path()).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().to_ascii_lowercase() != "exe")
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default().to_ascii_lowercase() != "ico");
        if otherFiles.count() == 0 {
            // 单文件程序
            for program in totalExeFiles {
                // 处理快捷方式信息
                let lnkName = getLnkAlia(&configInfo, &program);
                let cmdline = getLnkCmdline(&configInfo, &program);
                let icon = getLnkIcon(&configInfo, &program);

                lnk_count += 1;
                if list_mode {
                    println!("{}", program.to_str().unwrap());
                } else {
                    writeConsole(ConsoleType::Info, &format!("Create Shortcut: {}", program.to_str().unwrap()));
                    if createdir {
                        let parentPath = lnkPath.join(program.parent().unwrap().file_stem().unwrap());
                        fs::create_dir_all(&parentPath).ok();
                        let shortcutFile = parentPath.join(format!("{}.lnk", lnkName));
                        createShortcut(&program, &shortcutFile, cmdline, icon).ok();
                    } else {
                        let shortcutFile = lnkPath.join(format!("{}.lnk", lnkName));
                        createShortcut(&program, &*shortcutFile, cmdline, icon).ok();
                    }
                }
            }
            continue;
        }
        // 绿色软件
        let getMainProgram = || {
            // 1.判断特定规则
            if let Some(configInfo) = &configInfo {
                for item in configInfo.lnkInfo.iter() {
                    let filePath = rootPath.path().join(&item.name);
                    // 判断是否指定路径
                    if filePath.exists() {
                        return filePath;
                    }
                }
            }

            // 2.判断程序名是否包含目录名(转小写+移除空格+去符号)
            let rootPathName = rootPath.path().file_name().unwrap().to_str().unwrap()
                .to_lowercase().replace(" ", "").replace("-", "").replace("_", "");
            for exeFile in totalExeFiles.iter() {
                let exeStem = exeFile.file_stem().unwrap().to_str().unwrap().to_lowercase()
                    .replace(" ", "").replace("-", "").replace("_", "");
                // 双向包含检测：目录名包含程序名 或 程序名包含目录名
                if rootPathName.contains(&exeStem) || exeStem.contains(&rootPathName) {
                    return exeFile.to_path_buf();
                }
            }

            // 3.程序大小最大的即为主程序
            totalExeFiles.iter().max_by(|x, y| x.metadata().unwrap().len().cmp(&y.metadata().unwrap().len())).unwrap().to_path_buf()
        };
        let mainProgram = getMainProgram();

        // 处理快捷方式信息
        let lnkName = getLnkAlia(&configInfo, &mainProgram);
        let cmdline = getLnkCmdline(&configInfo, &mainProgram);
        let icon = getLnkIcon(&configInfo, &mainProgram);

        lnk_count += 1;

        // 标记父目录已处理
        processed_dirs.insert(mainProgram.parent().unwrap().to_path_buf());

        // 创建快捷方式
        if list_mode {
            println!("{}", mainProgram.to_str().unwrap());
        } else {
            writeConsole(ConsoleType::Info, &format!("Create Shortcut: {}", mainProgram.to_str().unwrap()));
            if createdir {
                let parentPath = lnkPath.join(mainProgram.parent().unwrap().file_stem().unwrap());
                fs::create_dir_all(&parentPath).ok();
                let shortcutFile = parentPath.join(format!("{}.lnk", lnkName));
                createShortcut(&mainProgram, &shortcutFile, cmdline, icon).ok();
            } else {
                let shortcutFile = lnkPath.join(format!("{}.lnk", lnkName));
                createShortcut(&mainProgram, &shortcutFile, cmdline, icon).ok();
            }
        }
    }

    if list_mode {
        return;
    }

    let duration = start_time.elapsed().as_secs_f32();
    if lnk_count == 0 {
        writeConsole(
            ConsoleType::Error,
            &format!(
                "No shortcuts created, Time: {:.2}s, Subfolders scanned: {}, Shortcuts created: {}.",
                duration, folder_count, lnk_count
            )
        );
    } else {
        writeConsole(
            ConsoleType::Success,
            &format!(
                "Operation completed, Time: {:.2}s, Subfolders scanned: {}, Shortcuts created: {}.",
                duration, folder_count, lnk_count
            )
        );
    }
}

/// 解析配置文件
pub fn parse_config_file(path: &Path) -> Result<ConfigInfo> {
    let config_dir = path.parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid config file path"))?
        .to_string_lossy();

    let mut config_content = String::new();
    File::open(path)?.read_to_string(&mut config_content)?;

    // 替换 %cd% 为配置文件所在目录
    let mut expanded = config_content
        .replace("%cd%", &config_dir.replace("\\","\\\\"))
        .replace("%CD%", &config_dir.replace("\\","\\\\"));

    // 处理系统环境变量（如 %APPDATA%）
    for (key, value) in env::vars() {
        let placeholder = format!("%{}%", key.to_lowercase());
        expanded = expanded.replace(&placeholder, &value.replace("\\","\\\\"));
    }

    Ok(serde_json::from_str(&expanded)?)
}

/// 获取快捷方式别名
fn getLnkAlia(configInfo: &Option<ConfigInfo>, program: &Path) -> String {
    if let Some(configInfo) = &configInfo {
        for item in configInfo.lnkInfo.iter() {
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
fn getLnkCmdline(configInfo: &Option<ConfigInfo>, program: &Path) -> Option<String> {
    if let Some(configInfo) = &configInfo {
        for item in configInfo.lnkInfo.iter() {
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
fn getLnkIcon(configInfo: &Option<ConfigInfo>, program: &Path) -> Option<String> {
    if let Some(configInfo) = configInfo {
        // 优先从配置文件获取
        for item in configInfo.lnkInfo.iter() {
            if item.name == program.file_name()?.to_str()? && !item.icon.is_empty() {
                let icon_path = processEnv(Path::new(&item.icon));
                if icon_path.exists() {
                    return Some(icon_path.to_str()?.to_string());
                } else {
                    writeConsole(ConsoleType::Warning, &format!("Icon not found: {}", icon_path.to_str()?));
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

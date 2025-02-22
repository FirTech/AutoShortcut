use std::{env, fs};
use std::fs::File;
use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use clap::{Parser};
use walkdir::WalkDir;
use anyhow::Result;

use crate::{ConfigInfo, ConsoleType, createShortcut, writeConsole};

#[derive(Parser, Debug)]
#[clap(version)]
#[clap(propagate_version = true)]
struct Cli {
    /// 程序目录
    targetPath: PathBuf,
    /// 图标路径
    lnkPath: PathBuf,
    /// 配置文件路径
    configPath: Option<PathBuf>,
    /// 是否建立目录
    #[clap(short, long)]
    createdir: bool,
    /// 安装程序脚本
    #[clap(short, long)]
    install: bool,
}

pub fn cli() {
    let cli: Cli = Cli::parse();
    AutoShortcut(&*cli.targetPath, &*processEnv(&cli.lnkPath), cli.configPath, cli.createdir, cli.install);
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
pub fn AutoShortcut(targetPath: &Path, lnkPath: &Path, configPath: Option<PathBuf>, createdir: bool, install: bool) {
    // 初始化统计计数器
    let mut folder_count = 0;
    let mut lnk_count = 0;

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
        .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default() == "exe") {
        let program = rootPath.path();

        // 处理快捷方式信息
        let lnkName = getLnkAlia(&configInfo, &program);
        let cmdline = getLnkCmdline(&configInfo, &program);
        let icon = getLnkIcon(&configInfo, &program);

        lnk_count += 1;
        writeConsole(ConsoleType::Info, &format!("Create Shortcut: {}", program.to_str().unwrap()));
        let shortcutFile = lnkPath.join(format!("{}.lnk", lnkName));
        createShortcut(&program, &*shortcutFile, cmdline, icon).ok();
    }

    // 遍历指定目录的子目录
    for rootPath in WalkDir::new(targetPath).max_depth(2).into_iter().skip(1).filter_map(|e| e.ok())
        .filter(|file| file.path().is_dir()) {
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
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default() == "exe")
            .filter(|file| if configInfo.is_none() { true } else { &configInfo.as_ref().unwrap().ignore.iter().filter(|&item| file.file_name().to_str().unwrap().contains(item)).count() == &0 })
            .map(|file| file.into_path()).collect();
        if totalExeFiles.len() == 0 {
            continue;
        }

        // 尝试安装软件
        if install {
            for installScript in WalkDir::new(&rootPath.path()).max_depth(1).into_iter().filter_map(|e| e.ok())
                .filter(|file| file.file_name().to_str().unwrap() == "setup.cmd" || file.file_name().to_str().unwrap() == "setup.bat" || file.file_name().to_str().unwrap() == "install.cmd" || file.file_name().to_str().unwrap() == "install.bat") {
                writeConsole(ConsoleType::Info, &format!("Install Script: {}", installScript.path().to_str().unwrap()));
                Command::new(installScript.path()).creation_flags(0x08000000)
                    .output().ok();
            }
        }

        // 判断根目录是否存在除exe文件以外的文件（判断绿色软件/单文件程序）
        let otherFiles = WalkDir::new(&rootPath.path()).into_iter().filter_map(|e| e.ok())
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default() != "exe")
            .filter(|file| file.path().is_file() && file.path().extension().unwrap_or_default() != "ico");
        if otherFiles.count() == 0 {
            // 单文件程序
            for program in totalExeFiles {
                // 处理快捷方式信息
                let lnkName = getLnkAlia(&configInfo, &program);
                let cmdline = getLnkCmdline(&configInfo, &program);
                let icon = getLnkIcon(&configInfo, &program);

                lnk_count += 1;
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
            continue;
        }
        // 绿色软件
        let getMainProgram = || {
            // 判断特定规则
            if let Some(configInfo) = &configInfo {
                for item in configInfo.lnkInfo.iter() {
                    let filePath = rootPath.path().join(&item.name);
                    // 判断是否指定路径
                    if filePath.exists() {
                        return filePath;
                    }
                }
            }

            // 判断程序名是否包含目录名
            let rootPathName = rootPath.path().file_name().unwrap().to_str().unwrap();
            for exeFile in totalExeFiles.iter() {
                if rootPathName.contains(exeFile.file_stem().unwrap().to_str().unwrap()) {
                    return exeFile.to_path_buf();
                }
            }

            // 程序大小最大的即为主程序
            totalExeFiles.iter().max_by(|x, y| x.metadata().unwrap().len().cmp(&y.metadata().unwrap().len())).unwrap().to_path_buf()
        };
        let mainProgram = getMainProgram();

        // 处理快捷方式信息
        let lnkName = getLnkAlia(&configInfo, &mainProgram);
        let cmdline = getLnkCmdline(&configInfo, &mainProgram);
        let icon = getLnkIcon(&configInfo, &mainProgram);

        lnk_count += 1;
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
    writeConsole(
        ConsoleType::Success,
        &format!(
            "Operation completed, Folders scanned: {}, Shortcuts created: {}.",
            folder_count, lnk_count
        )
    );
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
                return (*item.alia).to_string();
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

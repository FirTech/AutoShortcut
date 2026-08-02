use crate::config::ConfigInfo;
use crate::console::{ConsoleType, write_console};
use crate::directory::analyze_directory_tree;
use crate::execution::{AutoExecutionContext, execute_directory};
use crate::shortcut::create_program_shortcut;
use crate::utils::validate_shortcut_name_for_config;
use anyhow::{Result, anyhow};
use rust_i18n::t;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Creates shortcuts exclusively from explicit configuration entries.
pub(crate) fn config_shortcut(
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

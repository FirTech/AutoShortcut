use crate::config::{DEFAULT_NAME_TEMPLATE, Lnk, Template};
use crate::console::{ConsoleType, write_console};
use crate::template::process_template;
use crate::utils::{
    create_shortcut, get_shortcut_target, is_running_under_wow64, parse_hotkey, parse_icon_spec,
    replace_ignore_case, resolve_relative_path, validate_shortcut_name_for_config,
};
use anyhow::{Result, anyhow};
use rust_i18n::t;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

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
pub(crate) fn create_program_shortcut(
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

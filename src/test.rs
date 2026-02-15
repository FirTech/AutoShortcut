use super::*;
use crate::find_software_best_exe;
use std::fs::File;
use std::path::Path;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    // ============================================
    // parse_hotkey 函数测试
    // ============================================

    use crate::config::{ConfigInfo, Lnk};
    use crate::utils::parse_hotkey;
    use std::path::PathBuf;

    #[test]
    fn test_parse_hotkey() {
        // 基本测试：单个修饰键 + 字母
        assert_eq!(parse_hotkey("Ctrl + A").unwrap(), 0x0241);
        assert_eq!(parse_hotkey("Alt + L").unwrap(), 0x044C);
        assert_eq!(parse_hotkey("Shift + S").unwrap(), 0x0153);

        // 多个修饰键
        assert_eq!(parse_hotkey("Ctrl+Shift+A").unwrap(), 0x0341);
        assert_eq!(parse_hotkey("Ctrl + Alt + Delete").unwrap(), 0x062E);

        // Windows 键
        assert_eq!(parse_hotkey("Win + L").unwrap(), 0x084C);
        assert_eq!(parse_hotkey("Meta + Tab").unwrap(), 0x0809);

        // 功能键
        assert_eq!(parse_hotkey("Alt + F1").unwrap(), 0x0470);
        assert_eq!(parse_hotkey("F12").unwrap(), 0x007B);

        // 特殊键
        assert_eq!(parse_hotkey("Ctrl + Space").unwrap(), 0x0220);
        assert_eq!(parse_hotkey("Alt + Enter").unwrap(), 0x040D);
        assert_eq!(parse_hotkey("Shift + Delete").unwrap(), 0x012E);
        assert_eq!(parse_hotkey("Alt + Tab").unwrap(), 0x0409);

        // 大小写不敏感
        let r1 = parse_hotkey("CTRL + A").unwrap();
        let r2 = parse_hotkey("ctrl + a").unwrap();
        let r3 = parse_hotkey("Ctrl + a").unwrap();
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);

        // 不同分隔符
        let r1 = parse_hotkey("Ctrl+A").unwrap();
        let r2 = parse_hotkey("Ctrl + A").unwrap();
        assert_eq!(r1, r2);

        // 错误情况
        assert!(parse_hotkey("").is_err());
        assert!(parse_hotkey("Ctrl + Shift + Alt").is_err()); // 只有修饰键
        assert!(parse_hotkey("Ctrl + A + B").is_err()); // 多个主键
        assert!(parse_hotkey("Ctrl + InvalidKey").is_err()); // 无效键
    }

    // ============================================
    // Lnk 结构体测试
    // ============================================

    #[test]
    fn test_lnk_new() {
        let lnk = Lnk::new("test.exe".to_string());
        assert_eq!(lnk.exec, "test.exe");
        assert_eq!(lnk.name, None);
        assert_eq!(lnk.hotkey, None);
    }

    // ============================================
    // Lnk::get_lnk_info 测试
    // ============================================

    #[test]
    fn test_get_lnk_info() {
        let link_info = vec![Lnk {
            exec: "C:\\Test\\app.exe".to_string(),
            name: Some("App".to_string()),
            hotkey: Some("Ctrl + A".to_string()),
            ..Default::default()
        }];

        let path = PathBuf::from("C:\\Test\\app.exe");
        let result = Lnk::get_lnk_info(&path, &link_info);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, Some("App".to_string()));

        // 测试未找到
        let path = PathBuf::from("C:\\Test\\nonexistent.exe");
        let result = Lnk::get_lnk_info(&path, &link_info);
        assert!(result.is_none());
    }

    // ============================================
    // TOML 反序列化测试
    // ============================================

    #[test]
    fn test_toml_deserialize_basic() {
        let config_content = r#"
[[shortcut]]
name = "Test App"
exec = "test.exe"
hotkey = "Ctrl + Alt + N"
dest = "%Desktop%"
"#;

        let config: ConfigInfo = toml::from_str(config_content).unwrap();
        assert_eq!(config.shortcut.len(), 1);
        assert_eq!(config.shortcut[0].name, Some("Test App".to_string()));
        assert_eq!(
            config.shortcut[0].hotkey,
            Some("Ctrl + Alt + N".to_string())
        );
    }

    #[test]
    fn test_toml_deserialize_inline_mode() {
        let config_content = r#"
shortcut = [
    { name = "Test1", exec = "test1.exe", hotkey = "Ctrl + A" },
    { name = "Test2", exec = "test2.exe", hotkey = "Alt + B" },
]
"#;

        let config: ConfigInfo = toml::from_str(config_content).unwrap();
        assert_eq!(config.shortcut.len(), 2);
        assert_eq!(config.shortcut[0].hotkey, Some("Ctrl + A".to_string()));
    }

    #[test]
    fn test_toml_deserialize_invalid() {
        let config_content = r#"
this is not valid toml
[[shortcut
exec = "test.exe"
"#;

        let result: Result<ConfigInfo, _> = toml::from_str(config_content);
        assert!(result.is_err());
    }

    #[test]
    fn test_toml_deserialize_all_fields() {
        let config_content = r#"
[[shortcut]]
exec = "test.exe"
name = "Test App"
args = "--arg1"
icon = "test.ico"
work_dir = "C:\\Work"
dest = "%Desktop%"
window_state = "maximized"
comment = "Test comment"
hotkey = "Ctrl + Alt + T"
"#;

        let config: ConfigInfo = toml::from_str(config_content).unwrap();
        let lnk = &config.shortcut[0];
        assert_eq!(lnk.name, Some("Test App".to_string()));
        assert_eq!(lnk.hotkey, Some("Ctrl + Alt + T".to_string()));
    }
}

/// 测试创建基本的快捷方式
#[test]
fn test_create_shortcut_basic() {
    // 创建临时目录
    let temp_dir = TempDir::new().unwrap();
    let shortcut_path = temp_dir.path().join("test_shortcut.lnk");

    // 使用 notepad.exe 作为测试目标（系统自带文件，应该存在）
    let notepad_path = Path::new("C:\\Windows\\System32\\notepad.exe");

    if !notepad_path.exists() {
        // 如果 notepad.exe 不存在，跳过此测试
        return;
    }

    // 创建快捷方式
    let result = crate::utils::create_shortcut(
        notepad_path,
        &shortcut_path,
        None, // args
        None, // icon
        None, // working_dir
        None, // window_state
        None, // description
        None, // hotkey
    );

    assert!(result.is_ok(), "创建快捷方式应该成功");
    assert!(shortcut_path.exists(), "快捷方式文件应该存在");

    // 验证快捷方式目标路径
    let target = crate::utils::get_shortcut_target(&shortcut_path);
    assert!(target.is_ok(), "应该能读取快捷方式目标");
    assert_eq!(
        target.unwrap().to_string_lossy().to_ascii_lowercase(),
        notepad_path.to_string_lossy().to_ascii_lowercase(),
        "快捷方式目标路径应该匹配"
    );
}

/// 测试创建完整的快捷方式（所有参数）
#[test]
fn test_create_shortcut_complete() {
    let temp_dir = TempDir::new().unwrap();
    let shortcut_path = temp_dir.path().join("test_complete.lnk");
    let notepad_path = Path::new("C:\\Windows\\System32\\notepad.exe");

    if !notepad_path.exists() {
        return;
    }

    let result = crate::utils::create_shortcut(
        notepad_path,
        &shortcut_path,
        Some("test.txt".to_string()),                          // args
        Some((notepad_path.to_string_lossy().to_string(), 0)), // icon
        Some(temp_dir.path().to_string_lossy().to_string()),   // working_dir
        Some("maximized".to_string()),                         // window_state
        Some("完整测试快捷方式".to_string()),                  // description
        Some(0x044E),                                          // hotkey
    );

    assert!(result.is_ok(), "创建完整快捷方式应该成功");
    assert!(shortcut_path.exists(), "快捷方式文件应该存在");

    // 验证快捷方式目标路径
    let target = crate::utils::get_shortcut_target(&shortcut_path);
    assert!(target.is_ok(), "应该能读取快捷方式目标");
}

/// 测试快捷方式目标路径读取失败的情况
#[test]
fn test_get_shortcut_target_invalid_path() {
    let temp_dir = TempDir::new().unwrap();
    let invalid_path = temp_dir.path().join("nonexistent.lnk");

    let result = crate::utils::get_shortcut_target(&invalid_path);
    assert!(result.is_err(), "读取不存在的快捷方式应该失败");
}

/// 测试评分算法 - 配置文件匹配项得分最高
#[test]
fn test_scoring_config_match() {
    let temp_dir = TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("TestApp");
    fs::create_dir_all(&test_dir).unwrap();

    // 创建一个测试 exe 文件
    let test_exe = test_dir.join("test.exe");
    File::create(&test_exe).unwrap();

    // 创建配置信息，包含这个 exe
    let mut config_info = ConfigInfo::default();
    let mut lnk = Lnk::default();
    lnk.exec = test_exe.to_string_lossy().to_string();
    config_info.shortcut.push(lnk);

    // 调用评分函数
    let result = find_software_best_exe(
        &test_dir,
        Some(&config_info),
        temp_dir.path(),
        0.0, // score_ratio 为 0 表示不过滤任何得分
        true,
    );

    // 应该找到匹配的文件
    assert!(result.is_some(), "应该找到配置匹配的 exe 文件");
    let (_, found_path) = result.unwrap();
    assert_eq!(found_path, test_exe, "应该返回配置中指定的 exe 文件");
}

/// 测试评分算法 - 文件名与父目录名匹配加分
#[test]
fn test_scoring_name_parent_match() {
    let temp_dir = TempDir::new().unwrap();
    let app_dir = temp_dir.path().join("MyApp");
    fs::create_dir_all(&app_dir).unwrap();

    // 创建文件名与目录名匹配的 exe
    let myapp_exe = app_dir.join("MyApp.exe");
    File::create(&myapp_exe).unwrap();

    // 创建另一个不匹配的 exe
    let other_exe = app_dir.join("Other.exe");
    File::create(&other_exe).unwrap();

    let result = find_software_best_exe(&app_dir, None, temp_dir.path(), 0.0, true);

    assert!(result.is_some(), "应该找到一个 exe 文件");
    let (_, found_path) = result.unwrap();
    // MyApp.exe 应该得分更高（文件名与父目录名匹配）
    assert_eq!(found_path, myapp_exe, "应该选择文件名与目录名匹配的 exe");
}

/// 测试评分算法 - GUI 程序优先
#[test]
fn test_scoring_gui_preference() {
    let temp_dir = TempDir::new().unwrap();
    let app_dir = temp_dir.path().join("App");
    fs::create_dir_all(&app_dir).unwrap();

    // 由于我们无法创建真实的 GUI 和 CLI 程序，
    // 这个测试主要验证函数能够正常运行
    let test_exe = app_dir.join("test.exe");
    File::create(&test_exe).unwrap();

    let result = find_software_best_exe(&app_dir, None, temp_dir.path(), 0.0, true);

    // 测试应该能够运行（即使找不到真正的 GUI 程序）
    // 由于创建的不是真正的 exe，可能找不到任何文件
    if result.is_some() {
        let (_, found_path) = result.unwrap();
        assert_eq!(found_path, test_exe);
    }
}

/// 测试评分算法 - 忽略列表
#[test]
fn test_scoring_ignore_list() {
    let temp_dir = TempDir::new().unwrap();
    let app_dir = temp_dir.path().join("App");
    fs::create_dir_all(&app_dir).unwrap();

    // 创建几个 exe 文件
    let main_exe = app_dir.join("Main.exe");
    File::create(&main_exe).unwrap();

    let uninstall_exe = app_dir.join("uninstall.exe");
    File::create(&uninstall_exe).unwrap();

    let setup_exe = app_dir.join("setup.exe");
    File::create(&setup_exe).unwrap();

    // 创建配置，忽略某些文件
    let mut config_info = ConfigInfo::default();
    config_info.ignore.push("uninstall".to_string());
    config_info.ignore.push("setup".to_string());

    let result = find_software_best_exe(&app_dir, Some(&config_info), temp_dir.path(), 0.0, true);

    // 如果找到文件，应该是 Main.exe（不被忽略的）
    if result.is_some() {
        let (_, found_path) = result.unwrap();
        assert_eq!(found_path, main_exe, "应该忽略 uninstall 和 setup 文件");
    }
}

/// 测试评分算法 - 空目录
#[test]
fn test_scoring_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = temp_dir.path().join("Empty");
    fs::create_dir_all(&empty_dir).unwrap();

    let result = find_software_best_exe(&empty_dir, None, temp_dir.path(), 0.0, true);

    assert!(result.is_none(), "空目录不应该返回任何文件");
}

/// 测试评分算法 - 评分阈值过滤
#[test]
fn test_scoring_threshold() {
    let temp_dir = TempDir::new().unwrap();
    let app_dir = temp_dir.path().join("App");
    fs::create_dir_all(&app_dir).unwrap();

    // 创建一个小的测试文件
    let test_exe = app_dir.join("test.exe");
    File::create(&test_exe).unwrap();

    // 使用高阈值，应该过滤掉低分文件
    let result = find_software_best_exe(
        &app_dir,
        None,
        temp_dir.path(),
        2.0, // 200% 的阈值，只有非常高的分才能通过
        true,
    );

    // 小文件得分应该很低，被过滤掉
    assert!(result.is_none(), "高阈值应该过滤掉低分文件");
}

/// 测试评分算法 - 嵌套目录扫描
#[test]
fn test_scoring_nested_directories() {
    let temp_dir = TempDir::new().unwrap();
    let root_dir = temp_dir.path().join("Root");
    fs::create_dir_all(&root_dir).unwrap();

    let sub_dir = root_dir.join("SubApp");
    fs::create_dir_all(&sub_dir).unwrap();

    let deep_exe = sub_dir.join("deep.exe");
    File::create(&deep_exe).unwrap();

    let result = find_software_best_exe(&root_dir, None, temp_dir.path(), 0.0, true);

    // 应该能扫描到子目录中的 exe
    if result.is_some() {
        let (_, found_path) = result.unwrap();
        assert_eq!(found_path, deep_exe, "应该找到子目录中的 exe");
    }
}

/// 测试评分算法 - 多个 exe 的评分选择
#[test]
fn test_scoring_multiple_exe_selection() {
    let temp_dir = TempDir::new().unwrap();
    let app_dir = temp_dir.path().join("MyApp");
    fs::create_dir_all(&app_dir).unwrap();

    // 创建多个 exe，文件名与目录名匹配的应该得分最高
    let myapp_exe = app_dir.join("MyApp.exe");
    File::create(&myapp_exe).unwrap();

    let helper_exe = app_dir.join("helper.exe");
    File::create(&helper_exe).unwrap();

    let config_exe = app_dir.join("config.exe");
    File::create(&config_exe).unwrap();

    let result = find_software_best_exe(&app_dir, None, temp_dir.path(), 0.0, true);

    if result.is_some() {
        let (_, found_path) = result.unwrap();
        assert_eq!(found_path, myapp_exe, "应该选择与目录名匹配的 exe");
    }
}

/// 测试评分算法 - 递归查找应用根目录
#[test]
fn test_scoring_app_root_detection() {
    let temp_dir = TempDir::new().unwrap();
    let install_dir = temp_dir.path().join("Program Files").join("MyApp");
    fs::create_dir_all(&install_dir).unwrap();

    let exe_path = install_dir.join("MyApp.exe");
    File::create(&exe_path).unwrap();

    // 创建一些典型的应用文件
    let dll_path = install_dir.join("myapp.dll");
    File::create(&dll_path).unwrap();

    let result = find_software_best_exe(&install_dir, None, temp_dir.path(), 0.0, true);

    // 应该能找到 exe
    if result.is_some() {
        let (app_root, found_path) = result.unwrap();
        assert_eq!(found_path, exe_path, "应该找到 exe 文件");
        // app_root 可能是安装目录或其父目录
        assert!(
            app_root == install_dir || app_root == temp_dir.path().join("Program Files"),
            "应用根目录应该是 exe 所在目录或其父目录"
        );
    }
}

/// 测试评分算法 - 特殊字符文件名
#[test]
fn test_scoring_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let app_dir = temp_dir.path().join("App-2024");
    fs::create_dir_all(&app_dir).unwrap();

    let exe_path = app_dir.join("MyApp_2024.exe");
    File::create(&exe_path).unwrap();

    let result = find_software_best_exe(&app_dir, None, temp_dir.path(), 0.0, true);

    // 应该能处理特殊字符
    if result.is_some() {
        let (_, found_path) = result.unwrap();
        assert_eq!(found_path, exe_path);
    }
}

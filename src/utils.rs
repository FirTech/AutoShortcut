use std::env;
use std::path::{Path, PathBuf};
use mslnk::{MSLinkError, ShellLink};

/// 创建快捷方式
pub fn create_shortcut(target: &Path, link: &Path, args:  Option<String>, icon: Option<String>) -> Result<(), MSLinkError> {
    let mut sl = ShellLink::new(target)?;
    sl.set_arguments(args);
    sl.set_icon_location(icon);
    sl.create_lnk(link)
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

/// glob匹配函数
pub fn matches_glob(pattern: &str, filename: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let filename = filename.to_lowercase();

    // 处理通配符逻辑
    match (pattern.starts_with('*'), pattern.ends_with('*')) {
        // 后缀匹配 *.cmd
        (true, false) => filename.ends_with(&pattern[1..]),
        // 前缀匹配 setup*
        (false, true) => filename.starts_with(&pattern[..pattern.len()-1]),
        // 全匹配 * 或包含匹配 *test*
        _ => filename.contains(pattern.trim_matches('*'))
    }
}

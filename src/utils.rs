use std::path::{Path};
use mslnk::{MSLinkError, ShellLink};

/// 创建快捷方式
pub fn createShortcut(target: &Path, link: &Path, args:  Option<String>, icon: Option<String>) -> Result<(), MSLinkError> {
    let mut sl = ShellLink::new(target)?;
    sl.set_arguments(args);
    sl.set_icon_location(icon);
    sl.create_lnk(link)
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
        _ => filename.contains(&pattern.trim_matches('*'))
    }
}

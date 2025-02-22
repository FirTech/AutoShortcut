use std::path::{Path};
use mslnk::{MSLinkError, ShellLink};

/// 创建快捷方式
pub fn createShortcut(target: &Path, link: &Path, args:  Option<String>, icon: Option<String>) -> Result<(), MSLinkError> {
    let mut sl = ShellLink::new(target)?;
    sl.set_arguments(args);
    sl.set_icon_location(icon);
    sl.create_lnk(link)
}

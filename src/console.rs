use console::style;
use rust_i18n::t;

pub enum ConsoleType {
    /// 信息
    Info,
    /// 成功
    Success,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 调试
    Debug,
}

pub fn write_console(console_type: ConsoleType, message: &str) {
    let title = match &console_type {
        ConsoleType::Info => style(t!("console.info")).cyan(),
        ConsoleType::Success => style(t!("console.success")).green(),
        ConsoleType::Warning => style(t!("console.warning")).yellow(),
        ConsoleType::Error => style(t!("console.error")).red(),
        ConsoleType::Debug => style(t!("console.debug")).magenta()
    };
    println!("  {}      {}", &title, message);
}

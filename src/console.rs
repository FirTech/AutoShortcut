use console::style;

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
        ConsoleType::Info => style("Info   ").cyan(),
        ConsoleType::Success => style("Success").green(),
        ConsoleType::Warning => style("Warning").yellow(),
        ConsoleType::Error => style("Err    ").red(),
        ConsoleType::Debug => style("Debug  ").magenta()
    };
    println!("  {}      {}", &title, message);
}

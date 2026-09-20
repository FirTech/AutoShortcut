use console::style;
use rust_i18n::t;
use std::cmp::PartialEq;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};

static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

#[derive(PartialEq)]
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

impl ConsoleType {
    fn name(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Success => "SUCCESS",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
            Self::Debug => "DEBUG",
        }
    }
}

/// Initializes the optional plain-text log file.
///
/// The file is opened in append mode so separate invocations retain their
/// history. Parent directories are created when necessary.
pub fn init_log_file(path: Option<&Path>) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create log directory: {}", parent.display()))?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open log file: {}", path.display()))?;
    LOG_FILE
        .set(Mutex::new(file))
        .map_err(|_| anyhow::anyhow!("log file has already been initialized"))
}

fn write_log(level: &str, message: &str) {
    let Some(file) = LOG_FILE.get() else {
        return;
    };
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    if let Ok(mut file) = file.lock() {
        let _ = writeln!(file, "[{timestamp}] [{level}] {message}");
    }
}

/// Writes a plain line to stdout and, when enabled, to the log file.
pub fn write_plain(message: &str) {
    println!("{message}");
    write_log("OUTPUT", message);
}

/// 写入控制台
///
/// # 参数
/// - `consoleType`: 控制台类型
/// - `message`: 控制台消息
///
/// # 返回值
/// - `Ok(())`: 写入成功
pub fn write_console(console_type: ConsoleType, message: &str) {
    let title = match &console_type {
        ConsoleType::Info => style(t!("console.info")).cyan(),
        ConsoleType::Success => style(t!("console.success")).green(),
        ConsoleType::Warning => style(t!("console.warning")).yellow(),
        ConsoleType::Error => style(t!("console.error")).red(),
        ConsoleType::Debug => style(t!("console.debug")).magenta(),
    };

    if console_type == ConsoleType::Error {
        eprintln!("  {}      {}", &title, message);
    } else {
        println!("  {}      {}", &title, message);
    }
    write_log(console_type.name(), message);
}

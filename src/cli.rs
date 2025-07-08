use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(version)]
#[clap(propagate_version = true)]
pub struct Cli {
    /// 程序目录
    #[clap(help = "Program Path")]
    #[arg(value_parser = config_dir_validator)]
    pub targetPath: PathBuf,

    /// 快捷方式路径
    #[clap(help = "Shortcut Path")]
    #[arg(value_parser = config_dir_validator)]
    pub lnkPath: PathBuf,

    /// 配置文件路径
    #[clap(help = "Config Path")]
    #[arg(value_parser = config_file_validator)]
    pub configPath: Option<PathBuf>,

    /// 是否建立目录
    #[clap(help = "Create a directory")]
    #[clap(short, long)]
    pub createdir: bool,

    /// 安装程序脚本
    #[clap(help = "Install program script")]
    #[clap(short, long)]
    pub install: bool,

    /// 仅列出程序路径（不创建快捷方式）
    #[clap(help = "Only list program path")]
    #[clap(short, long)]
    pub list: bool,

    /// 调试模式
    #[clap(help = "Debug Model")]
    #[clap(short, long)]
    pub debug: bool,
}

/// 用于 clap 参数解析：验证路径必须为已存在文件。
///
/// 参数:
/// - `s`: 命令行中传入的字符串路径。
///
/// 返回:
/// - `Ok(PathBuf)`: 如果字符串成功解析为 PathBuf 且该路径是已存在的文件。
/// - `Err(String)`: 如果解析失败或路径不是已存在的文件，返回错误信息。
fn exist_file_parser(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);

    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()));
    }

    Ok(path)
}

/// 用于 clap 参数解析：验证路径必须为已存在目录。
///
/// 参数:
/// - `s`: 命令行中传入的字符串路径。
///
/// 返回:
/// - `Ok(PathBuf)`: 如果字符串成功解析为 PathBuf 且该路径是已存在的目录。
/// - `Err(String)`: 如果解析失败或路径不是已存在的目录，返回错误信息。
fn exist_dir_parser(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);

    if !path.exists() {
        return Err(format!("Dir does not exist: {}", path.display()));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a dir: {}", path.display()));
    }

    Ok(path)
}

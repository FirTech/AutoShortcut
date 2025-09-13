use crate::utils::process_env;
use clap::ArgAction;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(version)]
#[clap(propagate_version = true)]
#[command(disable_version_flag = true, arg_required_else_help = true)]
pub struct Cli {
    /// 程序目录
    #[clap(help = "Program Path")]
    #[clap(value_parser = exist_dir_parser, required_unless_present = "config")]
    pub targetPath: Option<PathBuf>,

    /// 快捷方式路径
    #[clap(help = "Shortcut Path")]
    #[clap(value_parser = exist_dir_parser, required_unless_present_any = &["config", "list", "start"]
    )]
    pub lnkPath: Option<PathBuf>,

    /// 配置文件路径
    #[clap(help = "Config Path")]
    #[clap(short = 'c',long, value_parser = exist_file_parser, required_unless_present_any = &["targetPath", "lnkPath"]
    )]
    pub config: Option<PathBuf>,

    /// 匹配配置文件中的信息创建快捷方式
    #[arg(short = 'm', long, requires = "config", help = "Create shortcuts to match profiles")]
    pub only_match: bool,

    /// 是否建立目录
    #[clap(help = "Create parent directory")]
    #[clap(short = 'd', long)]
    pub create_dir: bool,

    /// 安装程序脚本
    #[clap(help = "Install program script")]
    #[clap(short, long)]
    pub install: bool,

    /// 并行运行安装脚本
    #[clap(help = "Run install scripts in parallel instead of sequentially")]
    #[clap(short = 'p', long, requires = "config")]
    pub install_parallel: bool,

    /// 仅列出程序路径（不创建快捷方式）
    #[clap(help = "Only list program path")]
    #[clap(short, long)]
    pub list: bool,

    /// 使用原始文件名做快捷方式名称
    #[clap(short = 'f', long = "use-filename", help = "Use original file name for shortcut")]
    pub use_filename: bool,

    /// 评分阈值
    #[arg(short = 'r', long, help = "Ratio (0.0~1.0) of max possible score to use as threshold")]
    pub score_ratio: Option<f32>,

    /// 调试模式
    #[clap(help = "Debug model")]
    #[clap(long)]
    pub debug: bool,

    /// 启动程序
    #[clap(help = "Run program")]
    #[clap(short, long)]
    pub start: bool,

    /// 版本信息
    #[arg(short = 'v',long = "version",action = ArgAction::Version,help = "Print version")]
    version: Option<bool>,
}

/// 用于 clap 参数解析：验证路径必须为已存在文件。
///
/// # 参数:
/// - `s`: 命令行中传入的字符串路径。
///
/// # 返回值:
/// - `Ok(PathBuf)`: 如果字符串成功解析为 PathBuf 且该路径是已存在的文件。
/// - `Err(String)`: 如果解析失败或路径不是已存在的文件，返回错误信息。
fn exist_file_parser(s: &str) -> Result<PathBuf, String> {
    // 处理变量
    let path = PathBuf::from(process_env(normalize_drive_root(s), None));

    if !path.exists() {
        return Err(format!("File not found: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()));
    }

    Ok(path)
}

/// 用于 clap 参数解析：验证路径必须为已存在目录。
///
/// # 参数:
/// - `s`: 命令行中传入的字符串路径。
///
/// # 返回值:
/// - `Ok(PathBuf)`: 如果字符串成功解析为 PathBuf 且该路径是已存在的目录。
/// - `Err(String)`: 如果解析失败或路径不是已存在的目录，返回错误信息。
fn exist_dir_parser(s: &str) -> Result<PathBuf, String> {
    // 处理变量
    let path = PathBuf::from(process_env(normalize_drive_root(s), None));

    if !path.exists() {
        return Err(format!("Dir does not exist: {}", path.display()));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a dir: {}", path.display()));
    }

    Ok(path)
}

/// 如果 s 是 “X:”（只有盘符），就返回 “X:\”；否则原样返回
fn normalize_drive_root(s: &str) -> String {
    if s.len() == 2 && s.as_bytes()[1] == b':' && s.as_bytes()[0].is_ascii_alphabetic() {
        format!("{}\\", s)
    } else {
        s.to_string()
    }
}

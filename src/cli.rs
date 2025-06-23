use std::path::{PathBuf};
use clap::{Parser};

#[derive(Parser, Debug)]
#[clap(version)]
#[clap(propagate_version = true)]
pub struct Cli {
    /// 程序目录
    #[clap(help="Program Path")]
    pub targetPath: PathBuf,
    /// 快捷方式路径
    #[clap(help="Shortcut Path")]
    pub lnkPath: PathBuf,
    /// 配置文件路径
    #[clap(help="Config Path")]
    pub configPath: Option<PathBuf>,
    /// 是否建立目录
    #[clap(help="Create a directory")]
    #[clap(short, long)]
    pub createdir: bool,
    /// 安装程序脚本
    #[clap(help="Install program script")]
    #[clap(short, long)]
    pub install: bool,
    /// 仅列出程序路径（不创建快捷方式）
    #[clap(help="Only list program path")]
    #[clap(short, long)]
    pub list: bool,
}

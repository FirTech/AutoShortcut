// 禁用变量命名警告
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
// 禁用未使用代码警告
#![allow(dead_code)]

mod console;
mod utils;
mod cli;

use crate::console::{ConsoleType, writeConsole};
use crate::utils::createShortcut;
use serde::{Deserialize, Serialize};
use crate::cli::cli;

/// 程序信息列表
#[derive(Serialize, Deserialize, Debug)]
pub struct LnkInfo {
    /// 程序名
    name: String,
    /// 快捷方式别名
    #[serde(default)]
    alia: String,
    /// 图标
    #[serde(default)]
    icon: String,
    /// 命令行参数
    #[serde(default)]
    cmdline: String,
}

/// 配置文件信息
#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigInfo {
    /// 搜索深度
    #[serde(default)]
    searchDepth: usize,
    /// 忽略列表
    #[serde(default)]
    ignore: Vec<String>,
    /// 脚本规则
    #[serde(default)]
    pub scripts: Vec<String>,
    /// 程序信息列表
    #[serde(default)]
    lnkInfo: Vec<LnkInfo>,
}

fn main() {
    cli();
}

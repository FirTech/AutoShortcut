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
    alia: String,
    /// 命令行参数
    cmdline: String,
}

/// 配置文件信息
#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigInfo {
    /// 忽略列表
    ignore: Vec<String>,
    /// 程序信息列表
    lnkInfo: Vec<LnkInfo>,
}

fn main() {
    cli();
}

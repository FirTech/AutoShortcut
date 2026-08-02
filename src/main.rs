// 禁用变量命名警告
#![allow(non_snake_case)]
// 禁用未使用代码警告
#![allow(dead_code)]

mod cli;
mod config;
mod console;
mod directory;
mod execution;
mod installer;
mod selector;
mod shortcut;
mod template;
mod utils;
mod workflow;

#[cfg(test)]
mod test;

use crate::utils::launched_from_explorer;
use crate::workflow::{auto_shortcut, config_shortcut};
use anyhow::Result;
use clap::Parser;
use rust_i18n::{set_locale, t};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;
use sys_locale::get_locale;

/// 调试模式
static DEBUG: AtomicBool = AtomicBool::new(false);

// 国际化
rust_i18n::i18n!("locales");

fn main() -> Result<()> {
    // 设置国际化
    let system_locale = get_locale().unwrap_or("en".into());
    match system_locale.as_str() {
        "zh-CN" => set_locale("zh-CN"),
        "zh-TW" => set_locale("zh-TW"),
        _ => set_locale("en"),
    }

    // 判断是否从资源管理器启动
    if launched_from_explorer() && env::args().len() == 1 {
        println!("{}", t!("cmdline_tool_tips"));
        sleep(Duration::from_secs(5));
        return Ok(());
    }

    // 处理命令行
    let cli = crate::cli::Cli::parse();
    if cli.debug {
        DEBUG.store(true, Ordering::Relaxed);
    }

    // 配置文件模式
    if cli.config.is_some() && cli.targetPath.is_none() && cli.lnkPath.is_none() {
        let cfg = cli.config.unwrap();
        config_shortcut(
            cfg,
            cli.install,
            cli.install_parallel,
            cli.start,
            cli.use_filename,
        )?;
        return Ok(());
    }

    // 自动搜索模式
    auto_shortcut(
        &cli.targetPath.unwrap(),
        cli.lnkPath.as_deref(),
        cli.config.as_deref(),
        cli.only_match,
        cli.create_dir,
        cli.install,
        cli.install_parallel,
        cli.start,
        cli.list,
        cli.use_filename,
        cli.score_ratio,
    )?;
    Ok(())
}

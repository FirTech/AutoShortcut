use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path};
use serde::{Deserialize, Serialize};

/// 配置文件信息
#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigInfo {
    /// 搜索深度
    #[serde(default)]
    pub searchDepth: usize,
    /// 忽略列表
    #[serde(default)]
    pub ignore: Vec<String>,
    /// 脚本规则
    #[serde(default)]
    pub scripts: Vec<String>,
    /// 程序信息列表
    #[serde(default)]
    pub lnkInfo: Vec<LnkInfo>,
}

/// 程序信息列表
#[derive(Serialize, Deserialize,Clone, Debug)]
pub struct LnkInfo {
    /// 程序名
    pub name: String,
    /// 快捷方式别名
    #[serde(default)]
    pub alia: Option<String>,
    /// 图标
    #[serde(default)]
    pub icon: Option<String>,
    /// 命令行参数
    #[serde(default)]
    pub cmdline: Option<String>,
    /// 指定快捷方式位置
    #[serde(default)]
    pub location: Option<String>,
}

impl ConfigInfo {
    /// 解析配置文件
    pub fn parse_config_file(path: &Path) -> Result<ConfigInfo, Box<dyn Error>> {
        let config_dir = path.parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid config file path"))?
            .to_string_lossy();

        let mut config_content = String::new();
        File::open(path)?.read_to_string(&mut config_content)?;

        // 替换 %cd% 为配置文件所在目录
        let mut expanded = config_content
            .replace("%cd%", &config_dir.replace("\\","\\\\"))
            .replace("%CD%", &config_dir.replace("\\","\\\\"));

        // 处理系统环境变量（如 %APPDATA%）
        for (key, value) in env::vars() {
            let placeholder = format!("%{}%", key.to_lowercase());
            expanded = expanded.replace(&placeholder, &value.replace("\\","\\\\"));
        }

        Ok(serde_json::from_str(&expanded)?)
    }
}

impl LnkInfo {
    pub fn get_lnk_info(link_info: &Vec<LnkInfo>, program_path: &Path) -> Option<LnkInfo> {
        for item in link_info {
            if item.name == program_path.file_name().unwrap().to_str().unwrap() {
                return Some(item.clone());
            }
        }
        None
    }
}

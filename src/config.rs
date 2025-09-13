use crate::console::{write_console, ConsoleType};
use crate::utils::process_env;
use crate::DEBUG;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use toml::{Table, Value};

/// 默认名称模板
pub const DEFAULT_NAME_TEMPLATE: &str = "{product ? product : {desc ? desc : {orig_filename ? orig_filename : stem}}}";

fn default_name_template() -> Option<String> {
    Some(DEFAULT_NAME_TEMPLATE.to_string())
}

/// 配置文件信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigInfo {
    /// 快捷方式模版
    #[serde(default = "Template::default_template")]
    pub template: Option<Template>,

    /// 开启转义支持
    #[serde(default)]
    pub enable_escape: bool,

    /// 使用原始文件名
    #[serde(default)]
    pub use_filename: bool,

    /// 仅匹配配置文件
    #[serde(default)]
    pub only_match: bool,

    /// 评分阈值
    #[serde(default)]
    pub score_ratio: Option<f32>,

    /// 忽略列表
    #[serde(default)]
    pub ignore: Vec<String>,

    /// 安装脚本
    #[serde(default)]
    pub install: bool,

    /// 并行安装
    #[serde(default)]
    pub install_parallel: bool,

    /// 脚本规则
    #[serde(default)]
    pub scripts: Vec<String>,

    /// 程序信息列表
    #[serde(default)]
    pub shortcut: Vec<Lnk>,

    /// 映射表: 别名
    #[serde(default)]
    name: Table,

    /// 映射表: 工作目录
    #[serde(default)]
    work_dir: Table,

    /// 映射表: 命令行
    #[serde(default)]
    args: Table,

    /// 映射表: 图标
    #[serde(default)]
    icon: Table,

    /// 映射表: 位置
    #[serde(default)]
    dest: Table,

    /// 映射表: 显示模式
    #[serde(default)]
    window_state: Table,

    /// 映射表: 备注
    #[serde(default)]
    comment: Table,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Template {
    /// 快捷方式名称模版
    #[serde(default = "default_name_template")]
    pub name: Option<String>,

    /// 快捷方式位置模版
    #[serde(default)]
    pub dest: Option<String>,

    /// 快捷方式图标模版
    #[serde(default)]
    pub icon: Option<String>,

    /// 快捷方式工作目录模版
    #[serde(default)]
    pub work_dir: Option<String>,

    /// 快捷方式备注模版
    #[serde(default)]
    pub comment: Option<String>,
}

impl Template {
    fn default_template() -> Option<Template> {
        Some(Template {
            name: Some(DEFAULT_NAME_TEMPLATE.to_string()),
            icon: None,
            dest: None,
            work_dir: None,
            comment: None,
        })
    }
}

/// 程序信息列表
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Lnk {
    /// 程序名
    pub exec: String,

    /// 快捷方式名称
    #[serde(default)]
    pub name: Option<String>,

    /// 图标
    #[serde(default)]
    pub icon: Option<String>,

    /// 命令行参数
    #[serde(default)]
    pub args: Option<String>,

    /// 指定快捷方式位置
    #[serde(default)]
    pub dest: Option<String>,

    /// 起始位置
    #[serde(default)]
    pub work_dir: Option<String>,

    /// 显示模式
    #[serde(default)]
    pub window_state: Option<String>,

    /// 备注
    #[serde(default)]
    pub comment: Option<String>,
}

impl Lnk {
    pub fn new(exec: String) -> Lnk {
        Lnk {
            exec,
            name: None,
            icon: None,
            args: None,
            dest: None,
            work_dir: None,
            window_state: None,
            comment: None,
        }
    }
}

impl ConfigInfo {
    /// 解析配置文件
    ///
    /// # 参数
    ///
    /// * `path` - 配置文件路径
    ///
    /// # 返回值
    ///
    /// * `Result<ConfigInfo, Box<dyn Error>>` - 配置信息
    pub fn parse_config_file(path: &Path) -> Result<ConfigInfo, Box<dyn Error>> {
        // 读取配置
        let mut config_content = String::new();
        File::open(path)?.read_to_string(&mut config_content)?;

        // 判断是否启用转义
        let enable_escape = config_content.lines()
            .filter(|line| !line.trim_start().starts_with('#')) // 跳过注释行
            .any(|line| line.replace(" ", "").contains("enable_escape=true"));
        config_content = if enable_escape {
            if DEBUG.load(Ordering::Relaxed) {
                write_console(ConsoleType::Debug, &t!("config.enable_escape"));
            }
            config_content
        } else {
            // 将所有双引号内的Windows路径中的反斜杠进行特殊处理
            let mut result = String::new();
            let mut in_quotes = false;
            let mut chars = config_content.chars().peekable();

            while let Some(c) = chars.next() {
                if c == '"' {
                    in_quotes = !in_quotes;
                    result.push(c);
                } else if in_quotes && c == '\\' {
                    // 在双引号内遇到反斜杠，确保它被转义
                    result.push('\\');
                    result.push('\\');
                } else {
                    result.push(c);
                }
            }
            result
        };

        // 解析 Toml
        let mut config: ConfigInfo = toml::from_str(&config_content)?;

        // 处理 Toml 映射表
        if !config.name.is_empty() || !config.args.is_empty() || !config.icon.is_empty() || !config.dest.is_empty() {
            let mut map: BTreeMap<String, Lnk> = BTreeMap::new();
            for item in config.shortcut.drain(..) {
                map.insert(item.exec.clone(), item);
            }
            // 按 name 映射
            for (exe, alias) in &config.name {
                let e = map.entry(exe.clone()).or_insert_with(|| Lnk::new(exe.clone()));
                e.name = alias.as_str().map(|s| s.to_string());
            }
            // 按 work_dir 映射
            for (exe, work_dir) in &config.work_dir {
                let e = map.entry(exe.clone()).or_insert_with(|| Lnk::new(exe.clone()));
                e.work_dir = work_dir.as_str().map(|s| s.to_string());
            }
            // 按 args 映射
            for (exe, args) in &config.args {
                let e = map.entry(exe.clone()).or_insert_with(|| Lnk::new(exe.clone()));
                e.args = args.as_str().map(|s| s.to_string());
            }
            // 按 icon 映射
            for (exe, ic) in &config.icon {
                let e = map.entry(exe.clone()).or_insert_with(|| Lnk::new(exe.clone()));
                e.icon = ic.as_str().map(|s| s.to_string());
            }
            // 按 dest 映射
            for (exe, dest) in &config.dest {
                let e = map.entry(exe.clone()).or_insert_with(|| Lnk::new(exe.clone()));
                e.dest = dest.as_str().map(|s| s.to_string());
            }
            // 按 window_state 映射
            for (exe, window_state) in &config.window_state {
                let e = map.entry(exe.clone()).or_insert_with(|| Lnk::new(exe.clone()));
                e.window_state = window_state.as_str().map(|s| s.to_string());
            }
            // 按 comment 映射
            for (exe, comment) in &config.comment {
                let e = map.entry(exe.clone()).or_insert_with(|| Lnk::new(exe.clone()));
                e.comment = comment.as_str().map(|s| s.to_string());
            }
            config.shortcut = map.into_values().collect();
        }

        // 处理内置变量
        let mut doc: Value = config_content.parse()?;
        fn replace_in_value(val: &mut Value, config_path: &Path) {
            match val {
                Value::String(s) => {
                    *s = process_env(s.to_string(), Some(config_path));
                }
                Value::Table(table) => {
                    for (_key, v) in table.iter_mut() {
                        replace_in_value(v, config_path);
                    }
                }
                Value::Array(arr) => {
                    for v in arr.iter_mut() {
                        replace_in_value(v, config_path);
                    }
                }
                _ => {}
            }
        }
        replace_in_value(&mut doc, path);

        let config: ConfigInfo = doc.try_into()?;
        Ok(config)
    }
}

impl Lnk {
    /// 根据指定路径查找配置文件中对应的配置信息
    ///
    /// # 参数
    /// - `link_info`: 快捷方式信息列表
    /// - `program_path`: 程序路径
    pub fn get_lnk_info(program_path: &Path, link_info: &[Lnk]) -> Option<Lnk> {
        if let Some(kw) = link_info.iter().find(|kw| {
            let exec_cfg = PathBuf::from(&kw.exec);
            let expected = if exec_cfg.is_absolute() {
                exec_cfg.clone()
            } else {
                program_path.parent().unwrap().join(&exec_cfg)
            };
            expected.to_string_lossy().to_ascii_lowercase() == program_path.to_string_lossy().to_ascii_lowercase()
        }) {
            return Some(kw.clone());
        }
        None
    }
}

use crate::utils::{get_exe_company_name, get_exe_copyright, get_exe_description, get_exe_file_version, get_exe_original_filename, get_exe_product_name, get_program_arch, sanitize_description};
use chrono::{DateTime, Local, NaiveDateTime};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// 渲染模板
///
/// # 参数
///
/// - `path` - 程序路径
/// - `template` - 模板字符串
///
/// # 返回值
///
/// 渲染后的字符串
pub fn process_template(path: &Path, template: &str) -> String {
    let mut engine = TemplateEngine::new();
    let rendered = engine.render(template, &render_var(path)).unwrap();
    rendered
}

/// 渲染变量
///
/// # 参数
///
/// - `path` - 程序路径
///
/// # 返回值
///
/// 变量 HashMap
fn render_var(path: &Path) -> HashMap<String, String> {
    // 构造上下文（小写 key，不带{}）
    let mut vars: HashMap<String, String> = HashMap::new();

    // 程序路径
    vars.insert("exec".into(), path.display().to_string());

    // 程序文件名
    vars.insert("stem".into(), path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string());

    // 程序后缀名
    vars.insert("ext".into(), path.extension().and_then(|s| s.to_str()).unwrap_or_default().to_string());

    // 程序父路径
    vars.insert("parent".into(), path.parent().and_then(|s| s.to_str()).unwrap_or_default().to_string());

    // 程序父路径名称
    vars.insert("parent_name".into(), path.parent().and_then(|p| p.file_stem()).and_then(|s| s.to_str()).unwrap_or_default().to_string());

    // 程序描述，清理控制字符，合并空白，若为网址或明显无意义则变空
    let desc_raw = match get_exe_description(path) {
        Ok(Some(v)) => v,
        _ => String::new(),
    };
    vars.insert("desc".into(), sanitize_description(&desc_raw).unwrap_or("".to_string()));
    vars.insert("desc_raw".into(), desc_raw);

    // 产品名称
    let product_raw = match get_exe_product_name(path) {
        Ok(Some(v)) => v,
        _ => String::new(),
    };
    vars.insert("product".into(), sanitize_description(&product_raw).unwrap_or("".to_string()));
    vars.insert("product_raw".into(), product_raw.clone());

    // 公司名称
    vars.insert("company".into(), match get_exe_company_name(path) {
        Ok(Some(v)) => v,
        _ => String::new(),
    });

    // 原始文件名
    vars.insert("orig_filename".into(), match get_exe_original_filename(path) {
        Ok(Some(v)) => v,
        _ => String::new(),
    });

    // 版权信息
    vars.insert("copyright".into(), match get_exe_copyright(path) {
        Ok(Some(v)) => v,
        _ => String::new(),
    });

    // 程序版本
    vars.insert("version".into(), match get_exe_file_version(path) {
        Ok(Some(v)) => v,
        _ => String::new(),
    });

    // 程序架构
    let (arch_label, arch_num) = match get_program_arch(path) {
        Ok(0x014c) => (Some("x32".to_string()), Some("32".to_string())),
        Ok(0x8664) => (Some("x64".to_string()), Some("64".to_string())),
        Ok(0xAA64) => (Some("arm64".to_string()), Some("arm64".to_string())),
        _ => (None, None),
    };
    vars.insert("arch".into(), arch_label.clone().unwrap_or_default());
    vars.insert("arch_num".into(), arch_num.clone().unwrap_or_default());

    // 程序大小
    vars.insert("size".into(), match path.metadata() {
        Ok(metadata) => metadata.len().to_string(),
        _ => String::new(),
    });
    vars.insert("size_kb".into(), match path.metadata() {
        Ok(metadata) => format!("{:.1}", (metadata.len() as f64) / 1024.0),
        _ => String::new(),
    });
    vars.insert("size_mb".into(), match path.metadata() {
        Ok(metadata) => format!("{:.2}", (metadata.len() as f64) / 1024.0 / 1024.0),
        _ => String::new(),
    });
    vars.insert("size_gb".into(), match path.metadata() {
        Ok(metadata) => format!("{:.3}", (metadata.len() as f64) / 1024.0 / 1024.0 / 1024.0),
        _ => String::new(),
    });
    vars.insert("size_tb".into(), match path.metadata() {
        Ok(metadata) => format!("{:.4}", (metadata.len() as f64) / 1024.0 / 1024.0 / 1024.0 / 1024.0),
        _ => String::new(),
    });

    // 辅助：desc_or_stem
    vars.insert("desc_or_stem".into(), if !vars.get("desc").unwrap().is_empty() { vars.get("desc").unwrap().clone() } else { vars.get("stem").unwrap().clone() });

    // 创建时间
    vars.insert("create_time".into(), match path.metadata() {
        Ok(metadata) => {
            if let Ok(create_time) = metadata.created() {
                let date: DateTime<Local> = DateTime::from(create_time);
                date.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    });

    // 修改时间
    vars.insert("modified_time".into(), match path.metadata() {
        Ok(metadata) => {
            if let Ok(modified_time) = metadata.modified() {
                let date: DateTime<Local> = DateTime::from(modified_time);
                date.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    });

    // 访问时间
    vars.insert("accessed_time".into(), match path.metadata() {
        Ok(metadata) => {
            if let Ok(accessed_time) = metadata.accessed() {
                let date: DateTime<Local> = DateTime::from(accessed_time); // 本地时区
                date.format("%Y-%m-%d %H:%M:%S").to_string()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    });

    vars
}

/// 模板引擎结构体
pub struct TemplateEngine {
    // 可选：缓存已解析的模板以提高性能
    cache: HashMap<String, String>,
}

/// 模板引擎实现
impl TemplateEngine {
    /// 创建模板引擎实例
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// 渲染模板 - 支持任何实现了Serialize trait的类型
    pub fn render<C>(&mut self, template: &str, context: &C) -> Result<String, String>
    where
        C: Serialize,
    {
        // 如果缓存中存在已解析的模板，则直接使用
        if let Some(result) = self.cache.get(template) {
            return Ok(result.clone());
        }

        // 将传入的context转换为serde_json::Value
        let value_context = match serde_json::to_value(context) {
            Ok(value) => value,
            Err(_) => return Err("Unable to convert context to JSON value".to_string()),
        };

        // 手动扫描模板字符串，识别表达式
        let mut result = String::new();
        let mut i = 0;
        let chars: Vec<char> = template.chars().collect();

        while i < chars.len() {
            if chars[i] == '{' {
                // 查找表达式的结束位置
                let start = i + 1;
                let mut depth = 1;
                let mut end = start;

                while end < chars.len() && depth > 0 {
                    if chars[end] == '{' {
                        depth += 1;
                    } else if chars[end] == '}' {
                        depth -= 1;
                    }
                    end += 1;
                }

                if depth == 0 {
                    // 提取表达式内容并计算值 - 使用字符索引而不是字节索引
                    let expr_content: String = chars[start..end - 1].iter().collect();
                    let value = self.evaluate_expression(&expr_content, &value_context)?;
                    result.push_str(&value);
                    i = end;
                    continue;
                }
            }

            // 添加非表达式字符
            result.push(chars[i]);
            i += 1;
        }

        // 缓存结果
        self.cache.insert(template.to_string(), result.clone());

        Ok(result)
    }

    /// 计算表达式
    fn evaluate_expression(&self, expr: &str, context: &Value) -> Result<String, String> {
        let expr_trimmed = expr.trim();

        // 检查是否是默认值表达式
        if expr_trimmed.contains("??") {
            let parts: Vec<&str> = expr_trimmed.split("??").map(|s| s.trim()).collect();
            let value = self.evaluate_simple_expression(parts[0], context)?;
            if !value.is_empty() {
                return Ok(value);
            }
            return self.evaluate_simple_expression(parts[1], context);
        }

        // 检查是否是三元表达式
        if expr_trimmed.contains('?') {
            return self.evaluate_ternary_expression(expr_trimmed, context);
        }

        // 检查是否包含过滤器
        if expr_trimmed.contains('|') {
            let parts: Vec<&str> = expr_trimmed.split('|').map(|s| s.trim()).collect();
            let value = self.evaluate_simple_expression(parts[0], context)?;

            // 应用过滤器
            return self.apply_filters(value, &parts[1..]);
        }

        // 计算简单表达式
        self.evaluate_simple_expression(expr_trimmed, context)
    }

    /// 计算三元表达式
    fn evaluate_ternary_expression(&self, expr: &str, context: &Value) -> Result<String, String> {
        // 跟踪括号深度以正确识别三元表达式的各部分
        let mut bracket_depth = 0;
        let mut question_mark_pos = None;
        let mut colon_pos = None;

        // 第一遍扫描：找到第一个问号
        for (i, ch) in expr.char_indices() {
            match ch {
                '{' => bracket_depth += 1,
                '}' => if bracket_depth > 0 { bracket_depth -= 1; },
                '?' => if bracket_depth == 0 && question_mark_pos.is_none() {
                    question_mark_pos = Some(i);
                    break;
                },
                _ => {}
            }
        }

        if let Some(q_pos) = question_mark_pos {
            bracket_depth = 0;
            // 第二遍扫描：找到匹配的冒号
            for (i, ch) in expr.char_indices().skip(q_pos + 1) {
                match ch {
                    '{' => bracket_depth += 1,
                    '}' => if bracket_depth > 0 { bracket_depth -= 1; },
                    ':' => if bracket_depth == 0 {
                        colon_pos = Some(i);
                        break;
                    },
                    _ => {}
                }
            }

            if let Some(c_pos) = colon_pos {
                let condition = &expr[..q_pos].trim();
                let true_expr = &expr[q_pos + 1..c_pos].trim();
                let false_expr = &expr[c_pos + 1..].trim();

                let condition_result = self.evaluate_condition(condition, context)?;

                if condition_result {
                    return self.evaluate_expression(true_expr, context);
                } else {
                    // 处理假值表达式可能包含的大括号
                    let false_expr_trimmed = false_expr.trim();
                    let false_expr_to_eval = if false_expr_trimmed.starts_with('{') && false_expr_trimmed.ends_with('}') {
                        &false_expr_trimmed[1..false_expr_trimmed.len() - 1]
                    } else {
                        false_expr_trimmed
                    };
                    return self.evaluate_expression(false_expr_to_eval, context);
                }
            }
        }

        // 如果不是有效的三元表达式，返回空字符串
        Ok("".to_string())
    }

    /// 计算简单表达式（变量、字面量等）
    fn evaluate_simple_expression(&self, expr: &str, context: &Value) -> Result<String, String> {
        let expr = expr.trim();

        // 检查是否是字符串字面量
        if (expr.starts_with('"') && expr.ends_with('"')) || (expr.starts_with('\'') && expr.ends_with('\'')) {
            // 去除引号
            return Ok(expr[1..expr.len() - 1].to_string());
        }

        // 检查是否是数字字面量
        if expr.parse::<f64>().is_ok() {
            return Ok(expr.to_string());
        }

        // 检查是否包含字符串拼接
        if expr.contains('+') {
            let parts: Vec<&str> = expr.split('+').map(|s| s.trim()).collect();
            let mut result = String::new();

            for part in parts {
                let value = self.evaluate_simple_expression(part, context)?;
                result.push_str(&value);
            }

            return Ok(result);
        }

        // 获取变量值
        self.get_variable_value(expr, context)
    }

    /// 应用过滤器
    fn apply_filters(&self, mut value: String, filters: &[&str]) -> Result<String, String> {
        for filter_part in filters {
            // 解析过滤器名称和参数
            let mut filter_name = filter_part.trim();
            let mut filter_arg: Option<String> = None;

            // 使用冒号:作为参数分隔符
            if let Some(colon_pos) = self.find_top_level_char(filter_part, ':') {
                filter_name = &filter_part[..colon_pos].trim();
                let arg = &filter_part[colon_pos + 1..].trim();

                // 处理参数可能的引号
                filter_arg = Some(if (arg.starts_with('"') && arg.ends_with('"')) ||
                    (arg.starts_with('\'') && arg.ends_with('\'')) {
                    arg[1..arg.len() - 1].to_string()
                } else {
                    arg.to_string()
                });
            }

            // 应用过滤器
            value = match filter_name.to_ascii_lowercase().as_str() {
                // 转大写
                "upper" => value.to_uppercase(),
                // 转小写
                "lower" => value.to_lowercase(),
                // 去除空格
                "trim" => value.trim().to_string(),
                // 首字母大写
                "capitalize" => {
                    let mut chars = value.chars();
                    match chars.next() {
                        None => value,
                        Some(first) => {
                            let mut result = String::with_capacity(value.len());
                            result.extend(first.to_uppercase());
                            result.extend(chars);
                            result
                        }
                    }
                }
                // 切片操作
                "slice" => {
                    if let Some(arg) = &filter_arg {
                        // 支持 "start:end" 或 'start:end' 或 start:end（引号可选）
                        let mut s = arg.trim();
                        if s.len() >= 2 && ((s.starts_with('"') && s.ends_with('"')) ||
                            (s.starts_with('\'') && s.ends_with('\''))) {
                            s = &s[1..s.len() - 1];
                        }

                        // 使用逗号作为分隔符
                        let delimiter = if s.contains(',') { ',' } else { ' ' };

                        // 按分隔符分割成 start 与 end（都可为空）
                        let mut parts = s.splitn(2, delimiter);
                        let start_str = parts.next().unwrap_or("").trim();
                        let end_str = parts.next().unwrap_or("").trim();

                        // 当前字符串按 Unicode 字符收集
                        let chars: Vec<char> = value.chars().collect();
                        let len = chars.len() as isize;

                        // 解析 start（可选，支持负数）
                        let start_res: Result<isize, _> = if start_str.is_empty() {
                            Ok(0)
                        } else {
                            start_str.parse::<isize>()
                        };
                        let mut start_idx = match start_res {
                            Ok(v) => {
                                if v < 0 { len + v } else { v }
                            }
                            Err(_) => {
                                // 参数无法解析 -> 跳过该 filter（保持 value 不变）
                                continue;
                            }
                        };

                        // 解析 end（可选，支持负数）
                        let end_res: Result<isize, _> = if end_str.is_empty() {
                            Ok(len)
                        } else {
                            end_str.parse::<isize>()
                        };
                        let mut end_idx = match end_res {
                            Ok(v) => {
                                if v < 0 { len + v } else { v }
                            }
                            Err(_) => {
                                continue;
                            }
                        };

                        // Clamp 到 [0, len]
                        if start_idx < 0 { start_idx = 0; }
                        if end_idx < 0 { end_idx = 0; }
                        if start_idx > len { start_idx = len; }
                        if end_idx > len { end_idx = len; }

                        // 如果 start >= end -> 空
                        if start_idx >= end_idx {
                            value.clear();
                        } else {
                            let s_us = start_idx as usize;
                            let e_us = end_idx as usize;
                            value = chars[s_us..e_us].iter().collect();
                        }
                    }
                    value
                }
                // 默认值
                "default" => {
                    if let Some(arg) = &filter_arg {
                        if value.trim().is_empty() {
                            arg.to_string()
                        } else {
                            value
                        }
                    } else {
                        value
                    }
                }
                // 标题大小写
                "title" => {
                    let mut out = String::with_capacity(value.len());
                    let mut last_was_ws = true;
                    for ch in value.chars() {
                        if ch.is_whitespace() {
                            out.push(ch);
                            last_was_ws = true;
                        } else {
                            if last_was_ws {
                                // 首字母：大写
                                out.extend(ch.to_uppercase());
                            } else {
                                // 非首字母：转小写（以获得一致的 Title Case）
                                for low in ch.to_lowercase() {
                                    out.push(low);
                                }
                            }
                            last_was_ws = false;
                        }
                    }
                    out
                }
                // 字符串长度
                "length" => value.chars().count().to_string(),
                // 从字符串中删除指定内容
                "cut" => {
                    if let Some(arg) = &filter_arg {
                        value.replace(arg, "")
                    } else {
                        value
                    }
                }
                // 截断字符串
                "truncate" => {
                    if let Some(arg) = &filter_arg {
                        if let Ok(n) = arg.parse::<usize>() {
                            // 按 Unicode 字符截断
                            value.chars().take(n).collect()
                        } else {
                            value
                        }
                    } else {
                        value
                    }
                }
                // 字符串替换
                "replace" => {
                    if let Some(arg) = &filter_arg {
                        // 参数格式: old,new 或 'old','new' 或 "old","new"
                        let parts: Vec<String> = if arg.contains("','") || arg.contains('"') && arg.contains(",") && arg.contains('"') {
                            // 处理 'old','new' 或 "old","new" 格式
                            let mut result = Vec::new();
                            let mut in_quote = None;
                            let mut start = 0;
                            let chars: Vec<char> = arg.chars().collect();

                            for (i, c) in chars.iter().enumerate() {
                                if in_quote.is_none() {
                                    if *c == '\'' || *c == '"' {
                                        in_quote = Some(*c);
                                        start = i;
                                    }
                                } else if *c == in_quote.unwrap() && i > 0 && chars[i - 1] != '\\' {
                                    // 找到一个完整的引号字符串
                                    result.push(chars[start..i + 1].iter().collect());
                                    in_quote = None;
                                }
                            }

                            // 如果没有找到成对的引号，退回到简单分割
                            if result.len() < 2 {
                                arg.splitn(2, ',').map(|s| s.trim().to_string()).collect()
                            } else {
                                result
                            }
                        } else {
                            // 处理 old,new 格式
                            arg.splitn(2, ',').map(|s| s.trim().to_string()).collect()
                        };

                        if parts.len() >= 2 {
                            let old = if (parts[0].starts_with('"') && parts[0].ends_with('"')) ||
                                (parts[0].starts_with('\'') && parts[0].ends_with('\'')) {
                                parts[0][1..parts[0].len() - 1].to_string()
                            } else {
                                parts[0].clone()
                            };

                            let new = if (parts[1].starts_with('"') && parts[1].ends_with('"')) ||
                                (parts[1].starts_with('\'') && parts[1].ends_with('\'')) {
                                parts[1][1..parts[1].len() - 1].to_string()
                            } else {
                                parts[1].clone()
                            };

                            value.replace(&old, &new)
                        } else {
                            value
                        }
                    } else {
                        value
                    }
                }
                // 日期格式化
                "date" => {
                    if let Some(arg) = &filter_arg {
                        match NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S") {
                            Ok(naive_date) => {
                                // 获取本地时区偏移
                                let local = Local::now();
                                // 创建带有时区信息的DateTime
                                let date_with_timezone = DateTime::<Local>::from_naive_utc_and_offset(naive_date, *local.offset());
                                // 格式化输出
                                date_with_timezone.format(arg).to_string()
                            }
                            Err(_) => value, // 解析失败则返回原始值
                        }
                    } else {
                        value // 无参数则返回原始值
                    }
                }
                _ => value,
            };
        }

        Ok(value)
    }

    /// 查找顶层字符（忽略引号、转义字符和嵌套括号内的字符）
    fn find_top_level_char(&self, s: &str, target: char) -> Option<usize> {
        let mut in_single = false;
        let mut in_double = false;
        let mut depth = 0i32;
        let mut escaped = false;
        let mut i = 0usize;

        while i < s.len() {
            let ch = s[i..].chars().next().unwrap();
            let ch_len = ch.len_utf8();

            if !escaped {
                match ch {
                    '\\' => {
                        escaped = true;
                        i += ch_len;
                        continue;
                    }
                    '\'' if !in_double => {
                        in_single = !in_single;
                        i += ch_len;
                        continue;
                    }
                    '"' if !in_single => {
                        in_double = !in_double;
                        i += ch_len;
                        continue;
                    }
                    '{' if !in_single && !in_double => {
                        depth += 1;
                        i += ch_len;
                        continue;
                    }
                    '}' if !in_single && !in_double => {
                        if depth > 0 { depth -= 1; }
                        i += ch_len;
                        continue;
                    }
                    c if c == target && !in_single && !in_double && depth == 0 => {
                        return Some(i);
                    }
                    _ => {}
                }
            } else {
                escaped = false;
            }
            i += ch_len;
        }

        None
    }

    /// 计算条件表达式
    fn evaluate_condition(&self, condition: &str, context: &Value) -> Result<bool, String> {
        // 检查是否包含逻辑运算符
        if condition.contains("&&") {
            let parts: Vec<&str> = condition.split("&&").map(|s| s.trim()).collect();
            for part in parts {
                if !self.evaluate_condition(part, context)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }

        if condition.contains("||") {
            let parts: Vec<&str> = condition.split("||").map(|s| s.trim()).collect();
            for part in parts {
                if self.evaluate_condition(part, context)? {
                    return Ok(true);
                }
            }
            return Ok(false);
        }

        // 检查是否包含比较运算符
        // 注意：必须先检查较长的操作符，避免部分匹配
        let operators = ["==", "!=", ">=", "<=", ">", "<"];
        for op in operators.iter() {
            if condition.contains(op) {
                let parts: Vec<&str> = condition.split(op).map(|s| s.trim()).collect();
                if parts.len() != 2 {
                    return Err("比较表达式格式错误".to_string());
                }

                let left = self.evaluate_simple_expression(parts[0], context)?;
                let right = self.evaluate_simple_expression(parts[1], context)?;

                // 尝试将两边转换为数字进行比较
                if let (Ok(left_num), Ok(right_num)) = (left.parse::<f64>(), right.parse::<f64>()) {
                    return Ok(match *op {
                        "==" => left_num == right_num,
                        "!=" => left_num != right_num,
                        ">" => left_num > right_num,
                        ">=" => left_num >= right_num,
                        "<" => left_num < right_num,
                        "<=" => left_num <= right_num,
                        _ => false,
                    });
                }

                // 作为字符串比较
                return Ok(match *op {
                    "==" => left == right,
                    "!=" => left != right,
                    ">" => left > right,
                    ">=" => left >= right,
                    "<" => left < right,
                    "<=" => left <= right,
                    _ => false,
                });
            }
        }

        // 如果只是一个变量名，检查其是否为真值
        let value = self.evaluate_simple_expression(condition, context)?;
        Ok(!value.is_empty() && value != "false" && value != "0")
    }

    /// 从上下文中获取变量值
    fn get_variable_value(&self, var_name: &str, context: &Value) -> Result<String, String> {
        let var_name = var_name.trim();

        // 支持点号访问对象属性，例如 user.name
        let parts: Vec<&str> = var_name.split('.').collect();
        let mut current = context;

        for part in parts {
            match current {
                Value::Object(map) => {
                    if let Some(val) = map.get(part) {
                        current = val;
                    } else {
                        return Ok("".to_string());
                    }
                }
                _ => return Ok("".to_string()),
            }
        }

        // 将Value转换为字符串
        Ok(match current {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "".to_string(),
            _ => "".to_string(), // 对于数组和对象，返回空字符串
        })
    }
}

// 导出的渲染函数，方便直接使用
pub fn render_template<C>(template: &str, context: &C) -> Result<String, String>
where
    C: Serialize,
{
    let mut engine = TemplateEngine::new();
    engine.render(template, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_variable_replacement() {
        let template = "Hello, {name}!";
        let context = json!({
            "name": "World"
        });

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_ternary_operator() {
        let template = "{age >= 18 ? 'Adult' : 'Minor'}";
        let context = json!({
            "age": 20
        });

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "Adult");

        let context = json!({
            "age": 16
        });

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "Minor");
    }

    #[test]
    fn test_default_value() {
        let template = "Hello, {username ?? 'Guest'}!";
        let context = json!({});

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "Hello, Guest!");

        let context = json!({
            "username": "Admin"
        });

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "Hello, Admin!");
    }

    #[test]
    fn test_filters() {
        let template = "{name | upper}";
        let context = json!({
            "name": "john doe"
        });

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "JOHN DOE");

        let template = "{name | upper | slice:0,4}";
        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "JOHN");

        // 测试capitalize过滤器
        let template = "{name | capitalize}";
        let context = json!({
            "name": "john doe"
        });
        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "John doe");

        // 测试空字符串的capitalize过滤器
        let template = "{empty | capitalize}";
        let context = json!({
            "empty": ""
        });
        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "");

        // 测试replace过滤器
        let template = "{greeting | replace:Hello,Hi}";
        let context = json!({
            "greeting": "Hello, World!"
        });
        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "Hi, World!");

        // 测试带有引号的replace过滤器参数
        let template = "{path | replace:'/api','/v2/api'}";
        let context = json!({
            "path": "/api/users"
        });
        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "/v2/api/users");
    }

    #[test]
    fn test_nested_ternary_operator() {
        // 测试嵌套三元表达式: {product ? product : {desc ? desc : stem}}
        let template = "{product ? product : {desc ? desc : stem}}";

        // 场景1: product 有值
        let context = json!({
            "product": "高级产品",
            "desc": "产品描述",
            "stem": "基础内容"
        });
        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "高级产品");

        // 场景2: product 为空，desc 有值
        let context = json!({
            "product": "",
            "desc": "产品描述",
            "stem": "基础内容"
        });
        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "产品描述");

        // 场景3: product 和 desc 都为空，使用 stem
        let context = json!({
            "product": "",
            "desc": "",
            "stem": "基础内容"
        });
        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "基础内容");
    }
}

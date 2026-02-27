// 📄 biz-error doc-gen: 错误码文档生成模块
//
// 从 YAML 配置文件解析错误码定义，生成结构化 Markdown 文档。
// 通过 feature flag `doc-gen` 条件编译启用。

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

/// 文档生成配置（从 YAML 解析得到）
#[derive(Debug, Clone, PartialEq)]
pub struct DocConfig {
    /// 默认语言
    pub default_language: String,
    /// 支持的语言列表
    pub supported_languages: Vec<String>,
    /// 所有错误码定义
    pub errors: Vec<ErrorDefinition>,
}

/// 单个错误码定义
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorDefinition {
    /// 错误码名称（snake_case，如 "invalid_param"）
    pub name: String,
    /// 数字错误码（如 4000）
    pub code: i32,
    /// HTTP 状态码（如 400）
    pub http_status: u16,
    /// 多语言消息映射：语言 -> 消息文本
    pub messages: HashMap<String, String>,
}

/// 文档生成错误
#[derive(Debug)]
pub enum DocGenError {
    /// YAML 文件读取失败
    IoError { path: String, source: std::io::Error },
    /// YAML 解析失败
    YamlParseError { path: String, message: String },
    /// YAML 结构不符合预期（缺少必要字段等）
    InvalidStructure { path: String, message: String },
}

impl std::fmt::Display for DocGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocGenError::IoError { path, source } => {
                write!(f, "Failed to read file '{}': {}", path, source)
            }
            DocGenError::YamlParseError { path, message } => {
                write!(f, "Failed to parse YAML '{}': {}", path, message)
            }
            DocGenError::InvalidStructure { path, message } => {
                write!(f, "Invalid structure in '{}': {}", path, message)
            }
        }
    }
}

impl std::error::Error for DocGenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DocGenError::IoError { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// YAML 配置解析 + Markdown 文档生成
pub struct DocGenerator;

impl DocGenerator {
    /// 从 YAML 文件解析错误码定义
    pub fn parse_yaml(path: &Path) -> Result<DocConfig, DocGenError> {
        // 1. 读取文件
        let content = std::fs::read_to_string(path).map_err(|e| DocGenError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        // 2. serde_yaml 解析为 Value
        let value: serde_yaml::Value =
            serde_yaml::from_str(&content).map_err(|e| DocGenError::YamlParseError {
                path: path.display().to_string(),
                message: e.to_string(),
            })?;

        let mapping = value.as_mapping().ok_or_else(|| DocGenError::InvalidStructure {
            path: path.display().to_string(),
            message: "root must be a YAML mapping".to_string(),
        })?;

        // 3. 提取 default_language（默认 "en"）
        let default_language = mapping
            .get(serde_yaml::Value::String("default_language".to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("en")
            .to_string();

        // 4. 提取 supported_languages（可选，默认空列表）
        let supported_languages = mapping
            .get(serde_yaml::Value::String("supported_languages".to_string()))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // 5. 提取 errors mapping
        let errors_value =
            mapping
                .get(serde_yaml::Value::String("errors".to_string()))
                .ok_or_else(|| DocGenError::InvalidStructure {
                    path: path.display().to_string(),
                    message: "missing 'errors' field".to_string(),
                })?;

        let errors_mapping =
            errors_value
                .as_mapping()
                .ok_or_else(|| DocGenError::InvalidStructure {
                    path: path.display().to_string(),
                    message: "'errors' must be a mapping".to_string(),
                })?;

        let mut errors = Vec::new();

        for (key, entry) in errors_mapping {
            let name = key
                .as_str()
                .ok_or_else(|| DocGenError::InvalidStructure {
                    path: path.display().to_string(),
                    message: "error key must be a string".to_string(),
                })?
                .to_string();

            let entry_mapping =
                entry
                    .as_mapping()
                    .ok_or_else(|| DocGenError::InvalidStructure {
                        path: path.display().to_string(),
                        message: format!("error '{}' must be a mapping", name),
                    })?;

            // code 必须存在
            let code = entry_mapping
                .get(serde_yaml::Value::String("code".to_string()))
                .and_then(|v| v.as_i64())
                .ok_or_else(|| DocGenError::InvalidStructure {
                    path: path.display().to_string(),
                    message: format!("error '{}' missing required 'code' field", name),
                })? as i32;

            // http_status 默认 500
            let http_status = entry_mapping
                .get(serde_yaml::Value::String("http_status".to_string()))
                .and_then(|v| v.as_u64())
                .unwrap_or(500) as u16;

            // message 必须存在且为 mapping
            let msg_value = entry_mapping
                .get(serde_yaml::Value::String("message".to_string()))
                .ok_or_else(|| DocGenError::InvalidStructure {
                    path: path.display().to_string(),
                    message: format!("error '{}' missing required 'message' field", name),
                })?;

            let msg_mapping =
                msg_value
                    .as_mapping()
                    .ok_or_else(|| DocGenError::InvalidStructure {
                        path: path.display().to_string(),
                        message: format!("error '{}' 'message' must be a mapping", name),
                    })?;

            let mut messages = HashMap::new();
            for (lang_key, msg_val) in msg_mapping {
                if let (Some(lang), Some(msg)) = (lang_key.as_str(), msg_val.as_str()) {
                    messages.insert(lang.to_string(), msg.to_string());
                }
            }

            errors.push(ErrorDefinition {
                name,
                code,
                http_status,
                messages,
            });
        }

        Ok(DocConfig {
            default_language,
            supported_languages,
            errors,
        })
    }

    /// 从 DocConfig 生成 Markdown 文档字符串
    ///
    /// - `config`: 解析后的文档配置
    /// - `yaml_path`: 源 YAML 文件路径（用于元信息头部）
    pub fn generate_markdown(config: &DocConfig, yaml_path: &str) -> String {
        let mut md = String::new();

        // --- 头部元信息 ---
        let timestamp = Self::format_timestamp();
        md.push_str("# 错误码文档\n\n");
        md.push_str(&format!("> 生成时间: {}\n", timestamp));
        md.push_str(&format!("> 源文件: {}\n", yaml_path));
        md.push_str(&format!(
            "> 支持语言: {}\n",
            config.supported_languages.join(", ")
        ));

        // --- 总览表格 ---
        let default_lang = &config.default_language;
        md.push_str(&format!(
            "\n## 总览\n\n| 错误名称 | 错误码 | HTTP 状态码 | 消息 ({}) |\n",
            default_lang
        ));
        md.push_str("|----------|--------|------------|----------|\n");
        for err in &config.errors {
            let default_msg = err.messages.get(default_lang).map(|s| s.as_str()).unwrap_or("");
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                err.name, err.code, err.http_status, default_msg
            ));
        }

        // --- 按千位范围分组 ---
        // 收集所有分组并排序
        let mut groups: HashMap<i32, Vec<&ErrorDefinition>> = HashMap::new();
        for err in &config.errors {
            let group_key = err.code / 1000 * 1000;
            groups.entry(group_key).or_default().push(err);
        }
        let mut group_keys: Vec<i32> = groups.keys().copied().collect();
        group_keys.sort();

        for group_start in group_keys {
            let group_errors = &groups[&group_start];

            // 分组标题
            if group_start == 0 {
                md.push_str("\n## 0: 成功\n\n");
            } else {
                let group_end = group_start + 999;
                md.push_str(&format!("\n## {}-{}\n\n", group_start, group_end));
            }

            // 分组表格
            md.push_str(&format!(
                "| 错误名称 | 错误码 | HTTP 状态码 | 消息 ({}) |\n",
                default_lang
            ));
            md.push_str("|----------|--------|------------|----------|\n");
            for err in group_errors {
                let default_msg = err.messages.get(default_lang).map(|s| s.as_str()).unwrap_or("");
                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    err.name, err.code, err.http_status, default_msg
                ));
            }

            // 每个错误码的多语言消息详情
            for err in group_errors {
                md.push_str(&format!("\n### {} ({})\n\n", err.name, err.code));
                md.push_str("| 语言 | 消息 |\n");
                md.push_str("|------|------|\n");
                // 按语言排序以保证输出稳定
                let mut langs: Vec<&String> = err.messages.keys().collect();
                langs.sort();
                for lang in langs {
                    let msg = &err.messages[lang];
                    md.push_str(&format!("| {} | {} |\n", lang, msg));
                }
            }
        }

        md
    }

    /// 一步完成：解析 YAML + 生成 Markdown + 写入文件
    ///
    /// - `yaml_path`: YAML 配置文件路径
    /// - `output_path`: 输出文件路径，默认 `error_codes.md`
    pub fn generate(yaml_path: &Path, output_path: Option<&Path>) -> Result<(), DocGenError> {
        let config = Self::parse_yaml(yaml_path)?;

        let yaml_path_str = yaml_path.display().to_string();
        let markdown = Self::generate_markdown(&config, &yaml_path_str);

        let default_output = Path::new("error_codes.md");
        let out = output_path.unwrap_or(default_output);

        let mut file = std::fs::File::create(out).map_err(|e| DocGenError::IoError {
            path: out.display().to_string(),
            source: e,
        })?;
        file.write_all(markdown.as_bytes())
            .map_err(|e| DocGenError::IoError {
                path: out.display().to_string(),
                source: e,
            })?;

        Ok(())
    }

    /// 格式化当前时间戳为 "YYYY-MM-DD HH:MM:SS" 格式
    fn format_timestamp() -> String {
        use std::time::SystemTime;

        let duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();

        // 简单的日期时间计算（UTC）
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;

        // 从 Unix epoch (1970-01-01) 计算年月日
        let (year, month, day) = Self::days_to_ymd(days);

        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            year, month, day, hours, minutes, seconds
        )
    }

    /// 将 Unix epoch 以来的天数转换为 (year, month, day)
    fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
        // 从 1970-01-01 开始计算
        let mut year = 1970u64;

        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if days < days_in_year {
                break;
            }
            days -= days_in_year;
            year += 1;
        }

        let leap = Self::is_leap_year(year);
        let month_days: [u64; 12] = [
            31,
            if leap { 29 } else { 28 },
            31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
        ];

        let mut month = 0usize;
        while month < 12 && days >= month_days[month] {
            days -= month_days[month];
            month += 1;
        }

        (year, (month + 1) as u64, days + 1)
    }

    /// 判断是否为闰年
    fn is_leap_year(year: u64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
}

// 📦 代码生成模块 - 供业务项目使用
//
// 此模块提供从 YAML 配置生成错误码代码的功能
//
// # 使用方式
//
// 在项目的 `build.rs` 中调用：
//
// ```no_run
// fn main() {
//     biz_error::generate_error_codes(
//         "biz_errors.yaml",
//         "src/error_codes.rs"
//     ).expect("Failed to generate error codes");
// }
// ```

#[cfg(feature = "codegen")]
use std::fs::File;
#[cfg(feature = "codegen")]
use std::io::Write;
#[cfg(feature = "codegen")]
use std::path::Path;

/// 从 YAML 配置生成错误码代码
///
/// 通常在项目的 `build.rs` 中调用：
///
/// ```no_run
/// fn main() {
///     biz_error::generate_error_codes(
///         "biz_errors.yaml",
///         "src/error_codes.rs"
///     ).expect("Failed to generate error codes");
/// }
/// ```
#[cfg(feature = "codegen")]
pub fn generate_error_codes<P1, P2>(yaml_path: P1, output_path: P2) -> Result<(), Box<dyn std::error::Error>>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let yaml_path = yaml_path.as_ref();
    let output_path = output_path.as_ref();

    // 读取 YAML 配置
    let yaml_content = std::fs::read_to_string(yaml_path)
        .map_err(|e| format!("Failed to read YAML file {:?}: {}", yaml_path, e))?;

    // 解析 YAML
    let config: serde_yaml::Value = serde_yaml::from_str(&yaml_content)
        .map_err(|e| format!("Failed to parse YAML: {}", e))?;

    // 生成代码
    let generated_code = generate_from_config(&config)?;

    // 写入输出文件
    let mut file = File::create(output_path)
        .map_err(|e| format!("Failed to create output file {:?}: {}", output_path, e))?;

    file.write_all(generated_code.as_bytes())
        .map_err(|e| format!("Failed to write output file: {}", e))?;

    Ok(())
}

/// 从配置生成代码字符串（内部实现）
pub(crate) fn generate_from_config(config: &serde_yaml::Value) -> Result<String, Box<dyn std::error::Error>> {
    use std::fmt::Write;

    let errors = config["errors"].as_mapping()
        .ok_or("Missing 'errors' section in YAML")?;

    let default_lang = config["default_language"]
        .as_str()
        .unwrap_or("en");

    let mut enum_variants = String::new();
    let mut code_match_arms = String::new();
    let mut message_match_arms = String::new();
    let mut http_status_match_arms = String::new();
    let mut variant_list = Vec::new();

    for (key, value) in errors {
        let name = key.as_str().ok_or("Error key must be a string")?;
        let code = value["code"].as_i64().ok_or("Missing 'code' field")?;
        let http_status = value["http_status"].as_i64().unwrap_or(500);
        let messages = value["message"].as_mapping()
            .ok_or("Missing 'message' field")?;

        // 转换为 PascalCase
        let enum_name = to_pascal_case(name);
        variant_list.push(enum_name.clone());

        // 添加文档注释（使用默认语言）
        let doc_msg = messages
            .get(serde_yaml::Value::String(default_lang.to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        writeln!(enum_variants, "    /// {}", doc_msg)?;
        writeln!(enum_variants, "    {},", enum_name)?;

        // 添加 code 匹配分支
        writeln!(code_match_arms, "            ErrorCode::{} => {},", enum_name, code)?;

        // 添加 message 匹配分支（每种语言）
        for (lang, msg) in messages {
            let lang_str = lang.as_str().ok_or("Language key must be a string")?;
            let msg_str = msg.as_str().ok_or("Message value must be a string")?;
            writeln!(message_match_arms, "            (ErrorCode::{}, \"{}\") => \"{}\",",
                enum_name, lang_str, msg_str)?;
        }
        // 添加 fallback
        let fallback_msg = messages
            .get(serde_yaml::Value::String(default_lang.to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        writeln!(message_match_arms, "            (ErrorCode::{}, _) => \"{}\",",
            enum_name, fallback_msg)?;

        // 添加 http_status 匹配分支
        writeln!(http_status_match_arms, "            ErrorCode::{} => axum::http::StatusCode::from_u16({}).unwrap(),",
            enum_name, http_status)?;
    }

    // 生成 ALL_CONSTANTS 数组
    let all_constants = variant_list
        .iter()
        .map(|v| format!("ErrorCode::{}", v))
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        r#"// 🔥 此文件由 biz-error 自动生成，请勿手动编辑！
// 💡 如需修改错误码，请编辑 biz_errors.yaml 文件后重新编译

use biz_error::ErrorCode as ErrorCodeTrait;

/// 自动生成的业务错误码枚举
///
/// # Examples
///
/// ```
/// use crate::error_codes::ErrorCode;
///
/// let error = ErrorCode::InvalidParam;
/// assert_eq!(error.code(), 4000);
/// assert_eq!(error.message(), "INVALID PARAMETER");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {{
{enum_variants}
}}

// 实现 biz_error::ErrorCode trait
impl ErrorCodeTrait for ErrorCode {{
    fn code(&self) -> i32 {{
        match self {{
{code_match_arms}        }}
    }}

    fn message(&self) -> &'static str {{
        self.message_lang("{default_lang}")
    }}

    fn message_lang(&self, lang: &str) -> &'static str {{
        match (self, lang.as_ref()) {{
{message_match_arms}        }}
    }}

    fn http_status(&self) -> axum::http::StatusCode {{
        match self {{
{http_status_match_arms}        }}
    }}
}}

impl std::fmt::Display for ErrorCode {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        write!(f, "[{{}}] {{}}", self.code(), self.message())
    }}
}}

impl std::error::Error for ErrorCode {{}}

/// 所有错误码常量列表（用于反射或遍历）
pub const ALL_ERROR_CODES: &[ErrorCode] = &[{all_constants}];
"#,
        enum_variants = enum_variants,
        code_match_arms = code_match_arms,
        message_match_arms = message_match_arms,
        http_status_match_arms = http_status_match_arms,
        default_lang = default_lang,
        all_constants = all_constants,
    ))
}

/// 将 snake_case 转换为 PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                }
            }
        })
        .collect()
}

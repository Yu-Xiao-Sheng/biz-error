#[cfg(feature = "doc-gen")]
mod tests {
    use biz_error::docgen::{DocConfig, DocGenerator, ErrorDefinition};
    use std::collections::HashMap;

    fn sample_config() -> DocConfig {
        DocConfig {
            default_language: "en".to_string(),
            supported_languages: vec!["en".to_string(), "zh-CN".to_string()],
            errors: vec![
                ErrorDefinition {
                    name: "success".to_string(),
                    code: 0,
                    http_status: 200,
                    messages: HashMap::from([
                        ("en".to_string(), "SUCCESS".to_string()),
                        ("zh-CN".to_string(), "成功".to_string()),
                    ]),
                },
                ErrorDefinition {
                    name: "not_login".to_string(),
                    code: 1000,
                    http_status: 401,
                    messages: HashMap::from([
                        ("en".to_string(), "NOT LOGIN".to_string()),
                        ("zh-CN".to_string(), "未登录".to_string()),
                    ]),
                },
                ErrorDefinition {
                    name: "invalid_param".to_string(),
                    code: 4000,
                    http_status: 400,
                    messages: HashMap::from([
                        ("en".to_string(), "INVALID PARAMETER".to_string()),
                        ("zh-CN".to_string(), "参数无效".to_string()),
                    ]),
                },
            ],
        }
    }

    #[test]
    fn test_markdown_header() {
        let config = sample_config();
        let md = DocGenerator::generate_markdown(&config, "biz_errors.yaml");

        assert!(md.starts_with("# 错误码文档\n"));
        assert!(md.contains("> 源文件: biz_errors.yaml\n"));
        assert!(md.contains("> 支持语言: en, zh-CN\n"));
        assert!(md.contains("> 生成时间: "));
    }

    #[test]
    fn test_markdown_overview_table() {
        let config = sample_config();
        let md = DocGenerator::generate_markdown(&config, "test.yaml");

        assert!(md.contains("## 总览\n"));
        assert!(md.contains("| 错误名称 | 错误码 | HTTP 状态码 | 消息 (en) |"));
        assert!(md.contains("| success | 0 | 200 | SUCCESS |"));
        assert!(md.contains("| not_login | 1000 | 401 | NOT LOGIN |"));
        assert!(md.contains("| invalid_param | 4000 | 400 | INVALID PARAMETER |"));
    }

    #[test]
    fn test_markdown_grouping() {
        let config = sample_config();
        let md = DocGenerator::generate_markdown(&config, "test.yaml");

        // code 0 -> "0: 成功" group
        assert!(md.contains("## 0: 成功\n"));
        // code 1000 -> "1000-1999" group
        assert!(md.contains("## 1000-1999\n"));
        // code 4000 -> "4000-4999" group
        assert!(md.contains("## 4000-4999\n"));
    }

    #[test]
    fn test_markdown_multilang_details() {
        let config = sample_config();
        let md = DocGenerator::generate_markdown(&config, "test.yaml");

        // Each error should have a detail section with language table
        assert!(md.contains("### success (0)\n"));
        assert!(md.contains("### not_login (1000)\n"));
        assert!(md.contains("### invalid_param (4000)\n"));

        // Language detail tables
        assert!(md.contains("| 语言 | 消息 |"));
        assert!(md.contains("| en | NOT LOGIN |"));
        assert!(md.contains("| zh-CN | 未登录 |"));
    }

    #[test]
    fn test_markdown_overview_row_count() {
        let config = sample_config();
        let md = DocGenerator::generate_markdown(&config, "test.yaml");

        // Find the overview section and count data rows
        let overview_section = md.split("## 总览\n").nth(1).unwrap();
        // Take lines until next section (##)
        let overview_lines: Vec<&str> = overview_section
            .lines()
            .take_while(|l| !l.starts_with("## "))
            .filter(|l| l.starts_with("| ") && !l.starts_with("|--") && !l.contains("错误名称"))
            .collect();

        assert_eq!(overview_lines.len(), config.errors.len());
    }

    #[test]
    fn test_generate_with_yaml_file() {
        use std::path::Path;

        let yaml_path = Path::new("biz_errors.yaml.example");
        let output_path = Path::new("/tmp/test_error_codes.md");

        let result = DocGenerator::generate(yaml_path, Some(output_path));
        assert!(result.is_ok(), "generate failed: {:?}", result.err());

        let content = std::fs::read_to_string(output_path).unwrap();
        assert!(content.starts_with("# 错误码文档\n"));
        assert!(content.contains("## 总览\n"));
        assert!(content.contains("| success | 0 | 200 | SUCCESS |"));

        // Cleanup
        let _ = std::fs::remove_file(output_path);
    }
}

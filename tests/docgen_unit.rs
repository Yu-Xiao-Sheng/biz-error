#[cfg(feature = "doc-gen")]
mod tests {
    use biz_error::docgen::{DocConfig, DocGenError, DocGenerator};
    use std::path::Path;

    // ========== YAML 解析错误消息格式测试 ==========

    #[test]
    fn test_parse_nonexistent_file_returns_io_error_with_path() {
        let path = Path::new("nonexistent_file_12345.yaml");
        let result = DocGenerator::parse_yaml(path);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("nonexistent_file_12345.yaml"),
            "Error should contain file path, got: {}",
            err_msg
        );
        assert!(matches!(err, DocGenError::IoError { .. }));
    }

    #[test]
    fn test_parse_invalid_yaml_returns_parse_error_with_path() {
        // 写一个无效 YAML 到临时文件
        let tmp_path = Path::new("/tmp/test_invalid_yaml.yaml");
        std::fs::write(tmp_path, "{{{{invalid yaml content").unwrap();

        let result = DocGenerator::parse_yaml(tmp_path);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("test_invalid_yaml.yaml"),
            "Error should contain file path, got: {}",
            err_msg
        );
        assert!(matches!(err, DocGenError::YamlParseError { .. }));

        let _ = std::fs::remove_file(tmp_path);
    }

    #[test]
    fn test_parse_yaml_missing_errors_field() {
        let tmp_path = Path::new("/tmp/test_missing_errors.yaml");
        std::fs::write(tmp_path, "default_language: en\n").unwrap();

        let result = DocGenerator::parse_yaml(tmp_path);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("missing 'errors' field"),
            "Error should mention missing errors field, got: {}",
            err_msg
        );
        assert!(matches!(err, DocGenError::InvalidStructure { .. }));

        let _ = std::fs::remove_file(tmp_path);
    }

    #[test]
    fn test_parse_yaml_error_missing_code_field() {
        let tmp_path = Path::new("/tmp/test_missing_code.yaml");
        let content = r#"
default_language: en
errors:
  bad_error:
    http_status: 400
    message:
      en: "BAD"
"#;
        std::fs::write(tmp_path, content).unwrap();

        let result = DocGenerator::parse_yaml(tmp_path);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("bad_error") && err_msg.contains("code"),
            "Error should mention error name and missing code, got: {}",
            err_msg
        );

        let _ = std::fs::remove_file(tmp_path);
    }

    #[test]
    fn test_parse_yaml_error_missing_message_field() {
        let tmp_path = Path::new("/tmp/test_missing_message.yaml");
        let content = r#"
default_language: en
errors:
  no_msg_error:
    code: 9999
    http_status: 500
"#;
        std::fs::write(tmp_path, content).unwrap();

        let result = DocGenerator::parse_yaml(tmp_path);
        assert!(result.is_err());

        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("no_msg_error") && err_msg.contains("message"),
            "Error should mention error name and missing message, got: {}",
            err_msg
        );

        let _ = std::fs::remove_file(tmp_path);
    }

    // ========== Markdown 文档头部元信息格式测试 ==========

    #[test]
    fn test_markdown_header_format() {
        let config = DocConfig {
            default_language: "en".to_string(),
            supported_languages: vec!["en".to_string(), "zh-CN".to_string()],
            errors: vec![],
        };

        let md = DocGenerator::generate_markdown(&config, "my_errors.yaml");

        // 标题
        assert!(md.starts_with("# 错误码文档\n"));
        // 生成时间格式 YYYY-MM-DD HH:MM:SS
        assert!(md.contains("> 生成时间: "));
        // 源文件
        assert!(md.contains("> 源文件: my_errors.yaml\n"));
        // 支持语言
        assert!(md.contains("> 支持语言: en, zh-CN\n"));
    }

    #[test]
    fn test_markdown_empty_errors_produces_empty_overview() {
        let config = DocConfig {
            default_language: "en".to_string(),
            supported_languages: vec!["en".to_string()],
            errors: vec![],
        };

        let md = DocGenerator::generate_markdown(&config, "test.yaml");

        // 应该有总览标题但没有数据行
        assert!(md.contains("## 总览\n"));
        // 表头后面不应有数据行
        let overview_section = md.split("## 总览\n").nth(1).unwrap();
        let data_lines: Vec<&str> = overview_section
            .lines()
            .filter(|l| l.starts_with("| ") && !l.starts_with("|--") && !l.contains("错误名称"))
            .collect();
        assert_eq!(data_lines.len(), 0);
    }

    // ========== 默认输出路径测试 ==========

    #[test]
    fn test_generate_default_output_path() {
        let yaml_path = Path::new("biz_errors.yaml.example");
        let default_output = Path::new("error_codes.md");

        // 清理可能存在的旧文件
        let _ = std::fs::remove_file(default_output);

        let result = DocGenerator::generate(yaml_path, None);
        assert!(result.is_ok(), "generate failed: {:?}", result.err());

        // 验证默认输出文件存在
        assert!(
            default_output.exists(),
            "Default output file error_codes.md should exist"
        );

        let content = std::fs::read_to_string(default_output).unwrap();
        assert!(content.starts_with("# 错误码文档\n"));

        // 清理
        let _ = std::fs::remove_file(default_output);
    }

    // ========== 解析 biz_errors.yaml.example 完整性测试 ==========

    #[test]
    fn test_parse_example_yaml_completeness() {
        let path = Path::new("biz_errors.yaml.example");
        let config = DocGenerator::parse_yaml(path).expect("Failed to parse example YAML");

        assert_eq!(config.default_language, "en");
        assert!(config.supported_languages.contains(&"en".to_string()));
        assert!(config.supported_languages.contains(&"zh-CN".to_string()));
        assert!(config.supported_languages.contains(&"zh-TW".to_string()));

        // 验证错误码数量（example 文件中有 25 个错误码）
        assert!(
            config.errors.len() > 20,
            "Expected at least 20 error codes, got {}",
            config.errors.len()
        );

        // 验证 success 错误码
        let success = config.errors.iter().find(|e| e.name == "success");
        assert!(success.is_some(), "Should have 'success' error code");
        let success = success.unwrap();
        assert_eq!(success.code, 0);
        assert_eq!(success.http_status, 200);
        assert_eq!(success.messages.get("en"), Some(&"SUCCESS".to_string()));
    }
}

#[cfg(feature = "runtime-messages")]
mod tests {
    use biz_error::runtime::{MessageProvider, MessageRegistry};
    use std::collections::HashMap;
    use std::future::Future;

    /// 空的 MockProvider，不返回任何消息
    struct EmptyProvider;

    impl MessageProvider for EmptyProvider {
        fn load_message(
            &self,
            _code: i32,
            _lang: &str,
        ) -> impl Future<Output = Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>>
        {
            async { Ok(None) }
        }

        fn load_all_messages(
            &self,
        ) -> impl Future<
            Output = Result<
                HashMap<(i32, String), String>,
                Box<dyn std::error::Error + Send + Sync>,
            >,
        > {
            async { Ok(HashMap::new()) }
        }
    }

    /// MessageRegistry 未初始化时 get_message 应返回 None，不 panic
    /// 注意：由于 OnceLock 是全局的，此测试在其他测试初始化 Registry 之前运行时最有意义。
    /// 但即使 Registry 已初始化，对不存在的 key 查询也应返回 None。
    #[test]
    fn test_get_message_returns_none_for_nonexistent_key() {
        // 查询一个不太可能被其他测试加载的 key
        let result = MessageRegistry::get_message(999999, "xx-XX");
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_init_with_empty_provider() {
        let provider = EmptyProvider;
        let result = MessageRegistry::init(&provider).await;
        assert!(result.is_ok());

        // 初始化后查询不存在的消息仍返回 None
        assert_eq!(MessageRegistry::get_message(1234, "en"), None);
    }

    /// 综合测试：init、get_message、reload 在同一个测试中按顺序执行
    /// 避免全局 OnceLock 状态在多个测试间竞争
    #[tokio::test]
    async fn test_init_get_and_reload() {
        struct ProviderA;
        struct ProviderB;

        impl MessageProvider for ProviderA {
            fn load_message(
                &self,
                _code: i32,
                _lang: &str,
            ) -> impl Future<
                Output = Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>,
            > {
                async { Ok(None) }
            }

            fn load_all_messages(
                &self,
            ) -> impl Future<
                Output = Result<
                    HashMap<(i32, String), String>,
                    Box<dyn std::error::Error + Send + Sync>,
                >,
            > {
                async {
                    let mut map = HashMap::new();
                    map.insert((4000, "zh-CN".to_string()), "参数无效-动态".to_string());
                    map.insert((4000, "en".to_string()), "INVALID PARAM-dynamic".to_string());
                    map.insert((1000, "en".to_string()), "OLD MESSAGE".to_string());
                    Ok(map)
                }
            }
        }

        impl MessageProvider for ProviderB {
            fn load_message(
                &self,
                _code: i32,
                _lang: &str,
            ) -> impl Future<
                Output = Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>,
            > {
                async { Ok(None) }
            }

            fn load_all_messages(
                &self,
            ) -> impl Future<
                Output = Result<
                    HashMap<(i32, String), String>,
                    Box<dyn std::error::Error + Send + Sync>,
                >,
            > {
                async {
                    let mut map = HashMap::new();
                    map.insert((2000, "en".to_string()), "NEW MESSAGE".to_string());
                    Ok(map)
                }
            }
        }

        // === Phase 1: init with ProviderA ===
        MessageRegistry::init(&ProviderA).await.unwrap();

        assert_eq!(
            MessageRegistry::get_message(4000, "zh-CN"),
            Some("参数无效-动态".to_string())
        );
        assert_eq!(
            MessageRegistry::get_message(4000, "en"),
            Some("INVALID PARAM-dynamic".to_string())
        );
        assert_eq!(
            MessageRegistry::get_message(1000, "en"),
            Some("OLD MESSAGE".to_string())
        );
        // 不存在的 key
        assert_eq!(MessageRegistry::get_message(4000, "ja"), None);

        // === Phase 2: reload with ProviderB ===
        MessageRegistry::reload(&ProviderB).await.unwrap();

        // A's messages should be gone
        assert_eq!(MessageRegistry::get_message(4000, "zh-CN"), None);
        assert_eq!(MessageRegistry::get_message(1000, "en"), None);
        // B's message should be present
        assert_eq!(
            MessageRegistry::get_message(2000, "en"),
            Some("NEW MESSAGE".to_string())
        );
    }
}

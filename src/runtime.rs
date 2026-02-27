// 📦 运行时动态消息模块
//
// 💡 支持从数据库/Redis/远程配置中心加载错误消息
// 🔄 消息查询采用"动态优先、静态回退"策略
//
// # 条件编译
//
// 此模块仅在启用 `runtime-messages` feature 时可用。

use std::collections::HashMap;
use std::future::Future;
use std::sync::{OnceLock, RwLock};

/// 运行时消息提供者接口
///
/// 用户为自己的数据源（数据库、Redis、远程配置中心等）实现此 trait。
///
/// # Examples
///
/// ```rust,ignore
/// use biz_error::MessageProvider;
/// use std::collections::HashMap;
///
/// struct DbProvider { /* ... */ }
///
/// impl MessageProvider for DbProvider {
///     async fn load_message(
///         &self,
///         code: i32,
///         lang: &str,
///     ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
///         // 从数据库查询单条消息
///         Ok(None)
///     }
///
///     async fn load_all_messages(
///         &self,
///     ) -> Result<HashMap<(i32, String), String>, Box<dyn std::error::Error + Send + Sync>> {
///         // 从数据库批量加载所有消息
///         Ok(HashMap::new())
///     }
/// }
/// ```
pub trait MessageProvider: Send + Sync {
    /// 加载单条消息
    ///
    /// - `code`: 错误码数字（如 4000）
    /// - `lang`: 语言标识（如 "zh-CN"）
    /// - 返回 `None` 表示该数据源无此消息
    fn load_message(
        &self,
        code: i32,
        lang: &str,
    ) -> impl Future<Output = Result<Option<String>, Box<dyn std::error::Error + Send + Sync>>>;

    /// 批量加载所有消息
    ///
    /// 返回 `HashMap<(i32, String), String>`，key 为 (错误码, 语言)，value 为消息文本。
    /// 用于应用启动时一次性加载到内存缓存。
    fn load_all_messages(
        &self,
    ) -> impl Future<Output = Result<HashMap<(i32, String), String>, Box<dyn std::error::Error + Send + Sync>>>;
}

/// 全局消息注册中心
///
/// 管理运行时加载的错误消息缓存。线程安全，支持并发读取和运行时重载。
///
/// # Examples
///
/// ```rust,ignore
/// use biz_error::MessageRegistry;
///
/// // 应用启动时初始化
/// MessageRegistry::init(&my_provider).await?;
///
/// // 查询消息
/// let msg = MessageRegistry::get_message(4000, "zh-CN");
///
/// // 运行时重载
/// MessageRegistry::reload(&my_provider).await?;
/// ```
pub struct MessageRegistry;

/// 全局存储：OnceLock 保证单次分配，RwLock 支持并发读 + 独占写
static MESSAGES: OnceLock<RwLock<HashMap<(i32, String), String>>> = OnceLock::new();

impl MessageRegistry {
    /// 初始化注册中心，从 MessageProvider 批量加载消息
    ///
    /// 通常在应用启动时调用一次。重复调用会覆盖已有数据（等同于 reload）。
    pub async fn init(
        provider: &impl MessageProvider,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let messages = provider.load_all_messages().await?;
        let store = MESSAGES.get_or_init(|| RwLock::new(HashMap::new()));
        let mut guard = store.write().map_err(|e| format!("RwLock poisoned: {}", e))?;
        *guard = messages;
        Ok(())
    }

    /// 查询消息
    ///
    /// 返回 `Option<String>`。未初始化时返回 `None`，不 panic。
    pub fn get_message(code: i32, lang: &str) -> Option<String> {
        MESSAGES
            .get()?
            .read()
            .ok()?
            .get(&(code, lang.to_string()))
            .cloned()
    }

    /// 重新加载全部消息
    ///
    /// 获取写锁后整体替换 HashMap 内容。
    pub async fn reload(
        provider: &impl MessageProvider,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let messages = provider.load_all_messages().await?;
        let store = MESSAGES.get_or_init(|| RwLock::new(HashMap::new()));
        let mut guard = store.write().map_err(|e| format!("RwLock poisoned: {}", e))?;
        *guard = messages;
        Ok(())
    }
}

// 📦 biz-error: 业务错误码管理框架
//
// 💡 提供业务错误处理的基类和工具
// 🌍 支持国际化错误消息
// 🎯 提供 AppError 基类，支持自定义业务异常
//
// # 快速开始
//
// ## 1. 在 Cargo.toml 中添加依赖
//
// ```toml
// [dependencies]
// biz-error = { version = "0.1", features = ["axum", "codegen"] }
// ```
//
// ## 2. 创建 biz_errors.yaml 配置文件
//
// ```yaml
// default_language: en
// supported_languages:
//   - en
//   - zh-CN
// errors:
//   success:
//     code: 0
//     http_status: 200
//     message:
//       en: "SUCCESS"
//       zh-CN: "成功"
// ```
//
// ## 3. 在代码中使用（推荐方式 - 使用过程宏，不需要 build.rs）
//
// ```rust,ignore
// use biz_error::generate_error_codes;
//
// #[generate_error_codes("biz_errors.yaml")]
// mod error_codes {}
//
// use biz_error::AppError;
// use error_codes::ErrorCode;
//
// let error = AppError::new(ErrorCode::UserNotFound);
// ```
//
// ## 4. 传统方式（使用 build.rs）
//
// 如果你更喜欢使用 build.rs，可以这样：
//
// **Cargo.toml:**
// ```toml
// [dependencies]
// biz-error = { version = "0.1", features = ["axum"] }
//
// [build-dependencies]
// biz-error = { version = "0.1", features = ["codegen"] }
// ```
//
// **build.rs:**
// ```no_run
// fn main() {
//     biz_error::generate_error_codes(
//         "biz_errors.yaml",
//         "src/error_codes.rs"
//     ).expect("Failed to generate error codes");
// }
// ```
//
// **main.rs:**
// ```rust,ignore
// mod error_codes;
//
// use biz_error::AppError;
// use error_codes::ErrorCode;
// ```

// 代码生成模块（传统方式，需要 build.rs）
#[cfg(feature = "codegen")]
pub mod codegen;

// 重新导出过程宏（推荐方式，不需要 build.rs）
#[cfg(feature = "codegen")]
pub use biz_error_macros::generate_error_codes;

// 运行时动态消息模块
#[cfg(feature = "runtime-messages")]
pub mod runtime;

#[cfg(feature = "runtime-messages")]
pub use runtime::{MessageProvider, MessageRegistry};

// 错误码文档生成模块
#[cfg(feature = "doc-gen")]
pub mod docgen;

#[cfg(feature = "doc-gen")]
pub use docgen::{DocConfig, DocGenError, DocGenerator, ErrorDefinition};

use serde::Serialize;
use serde_json::Value;
use std::fmt;

#[cfg(feature = "axum")]
use axum::{
    response::{IntoResponse, Response},
    Json,
    http::StatusCode,
};

// ============================================
// ErrorCode trait - 业务错误码必须实现的接口
// ============================================

/// 错误码 trait
///
/// 所有由 biz-error-codegen 生成的错误码枚举都会自动实现此 trait。
pub trait ErrorCode: Copy + Clone + std::fmt::Debug + PartialEq + Eq + Send + Sync + 'static {
    /// 获取数字错误码
    fn code(&self) -> i32;

    /// 获取错误消息（默认语言）
    fn message(&self) -> &'static str;

    /// 获取指定语言的错误消息
    fn message_lang(&self, lang: &str) -> &'static str;

    /// 获取 HTTP 状态码（需要 axum feature）
    #[cfg(feature = "axum")]
    fn http_status(&self) -> StatusCode;
}

// ============================================
// ErrorCodeExt trait - 动态消息扩展
// ============================================

/// ErrorCode trait 的动态消息扩展
///
/// 启用 `runtime-messages` feature 后可用。
/// 提供"动态优先、静态回退"的消息查询策略。
///
/// # Examples
///
/// ```rust,ignore
/// use biz_error::{ErrorCode, ErrorCodeExt};
///
/// // 获取动态消息（默认语言 "en"），回退到静态消息
/// let msg = error_code.dynamic_message();
///
/// // 获取指定语言的动态消息，回退到静态消息
/// let msg = error_code.dynamic_message_lang("zh-CN");
/// ```
#[cfg(feature = "runtime-messages")]
pub trait ErrorCodeExt: ErrorCode {
    /// 获取动态消息（默认语言），回退到静态消息
    fn dynamic_message(&self) -> String {
        self.dynamic_message_lang("en")
    }

    /// 获取指定语言的动态消息，回退到静态消息
    fn dynamic_message_lang(&self, lang: &str) -> String {
        MessageRegistry::get_message(self.code(), lang)
            .unwrap_or_else(|| self.message_lang(lang).to_string())
    }
}

/// 为所有实现 ErrorCode 的类型自动实现 ErrorCodeExt
#[cfg(feature = "runtime-messages")]
impl<T: ErrorCode> ErrorCodeExt for T {}

// ============================================
// 错误响应结构
// ============================================

/// 标准错误响应体
///
/// # 字段说明
/// - `code`: 错误码（数字）
/// - `msg`: 错误消息（根据语言自动选择）
/// - `data`: 可选的附加数据
///
/// # Examples
///
/// ```json
/// {
///   "code": 4000,
///   "msg": "INVALID PARAMETER",
///   "data": {
///     "field": "user_id",
///     "reason": "must be greater than 0"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    /// 错误码
    pub code: i32,
    /// 错误消息
    pub msg: String,
    /// 可选的附加数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl ErrorResponse {
    /// 从错误码创建响应
    pub fn from_error_code<E: ErrorCode>(error_code: E) -> Self {
        Self {
            code: error_code.code(),
            msg: error_code.message().to_string(),
            data: None,
        }
    }

    /// 设置自定义消息
    pub fn with_msg(mut self, msg: impl Into<String>) -> Self {
        self.msg = msg.into();
        self
    }

    /// 设置附加数据
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

// ============================================
// AppError 基类 - 业务错误的顶级抽象
// ============================================

/// 业务错误基类
///
/// 这是所有业务错误的顶级抽象，类似于 Java 的异常基类。
///
/// # 核心特性
/// - **code**: 错误码（通过 ErrorCode 枚举获取）
/// - **msg**: 错误消息（支持自定义覆盖默认消息）
/// - **data**: 可选的业务数据（携带错误上下文）
///
/// # 设计理念
///
/// ```text
///     AppError (基类)
///          ↑
///          │ 继承/组合
///          ├─ UserError
///          ├─ OrderError
///          └─ PaymentError
/// ```
///
/// # Examples
///
/// ## 基础使用
///
/// ```rust,ignore
/// use biz_error::AppError;
/// use crate::error_codes::ErrorCode;
///
/// // 直接使用 ErrorCode 创建
/// let error: AppError<ErrorCode> = AppError::new(ErrorCode::InvalidParam);
/// assert_eq!(error.code(), 4000);
/// ```
///
/// ## 自定义消息和数据
///
/// ```rust,ignore
/// use biz_error::AppError;
/// use crate::error_codes::ErrorCode;
/// use serde_json::json;
///
/// let error = AppError::new(ErrorCode::InvalidParam)
///     .with_msg("用户ID不能为空")
///     .with_data(json!({ "field": "user_id" }));
/// ```
///
/// ## 自定义业务错误类型
///
/// ```rust,ignore
/// use biz_error::AppError;
/// use crate::error_codes::ErrorCode;
///
/// /// 用户相关错误
/// pub struct UserError(AppError<ErrorCode>);
///
/// impl UserError {
///     /// 用户不存在
///     pub fn not_found(id: u64) -> Self {
///         Self(AppError::new(ErrorCode::NotFound)
///             .with_msg(format!("用户 {} 不存在", id))
///             .with_data(serde_json::json!({ "user_id": id })))
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AppError<E: ErrorCode> {
    /// 错误码
    error_code: E,
    /// 自定义消息（覆盖默认消息）
    custom_msg: Option<String>,
    /// 附加数据
    data: Option<Value>,
    /// 语言标识（用于动态消息查询）
    #[cfg(feature = "runtime-messages")]
    lang: Option<String>,
}


impl<E: ErrorCode> AppError<E> {
    /// 创建新的业务错误
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use biz_error::AppError;
    /// use crate::error_codes::ErrorCode;
    ///
    /// let error = AppError::new(ErrorCode::InvalidParam);
    /// ```
    pub fn new(error_code: E) -> Self {
        Self {
            error_code,
            custom_msg: None,
            data: None,
            #[cfg(feature = "runtime-messages")]
            lang: None,
        }
    }

    /// 设置自定义消息
    ///
    /// # Examples
    ///
    /// ```
    /// use biz_error::{AppError, ErrorCode};
    ///
    /// let error = AppError::new(ErrorCode::InvalidParam)
    ///     .with_msg("用户ID不能为空");
    /// ```
    pub fn with_msg(mut self, msg: impl Into<String>) -> Self {
        self.custom_msg = Some(msg.into());
        self
    }

    /// 设置附加数据
    ///
    /// # Examples
    ///
    /// ```
    /// use biz_error::{AppError, ErrorCode};
    /// use serde_json::json;
    ///
    /// let error = AppError::new(ErrorCode::InvalidParam)
    ///     .with_data(json!({ "field": "user_id" }));
    /// ```
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// 设置语言标识（用于动态消息查询）
    ///
    /// 启用 `runtime-messages` feature 后可用。
    /// 设置后，`resolved_msg()` 和 `to_response()` 将使用指定语言查询动态消息。
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let error = AppError::new(ErrorCode::InvalidParam)
    ///     .with_lang("zh-CN");
    /// ```
    #[cfg(feature = "runtime-messages")]
    pub fn with_lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }

    /// 获取错误码枚举
    pub fn error_code(&self) -> E {
        self.error_code
    }

    /// 获取数字错误码
    pub fn code(&self) -> i32 {
        self.error_code.code()
    }

    /// 获取错误消息
    pub fn msg(&self) -> &str {
        self.custom_msg
            .as_deref()
            .unwrap_or_else(|| self.error_code.message())
    }

    /// 获取解析后的完整消息（runtime-messages 条件编译）
    ///
    /// 消息优先级链：
    /// 1. `custom_msg`（用户通过 `with_msg()` 设置）— 最高优先级
    /// 2. `dynamic_message_lang(lang)`（`runtime-messages` 启用时）— 中优先级
    /// 3. `error_code.message()`（静态消息）— 回退
    #[cfg(feature = "runtime-messages")]
    pub fn resolved_msg(&self) -> String {
        if let Some(ref msg) = self.custom_msg {
            return msg.clone();
        }
        let lang = self.lang.as_deref().unwrap_or("en");
        self.error_code.dynamic_message_lang(lang)
    }

    /// 获取附加数据
    pub fn data(&self) -> Option<&Value> {
        self.data.as_ref()
    }

    /// 转换为 ErrorResponse
    pub fn to_response(&self) -> ErrorResponse {
        let mut resp = ErrorResponse::from_error_code(self.error_code);

        // 当 runtime-messages 启用时，使用完整优先级链获取消息
        #[cfg(feature = "runtime-messages")]
        {
            resp = resp.with_msg(self.resolved_msg());
        }

        // 当 runtime-messages 未启用时，仅使用 custom_msg 覆盖
        #[cfg(not(feature = "runtime-messages"))]
        if let Some(ref msg) = self.custom_msg {
            resp = resp.with_msg(msg);
        }

        if let Some(ref data) = self.data {
            resp = resp.with_data(data.clone());
        }
        resp
    }

    /// 创建带数据的错误（便捷方法）
    pub fn with_code_and_data(error_code: E, data: Value) -> Self {
        Self::new(error_code).with_data(data)
    }
}

impl<E: ErrorCode> fmt::Display for AppError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code(), self.msg())
    }
}

impl<E: ErrorCode> std::error::Error for AppError<E> {}

// ============================================
// From 实现常见错误类型
// ============================================

impl<E: ErrorCode> From<E> for AppError<E> {
    fn from(error_code: E) -> Self {
        Self::new(error_code)
    }
}

// Note: Removed anyhow::Error conversion because we don't know what error code to use
// Users should explicitly create AppError with appropriate error code:
// AppError::new(YourErrorCode::InternalError).with_msg(err.to_string())

#[cfg(feature = "axum")]
impl<E: ErrorCode> IntoResponse for AppError<E> {
    fn into_response(self) -> Response {
        let status = self.error_code.http_status();
        let resp = self.to_response();
        (status, Json(resp)).into_response()
    }
}

// Note: Cannot implement IntoResponse for all ErrorCode trait implementers
// due to orphan rule. Users should use AppError<E> instead which implements IntoResponse.

/// 基础使用示例
use biz_error::{AppError, ErrorCode};
use serde_json::json;

/// 定义用户相关的业务错误类型
pub struct UserError(AppError);

impl UserError {
    /// 用户不存在
    pub fn not_found(id: u64) -> Self {
        Self(
            AppError::new(ErrorCode::NotFound)
                .with_msg(format!("用户 {} 不存在", id))
                .with_data(json!({"user_id": id}))
        )
    }

    /// 用户已存在
    pub fn already_exists(email: &str) -> Self {
        Self(
            AppError::new(ErrorCode::AlreadyExists)
                .with_msg(format!("用户 {} 已存在", email))
        )
    }

    /// 参数无效
    pub fn invalid_param(msg: &str) -> Self {
        Self(
            AppError::new(ErrorCode::InvalidParam)
                .with_msg(msg)
        )
    }
}

/// 模拟数据库操作
mod db {
    pub struct User {
        pub id: u64,
        pub name: String,
    }

    pub fn find_user(id: u64) -> Result<Option<User>, String> {
        if id == 0 {
            Err("Invalid user ID".to_string())
        } else if id == 404 {
            Ok(None)
        } else {
            Ok(Some(User {
                id,
                name: format!("User {}", id),
            }))
        }
    }
}

/// 获取用户
pub fn get_user(id: u64) -> Result<db::User, UserError> {
    if id == 0 {
        return Err(UserError::invalid_param("用户ID不能为0"));
    }

    db::find_user(id)
        .map_err(|e| UserError::invalid_param(&e))
        .and_then(|user| user.ok_or_else(|| UserError::not_found(id)))
}

#[tokio::main]
async fn main() {
    println!("=== biz-error 基础使用示例 ===\n");

    // 示例 1: 基础错误
    let error = AppError::new(ErrorCode::InvalidParam);
    println!("错误码: {}", error.code());
    println!("错误消息: {}", error.msg());
    println!("错误详情: {}\n", error);

    // 示例 2: 自定义消息和数据
    let error = AppError::new(ErrorCode::InvalidParam)
        .with_msg("用户ID格式错误")
        .with_data(json!({"field": "user_id", "expected": "number"}));
    println!("自定义错误: {}", error);
    if let Some(data) = error.data() {
        println!("附加数据: {}\n", data);
    }

    // 示例 3: 国际化
    println!("国际化消息:");
    println!("  EN: {}", ErrorCode::InvalidParam.message_lang("en"));
    println!("  ZH-CN: {}\n", ErrorCode::InvalidParam.message_lang("zh-CN"));

    // 示例 4: 使用自定义业务错误类型
    println!("=== 业务错误使用示例 ===\n");

    match get_user(0) {
        Ok(user) => println!("找到用户: {}", user.name),
        Err(e) => println!("错误: {}", e.0),
    }

    match get_user(404) {
        Ok(user) => println!("找到用户: {}", user.name),
        Err(e) => println!("错误: {}", e.0),
    }

    match get_user(123) {
        Ok(user) => println!("找到用户: {}", user.name),
        Err(e) => println!("错误: {}", e.0),
    }
}

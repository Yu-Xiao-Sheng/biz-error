# biz-error

<div align="center">

**🎯 业务错误码管理框架 - 让 Rust 错误处理更优雅**

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Crates.io](https://img.shields.io/crates/v/biz-error.svg)](https://crates.io/crates/biz-error)

[功能特性](#-功能特性) • [快速开始](#-快速开始) • [使用示例](#-使用示例) • [API 文档](#-api-文档) • [设计文档](DESIGN.md) • [架构图](ARCHITECTURE.md)

</div>

---

## 💡 简介

`biz-error` 是一个专为 Rust 业务系统设计的错误码管理框架。它通过 YAML 配置文件自动生成错误码枚举，提供了类型安全、易于维护的错误处理方式，让开发者能够优雅地处理业务错误。

### 🎯 解决的问题

你是否在 Rust 开发中遇到过以下困扰：

1. **错误码管理混乱**：代码中到处散落着硬编码的错误码数字，难以维护
2. **国际化支持困难**：错误消息写死在代码中，很难实现多语言支持
3. **缺少错误上下文**：错误信息简单，无法携带详细的业务数据
4. **难以扩展定制**：难以定义特定领域的业务错误类型，缺少灵活的错误组织方式
5. **代码重复**：每次定义错误都要重复写 code、msg 等字段
6. **配置繁琐**：需要维护 build.rs 文件，增加项目复杂度

`biz-error` 专门解决这些问题！**零 build.rs，一个属性宏搞定一切！**

---

## ✨ 核心特性

### 📝 配置驱动
- 只需维护 YAML 配置文件，自动生成类型安全的错误码枚举
- **过程宏自动生成**：无需 build.rs，一个属性宏即可完成代码生成
- 编译时生成，零运行时开销
- 修改配置后重新编译即可

### 🚀 极简集成
- **推荐方式**：使用 `#[generate_error_codes]` 过程宏，零配置
- **传统方式**：支持 build.rs 方式，向后兼容
- 自动导入 trait，开箱即用

### 🌍 内置国际化
- 一次配置，自动支持多语言错误消息
- 运行时动态切换语言
- 支持任意语言扩展

### 🎓 优秀的错误处理体系
- 提供 `AppError<E: ErrorCode>` 泛型基类
- 支持通过"组合"模式定义特定领域的业务错误类型
- 自定义消息和附加数据，灵活应对各种场景

### 🔌 完美的 Axum 集成
- 开箱即用的 `IntoResponse` 实现
- 自动映射 HTTP 状态码
- 标准 JSON 错误响应格式

### 💡 开发者友好
- IDE 自动补全，告别拼写错误
- 详细的文档注释，鼠标悬停即可查看错误含义
- 类型安全，编译时检查

---

## 🚀 快速开始

### 1. 添加依赖

```toml
[dependencies]
biz-error = { version = "0.1", features = ["axum", "codegen"] }
```

### 2. 创建配置文件

在项目根目录创建 `biz_errors.yaml`：

```yaml
default_language: zh-CN

errors:
  invalid_param:
    code: 4000
    http_status: 400
    message:
      en: "INVALID PARAMETER"
      zh-CN: "参数无效"

  user_not_found:
    code: 4004
    http_status: 404
    message:
      en: "USER NOT FOUND"
      zh-CN: "用户不存在"

  database_error:
    code: 5000
    http_status: 500
    message:
      en: "DATABASE ERROR"
      zh-CN: "数据库错误"
```

### 3. 使用过程宏自动生成代码

```rust
use biz_error::generate_error_codes;

// 一行代码，自动生成 ErrorCode 枚举！
#[generate_error_codes("biz_errors.yaml")]
mod error_codes {}

use biz_error::AppError;
use error_codes::ErrorCode;

// 开始使用！
fn get_user(id: u64) -> Result<User, AppError<ErrorCode>> {
    if id == 0 {
        return Err(AppError::new(ErrorCode::InvalidParam)
            .with_msg("用户ID不能为0"));
    }
    // ...
}
```

### 4. 在 Axum handler 中使用

```rust
use axum::{Json, response::IntoResponse};

async fn get_user_handler(
    Path(id): Path<u64>
) -> Result<Json<User>, AppError<ErrorCode>> {
    if id == 0 {
        return Err(AppError::new(ErrorCode::InvalidParam)
            .with_msg("用户ID不能为0")
            .with_data(serde_json::json!({ "user_id": id })));
    }
    // ...
}
```

就这么简单！不需要 build.rs，不需要手动生成代码，一切自动完成！

---

## 📚 使用示例

### 示例 1：自定义业务错误类型

```rust
use biz_error::AppError;
use error_codes::ErrorCode;
use serde_json::json;

/// 用户相关错误
pub struct UserError(AppError<ErrorCode>);

impl UserError {
    /// 用户不存在
    pub fn not_found(id: u64) -> Self {
        Self(AppError::new(ErrorCode::NotFound)
            .with_msg(format!("用户 {} 不存在", id))
            .with_data(json!({ "user_id": id })))
    }

    /// 用户已存在
    pub fn already_exists(email: &str) -> Self {
        Self(AppError::new(ErrorCode::AlreadyExists)
            .with_msg(format!("用户 {} 已存在", email)))
    }
}

// 在 handler 中使用
pub async fn get_user(id: u64) -> Result<Json<User>, UserError> {
    if id == 0 {
        return Err(UserError::not_found(id));
    }

    let user = db::find_user(id).await
        .map_err(|_| UserError::not_found(id))?;

    Ok(Json(user))
}
```

### 示例 2：国际化错误消息

```rust
use error_codes::ErrorCode;

// 获取中文消息
let msg = ErrorCode::InvalidParam.message_lang("zh-CN");
assert_eq!(msg, "参数无效");

// 获取英文消息
let msg = ErrorCode::InvalidParam.message_lang("en");
assert_eq!(msg, "INVALID PARAMETER");

// 获取默认语言消息
let msg = ErrorCode::InvalidParam.message();
```

### 示例 3：标准 JSON 响应格式

当使用 Axum 时，错误会自动转换为标准 JSON 响应：

```json
{
  "code": 4000,
  "msg": "参数无效",
  "data": {
    "field": "user_id",
    "reason": "不能为0"
  }
}
```

### 示例 4：完整的 HTTP handler 示例

```rust
use axum::{Json, extract::Path};
use biz_error::AppError;
use error_codes::ErrorCode;
use serde_json::json;

async fn update_user(
    Path(id): Path<u64>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<User>, AppError<ErrorCode>> {
    // 参数验证
    if payload.name.is_empty() {
        return Err(AppError::new(ErrorCode::InvalidParam)
            .with_msg("用户名不能为空")
            .with_data(json!({ "field": "name" })));
    }

    // 业务逻辑
    let user = db::update_user(id, payload).await
        .map_err(|e| AppError::new(ErrorCode::DatabaseError)
            .with_msg(format!("更新用户失败: {}", e)))?;

    Ok(Json(user))
}
```

---

## 🎨 设计理念

### 过程宏自动生成（推荐）

```text
┌─────────────────────┐
│   biz_errors.yaml   │  ← 你只需要维护配置文件
│   (配置文件)         │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────┐
│  #[generate_error_codes(...)]   │  ← 过程宏自动读取并生成
│  mod error_codes {}             │     （编译时，零运行时开销）
└──────────┬──────────────────────┘
           │
           ▼
┌─────────────────────┐
│  error_codes (模块) │  ← 自动生成的枚举（IDE 可索引）
│  - ErrorCode 枚举   │     （无需手动编辑）
│  - trait 实现       │
│  - ALL_ERROR_CODES  │
└─────────────────────┘
```

### 错误类型层次

```text
           ErrorCode (枚举)
                ↑
                │  实现 ErrorCode trait
                │
        AppError<E: ErrorCode> (泛型基类)
    ┌───────────┼───────────┐
    │           │           │
UserError  OrderError  PaymentError ...
  (组合)      (组合)        (组合)
```

### 传统方式（可选）

如果你更喜欢使用 build.rs，也可以这样：

**Cargo.toml:**
```toml
[dependencies]
biz-error = { version = "0.1", features = ["axum"] }

[build-dependencies]
biz-error = { version = "0.1", features = ["codegen"] }
```

**build.rs:**
```rust
fn main() {
    biz_error::generate_error_codes(
        "biz_errors.yaml",
        "src/error_codes.rs"
    ).expect("Failed to generate error codes");
}
```

**main.rs:**
```rust
mod error_codes;  // 包含生成的代码

use biz_error::AppError;
use error_codes::ErrorCode;
```

---

## 📖 配置文件详解

### 完整配置示例

```yaml
# 默认语言
default_language: zh-CN

# 支持的语言列表（可选）
supported_languages:
  - en
  - zh-CN
  - zh-TW

# 错误码定义
errors:
  # 错误名称（会转换为 PascalCase 枚举值）
  invalid_param:
    # 数字错误码
    code: 4000
    # HTTP 状态码（可选，默认 500）
    http_status: 400
    # 多语言消息
    message:
      en: "INVALID PARAMETER"
      zh-CN: "参数无效"
      zh-TW: "參數無效"
```

### 命名规则

- **配置文件中的错误名**：`snake_case`（如 `invalid_param`）
- **生成的枚举值**：`PascalCase`（如 `InvalidParam`）

---

## 🔧 API 文档

### AppError 基类

`AppError<E: ErrorCode>` - 泛型业务错误基类

| 方法 | 说明 |
|------|------|
| `new(error_code: E)` | 创建新错误 |
| `with_msg(msg: impl Into<String>)` | 设置自定义消息 |
| `with_data(data: Value)` | 设置附加数据 |
| `code() -> i32` | 获取错误码 |
| `msg() -> &str` | 获取错误消息 |
| `data() -> Option<&Value>` | 获取附加数据 |
| `to_response() -> ErrorResponse` | 转换为响应结构 |

### ErrorCode trait

所有生成的错误码枚举都会自动实现此 trait

| 方法 | 说明 |
|------|------|
| `code() -> i32` | 获取数字错误码 |
| `message() -> &'static str` | 获取默认语言消息 |
| `message_lang(lang: &str) -> &'static str` | 获取指定语言消息 |
| `http_status() -> StatusCode` | 获取 HTTP 状态码（需要 axum feature） |

---

## 🙋‍♀️ 常见问题

### Q: 修改配置文件后需要做什么？

A: 只需重新运行 `cargo build`，过程宏会自动重新生成代码。

### Q: 可以运行时动态添加错误码吗？

A: 不支持。错误码是编译时生成的，这样才能保证类型安全。

### Q: 如何添加新的语言？

A: 在 `biz_errors.yaml` 的每个错误的 `message` 字段添加新语言即可。

### Q: data 字段可以是任何类型吗？

A: `data` 字段是 `Option<serde_json::Value>`，可以是任何可序列化为 JSON 的数据。

### Q: 必须使用过程宏吗？

A: 不是必须的。你也可以使用传统的 build.rs 方式，两者功能完全相同。过程宏方式更简洁，推荐使用。

### Q: 生成的错误码可以导出给其他 crate 使用吗？

A: 可以。只需在定义错误码的模块中添加 `pub`，并在其他 crate 中正常引入即可。

---

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

### 开发计划

- [x] 过程宏自动生成（已完成）
- [ ] 支持从数据库加载错误消息
- [ ] 提供错误码文档生成工具
- [ ] 支持自定义错误响应格式
- [ ] 提供迁移工具（从其他错误处理库）

---

## 📄 License

本项目采用 MIT 或 Apache-2.0 双重许可。

---

## 🙏 致谢

感谢所有为这个项目做出贡献的开发者！

---

## 📚 更多文档

- 📖 [设计文档 (DESIGN.md)](DESIGN.md) - 详细的需求分析和设计方案
- 🏗️ [架构图 (ARCHITECTURE.md)](ARCHITECTURE.md) - 可视化的架构图和流程图
- 📝 [示例代码 (examples/)](examples/) - 完整的使用示例

---

<div align="center">

**让 Rust 错误处理更优雅 ⚡**

[官网](https://github.com/yourusername/biz-error) • [文档](https://docs.rs/biz-error) • [示例](examples/)

Made with ❤️ by Rust Community

</div>

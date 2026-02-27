# 实现计划: biz-error v0.2 — 数据库动态消息与文档生成

## 概述

按模块递增实现 v0.2 的两大功能：运行时动态消息（`runtime-messages` feature）和错误码文档生成（`doc-gen` feature）。每个模块先实现核心类型和接口，再集成到现有框架，最后编写测试验证正确性。所有新代码通过 feature flag 条件编译，不影响现有用户。

## 任务

- [x] 1. 配置 Feature Flags 和依赖项
  - 在 `Cargo.toml` 中新增 `runtime-messages = ["dep:tokio"]` 和 `doc-gen = ["dep:serde_yaml"]` feature
  - 添加 `tokio = { version = "1", features = ["sync"], optional = true }` 依赖
  - 确保 `default` feature 列表保持为 `["axum"]`，不包含新 feature
  - 确保 `doc-gen` 复用已有的 `serde_yaml` 可选依赖
  - 在 `[dev-dependencies]` 中添加 `proptest = "1.4"` 和 `tokio = { version = "1", features = ["full"] }`
  - _需求: 8.1, 8.2, 8.3, 8.4, 8.5_

- [x] 2. 实现运行时动态消息模块（`runtime-messages`）
  - [x] 2.1 创建 `src/runtime.rs`，实现 MessageProvider trait 和 MessageRegistry
    - 定义 `MessageProvider` trait，包含 `load_message` 和 `load_all_messages` 两个异步方法（使用 RPITIT，返回 `Result`）
    - 实现 `MessageRegistry` 结构体，使用 `OnceLock<RwLock<HashMap<(i32, String), String>>>` 全局存储
    - 实现 `MessageRegistry::init`、`get_message`、`reload` 方法
    - 在 `src/lib.rs` 中添加 `#[cfg(feature = "runtime-messages")] pub mod runtime;` 并重新导出公共类型
    - _需求: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

  - [x] 2.2 实现 ErrorCodeExt 扩展 trait
    - 在 `src/lib.rs` 中定义 `ErrorCodeExt` trait（条件编译 `runtime-messages`）
    - 实现 `dynamic_message` 和 `dynamic_message_lang` 方法，采用"动态优先、静态回退"策略
    - 添加 blanket impl `impl<T: ErrorCode> ErrorCodeExt for T {}`
    - _需求: 3.1, 3.2, 3.3, 3.4, 3.5_

  - [x] 2.3 扩展 AppError 集成动态消息
    - 在 `AppError<E>` 结构体中添加 `#[cfg(feature = "runtime-messages")] lang: Option<String>` 字段
    - 实现 `with_lang` 方法（条件编译）
    - 修改 `msg()` 方法，在 `runtime-messages` 启用时按优先级链返回消息：custom_msg > dynamic_message_lang > 静态消息
    - 修改 `to_response()` 方法同步使用新的 `msg()` 逻辑
    - 确保 `new()` 方法在条件编译下正确初始化 `lang: None`
    - _需求: 4.1, 4.2, 4.3, 4.4_

  - [ ]* 2.4 编写属性测试 — Property 1: MessageRegistry init/get 往返一致性
    - **Property 1: MessageRegistry init/get 往返一致性**
    - 创建 `tests/runtime_props.rs`，使用 proptest 生成随机 `HashMap<(i32, String), String>`
    - 通过 MockProvider 加载到 MessageRegistry 后，验证每个 `(code, lang)` 键的 `get_message` 返回值与原始值一致
    - **验证: 需求 2.1, 2.3**

  - [ ]* 2.5 编写属性测试 — Property 2: MessageRegistry reload 替换消息
    - **Property 2: MessageRegistry reload 替换消息**
    - 生成两组随机消息集合 A 和 B（有部分键重叠），先 init A 再 reload B
    - 验证 B 中的键返回 B 的值，仅在 A 中的键返回 None
    - **验证: 需求 2.4**

  - [ ]* 2.6 编写属性测试 — Property 3: dynamic_message_lang 动态优先静态回退
    - **Property 3: dynamic_message_lang 动态优先静态回退**
    - 生成随机错误码和语言，随机决定 Registry 中是否有对应消息
    - 验证有动态消息时返回动态消息，无动态消息时回退到静态消息
    - **验证: 需求 3.3, 3.4**

  - [ ]* 2.7 编写属性测试 — Property 4: message()/message_lang() 向后兼容
    - **Property 4: message()/message_lang() 向后兼容**
    - 验证无论 MessageRegistry 是否初始化，`message()` 和 `message_lang()` 始终返回编译时静态字符串
    - **验证: 需求 3.5**

  - [ ]* 2.8 编写属性测试 — Property 5: AppError 消息优先级链
    - **Property 5: AppError 消息优先级链**
    - 生成随机 ErrorCode、可选 custom_msg、可选 lang，验证 `msg()` 按优先级链返回正确消息
    - **验证: 需求 4.1, 4.2, 4.3**

- [x] 3. 检查点 — 运行时消息模块验证
  - 确保所有测试通过，询问用户是否有疑问。

- [x] 4. 实现错误码文档生成模块（`doc-gen`）
  - [x] 4.1 创建 `src/docgen.rs`，实现数据模型和 YAML 解析
    - 定义 `DocConfig`、`ErrorDefinition`、`DocGenError` 类型
    - 实现 `DocGenerator::parse_yaml` 方法，从 YAML 文件解析为 `DocConfig`
    - 实现 `DocGenError` 的 `Display` 和 `Error` trait，确保错误信息包含文件路径
    - 在 `src/lib.rs` 中添加 `#[cfg(feature = "doc-gen")] pub mod docgen;` 并重新导出公共类型
    - _需求: 5.1, 5.2, 5.3, 5.4, 5.5_

  - [x] 4.2 实现 Markdown 文档生成
    - 实现 `DocGenerator::generate_markdown` 方法，从 `DocConfig` 生成 Markdown 字符串
    - 生成文档头部元信息（生成时间、源文件路径、支持语言列表）
    - 生成总览表格（错误名称、数字码、HTTP 状态码、默认语言消息）
    - 按错误码千位范围分组（`code / 1000 * 1000`），每组生成独立二级标题和表格
    - 为每个错误码生成多语言消息详情列表
    - 实现 `DocGenerator::generate` 一步完成方法（解析 + 生成 + 写入文件），默认输出到 `error_codes.md`
    - _需求: 6.1, 6.2, 6.3, 6.4, 6.5_

  - [ ]* 4.3 编写属性测试 — Property 6: YAML 解析完整性往返
    - **Property 6: YAML 解析完整性往返**
    - 创建 `tests/docgen_props.rs`，使用 proptest 生成随机 `DocConfig`
    - 序列化为 YAML 后再解析，验证结果与原始值相等
    - **验证: 需求 5.1, 5.2, 7.1, 7.2**

  - [ ]* 4.4 编写属性测试 — Property 7: YAML 解析错误包含路径信息
    - **Property 7: YAML 解析错误包含路径信息**
    - 生成随机文件路径和无效 YAML 内容，验证 `DocGenError` 包含原始文件路径
    - **验证: 需求 5.3**

  - [ ]* 4.5 编写属性测试 — Property 8: Markdown 错误码条目数一致
    - **Property 8: Markdown 错误码条目数一致**
    - 生成随机 `DocConfig`，验证 Markdown 总览表格数据行数等于 `config.errors.len()`
    - **验证: 需求 6.1, 7.3**

  - [ ]* 4.6 编写属性测试 — Property 9: Markdown 按千位范围正确分组
    - **Property 9: Markdown 按千位范围正确分组**
    - 生成跨多个千位范围的随机错误码集合，验证同千位范围的错误码在同一分组下
    - **验证: 需求 6.2**

  - [ ]* 4.7 编写属性测试 — Property 10: Markdown 包含所有语言消息
    - **Property 10: Markdown 包含所有语言消息**
    - 生成随机 `ErrorDefinition` 和多语言消息，验证 Markdown 详情部分包含所有语言和消息文本
    - **验证: 需求 6.3**

- [x] 5. 检查点 — 文档生成模块验证
  - 确保所有测试通过，询问用户是否有疑问。

- [x] 6. 集成与最终验证
  - [x] 6.1 编写单元测试
    - 创建 `tests/runtime_unit.rs`：测试 MessageRegistry 未初始化时 `get_message` 返回 None
    - 创建 `tests/docgen_unit.rs`：测试具体 YAML 解析错误消息格式、Markdown 文档头部元信息格式、默认输出路径 `error_codes.md`
    - _需求: 2.5, 5.3, 6.4, 6.5_

  - [x] 6.2 验证 feature flag 编译隔离
    - 确保仅启用 `default` feature 时编译通过，不引入 v0.2 新增依赖
    - 确保 `runtime-messages` 和 `doc-gen` 可独立启用，互不依赖
    - _需求: 8.3, 8.4, 8.5_

- [x] 7. 最终检查点 — 全部测试通过
  - 确保所有测试通过，询问用户是否有疑问。

## 备注

- 标记 `*` 的子任务为可选任务，可跳过以加速 MVP 交付
- 每个任务引用了具体的需求编号，确保可追溯性
- 属性测试使用 `proptest` 库，每个属性对应设计文档中的一个正确性属性
- 检查点任务用于阶段性验证，确保增量开发的正确性
- 所有新代码通过 `#[cfg(feature = "...")]` 条件编译，保持向后兼容

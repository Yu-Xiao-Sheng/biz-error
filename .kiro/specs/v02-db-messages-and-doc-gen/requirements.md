# 需求文档

## 简介

biz-error v0.2 版本新增两大功能：

1. **数据库动态消息加载** — 支持在运行时从数据库（或其他外部数据源）加载错误消息，突破当前仅编译时静态生成 `&'static str` 的限制，使错误消息可在不重新编译的情况下更新。
2. **错误码文档生成工具** — 从 YAML 配置文件自动生成结构化的错误码文档（Markdown 格式），方便前后端团队协作和 API 文档维护。

## 术语表

- **ErrorCode_Trait**: `biz_error::ErrorCode` trait，所有错误码枚举必须实现的接口，提供 `code()`、`message()`、`message_lang()`、`http_status()` 方法
- **AppError**: `biz_error::AppError<E: ErrorCode>` 泛型业务错误基类，包含错误码、自定义消息和附加数据
- **MessageProvider**: 运行时错误消息提供者 trait，定义从外部数据源获取错误消息的接口
- **MessageRegistry**: 全局错误消息注册中心，管理运行时加载的错误消息缓存
- **YAML_Config**: biz_errors.yaml 配置文件，定义错误码、HTTP 状态码和多语言消息
- **DocGenerator**: 错误码文档生成器，从 YAML_Config 解析并输出 Markdown 格式文档
- **静态消息**: 编译时从 YAML_Config 生成的 `&'static str` 错误消息（v0.1 现有行为）
- **动态消息**: 运行时从数据库或其他外部数据源加载的 `String` 错误消息（v0.2 新增）

## 需求

### 需求 1: MessageProvider trait 定义

**用户故事:** 作为库开发者，我希望定义一个标准的消息提供者接口，以便用户可以实现自定义的消息加载逻辑（如从数据库、Redis、远程配置中心加载）。

#### 验收标准

1. THE MessageProvider SHALL 定义一个异步方法 `load_messages`，接收错误码（i32）和语言标识（&str）作为参数，返回 `Option<String>`
2. THE MessageProvider SHALL 定义一个异步方法 `load_all_messages`，返回所有错误码和语言的消息映射（HashMap<(i32, String), String>）
3. THE MessageProvider SHALL 作为一个 trait 使用 `async_trait` 或 Rust 原生 async trait（取决于 MSRV），使用户可以为任意数据源实现该接口
4. THE MessageProvider SHALL 通过 feature flag `runtime-messages` 进行条件编译，不影响仅使用静态消息的用户

### 需求 2: MessageRegistry 全局消息注册中心

**用户故事:** 作为应用开发者，我希望在应用启动时从数据库批量加载错误消息到内存缓存中，以便运行时高效查询动态消息。

#### 验收标准

1. THE MessageRegistry SHALL 提供 `init` 方法，接收一个实现了 MessageProvider 的实例，在应用启动时调用 `load_all_messages` 批量加载消息到内存
2. THE MessageRegistry SHALL 使用线程安全的内部数据结构（如 `RwLock<HashMap>`）存储已加载的消息
3. THE MessageRegistry SHALL 提供 `get_message` 方法，接收错误码（i32）和语言标识（&str），返回 `Option<&str>` 或 `Option<String>`
4. THE MessageRegistry SHALL 提供 `reload` 方法，支持在运行时重新从 MessageProvider 加载全部消息
5. WHEN MessageRegistry 未初始化时，THE MessageRegistry SHALL 使 `get_message` 返回 None，不产生 panic
6. THE MessageRegistry SHALL 通过 feature flag `runtime-messages` 进行条件编译

### 需求 3: ErrorCode trait 扩展以支持动态消息

**用户故事:** 作为应用开发者，我希望在启用动态消息后，`ErrorCode` 的 `message()` 和 `message_lang()` 方法能优先返回数据库中的消息，若数据库无对应消息则回退到编译时静态消息。

#### 验收标准

1. WHEN feature `runtime-messages` 启用时，THE ErrorCode_Trait SHALL 新增 `dynamic_message` 方法，返回 `String`，优先从 MessageRegistry 查询动态消息
2. WHEN feature `runtime-messages` 启用时，THE ErrorCode_Trait SHALL 新增 `dynamic_message_lang` 方法，接收语言参数，返回 `String`，优先从 MessageRegistry 查询动态消息
3. WHEN MessageRegistry 中存在对应错误码和语言的动态消息时，THE `dynamic_message_lang` 方法 SHALL 返回该动态消息
4. WHEN MessageRegistry 中不存在对应消息时，THE `dynamic_message_lang` 方法 SHALL 回退返回编译时静态消息（调用 `message_lang`）
5. THE 现有 `message()` 和 `message_lang()` 方法的签名和行为 SHALL 保持不变，确保向后兼容

### 需求 4: AppError 集成动态消息

**用户故事:** 作为应用开发者，我希望 AppError 在构建错误响应时能自动使用动态消息，无需手动查询 MessageRegistry。

#### 验收标准

1. WHEN feature `runtime-messages` 启用且未设置 custom_msg 时，THE AppError SHALL 在 `msg()` 方法中优先使用 `dynamic_message` 获取消息
2. WHEN 用户通过 `with_msg()` 设置了自定义消息时，THE AppError SHALL 优先使用自定义消息，忽略动态消息和静态消息
3. THE AppError SHALL 提供 `with_lang` 方法，允许指定语言标识，使 `msg()` 和 `to_response()` 使用指定语言的动态消息
4. WHEN feature `runtime-messages` 未启用时，THE AppError SHALL 保持 v0.1 的行为不变

### 需求 5: 错误码文档生成器（YAML 解析）

**用户故事:** 作为库使用者，我希望有一个工具能解析 YAML_Config 文件并提取所有错误码定义的结构化数据，以便生成文档。

#### 验收标准

1. THE DocGenerator SHALL 解析 YAML_Config 文件，提取每个错误码的名称（snake_case 键名）、数字码、HTTP 状态码和所有语言的消息
2. THE DocGenerator SHALL 解析 YAML_Config 中的 `default_language` 和 `supported_languages` 字段
3. IF YAML_Config 文件不存在或格式无效，THEN THE DocGenerator SHALL 返回包含文件路径和具体解析错误位置的错误信息
4. THE DocGenerator SHALL 将解析结果存储为结构化的中间表示（如 `Vec<ErrorDefinition>`），与输出格式解耦
5. THE DocGenerator SHALL 通过 feature flag `doc-gen` 进行条件编译

### 需求 6: Markdown 文档输出

**用户故事:** 作为团队负责人，我希望自动生成 Markdown 格式的错误码文档，包含错误码表格、分组信息和多语言消息，方便前后端团队查阅。

#### 验收标准

1. THE DocGenerator SHALL 生成包含错误码总览表格的 Markdown 文档，表格列包括：错误名称、数字码、HTTP 状态码、默认语言消息
2. THE DocGenerator SHALL 按错误码数字范围自动分组（如 1000-1999 认证错误、4000-4999 参数错误），每组生成独立的二级标题和表格
3. THE DocGenerator SHALL 为每个错误码生成详细的多语言消息列表，展示所有支持语言的消息内容
4. THE DocGenerator SHALL 在文档头部生成元信息，包括生成时间、源 YAML 文件路径、支持的语言列表
5. THE DocGenerator SHALL 支持通过命令行参数或 API 指定输出文件路径，默认输出到 `error_codes.md`

### 需求 7: 文档生成的 YAML 解析与输出往返一致性

**用户故事:** 作为开发者，我希望文档生成器的 YAML 解析是准确可靠的，解析后的结构化数据能完整反映原始 YAML 配置的内容。

#### 验收标准

1. FOR ALL 有效的 YAML_Config 文件，THE DocGenerator 解析后的结构化数据 SHALL 包含与原始 YAML 完全一致的错误码数量
2. FOR ALL 有效的 YAML_Config 中的错误码条目，THE DocGenerator 解析后的每个 ErrorDefinition SHALL 包含与原始 YAML 一致的 code、http_status 和所有语言消息
3. FOR ALL 有效的 YAML_Config 文件，THE DocGenerator 生成的 Markdown 文档中的错误码条目数量 SHALL 等于 YAML 中定义的错误码数量（往返一致性）

### 需求 8: Feature Flag 管理

**用户故事:** 作为库使用者，我希望新功能通过独立的 feature flag 控制，按需启用，不增加不必要的依赖和编译时间。

#### 验收标准

1. THE biz-error 库 SHALL 新增 `runtime-messages` feature flag，启用后引入 `tokio`（用于异步运行时）依赖
2. THE biz-error 库 SHALL 新增 `doc-gen` feature flag，启用后引入文档生成相关依赖
3. THE `default` feature 列表 SHALL 保持为 `["axum"]`，不包含 `runtime-messages` 和 `doc-gen`
4. WHEN 仅使用默认 feature 时，THE biz-error 库 SHALL 不引入任何 v0.2 新增的依赖项
5. THE `runtime-messages` 和 `doc-gen` feature SHALL 可独立启用，互不依赖

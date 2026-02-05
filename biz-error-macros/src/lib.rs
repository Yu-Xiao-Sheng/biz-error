// 📦 过程宏模块 - 在编译时生成错误码
//
// 使用过程宏可以让用户完全不需要 build.rs，
// 只需在模块上添加属性即可自动生成代码

use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, ItemMod};

/// 从 YAML 配置自动生成错误码的属性宏
///
/// # 使用方式
///
/// ```rust,ignore
/// use biz_error::generate_error_codes;
///
/// #[generate_error_codes("biz_errors.yaml")]
/// mod error_codes;
/// ```
///
/// # 完整示例
///
/// **Cargo.toml:**
/// ```toml
/// [dependencies]
/// biz-error = { version = "0.1", features = ["codegen"] }
/// ```
///
/// **main.rs / lib.rs:**
/// ```rust,ignore
/// use biz_error::generate_error_codes;
///
/// #[generate_error_codes("biz_errors.yaml")]
/// mod error_codes;
///
/// use error_codes::ErrorCode;
/// ```
#[proc_macro_attribute]
pub fn generate_error_codes(args: TokenStream, input: TokenStream) -> TokenStream {
    // 解析属性参数（YAML 文件路径）
    let yaml_path = match syn::parse::<LitStr>(args) {
        Ok(path) => path.value(),
        Err(e) => {
            let error = format!("Invalid YAML file path: {}", e);
            return quote!(compile_error!(#error)).into();
        }
    };

    // 解析输入的模块声明以获取模块名称
    let module = match syn::parse::<ItemMod>(input) {
        Ok(mod_item) => mod_item,
        Err(e) => {
            let error = format!("Invalid module declaration: {}", e);
            return quote!(compile_error!(#error)).into();
        }
    };

    let module_name = module.ident.clone();

    // 在编译时生成代码
    let expanded = match generate_error_codes_impl(&yaml_path) {
        Ok(code) => code,
        Err(e) => {
            let error_msg = format!("Failed to generate error codes from '{}': {}", yaml_path, e);
            return quote!(compile_error!(#error_msg)).into();
        }
    };

    // 将生成的代码包装在模块中
    let result = quote! {
        mod #module_name {
            #expanded
        }
    };

    result.into()
}

/// 在编译时从 YAML 生成错误码代码
fn generate_error_codes_impl(yaml_path: &str) -> Result<proc_macro2::TokenStream, Box<dyn std::error::Error>> {
    // 读取 YAML 文件
    let yaml_content = std::fs::read_to_string(yaml_path)?;

    // 解析 YAML
    let config: serde_yaml::Value = serde_yaml::from_str(&yaml_content)?;

    let errors = config["errors"].as_mapping()
        .ok_or("Missing 'errors' section in YAML")?;

    let default_lang = config["default_language"]
        .as_str()
        .unwrap_or("en");

    let mut enum_variants = Vec::new();
    let mut code_match_arms = Vec::new();
    let mut message_match_arms = Vec::new();
    let mut http_status_match_arms = Vec::new();
    let mut variant_names = Vec::new();

    for (key, value) in errors {
        let name = key.as_str().ok_or("Error key must be a string")?;
        let code = value["code"].as_i64().ok_or("Missing 'code' field")?;
        let code_i32 = code as i32;
        let http_status = value["http_status"].as_i64().unwrap_or(500);
        let http_status_u16 = http_status as u16;
        let messages = value["message"].as_mapping()
            .ok_or("Missing 'message' field")?;

        // 转换为 PascalCase
        let enum_name = to_pascal_case(name);
        variant_names.push(enum_name.clone());

        // 生成枚举变体
        let doc_msg = messages
            .get(serde_yaml::Value::String(default_lang.to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        enum_variants.push(quote! {
            #[doc = #doc_msg]
            #enum_name,
        });

        // 生成 code() 方法分支
        code_match_arms.push(quote! {
            ErrorCode::#enum_name => #code_i32,
        });

        // 生成 message() 方法分支
        for (lang, msg) in messages {
            let lang_str = lang.as_str().ok_or("Language key must be a string")?;
            let msg_str = msg.as_str().ok_or("Message value must be a string")?;
            message_match_arms.push(quote! {
                (ErrorCode::#enum_name, #lang_str) => #msg_str,
            });
        }

        let fallback_msg = messages
            .get(serde_yaml::Value::String(default_lang.to_string()))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        message_match_arms.push(quote! {
            (ErrorCode::#enum_name, _) => #fallback_msg,
        });

        // 生成 http_status() 方法分支
        http_status_match_arms.push(quote! {
            ErrorCode::#enum_name => ::axum::http::StatusCode::from_u16(#http_status_u16).unwrap(),
        });
    }

    let all_constants = variant_names.iter()
        .map(|v| quote! { ErrorCode::#v })
        .collect::<Vec<_>>();

    // 生成完整的错误码模块
    Ok(quote! {
        // 导入 ErrorCode trait
        use ::biz_error::ErrorCode as ErrorCodeTrait;

        // 自动生成的业务错误码枚举
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ErrorCode {
            #(#enum_variants)*
        }

        // 实现 biz_error::ErrorCode trait
        impl ::biz_error::ErrorCode for ErrorCode {
            fn code(&self) -> i32 {
                match self {
                    #(#code_match_arms)*
                }
            }

            fn message(&self) -> &'static str {
                self.message_lang(#default_lang)
            }

            fn message_lang(&self, lang: &str) -> &'static str {
                match (self, lang.as_ref()) {
                    #(#message_match_arms)*
                }
            }

            fn http_status(&self) -> ::axum::http::StatusCode {
                match self {
                    #(#http_status_match_arms)*
                }
            }
        }

        impl ::std::fmt::Display for ErrorCode {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "[{}] {}", self.code(), self.message())
            }
        }

        impl ::std::error::Error for ErrorCode {}

        /// 所有错误码常量列表
        pub const ALL_ERROR_CODES: &[ErrorCode] = &[#(#all_constants),*];
    })
}

/// 将 snake_case 转换为 PascalCase
fn to_pascal_case(s: &str) -> proc_macro2::Ident {
    let converted: String = s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + chars.as_str()
                }
            }
        })
        .collect();

    proc_macro2::Ident::new(&converted, proc_macro2::Span::call_site())
}

//! 业务翻译资源注册表。
//!
//! 通用业务翻译 API 只能操作已注册的资源类型和字段，避免调用方把翻译表当成任意 KV 表使用。
//! 后续接入权限系统和业务资源存在性校验时，也应从这里扩展资源级策略。

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    context::request_context::RequestContext,
    error::app_error::{AppError, AppResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessTranslationPolicy {
    /// 业务资源类型，例如 `form_definition`。
    resource_type: String,
    /// 允许写入翻译的字段白名单。
    allowed_fields: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BusinessTranslationRegistry {
    /// 资源类型到翻译策略的映射。
    policies: BTreeMap<String, BusinessTranslationPolicy>,
}

impl BusinessTranslationPolicy {
    /// 创建业务翻译策略。
    pub fn new<I, S>(resource_type: impl Into<String>, allowed_fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            resource_type: resource_type.into(),
            allowed_fields: allowed_fields
                .into_iter()
                .map(Into::into)
                .collect::<BTreeSet<_>>(),
        }
    }

    /// 返回资源类型。
    pub fn resource_type(&self) -> &str {
        &self.resource_type
    }

    /// 判断字段是否允许被通用翻译 API 操作。
    pub fn allows_field(&self, field_name: &str) -> bool {
        self.allowed_fields.contains(field_name)
    }

    /// 返回允许翻译的字段列表。
    pub fn allowed_fields(&self) -> Vec<String> {
        self.allowed_fields.iter().cloned().collect()
    }

    /// 预留业务资源存在性校验扩展点。
    ///
    /// 首版尚未接入具体业务模块 repository，因此默认通过；后续模块可以把这里替换为真实策略。
    pub fn resource_exists(&self, _resource_id: &str) -> AppResult<()> {
        Ok(())
    }

    /// 预留读取权限校验扩展点。
    ///
    /// 当前没有统一权限系统，先保留方法边界，避免后续把权限判断散落到 handler。
    pub fn check_read(&self, _ctx: &RequestContext, _resource_id: &str) -> AppResult<()> {
        Ok(())
    }

    /// 预留写入权限校验扩展点。
    ///
    /// 后续接入权限系统后，应在这里按资源类型复用原业务资源的写权限。
    pub fn check_write(&self, _ctx: &RequestContext, _resource_id: &str) -> AppResult<()> {
        Ok(())
    }
}

impl BusinessTranslationRegistry {
    /// 创建空注册表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建首版默认注册表。
    ///
    /// 当前先注册低代码平台最常见的配置型资源，后续业务模块落地时可以继续补充自己的策略。
    pub fn with_default_resources() -> Self {
        let mut registry = Self::new();
        registry.register(BusinessTranslationPolicy::new(
            "form_definition",
            ["name", "description", "display_name"],
        ));
        registry.register(BusinessTranslationPolicy::new(
            "field_definition",
            ["name", "description", "display_name", "label", "help_text"],
        ));
        registry.register(BusinessTranslationPolicy::new(
            "dictionary_item",
            ["name", "description", "display_name", "label"],
        ));
        registry.register(BusinessTranslationPolicy::new(
            "workflow_node",
            ["name", "description", "display_name", "label"],
        ));
        registry
    }

    /// 注册或覆盖资源策略。
    pub fn register(&mut self, policy: BusinessTranslationPolicy) {
        self.policies
            .insert(policy.resource_type().to_string(), policy);
    }

    /// 获取资源策略。
    pub fn policy(&self, resource_type: &str) -> Option<&BusinessTranslationPolicy> {
        self.policies.get(resource_type)
    }

    /// 校验资源类型是否已注册。
    pub fn validate_resource(&self, resource_type: &str) -> AppResult<&BusinessTranslationPolicy> {
        let normalized = normalize_resource_type(resource_type)?;
        self.policy(&normalized).ok_or_else(|| {
            AppError::param_invalid(format!("不支持的业务翻译资源类型: {resource_type}"))
        })
    }

    /// 校验字段集合并返回去重后的稳定字段列表。
    pub fn validate_fields<I, S>(&self, resource_type: &str, fields: I) -> AppResult<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let policy = self.validate_resource(resource_type)?;
        let mut normalized_fields = BTreeSet::new();

        for field in fields {
            let field_name = normalize_field_name(field.as_ref())?;
            if !policy.allows_field(&field_name) {
                return Err(AppError::param_invalid(format!(
                    "资源 {resource_type} 不支持字段 {field_name} 的多语言翻译"
                )));
            }
            normalized_fields.insert(field_name);
        }

        if normalized_fields.is_empty() {
            return Err(AppError::param_invalid("业务翻译字段不能为空"));
        }

        Ok(normalized_fields.into_iter().collect())
    }

    /// 校验写入字段映射。
    pub fn validate_field_map<I, S>(&self, resource_type: &str, fields: I) -> AppResult<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.validate_fields(resource_type, fields)
    }
}

/// 规范化资源类型。
///
/// 资源类型作为 API path 和数据库过滤条件，只允许 snake_case 风格安全字符。
pub fn normalize_resource_type(value: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if is_valid_identifier(trimmed) {
        Ok(trimmed.to_string())
    } else {
        Err(AppError::param_invalid(format!(
            "非法业务翻译资源类型: {value}"
        )))
    }
}

/// 规范化字段名。
///
/// 字段名必须稳定且可审计，禁止空格、路径分隔符或动态表达式进入翻译表。
pub fn normalize_field_name(value: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if is_valid_identifier(trimmed) {
        Ok(trimmed.to_string())
    } else {
        Err(AppError::param_invalid(format!(
            "非法业务翻译字段名: {value}"
        )))
    }
}

fn is_valid_identifier(value: &str) -> bool {
    let len = value.len();
    (1..=64).contains(&len)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::{BusinessTranslationRegistry, normalize_field_name, normalize_resource_type};

    #[test]
    fn registry_rejects_unregistered_resource() {
        // 未注册资源类型不能通过通用 API 写入，防止调用方绕过业务模块边界。
        let registry = BusinessTranslationRegistry::with_default_resources();

        assert!(registry.validate_resource("unknown_resource").is_err());
    }

    #[test]
    fn registry_rejects_field_outside_whitelist() {
        // 字段白名单是通用 API 的最小安全边界，不能允许任意列名进入翻译表。
        let registry = BusinessTranslationRegistry::with_default_resources();

        assert!(
            registry
                .validate_fields("form_definition", ["name", "secret"])
                .is_err()
        );
    }

    #[test]
    fn identifiers_must_use_safe_snake_case() {
        // 资源类型和字段名会进入 SQL 参数、日志和审计上下文，需要统一安全形态。
        assert!(normalize_resource_type("form_definition").is_ok());
        assert!(normalize_resource_type("FormDefinition").is_err());
        assert!(normalize_field_name("display_name").is_ok());
        assert!(normalize_field_name("displayName").is_err());
    }
}

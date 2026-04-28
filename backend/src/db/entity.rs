//! 持久化实体基础字段。
//!
//! Rust 没有继承式基础类，业务实体应通过组合 `BaseFields` 复用统一基础字段，并通过 trait
//! 显式暴露基础字段访问入口，避免各模块自行定义不一致的审计、乐观锁和逻辑删除字段。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{context::request_context::RequestContext, utils::time::now_utc};

/// 匿名请求写入基础字段时使用的操作者标识。
pub const ANONYMOUS_OPERATOR: &str = "anonymous";

/// 系统任务写入基础字段时建议使用的操作者标识。
pub const SYSTEM_OPERATOR: &str = "system";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct BaseFields {
    /// 数据版本号，用于乐观锁、并发控制或变更检测。
    pub version: i64,
    /// 逻辑删除标记，用于区分有效数据与已删除数据。
    pub deleted: bool,
    /// 创建人标识。
    pub created_by: String,
    /// 创建时间，统一使用 UTC。
    pub created_time: DateTime<Utc>,
    /// 最后更新人标识。
    pub updated_by: String,
    /// 最后更新时间，统一使用 UTC。
    pub updated_time: DateTime<Utc>,
    /// 删除人标识，未删除时为空。
    pub deleted_by: Option<String>,
    /// 删除时间，未删除时为空。
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 持久化实体暴露基础字段的只读 trait。
pub trait HasBaseFields {
    /// 返回实体基础字段。
    fn base_fields(&self) -> &BaseFields;
}

/// 持久化实体暴露基础字段的可变 trait。
pub trait HasBaseFieldsMut: HasBaseFields {
    /// 返回实体基础字段的可变引用。
    fn base_fields_mut(&mut self) -> &mut BaseFields;
}

impl BaseFields {
    /// 构造新增实体默认基础字段。
    ///
    /// 新增数据固定从 `version = 1` 和 `deleted = false` 开始，创建人与更新人保持一致。
    pub fn new_for_create(operator: impl Into<String>) -> Self {
        let operator = normalize_operator(operator);
        let now = now_utc();
        Self {
            version: 1,
            deleted: false,
            created_by: operator.clone(),
            created_time: now,
            updated_by: operator,
            updated_time: now,
            deleted_by: None,
            deleted_time: None,
        }
    }
}

/// 从请求上下文解析写入基础字段使用的操作者。
///
/// 首版未接入完整鉴权时，匿名请求统一写入 `anonymous`，避免基础字段为空字符串。
pub fn operator_from_context(ctx: &RequestContext) -> String {
    ctx.user_id
        .as_deref()
        .map(str::trim)
        .filter(|user_id| !user_id.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| ANONYMOUS_OPERATOR.to_string())
}

fn normalize_operator(operator: impl Into<String>) -> String {
    let operator = operator.into();
    let trimmed = operator.trim();
    if trimmed.is_empty() {
        ANONYMOUS_OPERATOR.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::context::request_context::RequestContext;

    use super::{BaseFields, operator_from_context};

    #[test]
    fn base_fields_serializes_to_camel_case() {
        // 对外 JSON 字段必须保持 camelCase，数据库和 Rust 持久化字段仍使用 snake_case。
        let base = BaseFields::new_for_create("user_1");
        let value = serde_json::to_value(base).expect("基础字段必须可序列化");

        assert_eq!(value["version"], json!(1));
        assert_eq!(value["deleted"], json!(false));
        assert_eq!(value["createdBy"], json!("user_1"));
        assert_eq!(value["updatedBy"], json!("user_1"));
        assert!(value.get("created_by").is_none());
    }

    #[test]
    fn operator_from_context_uses_user_id_when_available() {
        // 已认证请求必须把当前用户写入审计基础字段。
        let mut ctx = RequestContext::anonymous("trace_12345678");
        ctx.user_id = Some("user_1".to_string());

        assert_eq!(operator_from_context(&ctx), "user_1");
    }

    #[test]
    fn operator_from_context_falls_back_to_anonymous() {
        // 匿名请求不能留下空操作者，否则后续审计和排查无法判断写入来源。
        let ctx = RequestContext::anonymous("trace_12345678");

        assert_eq!(operator_from_context(&ctx), "anonymous");
    }
}

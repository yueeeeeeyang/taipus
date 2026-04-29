//! 业务数据多语言模型。
//!
//! 当前底座先定义稳定数据结构和接口契约，具体业务模块后续通过统一业务翻译表按资源维度读写。

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessTranslation {
    /// 翻译记录主键，独立于业务资源主键。
    pub id: String,
    /// 业务资源类型，例如 `form_definition` 或 `dictionary_item`。
    pub resource_type: String,
    /// 业务资源主键。
    pub resource_id: String,
    /// 被翻译字段名，例如 `name` 或 `description`。
    pub field_name: String,
    /// 翻译 locale，例如 `zh-CN` 或 `en-US`。
    pub locale: String,
    /// 翻译文本。
    pub text_value: String,
    /// 业务翻译版本号，用于资源级乐观锁和变更检测。
    pub version: i64,
    /// 逻辑删除标记，查询 active 翻译时必须过滤为 `false`。
    pub deleted: bool,
    /// 创建人标识，首版未登录场景统一写入 `anonymous`。
    pub created_by: String,
    /// 创建时间，持久化和接口均保持 UTC 时间。
    pub created_time: DateTime<Utc>,
    /// 最后更新人标识。
    pub updated_by: String,
    /// 最后更新时间，持久化和接口均保持 UTC 时间。
    pub updated_time: DateTime<Utc>,
    /// 删除人标识，未删除记录保持为空。
    pub deleted_by: Option<String>,
    /// 删除时间，未删除记录保持为空。
    pub deleted_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedText<T>
where
    T: Serialize,
{
    /// 当前实际返回值。
    pub value: T,
    /// 调用方请求的 locale。
    pub requested_locale: Option<String>,
    /// 实际命中的 locale。
    pub locale: String,
    /// 是否因为目标语言缺失而使用 fallback。
    pub translation_missing: bool,
}

/// 业务翻译字段集合。
///
/// 第一层 key 是业务字段名，第二层 key 是 locale，value 是该 locale 下的翻译文本。
pub type BusinessTranslationFieldMap = BTreeMap<String, BTreeMap<String, String>>;

/// 业务模块批量本地化请求。
#[derive(Debug, Clone)]
pub struct BusinessTranslationLocalizeRequest {
    /// 业务资源类型，例如 `form_definition`。
    pub resource_type: String,
    /// 业务资源主键。
    pub resource_id: String,
    /// 需要本地化的字段名。
    pub field_name: String,
    /// 业务主表原始值，作为最终 fallback。
    pub default_value: String,
}

impl BusinessTranslationLocalizeRequest {
    /// 构造批量本地化请求。
    pub fn new(
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        field_name: impl Into<String>,
        default_value: impl Into<String>,
    ) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            field_name: field_name.into(),
            default_value: default_value.into(),
        }
    }
}

/// 业务翻译写入命令。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessTranslationWriteCommand {
    /// 资源级乐观锁版本，必须等于当前 active 翻译最大版本；新资源传 0。
    pub version: i64,
    /// 需要覆盖写入的字段翻译集合。
    pub fields: BusinessTranslationFieldMap,
}

/// 业务翻译读取响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessTranslationReadResponse {
    /// 业务资源类型。
    pub resource_type: String,
    /// 业务资源主键。
    pub resource_id: String,
    /// 当前 active 翻译最大版本。
    pub version: i64,
    /// 按字段和 locale 分组后的翻译文本。
    pub fields: BusinessTranslationFieldMap,
}

/// 业务翻译写入响应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessTranslationWriteResponse {
    /// 业务资源类型。
    pub resource_type: String,
    /// 业务资源主键。
    pub resource_id: String,
    /// 写入后的 active 翻译最大版本。
    pub version: i64,
    /// 本次写入覆盖后的字段翻译集合。
    pub fields: BusinessTranslationFieldMap,
}

/// repository 查询返回的轻量翻译记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusinessTranslationValue {
    /// 业务资源主键。
    pub resource_id: String,
    /// 被翻译字段名。
    pub field_name: String,
    /// 翻译 locale。
    pub locale: String,
    /// 翻译文本。
    pub text_value: String,
    /// 翻译版本号。
    pub version: i64,
}

#[cfg(test)]
mod tests {
    use super::BusinessTranslation;
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    #[test]
    fn business_translation_serializes_base_fields_as_camel_case() {
        // 业务模型字段在 Rust 内部保持 snake_case，对外 JSON 必须稳定输出 camelCase。
        let translation = BusinessTranslation {
            id: "translation_1".to_string(),
            resource_type: "form_definition".to_string(),
            resource_id: "form_1".to_string(),
            field_name: "name".to_string(),
            locale: "zh-CN".to_string(),
            text_value: "客户登记".to_string(),
            version: 1,
            deleted: false,
            created_by: "user_1".to_string(),
            created_time: Utc
                .with_ymd_and_hms(2026, 4, 28, 0, 0, 0)
                .single()
                .expect("测试时间必须合法"),
            updated_by: "user_1".to_string(),
            updated_time: Utc
                .with_ymd_and_hms(2026, 4, 28, 0, 0, 1)
                .single()
                .expect("测试时间必须合法"),
            deleted_by: None,
            deleted_time: None,
        };

        let value = serde_json::to_value(translation).expect("业务翻译必须可以序列化");

        assert_eq!(value["createdBy"], json!("user_1"));
        assert_eq!(value["updatedBy"], json!("user_1"));
        assert_eq!(value["deletedBy"], json!(null));
        assert!(value.get("created_by").is_none());
    }
}

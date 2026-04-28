//! 业务数据多语言模型。
//!
//! 当前底座先定义稳定数据结构和接口契约，具体业务模块后续通过统一业务翻译表按资源维度读写。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::db::entity::{BaseFields, HasBaseFields, HasBaseFieldsMut};

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
    /// 统一基础字段，对外 JSON 通过 flatten 保持 `version/createdBy` 等字段平铺。
    #[serde(flatten)]
    pub base: BaseFields,
}

impl HasBaseFields for BusinessTranslation {
    /// 返回业务翻译的统一基础字段。
    fn base_fields(&self) -> &BaseFields {
        &self.base
    }
}

impl HasBaseFieldsMut for BusinessTranslation {
    /// 返回业务翻译的统一基础字段可变引用。
    fn base_fields_mut(&mut self) -> &mut BaseFields {
        &mut self.base
    }
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

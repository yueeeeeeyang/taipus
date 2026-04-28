//! 业务数据多语言模型。
//!
//! 当前底座先定义稳定数据结构和接口契约，具体业务模块后续通过统一业务翻译表按资源维度读写。

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
    /// 乐观锁版本号。
    pub version: i64,
    /// 逻辑删除标记。
    pub deleted: bool,
    /// 创建人标识。
    pub created_by: String,
    /// 创建时间。
    pub created_time: DateTime<Utc>,
    /// 最后更新人标识。
    pub updated_by: String,
    /// 最后更新时间。
    pub updated_time: DateTime<Utc>,
    /// 删除人标识。
    pub deleted_by: Option<String>,
    /// 删除时间。
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

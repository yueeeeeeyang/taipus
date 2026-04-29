//! 示例持久化模型占位。
//!
//! 真实业务实体必须包含 version、deleted、created_by、created_time、updated_by、updated_time、
//! deleted_by、deleted_time 等统一基础字段。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ExampleEntity {
    /// 示例实体主键。
    pub id: String,
    /// 示例实体名称。
    pub name: String,
    /// 示例实体版本号，业务更新时必须通过 `WHERE version = ?` 做乐观锁校验。
    pub version: i64,
    /// 逻辑删除标记，默认查询必须过滤为 `false`。
    pub deleted: bool,
    /// 创建人标识。
    pub created_by: String,
    /// 创建时间，统一保存 UTC 时间。
    pub created_time: DateTime<Utc>,
    /// 最后更新人标识。
    pub updated_by: String,
    /// 最后更新时间，统一保存 UTC 时间。
    pub updated_time: DateTime<Utc>,
    /// 删除人标识，未删除时为空。
    pub deleted_by: Option<String>,
    /// 删除时间，未删除时为空。
    pub deleted_time: Option<DateTime<Utc>>,
}

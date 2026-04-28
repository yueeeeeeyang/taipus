//! 示例持久化模型占位。
//!
//! 真实业务实体必须包含 version、deleted、created_by、created_time、updated_by、updated_time、
//! deleted_by、deleted_time 等统一基础字段。

use serde::{Deserialize, Serialize};

use crate::db::entity::{BaseFields, HasBaseFields, HasBaseFieldsMut};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ExampleEntity {
    /// 示例实体主键。
    pub id: String,
    /// 示例实体名称。
    pub name: String,
    /// 统一基础字段，业务实体通过组合而不是继承复用基础字段。
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub base: BaseFields,
}

impl HasBaseFields for ExampleEntity {
    /// 返回示例实体基础字段。
    fn base_fields(&self) -> &BaseFields {
        &self.base
    }
}

impl HasBaseFieldsMut for ExampleEntity {
    /// 返回示例实体基础字段可变引用。
    fn base_fields_mut(&mut self) -> &mut BaseFields {
        &mut self.base
    }
}

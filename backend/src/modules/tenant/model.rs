//! 租户持久化模型。
//!
//! 租户是平台级主数据，本模型显式平铺基础字段，便于审查乐观锁、逻辑删除和审计语义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::app_error::{AppError, AppResult};

/// 租户状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TenantStatus {
    /// 正常可用。
    Enabled,
    /// 管理员禁用。
    Disabled,
    /// 欠费、风控或运维原因暂停。
    Suspended,
}

impl TenantStatus {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Suspended => "suspended",
        }
    }
}

impl TryFrom<&str> for TenantStatus {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            "suspended" => Ok(Self::Suspended),
            _ => Err(AppError::param_invalid(
                "租户状态只允许 enabled、disabled 或 suspended",
            )),
        }
    }
}

/// 租户隔离模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TenantIsolationMode {
    /// 共享库共享表，通过 tenant_id 隔离。
    SharedSchema,
    /// 共享数据库、每租户独立 schema，首版仅预留。
    SchemaPerTenant,
    /// 每租户独立数据库，首版仅预留。
    DatabasePerTenant,
}

impl TenantIsolationMode {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SharedSchema => "shared_schema",
            Self::SchemaPerTenant => "schema_per_tenant",
            Self::DatabasePerTenant => "database_per_tenant",
        }
    }
}

impl TryFrom<&str> for TenantIsolationMode {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "shared_schema" => Ok(Self::SharedSchema),
            "schema_per_tenant" => Ok(Self::SchemaPerTenant),
            "database_per_tenant" => Ok(Self::DatabasePerTenant),
            _ => Err(AppError::param_invalid(
                "租户隔离模式只允许 shared_schema、schema_per_tenant 或 database_per_tenant",
            )),
        }
    }
}

/// 租户持久化实体。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    /// 租户主键，也是业务表中的 tenant_id。
    pub id: String,
    /// 租户编码，全局未删除数据唯一。
    pub tenant_code: String,
    /// 租户名称。
    pub name: String,
    /// 租户名称全拼，用于搜索。
    pub name_full_pinyin: String,
    /// 租户名称简拼，用于搜索。
    pub name_simple_pinyin: String,
    /// 租户状态。
    pub status: String,
    /// 隔离模式，首版只允许 shared_schema 写入。
    pub isolation_mode: String,
    /// 租户主域名。
    pub primary_domain: Option<String>,
    /// 备注。
    pub remark: Option<String>,
    /// 数据版本号，用于乐观锁。
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

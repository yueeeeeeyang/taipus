//! 租户接口 DTO。
//!
//! DTO 只表达 HTTP 契约，字段使用 camelCase；拼音字段由 service 根据名称生成，不信任前端输入。

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::response::page::PageQuery;

/// 租户逻辑删除请求 query。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionQuery {
    /// 调用方读取到的数据版本号，用于乐观锁删除。
    pub version: i64,
}

/// 租户新增请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantRequest {
    /// 租户编码，全局未删除数据唯一。
    pub tenant_code: String,
    /// 租户名称。
    pub name: String,
    /// 隔离模式，首版只允许 shared_schema。
    pub isolation_mode: String,
    /// 租户主域名。
    pub primary_domain: Option<String>,
    /// 备注。
    pub remark: Option<String>,
}

/// 租户修改请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTenantRequest {
    /// 乐观锁版本号。
    pub version: i64,
    /// 租户编码。
    pub tenant_code: String,
    /// 租户名称。
    pub name: String,
    /// 隔离模式，首版只允许 shared_schema。
    pub isolation_mode: String,
    /// 租户主域名。
    pub primary_domain: Option<String>,
    /// 备注。
    pub remark: Option<String>,
}

/// 租户状态修改请求。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TenantStatusRequest {
    /// 乐观锁版本号。
    pub version: i64,
}

/// 租户分页查询。
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TenantPageQuery {
    /// 通用分页参数。
    #[serde(flatten)]
    pub page: PageQuery,
    /// 租户编码模糊查询。
    pub tenant_code: Option<String>,
    /// 名称、全拼、简拼模糊查询。
    pub name: Option<String>,
    /// 租户状态精确查询。
    pub status: Option<String>,
    /// 隔离模式精确查询。
    pub isolation_mode: Option<String>,
    /// 主域名模糊查询。
    pub primary_domain: Option<String>,
    /// 创建时间起点。
    pub created_time_start: Option<DateTime<Utc>>,
    /// 创建时间终点。
    pub created_time_end: Option<DateTime<Utc>>,
    /// 更新时间起点。
    pub updated_time_start: Option<DateTime<Utc>>,
    /// 更新时间终点。
    pub updated_time_end: Option<DateTime<Utc>>,
}

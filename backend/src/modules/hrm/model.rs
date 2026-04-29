//! HRM 持久化模型。
//!
//! 本模块只定义数据库实体和稳定枚举，不承载业务校验。所有持久化实体都显式平铺基础字段，
//! 便于 SQL、模型和接口契约逐项审查，避免通用实体抽象隐藏关键写入语义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::app_error::{AppError, AppResult};

/// HRM 主数据状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HrmStatus {
    /// 可用状态，可以作为新增任职关系或候选数据。
    Enabled,
    /// 禁用状态，历史关系保留，但不得继续作为新增关系候选。
    Disabled,
}

impl HrmStatus {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

impl TryFrom<&str> for HrmStatus {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            _ => Err(AppError::param_invalid(
                "HRM 状态只允许 enabled 或 disabled",
            )),
        }
    }
}

/// 组织机构类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OrgType {
    /// 分部，可以包含下级分部和部门。
    Branch,
    /// 部门，只能包含下级部门。
    Department,
}

impl OrgType {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Branch => "branch",
            Self::Department => "department",
        }
    }
}

impl TryFrom<&str> for OrgType {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "branch" => Ok(Self::Branch),
            "department" => Ok(Self::Department),
            _ => Err(AppError::param_invalid(
                "组织类型只允许 branch 或 department",
            )),
        }
    }
}

/// 用户持久化实体。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HrmUser {
    /// 用户主键。
    pub id: String,
    /// 租户标识，所有 HRM 查询和写入都必须携带。
    pub tenant_id: String,
    /// 工号，租户内未删除数据唯一。
    pub employee_no: String,
    /// 用户姓名。
    pub name: String,
    /// 姓名全拼，用于拼音搜索。
    pub name_full_pinyin: String,
    /// 姓名简拼，用于拼音搜索。
    pub name_simple_pinyin: String,
    /// 手机号，首版不作为登录账号。
    pub mobile: Option<String>,
    /// 邮箱，首版不作为登录账号。
    pub email: Option<String>,
    /// 用户排序号，数值越小越靠前。
    pub sort_no: i64,
    /// 用户状态，数据库值为 `enabled` 或 `disabled`。
    pub status: String,
    /// 数据版本号，用于乐观锁。
    pub version: i64,
    /// 逻辑删除标记。
    pub deleted: bool,
    /// 创建人标识。
    pub created_by: String,
    /// 创建时间，统一保存 UTC 时间。
    pub created_time: DateTime<Utc>,
    /// 最后更新人标识。
    pub updated_by: String,
    /// 最后更新时间，统一保存 UTC 时间。
    pub updated_time: DateTime<Utc>,
    /// 删除人标识。
    pub deleted_by: Option<String>,
    /// 删除时间。
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 组织机构持久化实体。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HrmOrg {
    /// 组织主键。
    pub id: String,
    /// 租户标识。
    pub tenant_id: String,
    /// 父组织 ID，根分部为空。
    pub parent_id: Option<String>,
    /// 组织编码，租户内未删除数据唯一。
    pub org_code: String,
    /// 组织名称。
    pub name: String,
    /// 组织名称全拼。
    pub name_full_pinyin: String,
    /// 组织名称简拼。
    pub name_simple_pinyin: String,
    /// 组织类型，数据库值为 `branch` 或 `department`。
    pub org_type: String,
    /// 同级排序号。
    pub sort_no: i64,
    /// 组织状态。
    pub status: String,
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

/// 岗位持久化实体。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HrmPost {
    /// 岗位主键。
    pub id: String,
    /// 租户标识。
    pub tenant_id: String,
    /// 岗位编码，租户内未删除数据唯一。
    pub post_code: String,
    /// 岗位名称。
    pub name: String,
    /// 岗位名称全拼。
    pub name_full_pinyin: String,
    /// 岗位名称简拼。
    pub name_simple_pinyin: String,
    /// 排序号。
    pub sort_no: i64,
    /// 岗位状态。
    pub status: String,
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

/// 用户组织岗位关系持久化实体。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct HrmUserOrgPost {
    /// 关系主键。
    pub id: String,
    /// 租户标识。
    pub tenant_id: String,
    /// 用户 ID。
    pub user_id: String,
    /// 组织 ID。
    pub org_id: String,
    /// 岗位 ID。
    pub post_id: String,
    /// 是否主组织关系。
    pub primary_org: bool,
    /// 是否主岗位关系。
    pub primary_post: bool,
    /// 任职关系排序号。
    pub sort_no: i64,
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

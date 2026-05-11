//! 权限模块持久化模型。
//!
//! 所有持久化实体显式平铺基础字段，便于审查乐观锁、逻辑删除和审计语义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::app_error::{AppError, AppResult};

/// 权限资源类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionResourceType {
    /// 应用资源。
    Application,
    /// 菜单资源。
    Menu,
    /// 按钮资源。
    Button,
    /// 后端接口资源。
    Api,
}

impl PermissionResourceType {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Menu => "menu",
            Self::Button => "button",
            Self::Api => "api",
        }
    }
}

impl TryFrom<&str> for PermissionResourceType {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "application" => Ok(Self::Application),
            "menu" => Ok(Self::Menu),
            "button" => Ok(Self::Button),
            "api" => Ok(Self::Api),
            _ => Err(AppError::param_invalid(
                "资源类型只允许 application、menu、button 或 api",
            )),
        }
    }
}

/// 通用启用状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionStatus {
    /// 正常启用。
    Enabled,
    /// 管理员禁用。
    Disabled,
}

impl PermissionStatus {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

impl TryFrom<&str> for PermissionStatus {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            _ => Err(AppError::param_invalid("状态只允许 enabled 或 disabled")),
        }
    }
}

/// 授权效果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantEffect {
    /// 允许访问。
    Allow,
    /// 显式拒绝访问。
    Deny,
}

impl GrantEffect {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }
}

impl TryFrom<&str> for GrantEffect {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "allow" => Ok(Self::Allow),
            "deny" => Ok(Self::Deny),
            _ => Err(AppError::param_invalid("授权效果只允许 allow 或 deny")),
        }
    }
}

/// 应用资源。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PermissionApplication {
    pub id: String,
    pub tenant_id: Option<String>,
    pub app_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub platform: String,
    pub home_path: Option<String>,
    pub icon: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 菜单资源。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PermissionMenu {
    pub id: String,
    pub tenant_id: Option<String>,
    pub app_id: String,
    pub parent_id: Option<String>,
    pub menu_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub platform: String,
    pub route_path: String,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub visible: bool,
    pub keep_alive: bool,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 按钮资源。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PermissionButton {
    pub id: String,
    pub tenant_id: Option<String>,
    pub app_id: String,
    pub menu_id: String,
    pub button_code: String,
    pub name: String,
    pub action_key: String,
    pub button_type: String,
    pub icon: Option<String>,
    pub sort_no: i64,
    pub status: String,
    pub remark: Option<String>,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 接口资源。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PermissionApi {
    pub id: String,
    pub tenant_id: Option<String>,
    pub app_id: Option<String>,
    pub api_code: String,
    pub name: String,
    pub http_method: String,
    pub path_pattern: String,
    pub normalized_path: String,
    pub related_menu_id: Option<String>,
    pub related_button_id: Option<String>,
    pub public_access: bool,
    pub auth_required: bool,
    pub status: String,
    pub remark: Option<String>,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 角色。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRole {
    pub id: String,
    pub tenant_id: String,
    pub role_code: String,
    pub name: String,
    pub name_full_pinyin: String,
    pub name_simple_pinyin: String,
    pub role_type: String,
    pub status: String,
    pub sort_no: i64,
    pub remark: Option<String>,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 授权规则。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PermissionGrant {
    pub id: String,
    pub tenant_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub effect: String,
    pub grant_source: String,
    pub condition_type: Option<String>,
    pub condition_value: Option<String>,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 账号角色关系。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AccountRole {
    pub id: String,
    pub tenant_id: String,
    pub account_id: String,
    pub role_id: String,
    pub status: String,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

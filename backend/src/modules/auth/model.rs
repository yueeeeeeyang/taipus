//! 认证模块持久化模型。
//!
//! 所有实体显式平铺基础字段，便于审查逻辑删除、乐观锁和安全审计语义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::app_error::{AppError, AppResult};

/// 账号状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountStatus {
    /// 正常可登录。
    Enabled,
    /// 管理员禁用。
    Disabled,
    /// 登录失败过多或风控锁定。
    Locked,
    /// 密码已过期，只允许后续改密流程。
    PasswordExpired,
}

impl AccountStatus {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
            Self::Locked => "locked",
            Self::PasswordExpired => "password_expired",
        }
    }
}

impl TryFrom<&str> for AccountStatus {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            "locked" => Ok(Self::Locked),
            "password_expired" => Ok(Self::PasswordExpired),
            _ => Err(AppError::param_invalid(
                "账号状态只允许 enabled、disabled、locked 或 password_expired",
            )),
        }
    }
}

/// 账号租户关系状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountTenantStatus {
    /// 正常可访问租户。
    Enabled,
    /// 禁用该账号在租户下的访问关系。
    Disabled,
}

impl AccountTenantStatus {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

impl TryFrom<&str> for AccountTenantStatus {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            _ => Err(AppError::param_invalid(
                "账号租户关系状态只允许 enabled 或 disabled",
            )),
        }
    }
}

/// 刷新令牌状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RefreshTokenStatus {
    /// 正常可用。
    Active,
    /// 已轮换，旧令牌不可再用。
    Rotated,
    /// 已主动废弃。
    Revoked,
    /// 已过期。
    Expired,
    /// 疑似泄露或重放。
    Compromised,
}

impl RefreshTokenStatus {
    /// 返回数据库中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Rotated => "rotated",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
            Self::Compromised => "compromised",
        }
    }
}

impl TryFrom<&str> for RefreshTokenStatus {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "active" => Ok(Self::Active),
            "rotated" => Ok(Self::Rotated),
            "revoked" => Ok(Self::Revoked),
            "expired" => Ok(Self::Expired),
            "compromised" => Ok(Self::Compromised),
            _ => Err(AppError::param_invalid(
                "刷新令牌状态只允许 active、rotated、revoked、expired 或 compromised",
            )),
        }
    }
}

/// 登录客户端类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientType {
    /// Web PC 端。
    Pc,
    /// 移动端。
    Mobile,
    /// 系统集成预留。
    Api,
}

impl ClientType {
    /// 返回数据库和 JWT 中保存的稳定小写值。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pc => "pc",
            Self::Mobile => "mobile",
            Self::Api => "api",
        }
    }
}

impl TryFrom<&str> for ClientType {
    type Error = AppError;

    fn try_from(value: &str) -> AppResult<Self> {
        match value {
            "pc" => Ok(Self::Pc),
            "mobile" => Ok(Self::Mobile),
            "api" => Ok(Self::Api),
            _ => Err(AppError::param_invalid(
                "clientType 只允许 pc、mobile 或 api",
            )),
        }
    }
}

/// 账号实体。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub display_name_full_pinyin: String,
    pub display_name_simple_pinyin: String,
    pub password_hash: String,
    pub password_algo: String,
    pub password_updated_time: Option<DateTime<Utc>>,
    pub status: String,
    pub hrm_user_id: Option<String>,
    pub last_login_time: Option<DateTime<Utc>>,
    pub last_login_ip: Option<String>,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 账号租户关系实体。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AccountTenant {
    pub id: String,
    pub account_id: String,
    pub tenant_id: String,
    pub status: String,
    pub is_default: bool,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

/// 刷新令牌会话实体。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenSession {
    pub id: String,
    pub account_id: String,
    pub tenant_id: String,
    pub token_hash: String,
    pub token_family: String,
    pub status: String,
    pub client_type: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub expires_time: DateTime<Utc>,
    pub last_used_time: Option<DateTime<Utc>>,
    pub revoked_by: Option<String>,
    pub revoked_time: Option<DateTime<Utc>>,
    pub revoked_reason: Option<String>,
    pub version: i64,
    pub deleted: bool,
    pub created_by: String,
    pub created_time: DateTime<Utc>,
    pub updated_by: String,
    pub updated_time: DateTime<Utc>,
    pub deleted_by: Option<String>,
    pub deleted_time: Option<DateTime<Utc>>,
}

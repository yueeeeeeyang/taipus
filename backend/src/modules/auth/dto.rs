//! 认证模块接口 DTO。
//!
//! DTO 只表达 HTTP 契约，字段使用 camelCase；账号名称拼音和令牌安全字段由后端生成。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::response::page::PageQuery;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionQuery {
    pub version: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub tenant_id: Option<String>,
    pub client_type: String,
    pub device_id: Option<String>,
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: String,
    pub client_type: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchTenantRequest {
    pub tenant_id: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    pub refresh_expires_in: i64,
    pub token_type: String,
    pub tenant_id: String,
    pub account: AccountSummary,
    pub tenants: Vec<AccountTenantSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub hrm_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTenantSummary {
    pub id: String,
    pub account_id: String,
    pub tenant_id: String,
    pub tenant_name: Option<String>,
    pub status: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub status: String,
    pub hrm_user_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountRequest {
    pub version: i64,
    pub username: String,
    pub display_name: String,
    pub status: String,
    pub hrm_user_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatusRequest {
    pub version: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    pub version: i64,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountPageQuery {
    #[serde(flatten)]
    pub page: PageQuery,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub status: Option<String>,
    pub hrm_user_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountTenantRequest {
    pub account_id: String,
    pub tenant_id: String,
    pub status: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountTenantRequest {
    pub version: i64,
    pub status: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountTenantPageQuery {
    #[serde(flatten)]
    pub page: PageQuery,
    pub account_id: Option<String>,
    pub tenant_id: Option<String>,
    pub status: Option<String>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionPageQuery {
    #[serde(flatten)]
    pub page: PageQuery,
    pub account_id: Option<String>,
    pub tenant_id: Option<String>,
    pub status: Option<String>,
    pub client_type: Option<String>,
    pub expires_time_start: Option<DateTime<Utc>>,
    pub expires_time_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeSessionRequest {
    pub version: i64,
    pub reason: Option<String>,
}
